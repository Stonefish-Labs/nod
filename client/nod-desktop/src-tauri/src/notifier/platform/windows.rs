use nod_client_core::models::Request;
use tokio::sync::mpsc;
use windows::{
    core::{IInspectable, Interface, HSTRING},
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastActivatedEventArgs, ToastNotification, ToastNotificationManager},
};

use crate::notifier::{windows_toast::windows_toast_xml, NotificationActivation};

const TOAST_APP_ID: &str = "com.stonefishlabs.nod.desktop";

/// Windows only displays toasts for a registered AppUserModelID. Installers
/// register one via a Start-menu shortcut; a bare unzipped exe has to use the
/// per-user registry registration instead, refreshed on every launch.
pub(crate) fn register_toast_app_id(app: &tauri::App) -> anyhow::Result<()> {
    use tauri::Manager;

    let data_dir = app.path().app_local_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let icon_path = data_dir.join("toast-icon.png");
    std::fs::write(&icon_path, include_bytes!("../../../icons/128x128.png"))?;

    let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .create_subkey(format!(r"Software\Classes\AppUserModelId\{TOAST_APP_ID}"))?;
    key.set_value("DisplayName", &"Nod")?;
    key.set_value("IconUri", &icon_path.to_string_lossy().to_string())?;
    Ok(())
}

pub(crate) async fn show_notification(
    request: &Request,
    activations: mpsc::Sender<NotificationActivation>,
) -> anyhow::Result<()> {
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(windows_toast_xml(request)))?;
    let toast = ToastNotification::CreateToastNotification(&document)?;
    let request_id = request.id.clone();
    // The handler's generics pin the closure argument types — the activation
    // callback receives (&Option<ToastNotification>, &Option<IInspectable>).
    let handler = TypedEventHandler::<ToastNotification, IInspectable>::new(move |_, args| {
        let option_id = args
            .as_ref()
            .and_then(|args| args.cast::<ToastActivatedEventArgs>().ok())
            .and_then(|args| args.Arguments().ok())
            .map(|arguments| arguments.to_string_lossy())
            .filter(|arguments| !arguments.is_empty());
        let activation = match option_id.as_deref() {
            Some("open") | None => NotificationActivation::Open {
                request_id: Some(request_id.clone()),
            },
            Some(option_id) => NotificationActivation::Submit {
                request_id: request_id.clone(),
                option_id: option_id.to_string(),
            },
        };
        let _ = activations.blocking_send(activation);
        Ok(())
    });
    toast.Activated(&handler)?;
    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(TOAST_APP_ID))?;
    notifier.Show(&toast)?;
    Ok(())
}

pub(crate) async fn remove_notification(_request_id: &str) -> anyhow::Result<()> {
    Ok(())
}
