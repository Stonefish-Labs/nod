use std::time::{Duration, Instant};

use nod_client_core::{models::Event, NodClientEvent};

const FLASH_DURATION: Duration = Duration::from_millis(900);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AlertEffect {
    pub ring_bell: bool,
}

#[derive(Debug, Clone)]
pub struct AlertState {
    muted: bool,
    active_event_id: Option<String>,
    message: Option<String>,
    flash_until: Option<Instant>,
}

impl Default for AlertState {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertState {
    pub fn new() -> Self {
        Self {
            muted: false,
            active_event_id: None,
            message: None,
            flash_until: None,
        }
    }

    pub fn apply_runtime_event(&mut self, event: &NodClientEvent, now: Instant) -> AlertEffect {
        match event {
            NodClientEvent::NotificationCandidate { event } => self.alert_for(event, now),
            NodClientEvent::NotificationRemoved { event_id } => {
                self.remove_event(event_id);
                AlertEffect::default()
            }
            _ => AlertEffect::default(),
        }
    }

    pub fn tick(&mut self, now: Instant) {
        if self
            .flash_until
            .map(|flash_until| now >= flash_until)
            .unwrap_or(false)
        {
            self.flash_until = None;
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            self.flash_until = None;
        }
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    pub fn flashing(&self) -> bool {
        self.flash_until.is_some()
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    fn alert_for(&mut self, event: &Event, now: Instant) -> AlertEffect {
        self.active_event_id = Some(event.id.clone());
        self.message = Some(format!("New notification: {}", event.title));

        if self.muted {
            return AlertEffect::default();
        }

        self.flash_until = Some(now + FLASH_DURATION);
        AlertEffect { ring_bell: true }
    }

    fn remove_event(&mut self, event_id: &str) {
        if self.active_event_id.as_deref() == Some(event_id) {
            self.active_event_id = None;
            self.message = None;
            self.flash_until = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use nod_client_core::NodClientEvent;

    use super::*;
    use crate::test_support::event;

    #[test]
    fn notification_candidate_rings_once_and_flashes() {
        let now = Instant::now();
        let mut alerts = AlertState::new();
        let effect = alerts.apply_runtime_event(
            &NodClientEvent::NotificationCandidate {
                event: Box::new(event("new", "default")),
            },
            now,
        );

        assert!(effect.ring_bell);
        assert!(alerts.flashing());
        assert_eq!(alerts.message(), Some("New notification: new"));

        alerts.tick(now + Duration::from_secs(2));
        assert!(!alerts.flashing());
        assert_eq!(alerts.message(), Some("New notification: new"));
    }

    #[test]
    fn mute_suppresses_bell_and_flash() {
        let now = Instant::now();
        let mut alerts = AlertState::new();
        alerts.toggle_mute();

        let effect = alerts.apply_runtime_event(
            &NodClientEvent::NotificationCandidate {
                event: Box::new(event("quiet", "default")),
            },
            now,
        );

        assert!(!effect.ring_bell);
        assert!(!alerts.flashing());
        assert_eq!(alerts.message(), Some("New notification: quiet"));
    }

    #[test]
    fn removal_clears_active_alert() {
        let now = Instant::now();
        let mut alerts = AlertState::new();
        alerts.apply_runtime_event(
            &NodClientEvent::NotificationCandidate {
                event: Box::new(event("done", "default")),
            },
            now,
        );

        alerts.apply_runtime_event(
            &NodClientEvent::NotificationRemoved {
                event_id: "done".to_string(),
            },
            now,
        );

        assert!(!alerts.flashing());
        assert_eq!(alerts.message(), None);
    }
}
