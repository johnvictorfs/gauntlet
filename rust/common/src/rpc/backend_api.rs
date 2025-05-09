use std::collections::HashMap;

use gauntlet_utils::channel::RequestError;
use gauntlet_utils::channel::RequestResult;
use gauntlet_utils_macros::boundary_gen;
use thiserror::Error;
use tonic::transport::Channel;
use tonic::Code;
use tonic::Request;

use crate::model::DownloadStatus;
use crate::model::EntrypointId;
use crate::model::KeyboardEventOrigin;
use crate::model::LocalSaveData;
use crate::model::PhysicalKey;
use crate::model::PhysicalShortcut;
use crate::model::PluginId;
use crate::model::PluginPreferenceUserData;
use crate::model::SearchResult;
use crate::model::SettingsEntrypoint;
use crate::model::SettingsEntrypointType;
use crate::model::SettingsGeneratedEntrypoint;
use crate::model::SettingsPlugin;
use crate::model::SettingsTheme;
use crate::model::UiPropertyValue;
use crate::model::UiSetupData;
use crate::model::UiWidgetId;
use crate::model::WindowPositionMode;
use crate::rpc::grpc::rpc_backend_client::RpcBackendClient;
use crate::rpc::grpc::RpcDownloadPluginRequest;
use crate::rpc::grpc::RpcDownloadStatus;
use crate::rpc::grpc::RpcDownloadStatusRequest;
use crate::rpc::grpc::RpcEntrypointTypeSettings;
use crate::rpc::grpc::RpcGetGlobalEntrypointShortcutRequest;
use crate::rpc::grpc::RpcGetGlobalShortcutRequest;
use crate::rpc::grpc::RpcGetThemeRequest;
use crate::rpc::grpc::RpcGetWindowPositionModeRequest;
use crate::rpc::grpc::RpcPingRequest;
use crate::rpc::grpc::RpcPluginsRequest;
use crate::rpc::grpc::RpcRemovePluginRequest;
use crate::rpc::grpc::RpcRunActionRequest;
use crate::rpc::grpc::RpcSaveLocalPluginRequest;
use crate::rpc::grpc::RpcSetEntrypointStateRequest;
use crate::rpc::grpc::RpcSetGlobalEntrypointShortcutRequest;
use crate::rpc::grpc::RpcSetGlobalShortcutRequest;
use crate::rpc::grpc::RpcSetPluginStateRequest;
use crate::rpc::grpc::RpcSetPreferenceValueRequest;
use crate::rpc::grpc::RpcSetThemeRequest;
use crate::rpc::grpc::RpcSetWindowPositionModeRequest;
use crate::rpc::grpc::RpcShortcut;
use crate::rpc::grpc::RpcShowSettingsWindowRequest;
use crate::rpc::grpc::RpcShowWindowRequest;
use crate::rpc::grpc_convert::plugin_preference_from_rpc;
use crate::rpc::grpc_convert::plugin_preference_user_data_from_rpc;
use crate::rpc::grpc_convert::plugin_preference_user_data_to_rpc;

#[allow(async_fn_in_trait)]
#[boundary_gen]
pub trait BackendForFrontendApi {
    async fn setup_data(&self) -> RequestResult<UiSetupData>;

    async fn setup_response(
        &self,
        global_shortcut_error: Option<String>,
        global_entrypoint_shortcuts_errors: HashMap<(PluginId, EntrypointId), Option<String>>,
    ) -> RequestResult<()>;

    async fn search(&self, text: String, render_inline_view: bool) -> RequestResult<Vec<SearchResult>>;

    async fn request_view_render(
        &self,
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
    ) -> RequestResult<HashMap<String, PhysicalShortcut>>;

    async fn request_view_close(&self, plugin_id: PluginId) -> RequestResult<()>;

    async fn request_run_command(&self, plugin_id: PluginId, entrypoint_id: EntrypointId) -> RequestResult<()>;

    async fn request_run_generated_entrypoint(
        &self,
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
        action_index: usize,
    ) -> RequestResult<()>;

    async fn send_view_event(
        &self,
        plugin_id: PluginId,
        widget_id: UiWidgetId,
        event_name: String,
        event_arguments: Vec<UiPropertyValue>,
    ) -> RequestResult<()>;

    async fn send_keyboard_event(
        &self,
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
        origin: KeyboardEventOrigin,
        key: PhysicalKey,
        modifier_shift: bool,
        modifier_control: bool,
        modifier_alt: bool,
        modifier_meta: bool,
    ) -> RequestResult<()>;

    async fn send_open_event(&self, plugin_id: PluginId, href: String) -> RequestResult<()>;

    async fn open_settings_window(&self) -> RequestResult<()>;

    async fn open_settings_window_preferences(
        &self,
        plugin_id: PluginId,
        entrypoint_id: Option<EntrypointId>,
    ) -> RequestResult<()>;

    async fn inline_view_shortcuts(&self) -> RequestResult<HashMap<PluginId, HashMap<String, PhysicalShortcut>>>;

    async fn run_entrypoint(&self, plugin_id: PluginId, entrypoint_id: EntrypointId) -> RequestResult<()>;
}

#[derive(Error, Debug, Clone)]
pub enum BackendApiError {
    #[error("Timeout Error")]
    Timeout,
    #[error("Internal Backend Error: {display:?}")]
    Internal { display: String },
}

impl From<tonic::Status> for BackendApiError {
    fn from(error: tonic::Status) -> BackendApiError {
        match error.code() {
            Code::Ok => unreachable!(),
            Code::DeadlineExceeded => BackendApiError::Timeout,
            _ => {
                BackendApiError::Internal {
                    display: format!("{}", error.message()),
                }
            }
        }
    }
}

impl From<prost::UnknownEnumValue> for BackendApiError {
    fn from(error: prost::UnknownEnumValue) -> BackendApiError {
        BackendApiError::Internal {
            display: format!("{}", error),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackendApi {
    client: RpcBackendClient<Channel>,
}

impl BackendApi {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: RpcBackendClient::connect("http://127.0.0.1:42320").await?,
        })
    }

    pub async fn ping(&mut self) -> Result<(), BackendApiError> {
        let _ = self.client.ping(Request::new(RpcPingRequest::default())).await?;

        Ok(())
    }

    pub async fn show_window(&mut self) -> Result<(), BackendApiError> {
        let _ = self
            .client
            .show_window(Request::new(RpcShowWindowRequest::default()))
            .await?;

        Ok(())
    }

    pub async fn show_settings_window(&mut self) -> Result<(), BackendApiError> {
        let _ = self
            .client
            .show_settings_window(Request::new(RpcShowSettingsWindowRequest::default()))
            .await?;

        Ok(())
    }

    pub async fn run_action(
        &mut self,
        plugin_id: String,
        entrypoint_id: String,
        action_id: String,
    ) -> Result<(), BackendApiError> {
        let _ = self
            .client
            .run_action(Request::new(RpcRunActionRequest {
                plugin_id,
                entrypoint_id,
                action_id,
            }))
            .await?;

        Ok(())
    }

    pub async fn plugins(&mut self) -> Result<HashMap<PluginId, SettingsPlugin>, BackendApiError> {
        let plugins = self
            .client
            .plugins(Request::new(RpcPluginsRequest::default()))
            .await?
            .into_inner()
            .plugins
            .into_iter()
            .map(|plugin| {
                let entrypoints: HashMap<_, _> = plugin
                    .entrypoints
                    .into_iter()
                    .map(|entrypoint| {
                        let id = EntrypointId::from_string(entrypoint.entrypoint_id);
                        let entrypoint_type: RpcEntrypointTypeSettings =
                            entrypoint.entrypoint_type.try_into().expect("download status failed"); // TODO proper error handling

                        let entrypoint_type = match entrypoint_type {
                            RpcEntrypointTypeSettings::SCommand => SettingsEntrypointType::Command,
                            RpcEntrypointTypeSettings::SView => SettingsEntrypointType::View,
                            RpcEntrypointTypeSettings::SInlineView => SettingsEntrypointType::InlineView,
                            RpcEntrypointTypeSettings::SEntrypointGenerator => {
                                SettingsEntrypointType::EntrypointGenerator
                            }
                        };

                        let entrypoint = SettingsEntrypoint {
                            enabled: entrypoint.enabled,
                            entrypoint_id: id.clone(),
                            entrypoint_name: entrypoint.entrypoint_name.clone(),
                            entrypoint_description: entrypoint.entrypoint_description,
                            entrypoint_type,
                            preferences: entrypoint
                                .preferences
                                .into_iter()
                                .map(|(key, value)| (key, plugin_preference_from_rpc(value)))
                                .collect(),
                            preferences_user_data: entrypoint
                                .preferences_user_data
                                .into_iter()
                                .map(|(key, value)| (key, plugin_preference_user_data_from_rpc(value)))
                                .collect(),
                            generated_entrypoints: entrypoint
                                .generated_entrypoints
                                .into_iter()
                                .map(|(entrypoint_id, data)| {
                                    let generated_entrypoint = SettingsGeneratedEntrypoint {
                                        entrypoint_id: EntrypointId::from_string(data.entrypoint_id),
                                        entrypoint_name: data.entrypoint_name,
                                    };

                                    (EntrypointId::from_string(entrypoint_id), generated_entrypoint)
                                })
                                .collect(),
                        };
                        (id, entrypoint)
                    })
                    .collect();

                let id = PluginId::from_string(plugin.plugin_id);
                let plugin = SettingsPlugin {
                    plugin_id: id.clone(),
                    plugin_name: plugin.plugin_name,
                    plugin_description: plugin.plugin_description,
                    enabled: plugin.enabled,
                    entrypoints,
                    preferences: plugin
                        .preferences
                        .into_iter()
                        .map(|(key, value)| (key, plugin_preference_from_rpc(value)))
                        .collect(),
                    preferences_user_data: plugin
                        .preferences_user_data
                        .into_iter()
                        .map(|(key, value)| (key, plugin_preference_user_data_from_rpc(value)))
                        .collect(),
                };

                (id, plugin)
            })
            .collect();

        Ok(plugins)
    }

    pub async fn set_plugin_state(&mut self, plugin_id: PluginId, enabled: bool) -> Result<(), BackendApiError> {
        let request = RpcSetPluginStateRequest {
            plugin_id: plugin_id.to_string(),
            enabled,
        };

        self.client.set_plugin_state(Request::new(request)).await?;

        Ok(())
    }

    pub async fn set_entrypoint_state(
        &mut self,
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
        enabled: bool,
    ) -> Result<(), BackendApiError> {
        let request = RpcSetEntrypointStateRequest {
            plugin_id: plugin_id.to_string(),
            entrypoint_id: entrypoint_id.to_string(),
            enabled,
        };

        self.client.set_entrypoint_state(Request::new(request)).await?;

        Ok(())
    }

    pub async fn set_global_shortcut(
        &mut self,
        shortcut: Option<PhysicalShortcut>,
    ) -> Result<Option<String>, BackendApiError> {
        let request = RpcSetGlobalShortcutRequest {
            shortcut: shortcut.map(|shortcut| {
                RpcShortcut {
                    physical_key: shortcut.physical_key.to_value(),
                    modifier_shift: shortcut.modifier_shift,
                    modifier_control: shortcut.modifier_control,
                    modifier_alt: shortcut.modifier_alt,
                    modifier_meta: shortcut.modifier_meta,
                }
            }),
        };

        let error = self
            .client
            .set_global_shortcut(Request::new(request))
            .await?
            .into_inner()
            .error;

        Ok(error)
    }

    pub async fn get_global_shortcut(&mut self) -> Result<(Option<PhysicalShortcut>, Option<String>), BackendApiError> {
        let response = self
            .client
            .get_global_shortcut(Request::new(RpcGetGlobalShortcutRequest::default()))
            .await?;

        let response = response.into_inner();

        Ok((
            response.shortcut.map(|shortcut| {
                PhysicalShortcut {
                    physical_key: PhysicalKey::from_value(shortcut.physical_key),
                    modifier_shift: shortcut.modifier_shift,
                    modifier_control: shortcut.modifier_control,
                    modifier_alt: shortcut.modifier_alt,
                    modifier_meta: shortcut.modifier_meta,
                }
            }),
            response.error,
        ))
    }

    pub async fn set_global_entrypoint_shortcut(
        &mut self,
        plugin_id: PluginId,
        entrypoint_id: EntrypointId,
        shortcut: Option<PhysicalShortcut>,
    ) -> Result<Option<String>, BackendApiError> {
        let request = RpcSetGlobalEntrypointShortcutRequest {
            plugin_id: plugin_id.to_string(),
            entrypoint_id: entrypoint_id.to_string(),
            shortcut: shortcut.map(|shortcut| {
                RpcShortcut {
                    physical_key: shortcut.physical_key.to_value(),
                    modifier_shift: shortcut.modifier_shift,
                    modifier_control: shortcut.modifier_control,
                    modifier_alt: shortcut.modifier_alt,
                    modifier_meta: shortcut.modifier_meta,
                }
            }),
        };

        let error = self
            .client
            .set_global_entrypoint_shortcut(request)
            .await?
            .into_inner()
            .error;

        Ok(error)
    }

    pub async fn get_global_entrypoint_shortcuts(
        &mut self,
    ) -> Result<HashMap<(PluginId, EntrypointId), (PhysicalShortcut, Option<String>)>, BackendApiError> {
        let response = self
            .client
            .get_global_entrypoint_shortcut(Request::new(RpcGetGlobalEntrypointShortcutRequest::default()))
            .await?;

        let response = response
            .into_inner()
            .shortcuts
            .into_iter()
            .map(|data| {
                let plugin_id = PluginId::from_string(data.plugin_id);
                let entrypoint_id = EntrypointId::from_string(data.entrypoint_id);
                let shortcut = data.shortcut.unwrap();
                let shortcut = PhysicalShortcut {
                    physical_key: PhysicalKey::from_value(shortcut.physical_key),
                    modifier_shift: shortcut.modifier_shift,
                    modifier_control: shortcut.modifier_control,
                    modifier_alt: shortcut.modifier_alt,
                    modifier_meta: shortcut.modifier_meta,
                };

                ((plugin_id, entrypoint_id), (shortcut, data.error))
            })
            .collect();

        Ok(response)
    }

    pub async fn set_theme(&mut self, theme: SettingsTheme) -> Result<(), BackendApiError> {
        let theme = match theme {
            SettingsTheme::AutoDetect => "AutoDetect",
            SettingsTheme::ThemeFile => "ThemeFile",
            SettingsTheme::Config => "Config",
            SettingsTheme::MacOSLight => "MacOSLight",
            SettingsTheme::MacOSDark => "MacOSDark",
            SettingsTheme::Legacy => "Legacy",
        };

        let request = RpcSetThemeRequest {
            theme: theme.to_string(),
        };

        self.client.set_theme(Request::new(request)).await?;

        Ok(())
    }

    pub async fn get_theme(&mut self) -> Result<SettingsTheme, BackendApiError> {
        let response = self
            .client
            .get_theme(Request::new(RpcGetThemeRequest::default()))
            .await?;

        let theme = response.into_inner().theme;

        let theme = match theme.as_str() {
            "AutoDetect" => SettingsTheme::AutoDetect,
            "ThemeFile" => SettingsTheme::ThemeFile,
            "Config" => SettingsTheme::Config,
            "MacOSLight" => SettingsTheme::MacOSLight,
            "MacOSDark" => SettingsTheme::MacOSDark,
            "Legacy" => SettingsTheme::Legacy,
            _ => unreachable!(),
        };

        Ok(theme)
    }

    pub async fn set_window_position_mode(&mut self, mode: WindowPositionMode) -> Result<(), BackendApiError> {
        let mode = match mode {
            WindowPositionMode::Static => "Static",
            WindowPositionMode::ActiveMonitor => "ActiveMonitor",
        };

        let request = RpcSetWindowPositionModeRequest { mode: mode.to_string() };

        self.client.set_window_position_mode(Request::new(request)).await?;

        Ok(())
    }

    pub async fn get_window_position_mode(&mut self) -> Result<WindowPositionMode, BackendApiError> {
        let response = self
            .client
            .get_window_position_mode(Request::new(RpcGetWindowPositionModeRequest::default()))
            .await?;

        let mode = response.into_inner().mode;

        let mode = match mode.as_str() {
            "Static" => WindowPositionMode::Static,
            "ActiveMonitor" => WindowPositionMode::ActiveMonitor,
            _ => unreachable!(),
        };

        Ok(mode)
    }

    pub async fn set_preference_value(
        &mut self,
        plugin_id: PluginId,
        entrypoint_id: Option<EntrypointId>,
        id: String,
        user_data: PluginPreferenceUserData,
    ) -> Result<(), BackendApiError> {
        let request = RpcSetPreferenceValueRequest {
            plugin_id: plugin_id.to_string(),
            entrypoint_id: entrypoint_id.map(|id| id.to_string()).unwrap_or_default(),
            preference_id: id,
            preference_value: Some(plugin_preference_user_data_to_rpc(user_data)),
        };

        self.client.set_preference_value(Request::new(request)).await?;

        Ok(())
    }

    pub async fn download_plugin(&mut self, plugin_id: PluginId) -> Result<(), BackendApiError> {
        let request = RpcDownloadPluginRequest {
            plugin_id: plugin_id.to_string(),
        };

        self.client.download_plugin(Request::new(request)).await?;

        Ok(())
    }

    pub async fn download_status(&mut self) -> Result<HashMap<PluginId, DownloadStatus>, BackendApiError> {
        let plugins = self
            .client
            .download_status(Request::new(RpcDownloadStatusRequest::default()))
            .await?
            .into_inner()
            .status_per_plugin
            .into_iter()
            .map(|(plugin_id, status)| {
                let plugin_id = PluginId::from_string(plugin_id);

                let status = match status.status.try_into()? {
                    RpcDownloadStatus::InProgress => DownloadStatus::InProgress,
                    RpcDownloadStatus::Done => DownloadStatus::Done,
                    RpcDownloadStatus::Failed => {
                        DownloadStatus::Failed {
                            message: status.message,
                        }
                    }
                };

                Ok::<(PluginId, DownloadStatus), BackendApiError>((plugin_id, status))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(plugins)
    }

    pub async fn remove_plugin(&mut self, plugin_id: PluginId) -> Result<(), BackendApiError> {
        let request = RpcRemovePluginRequest {
            plugin_id: plugin_id.to_string(),
        };

        self.client.remove_plugin(Request::new(request)).await?;

        Ok(())
    }

    pub async fn save_local_plugin(&mut self, path: String) -> Result<LocalSaveData, BackendApiError> {
        let request = RpcSaveLocalPluginRequest { path };

        let response = self.client.save_local_plugin(Request::new(request)).await?.into_inner();

        Ok(LocalSaveData {
            stdout_file_path: response.stdout_file_path,
            stderr_file_path: response.stderr_file_path,
        })
    }
}
