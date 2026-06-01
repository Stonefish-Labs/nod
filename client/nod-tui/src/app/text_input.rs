use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextInput {
    value: String,
}

impl TextInput {
    pub(super) fn new() -> Self {
        Self {
            value: String::new(),
        }
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                self.value.pop();
            }
            KeyCode::Char(character)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.value.push(character);
            }
            _ => {}
        }
    }

    pub(super) fn handle_code_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(character) if character.is_ascii_alphanumeric() => {
                self.value.push(character.to_ascii_uppercase());
            }
            _ => self.handle_key(key),
        }
    }
}

impl From<String> for TextInput {
    fn from(value: String) -> Self {
        Self { value }
    }
}
