use nod_client_core::{
    models::{ClientState, Request, UserDevice},
    EnrollParams, NotificationPreferenceParams, RenameDeviceParams, RevokeDeviceParams,
    SelectRequestParams, SelectServerParams, SetSubscriptionParams, SourceParams,
    SubmitOptionParams,
};

#[derive(Debug, Clone)]
pub(crate) enum RuntimeCommand {
    Enroll(EnrollParams),
    Refresh,
    ConnectSync,
    SelectServer(SelectServerParams),
    ForgetServer(SelectServerParams),
    SelectSource(SourceParams),
    SelectRequest(SelectRequestParams),
    SubmitOption(SubmitOptionParams),
    ClearSource(SourceParams),
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
            Self::SelectSource(_) => "Loading source",
            Self::SelectRequest(_) => "Selecting request",
            Self::SubmitOption(_) => "Submitting option",
            Self::ClearSource(_) => "Clearing source",
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
            (Self::SelectSource(left), Self::SelectSource(right))
            | (Self::ClearSource(left), Self::ClearSource(right)) => {
                left.source_id == right.source_id
            }
            (Self::SelectRequest(left), Self::SelectRequest(right)) => {
                left.request_id == right.request_id
            }
            (Self::SubmitOption(left), Self::SubmitOption(right)) => {
                left.request_id == right.request_id
                    && left.option_id == right.option_id
                    && left.text == right.text
            }
            (Self::SetSubscription(left), Self::SetSubscription(right)) => {
                left.source_id == right.source_id && left.subscribed == right.subscribed
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
    Request(Box<Request>),
    Device(Box<UserDevice>),
    Devices(Vec<UserDevice>),
    None,
}
