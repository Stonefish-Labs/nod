use nod_client_core::{models::ClientState, NotificationPreferenceParams, SetSubscriptionParams};

use super::{navigation::moved_index, RuntimeCommand, SOUND_OPTIONS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    Channels,
    Sound,
    Devices,
}

impl SettingsTab {
    fn next(self) -> Self {
        match self {
            Self::Channels => Self::Sound,
            Self::Sound => Self::Devices,
            Self::Devices => Self::Channels,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Channels => Self::Devices,
            Self::Sound => Self::Channels,
            Self::Devices => Self::Sound,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsState {
    tab: SettingsTab,
    selected_index: usize,
}

impl SettingsState {
    pub(super) fn new() -> Self {
        Self {
            tab: SettingsTab::Channels,
            selected_index: 0,
        }
    }

    pub(crate) fn tab(&self) -> SettingsTab {
        self.tab
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub(super) fn next_tab(&mut self, state: &ClientState, device_count: usize) {
        self.tab = self.tab.next();
        self.clamp_selection(state, device_count);
    }

    pub(super) fn previous_tab(&mut self, state: &ClientState, device_count: usize) {
        self.tab = self.tab.previous();
        self.clamp_selection(state, device_count);
    }

    pub(super) fn move_selection(
        &mut self,
        delta: isize,
        state: &ClientState,
        device_count: usize,
    ) {
        let count = self.item_count(state, device_count);
        if count == 0 {
            self.selected_index = 0;
            return;
        }
        self.selected_index = moved_index(self.selected_index, count, delta);
    }

    pub(super) fn selected_device<'a>(
        &self,
        devices: &'a [nod_client_core::models::UserDevice],
    ) -> Option<&'a nod_client_core::models::UserDevice> {
        if self.tab != SettingsTab::Devices {
            return None;
        }
        devices.get(self.selected_index)
    }

    pub(super) fn command_for_selected(&self, state: &ClientState) -> Vec<RuntimeCommand> {
        match self.tab {
            SettingsTab::Channels => state
                .channels
                .get(self.selected_index)
                .map(|channel| {
                    RuntimeCommand::SetSubscription(SetSubscriptionParams {
                        channel_id: channel.id.clone(),
                        subscribed: !channel.subscribed,
                    })
                })
                .into_iter()
                .collect(),
            SettingsTab::Sound => vec![RuntimeCommand::SetNotificationPreference(
                NotificationPreferenceParams {
                    notification_sound: SOUND_OPTIONS[self.selected_index].to_string(),
                },
            )],
            SettingsTab::Devices => Vec::new(),
        }
    }

    fn item_count(&self, state: &ClientState, device_count: usize) -> usize {
        match self.tab {
            SettingsTab::Channels => state.channels.len(),
            SettingsTab::Sound => SOUND_OPTIONS.len(),
            SettingsTab::Devices => device_count,
        }
    }

    fn clamp_selection(&mut self, state: &ClientState, device_count: usize) {
        let count = self.item_count(state, device_count);
        if count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= count {
            self.selected_index = count - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::client_state;

    #[test]
    fn tab_changes_clamp_selection_to_available_items() {
        let mut state = client_state();
        state.channels.clear();
        let mut settings = SettingsState {
            tab: SettingsTab::Channels,
            selected_index: 10,
        };

        settings.next_tab(&state, 0);
        assert_eq!(settings.tab(), SettingsTab::Sound);
        assert_eq!(settings.selected_index(), SOUND_OPTIONS.len() - 1);

        settings.next_tab(&state, 0);
        assert_eq!(settings.tab(), SettingsTab::Devices);
        assert_eq!(settings.selected_index(), 0);
    }
}
