use nod_client_core::models::ClientState;
use nod_client_core::{
    ChannelParams, EnrollParams, NotificationPreferenceParams, RenameDeviceParams,
    RevokeDeviceParams, SelectEventParams, SelectServerParams, SetSubscriptionParams,
    SubmitActionParams,
};
use tauri::State;

use crate::{desktop_state::DesktopState, external_url::open_url};

#[tauri::command]
pub(crate) async fn state(state: State<'_, DesktopState>) -> Result<ClientState, String> {
    Ok(state.runtime.lock().await.state().await)
}

#[tauri::command]
pub(crate) async fn enroll(
    state: State<'_, DesktopState>,
    params: EnrollParams,
) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.enroll(params).await)
}

#[tauri::command]
pub(crate) async fn refresh(state: State<'_, DesktopState>) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.refresh().await)
}

#[tauri::command]
pub(crate) async fn select_server(
    state: State<'_, DesktopState>,
    params: SelectServerParams,
) -> Result<ClientState, String> {
    command_result(
        state
            .runtime
            .lock()
            .await
            .select_server(params.server_id)
            .await,
    )
}

#[tauri::command]
pub(crate) async fn forget_server(
    state: State<'_, DesktopState>,
    params: SelectServerParams,
) -> Result<ClientState, String> {
    command_result(
        state
            .runtime
            .lock()
            .await
            .forget_server(&params.server_id)
            .await,
    )
}

#[tauri::command]
pub(crate) async fn select_channel(
    state: State<'_, DesktopState>,
    params: ChannelParams,
) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.select_channel(params).await)
}

#[tauri::command]
pub(crate) async fn select_event(
    state: State<'_, DesktopState>,
    params: SelectEventParams,
) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.select_event(params).await)
}

#[tauri::command]
pub(crate) async fn submit_action(
    state: State<'_, DesktopState>,
    params: SubmitActionParams,
) -> Result<nod_client_core::models::Event, String> {
    command_result(state.runtime.lock().await.submit_action(params).await)
}

#[tauri::command]
pub(crate) async fn clear_channel(
    state: State<'_, DesktopState>,
    params: ChannelParams,
) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.clear_channel(params).await)
}

#[tauri::command]
pub(crate) async fn set_subscription(
    state: State<'_, DesktopState>,
    params: SetSubscriptionParams,
) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.set_subscription(params).await)
}

#[tauri::command]
pub(crate) async fn set_notification_preference(
    state: State<'_, DesktopState>,
    params: NotificationPreferenceParams,
) -> Result<ClientState, String> {
    command_result(
        state
            .runtime
            .lock()
            .await
            .set_notification_preference(&params.notification_sound)
            .await,
    )
}

#[tauri::command]
pub(crate) async fn list_devices(
    state: State<'_, DesktopState>,
) -> Result<Vec<nod_client_core::models::UserDevice>, String> {
    command_result(state.runtime.lock().await.list_devices().await)
}

#[tauri::command]
pub(crate) async fn rename_device(
    state: State<'_, DesktopState>,
    params: RenameDeviceParams,
) -> Result<nod_client_core::models::UserDevice, String> {
    command_result(state.runtime.lock().await.rename_device(params).await)
}

#[tauri::command]
pub(crate) async fn revoke_device(
    state: State<'_, DesktopState>,
    params: RevokeDeviceParams,
) -> Result<ClientState, String> {
    command_result(state.runtime.lock().await.revoke_device(params).await)
}

#[tauri::command]
pub(crate) fn open_external_url(url: String) -> Result<(), String> {
    open_url(&url).map_err(|error| error.to_string())
}

fn command_result<T>(result: anyhow::Result<T>) -> Result<T, String> {
    result.map_err(|error| error.to_string())
}
