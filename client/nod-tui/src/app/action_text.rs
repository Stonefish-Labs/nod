use crossterm::event::{KeyCode, KeyEvent};
use nod_client_core::{models::Event, SubmitActionParams};

use super::{is_close_key, RuntimeCommand, TextInput};
use crate::domain::ActionChoice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionTextForm {
    event_id: String,
    action_id: String,
    label: String,
    placeholder: Option<String>,
    input: TextInput,
}

impl ActionTextForm {
    pub(super) fn from_choice(event: &Event, action: ActionChoice<'_>) -> Self {
        Self {
            event_id: event.id.clone(),
            action_id: action.id.to_string(),
            label: action.label.to_string(),
            placeholder: action.placeholder.map(ToString::to_string),
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
            return ModalResult::commands(vec![RuntimeCommand::SubmitAction(SubmitActionParams {
                event_id: self.event_id.clone(),
                action_id: self.action_id.clone(),
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
    use nod_client_core::models::{Action, ActionKind};

    use super::*;
    use crate::test_support::event;

    #[test]
    fn escape_closes_without_command() {
        let event = event("deploy", "default");
        let action = Action {
            id: "approve_notes".to_string(),
            label: "Approve".to_string(),
            kind: ActionKind::ApproveWithText,
            style: "default".to_string(),
            requires_text: true,
            text_placeholder: None,
            destructive: false,
            foreground: false,
        };
        let choice = ActionChoice {
            id: &action.id,
            label: &action.label,
            placeholder: action.text_placeholder.as_deref(),
            requires_text: true,
        };
        let mut form = ActionTextForm::from_choice(&event, choice);

        let result = form.handle_key(key(KeyCode::Esc));

        assert!(result.modal.is_none());
        assert!(result.commands.is_empty());
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
}
