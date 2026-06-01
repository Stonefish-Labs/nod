use anyhow::Result;
use async_trait::async_trait;
use nod_client_core::{
    models::{ClientState, Event, UserDevice},
    ChannelParams, EnrollParams, NotificationPreferenceParams, RenameDeviceParams,
    RevokeDeviceParams, SelectEventParams, SelectServerParams, SetSubscriptionParams,
    SubmitActionParams,
};

#[async_trait]
pub(crate) trait RuntimePort {
    async fn enroll(&mut self, params: EnrollParams) -> Result<ClientState>;
    async fn refresh(&mut self) -> Result<ClientState>;
    async fn connect_sync(&mut self) -> Result<()>;
    async fn select_server(&mut self, params: SelectServerParams) -> Result<ClientState>;
    async fn forget_server(&mut self, params: SelectServerParams) -> Result<ClientState>;
    async fn select_channel(&mut self, params: ChannelParams) -> Result<ClientState>;
    async fn select_event(&mut self, params: SelectEventParams) -> Result<ClientState>;
    async fn submit_action(&mut self, params: SubmitActionParams) -> Result<Event>;
    async fn clear_channel(&mut self, params: ChannelParams) -> Result<ClientState>;
    async fn set_subscription(&mut self, params: SetSubscriptionParams) -> Result<ClientState>;
    async fn set_notification_preference(
        &mut self,
        params: NotificationPreferenceParams,
    ) -> Result<ClientState>;
    async fn list_devices(&mut self) -> Result<Vec<UserDevice>>;
    async fn rename_device(&mut self, params: RenameDeviceParams) -> Result<UserDevice>;
    async fn revoke_device(&mut self, params: RevokeDeviceParams) -> Result<ClientState>;
}
