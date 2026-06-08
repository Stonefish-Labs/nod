use nod_client_core::models::Request;
use tokio::sync::mpsc;
use windows::{
    core::HSTRING,
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastNotification, ToastNotificationManager},
};

use crate::notifier::{windows_toast::windows_toast_xml, NotificationActivation};

pub(crate) async fn show_notification(
    request: &Request,
    activations: mpsc::Sender<NotificationActivation>,
) -> anyhow::Result<()> {
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(windows_toast_xml(request)))?;
    let toast = ToastNotification::CreateToastNotification(&document)?;
    let request_id = request.id.clone();
    toast.Activated(&TypedEventHandler::new(move |_, args| {
        let option_id = args
            .and_then(|args| {
                args.cast::<windows::UI::Notifications::ToastActivatedEventArgs>()
                    .ok()
            })
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
    }))?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("Nod"))?;
    notifier.Show(&toast)?;
    Ok(())
}

pub(crate) async fn remove_notification(_request_id: &str) -> anyhow::Result<()> {
    Ok(())
}
