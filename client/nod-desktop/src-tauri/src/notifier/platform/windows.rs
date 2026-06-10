use nod_client_core::models::Request;
use tokio::sync::mpsc;
use windows::{
    core::{IInspectable, Interface, HSTRING},
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastActivatedEventArgs, ToastNotification, ToastNotificationManager},
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
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("Nod"))?;
    notifier.Show(&toast)?;
    Ok(())
}

pub(crate) async fn remove_notification(_request_id: &str) -> anyhow::Result<()> {
    Ok(())
}
