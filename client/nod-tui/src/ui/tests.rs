use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use crate::{app::AppState, test_support::client_state};

use super::render;

#[test]
fn renders_registered_main_screen() {
    render_app(&AppState::new(client_state()));
}

#[test]
fn renders_enrollment_screen() {
    let mut state = client_state();
    state.is_registered = false;

    render_app(&AppState::new(state));
}

#[test]
fn renders_settings_modal() {
    let mut app = AppState::new(client_state());

    app.handle_key(key(KeyCode::Char(',')));

    render_app(&app);
}

#[test]
fn renders_empty_device_list() {
    let mut app = AppState::new(client_state());

    app.handle_key(key(KeyCode::Char(',')));
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Tab));

    render_app(&app);
}

fn render_app(app: &AppState) {
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).expect("test backend should initialize");

    terminal
        .draw(|frame| render(frame, app))
        .expect("render should complete without panicking");
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
