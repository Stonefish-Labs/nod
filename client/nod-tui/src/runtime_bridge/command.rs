use nod_client_core::{
    models::{ClientState, Event, UserDevice},
    ChannelParams, EnrollParams, NotificationPreferenceParams, RenameDeviceParams,
    RevokeDeviceParams, SelectEventParams, SelectServerParams, SetSubscriptionParams,
    SubmitActionParams,
};

#[derive(Debug, Clone)]
pub(crate) enum RuntimeCommand {
    Enroll(EnrollParams),
    Refresh,
    ConnectSync,
    SelectServer(SelectServerParams),
    ForgetServer(SelectServerParams),
    SelectChannel(ChannelParams),
    SelectEvent(SelectEventParams),
    SubmitAction(SubmitActionParams),
    ClearChannel(ChannelParams),
    SetSubscription(SetSubscriptionParams),
    SetNotificationPreference(NotificationPreferenceParams),
    ListDevices,
    RenameDevice(RenameDeviceParams),
    RevokeDevice(RevokeDeviceParams),
}

impl RuntimeCommand {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Enroll(_) => "Enrolling",
            Self::Refresh => "Refreshing",
            Self::ConnectSync => "Connecting sync",
            Self::SelectServer(_) => "Switching server",
            Self::ForgetServer(_) => "Forgetting server",
            Self::SelectChannel(_) => "Loading channel",
            Self::SelectEvent(_) => "Selecting notification",
            Self::SubmitAction(_) => "Submitting action",
            Self::ClearChannel(_) => "Clearing channel",
            Self::SetSubscription(_) => "Updating subscription",
            Self::SetNotificationPreference(_) => "Updating notification sound",
            Self::ListDevices => "Loading devices",
            Self::RenameDevice(_) => "Renaming device",
            Self::RevokeDevice(_) => "Revoking device",
        }
    }
}

#[cfg(test)]
impl PartialEq for RuntimeCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Enroll(left), Self::Enroll(right)) => {
                left.base_url == right.base_url
                    && left.device_name == right.device_name
                    && left.code == right.code
                    && left.notification_sound == right.notification_sound
                    && left.platform == right.platform
            }
            (Self::Refresh, Self::Refresh) => true,
            (Self::ConnectSync, Self::ConnectSync) => true,
            (Self::SelectServer(left), Self::SelectServer(right))
            | (Self::ForgetServer(left), Self::ForgetServer(right)) => {
                left.server_id == right.server_id
            }
            (Self::SelectChannel(left), Self::SelectChannel(right))
            | (Self::ClearChannel(left), Self::ClearChannel(right)) => {
                left.channel_id == right.channel_id
            }
            (Self::SelectEvent(left), Self::SelectEvent(right)) => left.event_id == right.event_id,
            (Self::SubmitAction(left), Self::SubmitAction(right)) => {
                left.event_id == right.event_id
                    && left.action_id == right.action_id
                    && left.text == right.text
            }
            (Self::SetSubscription(left), Self::SetSubscription(right)) => {
                left.channel_id == right.channel_id && left.subscribed == right.subscribed
            }
            (Self::SetNotificationPreference(left), Self::SetNotificationPreference(right)) => {
                left.notification_sound == right.notification_sound
            }
            (Self::ListDevices, Self::ListDevices) => true,
            (Self::RenameDevice(left), Self::RenameDevice(right)) => {
                left.device_id == right.device_id && left.name == right.name
            }
            (Self::RevokeDevice(left), Self::RevokeDevice(right)) => {
                left.device_id == right.device_id
            }
            _ => false,
        }
    }
}

#[cfg(test)]
impl Eq for RuntimeCommand {}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeCommandOutcome {
    State(Box<ClientState>),
    Event(Box<Event>),
    Device(Box<UserDevice>),
    Devices(Vec<UserDevice>),
    None,
}
