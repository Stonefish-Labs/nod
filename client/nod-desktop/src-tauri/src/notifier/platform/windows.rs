use nod_client_core::models::Event;
use tokio::sync::mpsc;
use windows::{
    core::HSTRING,
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastNotification, ToastNotificationManager},
};

use crate::notifier::{windows_toast::windows_toast_xml, NotificationActivation};

pub(crate) async fn show_notification(
    event: &Event,
    activations: mpsc::Sender<NotificationActivation>,
) -> anyhow::Result<()> {
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(windows_toast_xml(event)))?;
    let toast = ToastNotification::CreateToastNotification(&document)?;
    let event_id = event.id.clone();
    toast.Activated(&TypedEventHandler::new(move |_, args| {
        let action_id = args
            .and_then(|args| {
                args.cast::<windows::UI::Notifications::ToastActivatedEventArgs>()
                    .ok()
            })
            .and_then(|args| args.Arguments().ok())
            .map(|arguments| arguments.to_string_lossy())
            .filter(|arguments| !arguments.is_empty());
        let activation = match action_id.as_deref() {
            Some("open") | None => NotificationActivation::Open {
                event_id: Some(event_id.clone()),
            },
            Some(action_id) => NotificationActivation::Submit {
                event_id: event_id.clone(),
                action_id: action_id.to_string(),
            },
        };
        let _ = activations.blocking_send(activation);
        Ok(())
    }))?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("Nod"))?;
    notifier.Show(&toast)?;
    Ok(())
}

pub(crate) async fn remove_notification(_event_id: &str) -> anyhow::Result<()> {
    Ok(())
}
