mod format;
mod layout;
mod modals;
mod panes;
#[cfg(test)]
mod tests;

use ratatui::Frame;

use crate::app::AppState;

pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState) {
    let area = frame.area();
    if !app.is_registered() {
        panes::render_enrollment(frame, area, app);
    } else {
        panes::render_main(frame, area, app);
    }

    if let Some(modal) = app.modal() {
        modals::render_modal(frame, area, app, modal);
    }
}
