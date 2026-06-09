use crossterm::event::{KeyCode, KeyEvent};
use nod_client_core::EnrollParams;

use super::{
    navigation::{next_index, previous_index},
    RuntimeCommand, TextInput, SOUND_OPTIONS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnrollmentField {
    Server,
    Device,
    Code,
    Sound,
}

impl EnrollmentField {
    fn next(self) -> Self {
        match self {
            Self::Server => Self::Device,
            Self::Device => Self::Code,
            Self::Code => Self::Sound,
            Self::Sound => Self::Server,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnrollmentForm {
    base_url: TextInput,
    device_name: TextInput,
    code: TextInput,
    notification_sound: usize,
    active_field: EnrollmentField,
}

impl EnrollmentForm {
    pub(super) fn new() -> Self {
        Self {
            base_url: TextInput::new(),
            device_name: TextInput::from(default_device_name()),
            code: TextInput::new(),
            notification_sound: 0,
            active_field: EnrollmentField::Server,
        }
    }

    pub(crate) fn base_url(&self) -> &TextInput {
        &self.base_url
    }

    pub(crate) fn device_name(&self) -> &TextInput {
        &self.device_name
    }

    pub(crate) fn code(&self) -> &TextInput {
        &self.code
    }

    pub(crate) fn active_field(&self) -> EnrollmentField {
        self.active_field
    }

    pub(crate) fn selected_sound(&self) -> &str {
        SOUND_OPTIONS[self.notification_sound]
    }

    pub(super) fn submit_ready(&self) -> bool {
        !self.base_url.value().trim().is_empty()
            && !self.device_name.value().trim().is_empty()
            && self.code.value().trim().len() >= 8
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> (Option<Self>, Option<RuntimeCommand>) {
        if key.code == KeyCode::Tab {
            self.active_field = self.active_field.next();
            return (Some(self.clone()), None);
        }

        if key.code == KeyCode::Enter && self.submit_ready() {
            return (
                Some(self.clone()),
                Some(RuntimeCommand::Enroll(self.enroll_params())),
            );
        }

        match self.active_field {
            EnrollmentField::Server => self.base_url.handle_key(key),
            EnrollmentField::Device => self.device_name.handle_key(key),
            EnrollmentField::Code => self.code.handle_code_key(key),
            EnrollmentField::Sound => self.handle_sound_key(key),
        }
        (Some(self.clone()), None)
    }

    fn enroll_params(&self) -> EnrollParams {
        EnrollParams {
            base_url: self.base_url.value().trim().to_string(),
            device_name: self.device_name.value().trim().to_string(),
            code: self.code.value().trim().to_string(),
            notification_sound: Some(self.selected_sound().to_string()),
            platform: None,
            // The TUI is a desktop client without Apple push or App Attest.
            native_app_id: None,
            push_provider: None,
            push_token: None,
            attestation: None,
        }
    }

    fn handle_sound_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Up | KeyCode::Char('k') => {
                self.notification_sound =
                    previous_index(self.notification_sound, SOUND_OPTIONS.len())
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Char('j') | KeyCode::Char(' ') => {
                self.notification_sound = next_index(self.notification_sound, SOUND_OPTIONS.len())
            }
            _ => {}
        }
    }
}

fn default_device_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Nod TUI".to_string())
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    #[test]
    fn submit_ready_requires_server_device_and_code() {
        let mut form = EnrollmentForm::new();

        assert!(!form.submit_ready());

        form.base_url = TextInput::from("http://localhost:8767".to_string());
        form.device_name = TextInput::from("terminal".to_string());
        form.code = TextInput::from("ABCDEFGH".to_string());

        assert!(form.submit_ready());
    }

    #[test]
    fn submit_keeps_form_visible_while_enrollment_runs() {
        let mut form = EnrollmentForm::new();
        form.base_url = TextInput::from("http://localhost:8767".to_string());
        form.device_name = TextInput::from("terminal".to_string());
        form.code = TextInput::from("ABCDEFGH".to_string());

        let (next_form, command) = form.handle_key(key(KeyCode::Enter));

        assert!(next_form.is_some());
        assert!(matches!(command, Some(RuntimeCommand::Enroll(_))));
    }

    #[test]
    fn sound_selection_moves_within_options() {
        let mut form = EnrollmentForm::new();
        form.active_field = EnrollmentField::Sound;

        form.handle_key(key(KeyCode::Right));
        assert_eq!(form.selected_sound(), "nod_ping.wav");

        form.handle_key(key(KeyCode::Left));
        form.handle_key(key(KeyCode::Left));
        assert_eq!(form.selected_sound(), "default");
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
}
