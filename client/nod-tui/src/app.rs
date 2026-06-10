mod enrollment;
mod focus;
mod navigation;
mod option_text;
mod rename_device;
mod settings;
mod text_input;

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nod_client_core::{
    models::{ClientState, OptionKind, Request, UserDevice},
    ChannelParams, NodClientMessage, RevokeDeviceParams, SelectRequestParams, SelectServerParams,
    SubmitOptionParams,
};

pub(crate) use enrollment::{EnrollmentField, EnrollmentForm};
pub(crate) use focus::Focus;
use navigation::{request_matches, selected_id_after};
pub(crate) use option_text::OptionTextForm;
pub(crate) use rename_device::RenameDeviceForm;
pub(crate) use settings::{SettingsState, SettingsTab};
pub(crate) use text_input::TextInput;

use crate::{
    alerts::{AlertEffect, AlertState},
    domain,
    runtime_bridge::{RuntimeCommand, RuntimeCommandOutcome},
};

pub(crate) const SOUND_OPTIONS: &[&str] = &[
    "default",
    "nod_ping.wav",
    "nod_chime.wav",
    "nod_low.wav",
    "silent",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Modal {
    Enrollment(EnrollmentForm),
    OptionText(OptionTextForm),
    Settings(SettingsState),
    RenameDevice(RenameDeviceForm),
    Filter(TextInput),
    Help,
}

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    client_state: ClientState,
    devices: Vec<UserDevice>,
    focus: Focus,
    modal: Option<Modal>,
    filter: String,
    error: Option<String>,
    status: String,
    running: Option<String>,
    alerts: AlertState,
    should_quit: bool,
}

impl AppState {
    pub(crate) fn new(client_state: ClientState) -> Self {
        // Keep unregistered sessions inside enrollment; runtime-backed commands
        // need the device and server profile created by that flow.
        let modal = if client_state.is_registered {
            None
        } else {
            Some(Modal::Enrollment(EnrollmentForm::new()))
        };
        Self {
            client_state,
            devices: Vec::new(),
            focus: Focus::Requests,
            modal,
            filter: String::new(),
            error: None,
            status: "Ready".to_string(),
            running: None,
            alerts: AlertState::new(),
            should_quit: false,
        }
    }

    pub(crate) fn client_state(&self) -> &ClientState {
        &self.client_state
    }

    pub(crate) fn devices(&self) -> &[UserDevice] {
        &self.devices
    }

    pub(crate) fn focus(&self) -> Focus {
        self.focus
    }

    pub(crate) fn modal(&self) -> Option<&Modal> {
        self.modal.as_ref()
    }

    pub(crate) fn filter(&self) -> &str {
        &self.filter
    }

    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn running(&self) -> Option<&str> {
        self.running.as_deref()
    }

    pub(crate) fn alerts(&self) -> &AlertState {
        &self.alerts
    }

    pub(crate) fn is_registered(&self) -> bool {
        self.client_state.is_registered
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub(crate) fn visible_requests(&self) -> Vec<&Request> {
        let query = self.filter.trim().to_lowercase();
        domain::ordered_requests(&self.client_state.requests)
            .into_iter()
            .filter(|request| query.is_empty() || request_matches(request, &query))
            .collect()
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Vec<RuntimeCommand> {
        if !is_pressed_key(key) {
            return Vec::new();
        }

        if is_interrupt_key(key) {
            self.should_quit = true;
            self.modal = None;
            return Vec::new();
        }

        if self.modal.is_some() {
            return self.handle_modal_key(key);
        }

        self.handle_main_key(key)
    }

    pub(crate) fn begin_command(&mut self, command: &RuntimeCommand) {
        self.error = None;
        self.status = command.label().to_string();
        self.running = Some(command.label().to_string());
    }

    pub(crate) fn apply_runtime_outcome(&mut self, outcome: RuntimeCommandOutcome) {
        match outcome {
            RuntimeCommandOutcome::State(state) => self.apply_state(*state),
            RuntimeCommandOutcome::Request(request) => self.apply_request(*request),
            RuntimeCommandOutcome::Device(device) => self.apply_device(*device),
            RuntimeCommandOutcome::Devices(devices) => self.devices = devices,
            RuntimeCommandOutcome::None => {}
        }
        self.running = None;
        self.status = "Ready".to_string();
    }

    pub(crate) fn apply_runtime_message(&mut self, request: NodClientMessage) -> AlertEffect {
        // Only live request messages drive terminal alerts; state snapshots
        // can refresh the screen without ringing for already-known work.
        let alert_effect = self.alerts.apply_runtime_message(&request, Instant::now());
        match request {
            NodClientMessage::Ready { state_path } => {
                self.status = format!("State: {state_path}");
            }
            NodClientMessage::State(state) => self.apply_state(*state),
            NodClientMessage::SyncStatus { connected } => {
                self.client_state.is_sync_connected = connected;
            }
            NodClientMessage::AuthRevoked {} => {
                self.set_error("This device registration was revoked.".to_string());
            }
            NodClientMessage::ResyncRequired {} => {
                self.status = "Server requested resync. Press R to refresh.".to_string();
            }
            NodClientMessage::TransientError { message } => self.set_error(message),
            NodClientMessage::NotificationCandidate { .. }
            | NodClientMessage::NotificationRemoved { .. } => {}
        }
        alert_effect
    }

    pub(crate) fn tick(&mut self) {
        self.alerts.tick(Instant::now());
    }

    pub(crate) fn set_error(&mut self, message: String) {
        self.running = None;
        self.error = Some(message);
    }

    fn apply_state(&mut self, state: ClientState) {
        self.client_state = state;
        self.devices = self.client_state.devices.clone();
        if self.client_state.is_registered {
            if matches!(self.modal, Some(Modal::Enrollment(_))) {
                self.modal = None;
            }
        } else {
            self.modal = Some(Modal::Enrollment(EnrollmentForm::new()));
        }
    }

    fn apply_request(&mut self, request: Request) {
        if let Some(existing) = self
            .client_state
            .requests
            .iter_mut()
            .find(|existing| existing.id == request.id)
        {
            *existing = request;
        } else {
            self.client_state.requests.insert(0, request);
        }
    }

    fn apply_device(&mut self, device: UserDevice) {
        if let Some(existing) = self
            .devices
            .iter_mut()
            .find(|existing| existing.id == device.id)
        {
            *existing = device;
        } else {
            self.devices.push(device);
        }
    }

    fn handle_main_key(&mut self, key: KeyEvent) -> Vec<RuntimeCommand> {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                Vec::new()
            }
            KeyCode::Tab => {
                self.focus = self.focus.next();
                Vec::new()
            }
            KeyCode::Char('s') => {
                self.focus = Focus::Servers;
                Vec::new()
            }
            KeyCode::Char(',') => {
                self.modal = Some(Modal::Settings(SettingsState::new()));
                vec![RuntimeCommand::ListDevices]
            }
            KeyCode::Char('?') => {
                self.modal = Some(Modal::Help);
                Vec::new()
            }
            KeyCode::Char('/') => {
                self.modal = Some(Modal::Filter(TextInput::from(self.filter.clone())));
                Vec::new()
            }
            KeyCode::Char('R') => vec![RuntimeCommand::Refresh],
            KeyCode::Char('m') => {
                self.alerts.toggle_mute();
                Vec::new()
            }
            KeyCode::Char('a') => self.command_for_option_kind(OptionKind::Approve),
            KeyCode::Char('r') => self.command_for_option_kind(OptionKind::Reject),
            KeyCode::Char('d') => self.command_for_option_kind(OptionKind::Dismiss),
            KeyCode::Char('n') => self.command_for_text_option(),
            KeyCode::Char('c') => self.command_for_clear_channel(),
            KeyCode::Enter => {
                if self.focus == Focus::Requests {
                    self.focus = Focus::Detail;
                }
                Vec::new()
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Left => {
                self.focus = Focus::Channels;
                Vec::new()
            }
            KeyCode::Right => {
                self.focus = Focus::Requests;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) -> Vec<RuntimeCommand> {
        let Some(modal) = self.modal.take() else {
            return Vec::new();
        };

        match modal {
            Modal::Enrollment(mut form) => {
                let (next_modal, command) = form.handle_key(key);
                self.modal = next_modal.map(Modal::Enrollment);
                command.into_iter().collect()
            }
            Modal::OptionText(mut form) => {
                let result = form.handle_key(key);
                self.modal = result.modal.map(Modal::OptionText);
                result.commands
            }
            Modal::Settings(mut settings) => {
                let result = self.handle_settings_key(&mut settings, key);
                if result.keep_settings_open {
                    self.modal = Some(Modal::Settings(settings));
                }
                result.commands
            }
            Modal::RenameDevice(mut form) => {
                let result = form.handle_key(key);
                self.modal = result.modal.map(Modal::RenameDevice);
                result.commands
            }
            Modal::Filter(mut input) => {
                self.handle_filter_key(&mut input, key);
                Vec::new()
            }
            Modal::Help => {
                if is_close_key(key) {
                    self.modal = None;
                } else {
                    self.modal = Some(Modal::Help);
                }
                Vec::new()
            }
        }
    }

    fn handle_settings_key(
        &mut self,
        settings: &mut SettingsState,
        key: KeyEvent,
    ) -> SettingsResult {
        if is_close_key(key) {
            return SettingsResult::closed();
        }

        match key.code {
            KeyCode::Tab => settings.next_tab(&self.client_state, self.devices.len()),
            KeyCode::BackTab => settings.previous_tab(&self.client_state, self.devices.len()),
            KeyCode::Up | KeyCode::Char('k') => {
                settings.move_selection(-1, &self.client_state, self.devices.len())
            }
            KeyCode::Down | KeyCode::Char('j') => {
                settings.move_selection(1, &self.client_state, self.devices.len())
            }
            KeyCode::Char('m') => self.alerts.toggle_mute(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                return SettingsResult::open(settings.command_for_selected(&self.client_state));
            }
            KeyCode::Char('r') => {
                if let Some(device) = settings.selected_device(&self.devices) {
                    self.modal = Some(Modal::RenameDevice(RenameDeviceForm::new(device)));
                    return SettingsResult::closed();
                }
            }
            KeyCode::Char('x') => {
                if let Some(device) = settings.selected_device(&self.devices) {
                    return SettingsResult::open(vec![RuntimeCommand::RevokeDevice(
                        RevokeDeviceParams {
                            device_id: device.id.clone(),
                        },
                    )]);
                }
            }
            KeyCode::Char('f') => {
                if let Some(server_id) = domain::selected_server_id(&self.client_state) {
                    return SettingsResult::open(vec![RuntimeCommand::ForgetServer(
                        SelectServerParams {
                            server_id: server_id.to_string(),
                        },
                    )]);
                }
            }
            _ => {}
        }

        SettingsResult::open(Vec::new())
    }

    fn handle_filter_key(&mut self, input: &mut TextInput, key: KeyEvent) {
        if is_close_key(key) {
            self.modal = None;
            return;
        }

        match key.code {
            KeyCode::Enter => {
                self.filter = input.value().trim().to_string();
                self.modal = None;
            }
            _ => {
                input.handle_key(key);
                self.modal = Some(Modal::Filter(input.clone()));
            }
        }
    }

    fn move_selection(&mut self, delta: isize) -> Vec<RuntimeCommand> {
        match self.focus {
            Focus::Servers => self.move_server(delta),
            Focus::Channels => self.move_channel(delta),
            Focus::Requests => self.move_request(delta),
            Focus::Detail => Vec::new(),
        }
    }

    fn move_server(&self, delta: isize) -> Vec<RuntimeCommand> {
        let Some(next_id) = selected_id_after(
            self.client_state
                .servers
                .iter()
                .map(|server| server.id.as_str()),
            domain::selected_server_id(&self.client_state),
            delta,
        ) else {
            return Vec::new();
        };

        vec![RuntimeCommand::SelectServer(SelectServerParams {
            server_id: next_id,
        })]
    }

    fn move_channel(&self, delta: isize) -> Vec<RuntimeCommand> {
        let channels = domain::subscribed_channels(&self.client_state);
        let current =
            domain::selected_channel(&self.client_state).map(|channel| channel.id.as_str());
        let Some(next_id) = selected_id_after(
            channels.iter().map(|channel| channel.id.as_str()),
            current,
            delta,
        ) else {
            return Vec::new();
        };

        vec![RuntimeCommand::SelectChannel(ChannelParams {
            channel_id: next_id,
        })]
    }

    fn move_request(&self, delta: isize) -> Vec<RuntimeCommand> {
        let requests = self.visible_requests();
        let current =
            domain::selected_request(&self.client_state).map(|request| request.id.as_str());
        let Some(next_id) = selected_id_after(
            requests.iter().map(|request| request.id.as_str()),
            current,
            delta,
        ) else {
            return Vec::new();
        };

        vec![RuntimeCommand::SelectRequest(SelectRequestParams {
            request_id: next_id,
        })]
    }

    fn command_for_option_kind(&mut self, kind: OptionKind) -> Vec<RuntimeCommand> {
        let Some(request) = domain::selected_request(&self.client_state) else {
            self.set_error("No request selected.".to_string());
            return Vec::new();
        };
        let Some(option) = domain::option_for_kind(request, kind) else {
            self.set_error("Selected request does not offer that option.".to_string());
            return Vec::new();
        };

        if option.requires_text {
            self.modal = Some(Modal::OptionText(OptionTextForm::from_choice(
                request, option,
            )));
            Vec::new()
        } else {
            vec![RuntimeCommand::SubmitOption(SubmitOptionParams {
                request_id: request.id.clone(),
                option_id: option.id.to_string(),
                text: None,
            })]
        }
    }

    fn command_for_text_option(&mut self) -> Vec<RuntimeCommand> {
        let Some(request) = domain::selected_request(&self.client_state) else {
            self.set_error("No request selected.".to_string());
            return Vec::new();
        };
        let Some(option) = domain::first_text_option(request) else {
            self.set_error("Selected request has no text option.".to_string());
            return Vec::new();
        };

        self.modal = Some(Modal::OptionText(OptionTextForm::from_choice(
            request, option,
        )));
        Vec::new()
    }

    fn command_for_clear_channel(&self) -> Vec<RuntimeCommand> {
        let Some(channel) = domain::selected_channel(&self.client_state) else {
            return Vec::new();
        };

        vec![RuntimeCommand::ClearChannel(ChannelParams {
            channel_id: channel.id.clone(),
        })]
    }
}

#[derive(Debug, Clone)]
struct SettingsResult {
    keep_settings_open: bool,
    commands: Vec<RuntimeCommand>,
}

impl SettingsResult {
    fn open(commands: Vec<RuntimeCommand>) -> Self {
        Self {
            keep_settings_open: true,
            commands,
        }
    }

    fn closed() -> Self {
        Self {
            keep_settings_open: false,
            commands: Vec::new(),
        }
    }
}

fn is_pressed_key(key: KeyEvent) -> bool {
    matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

fn is_close_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
}

fn is_interrupt_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use nod_client_core::models::{OptionKind, RequestOption};

    use super::*;
    use crate::test_support::{client_state, request};

    #[test]
    fn request_navigation_selects_next_visible_request() {
        let mut state = client_state();
        state.requests = vec![request("first", "default"), request("second", "default")];
        state.selected_request_id = Some("second".to_string());
        let mut app = AppState::new(state);

        let commands = app.handle_key(key(KeyCode::Down));

        assert_eq!(
            commands,
            vec![RuntimeCommand::SelectRequest(SelectRequestParams {
                request_id: "first".to_string()
            })]
        );
    }

    #[test]
    fn filter_limits_request_navigation() {
        let mut state = client_state();
        state.requests = vec![request("deploy", "default"), request("backup", "default")];
        state.selected_request_id = Some("deploy".to_string());
        let mut app = AppState::new(state);
        app.filter = "backup".to_string();

        let commands = app.handle_key(key(KeyCode::Down));

        assert_eq!(
            commands,
            vec![RuntimeCommand::SelectRequest(SelectRequestParams {
                request_id: "backup".to_string()
            })]
        );
    }

    #[test]
    fn text_option_opens_editor_and_submits_text() {
        let mut state = client_state();
        let mut candidate = request("deploy", "default");
        candidate.options = vec![RequestOption {
            id: "approve_notes".to_string(),
            label: "Approve with notes".to_string(),
            kind: OptionKind::ApproveWithText,
            style: "default".to_string(),
            requires_text: true,
            text_placeholder: Some("Notes".to_string()),
            destructive: false,
            foreground: false,
        }];
        state.requests = vec![candidate];
        state.selected_request_id = Some("deploy".to_string());
        let mut app = AppState::new(state);

        assert!(app.handle_key(key(KeyCode::Char('n'))).is_empty());
        assert!(matches!(app.modal(), Some(Modal::OptionText(_))));

        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Char('k')));
        let commands = app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            commands,
            vec![RuntimeCommand::SubmitOption(SubmitOptionParams {
                request_id: "deploy".to_string(),
                option_id: "approve_notes".to_string(),
                text: Some("ok".to_string())
            })]
        );
    }

    #[test]
    fn settings_channel_toggle_uses_core_subscription_command() {
        let mut app = AppState::new(client_state());
        app.modal = Some(Modal::Settings(SettingsState::new()));

        let commands = app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            commands,
            vec![RuntimeCommand::SetSubscription(
                nod_client_core::SetSubscriptionParams {
                    channel_id: "default".to_string(),
                    subscribed: false
                }
            )]
        );
    }

    #[test]
    fn help_modal_closes_without_command() {
        let mut app = AppState::new(client_state());

        assert!(app.handle_key(key(KeyCode::Char('?'))).is_empty());
        assert!(matches!(app.modal(), Some(Modal::Help)));

        assert!(app.handle_key(key(KeyCode::Esc)).is_empty());
        assert!(app.modal().is_none());
    }

    #[test]
    fn interrupt_key_quits_from_enrollment_modal() {
        let mut state = client_state();
        state.is_registered = false;
        let mut app = AppState::new(state);

        assert!(matches!(app.modal(), Some(Modal::Enrollment(_))));
        assert!(app
            .handle_key(key_with_modifiers(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            ))
            .is_empty());

        assert!(app.should_quit());
        assert!(app.modal().is_none());
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }
}
