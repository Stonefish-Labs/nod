use nod_client_core::models::Request;

use super::options::desktop_notification_options;

pub(super) fn windows_toast_xml(request: &Request) -> String {
    let body = if request.summary.trim().is_empty() {
        request.body_markdown.as_str()
    } else {
        request.summary.as_str()
    };
    let options = desktop_notification_options(request)
        .into_iter()
        .map(|option| {
            format!(
                "<option content=\"{}\" arguments=\"{}\" activationType=\"foreground\"/>",
                xml_escape(&option.label),
                xml_escape(&option.id)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<toast launch=\"open\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><options>{}</options></toast>",
        xml_escape(&request.title),
        xml_escape(body),
        options
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
