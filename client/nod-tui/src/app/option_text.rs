use crossterm::event::{KeyCode, KeyEvent};
use nod_client_core::{models::Request, SubmitOptionParams};

use super::{is_close_key, RuntimeCommand, TextInput};
use crate::domain::OptionChoice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionTextForm {
    request_id: String,
    option_id: String,
    label: String,
    placeholder: Option<String>,
    input: TextInput,
}

impl OptionTextForm {
    pub(super) fn from_choice(request: &Request, option: OptionChoice<'_>) -> Self {
        Self {
            request_id: request.id.clone(),
            option_id: option.id.to_string(),
            label: option.label.to_string(),
            placeholder: option.placeholder.map(ToString::to_string),
            input: TextInput::new(),
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    pub(crate) fn input(&self) -> &TextInput {
        &self.input
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> ModalResult<Self> {
        if is_close_key(key) {
            return ModalResult::closed();
        }

        if key.code == KeyCode::Enter {
            return ModalResult::commands(vec![RuntimeCommand::SubmitOption(SubmitOptionParams {
                request_id: self.request_id.clone(),
                option_id: self.option_id.clone(),
                text: Some(self.input.value().to_string()),
            })]);
        }

        self.input.handle_key(key);
        ModalResult::open(self.clone())
    }
}

#[derive(Debug, Clone)]
pub(super) struct ModalResult<T> {
    pub(super) modal: Option<T>,
    pub(super) commands: Vec<RuntimeCommand>,
}

impl<T> ModalResult<T> {
    pub(super) fn open(modal: T) -> Self {
        Self {
            modal: Some(modal),
            commands: Vec::new(),
        }
    }

    pub(super) fn closed() -> Self {
        Self {
            modal: None,
            commands: Vec::new(),
        }
    }

    pub(super) fn commands(commands: Vec<RuntimeCommand>) -> Self {
        Self {
            modal: None,
            commands,
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use nod_client_core::models::{OptionKind, RequestOption};

    use super::*;
    use crate::test_support::request;

    #[test]
    fn escape_closes_without_command() {
        let request = request("deploy", "default");
        let option = RequestOption {
            id: "approve_notes".to_string(),
            label: "Approve".to_string(),
            kind: OptionKind::ApproveWithText,
            style: "default".to_string(),
            requires_text: true,
            text_placeholder: None,
            destructive: false,
            foreground: false,
        };
        let choice = OptionChoice {
            id: &option.id,
            label: &option.label,
            placeholder: option.text_placeholder.as_deref(),
            requires_text: true,
        };
        let mut form = OptionTextForm::from_choice(&request, choice);

        let result = form.handle_key(key(KeyCode::Esc));

        assert!(result.modal.is_none());
        assert!(result.commands.is_empty());
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
}
