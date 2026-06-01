use nod_client_core::models::Event;

use super::actions::desktop_notification_actions;

pub(super) fn windows_toast_xml(event: &Event) -> String {
    let body = if event.summary.trim().is_empty() {
        event.body_markdown.as_str()
    } else {
        event.summary.as_str()
    };
    let actions = desktop_notification_actions(event)
        .into_iter()
        .map(|action| {
            format!(
                "<action content=\"{}\" arguments=\"{}\" activationType=\"foreground\"/>",
                xml_escape(&action.label),
                xml_escape(&action.id)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<toast launch=\"open\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><actions>{}</actions></toast>",
        xml_escape(&event.title),
        xml_escape(body),
        actions
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
