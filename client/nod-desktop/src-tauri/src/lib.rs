mod commands;
mod desktop_state;
mod external_url;
mod notifier;
mod runtime_events;
mod tray;
mod window;

use std::{error::Error, sync::Arc};

use nod_client_core::{NodClientEvent, NodClientRuntime};
use tauri::{App, AppHandle, Manager};
use tokio::sync::{mpsc, Mutex};

use crate::{
    desktop_state::DesktopState, notifier::DesktopNotifier, runtime_events::forward_runtime_events,
    tray::install_tray, window::focus_main_window,
};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::{notifier::NotificationActivation, runtime_events::handle_notification_activations};

const RUNTIME_EVENT_BUFFER: usize = 128;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const NOTIFICATION_ACTIVATION_BUFFER: usize = 32;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            focus_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(setup_desktop)
        .invoke_handler(tauri::generate_handler![
            commands::state,
            commands::enroll,
            commands::refresh,
            commands::select_server,
            commands::forget_server,
            commands::select_channel,
            commands::select_event,
            commands::submit_action,
            commands::clear_channel,
            commands::set_subscription,
            commands::set_notification_preference,
            commands::list_devices,
            commands::rename_device,
            commands::revoke_device,
            commands::open_external_url
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Nod desktop");
}

fn setup_desktop(app: &mut App) -> Result<(), Box<dyn Error>> {
    install_tray(app)?;

    let app_handle = app.handle().clone();
    let (event_tx, event_rx) = mpsc::channel::<NodClientEvent>(RUNTIME_EVENT_BUFFER);
    let runtime = tauri::async_runtime::block_on(NodClientRuntime::new(event_tx))?;
    let runtime = Arc::new(Mutex::new(runtime));
    let notifier = desktop_notifier(app_handle.clone(), runtime.clone());

    app.manage(DesktopState::new(runtime.clone()));

    tauri::async_runtime::spawn(forward_runtime_events(
        app_handle.clone(),
        notifier,
        event_rx,
    ));
    tauri::async_runtime::spawn(async move {
        let runtime = runtime.lock().await;
        runtime.emit_ready().await;
        runtime.emit_state().await;
    });

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn desktop_notifier(
    app_handle: AppHandle,
    runtime: Arc<Mutex<NodClientRuntime>>,
) -> DesktopNotifier {
    let (activation_tx, activation_rx) =
        mpsc::channel::<NotificationActivation>(NOTIFICATION_ACTIVATION_BUFFER);
    tauri::async_runtime::spawn(handle_notification_activations(
        app_handle,
        runtime,
        activation_rx,
    ));
    DesktopNotifier::new(activation_tx)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn desktop_notifier(
    _app_handle: AppHandle,
    _runtime: Arc<Mutex<NodClientRuntime>>,
) -> DesktopNotifier {
    DesktopNotifier::new()
}
