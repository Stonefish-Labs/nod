use anyhow::{anyhow, Result};

use crate::{
    api::{
        display_name_for, normalize_base_url, profile_id_for, EnrollDeviceRequest,
        SubmitActionRequest,
    },
    models::{ClientState, DevicePlatform, Event, EventStatus, ServerProfile, UserDevice},
    signing::StoredSigningKey,
};

use super::{
    session::DecisionSignatureInput, ChannelParams, EnrollParams, NodClientEvent, NodClientRuntime,
    RenameDeviceParams, RevokeDeviceParams, SelectEventParams, SetSubscriptionParams,
    SubmitActionParams,
};

const REFRESH_EVENT_LIMIT: usize = 500;

impl NodClientRuntime {
    pub async fn enroll(&mut self, params: EnrollParams) -> Result<ClientState> {
        let normalized_url = normalize_base_url(&params.base_url);
        let profile_id = profile_id_for(&normalized_url);
        let signing_key = StoredSigningKey::generate();
        let device_signing_key = signing_key.device_signing_key()?;
        let api = crate::api::NodApi::new(&normalized_url, None)?;
        let response = api
            .enroll(EnrollDeviceRequest {
                code: &params.code.trim().to_ascii_uppercase(),
                device_name: params.device_name.trim(),
                platform: params
                    .platform
                    .unwrap_or_else(DevicePlatform::current_desktop),
                signing_key: Some(&device_signing_key),
            })
            .await?;
        let profile = ServerProfile {
            id: profile_id.clone(),
            name: display_name_for(&normalized_url),
            base_url_string: normalized_url,
            device_name: params.device_name.trim().to_string(),
            device_id: Some(response.device_id.clone()),
            user_id: Some(response.user_id.clone()),
            user_name: Some(response.user_name.clone()),
        };

        {
            let mut persisted = self.persisted.lock().await;
            self.store
                .save_token(&mut persisted, &profile_id, &response.token)
                .await?;
            self.store
                .save_signing_key(&mut persisted, &profile_id, &signing_key)
                .await?;
            persisted.selected_server_id = Some(profile_id.clone());
            persisted.notification_sound = params
                .notification_sound
                .unwrap_or_else(|| persisted.notification_sound.clone());
            upsert_profile(&mut persisted.servers, profile.clone());
            self.store.save(persisted.clone()).await?;
        }

        {
            let mut reducer = self.reducer.lock().await;
            reducer.upsert_server(profile);
            reducer.set_selected_server(profile_id);
            reducer.state.channels = response.channels;
            reducer.state.devices = response.devices;
            reducer.state.is_registered = true;
            reducer.set_notification_delivery_mode(response.notification_delivery.mode);
        }

        self.refresh().await
    }

    pub async fn select_server(&mut self, server_id: String) -> Result<ClientState> {
        {
            let mut persisted = self.persisted.lock().await;
            if !persisted
                .servers
                .iter()
                .any(|server| server.id == server_id)
            {
                return Err(anyhow!("unknown server: {server_id}"));
            }
            persisted.selected_server_id = Some(server_id.clone());
            self.store.save(persisted.clone()).await?;
        }

        self.disconnect_sync().await;
        self.reducer.lock().await.set_selected_server(server_id);
        self.refresh().await?;
        self.connect_sync().await?;
        Ok(self.state().await)
    }

    pub async fn forget_server(&mut self, server_id: &str) -> Result<ClientState> {
        self.disconnect_sync().await;
        {
            let mut persisted = self.persisted.lock().await;
            persisted.servers.retain(|server| server.id != server_id);
            self.store.delete_token(&mut persisted, server_id).await?;
            self.store
                .delete_signing_key(&mut persisted, server_id)
                .await?;
            if persisted.selected_server_id.as_deref() == Some(server_id) {
                persisted.selected_server_id =
                    persisted.servers.first().map(|server| server.id.clone());
            }
            self.store.save(persisted.clone()).await?;
        }

        self.reducer.lock().await.remove_server(server_id);
        self.emit_state().await;
        Ok(self.state().await)
    }

    pub async fn refresh(&mut self) -> Result<ClientState> {
        let api = self.api().await?;
        let current_user = api.current_user().await?;
        let mut devices = api.devices().await?;
        if !devices
            .iter()
            .any(|device| device.id == current_user.current_device.id)
        {
            devices.insert(0, current_user.current_device.clone());
        }

        let channels = api.channels().await?;
        let events = api.events(None, Some(REFRESH_EVENT_LIMIT)).await?;
        let candidates = {
            let mut reducer = self.reducer.lock().await;
            reducer.set_notification_delivery_mode(current_user.notification_delivery.mode);
            reducer.apply_refresh(Some(current_user.user), devices, channels, events)
        };
        self.emit_notifications(candidates).await;
        self.emit_state().await;
        Ok(self.state().await)
    }

    pub async fn submit_action(&mut self, params: SubmitActionParams) -> Result<Event> {
        let text = params.text.as_deref().and_then(trimmed_text);
        let signature = self
            .decision_signature(DecisionSignatureInput {
                event_id: &params.event_id,
                action_id: &params.action_id,
                text,
            })
            .await?;
        let event = self
            .api()
            .await?
            .submit_action(SubmitActionRequest {
                event_id: &params.event_id,
                action_id: &params.action_id,
                text,
                signature: signature.as_ref(),
            })
            .await?;
        self.reducer.lock().await.apply_event_update(event.clone());
        if event.status != EventStatus::Pending {
            self.emit_event(NodClientEvent::NotificationRemoved {
                event_id: event.id.clone(),
            })
            .await;
        }
        self.emit_state().await;
        Ok(event)
    }

    pub async fn clear_channel(&mut self, params: ChannelParams) -> Result<ClientState> {
        self.api().await?.clear_channel(&params.channel_id).await?;
        self.refresh().await
    }

    pub async fn set_subscription(&mut self, params: SetSubscriptionParams) -> Result<ClientState> {
        self.api()
            .await?
            .set_subscription(&params.channel_id, params.subscribed)
            .await?;
        self.refresh().await
    }

    pub async fn set_notification_preference(
        &mut self,
        notification_sound: &str,
    ) -> Result<ClientState> {
        self.api()
            .await?
            .set_notification_sound(notification_sound)
            .await?;
        {
            let mut persisted = self.persisted.lock().await;
            persisted.notification_sound = notification_sound.to_string();
            self.store.save(persisted.clone()).await?;
        }
        self.reducer.lock().await.state.notification_sound = notification_sound.to_string();
        self.emit_state().await;
        Ok(self.state().await)
    }

    pub async fn list_devices(&mut self) -> Result<Vec<UserDevice>> {
        let devices = self.api().await?.devices().await?;
        self.reducer.lock().await.state.devices = devices.clone();
        self.emit_state().await;
        Ok(devices)
    }

    pub async fn rename_device(&mut self, params: RenameDeviceParams) -> Result<UserDevice> {
        let device = self
            .api()
            .await?
            .rename_device(&params.device_id, &params.name)
            .await?;
        let mut reducer = self.reducer.lock().await;
        if let Some(existing) = reducer
            .state
            .devices
            .iter_mut()
            .find(|existing| existing.id == device.id)
        {
            *existing = device.clone();
        }
        drop(reducer);
        self.emit_state().await;
        Ok(device)
    }

    pub async fn revoke_device(&mut self, params: RevokeDeviceParams) -> Result<ClientState> {
        self.api().await?.revoke_device(&params.device_id).await?;
        self.refresh_or_forget_current(&params.device_id).await
    }

    pub async fn select_channel(&mut self, params: ChannelParams) -> Result<ClientState> {
        self.reducer.lock().await.state.selected_channel_id = Some(params.channel_id);
        self.refresh().await
    }

    pub async fn select_event(&mut self, params: SelectEventParams) -> Result<ClientState> {
        self.reducer.lock().await.state.selected_event_id = Some(params.event_id);
        self.emit_state().await;
        Ok(self.state().await)
    }

    async fn refresh_or_forget_current(&mut self, revoked_device_id: &str) -> Result<ClientState> {
        let current_was_revoked = self
            .reducer
            .lock()
            .await
            .selected_server()
            .and_then(|server| server.device_id.as_deref())
            == Some(revoked_device_id);
        if current_was_revoked {
            let server_id = {
                self.reducer
                    .lock()
                    .await
                    .selected_server()
                    .map(|server| server.id.clone())
            };
            if let Some(server_id) = server_id {
                return self.forget_server(&server_id).await;
            }
        }
        self.refresh().await
    }
}

fn upsert_profile(profiles: &mut Vec<ServerProfile>, profile: ServerProfile) {
    if let Some(existing) = profiles
        .iter_mut()
        .find(|existing| existing.id == profile.id)
    {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
}

fn trimmed_text(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
