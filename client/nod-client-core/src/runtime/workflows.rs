use anyhow::{anyhow, Result};

use crate::{
    api::{
        display_name_for, normalize_base_url, profile_id_for, EnrollDeviceRequest,
        SubmitOptionRequest,
    },
    models::{
        ClientState, DevicePlatform, DeviceSigningKey, Request, RequestStatus, ServerProfile,
        UserDevice,
    },
    signing::{DeviceSigner, StoredSigningKey},
};

use super::{
    session::DecisionSignatureInput, EnrollParams, NodClientMessage, NodClientRuntime,
    RegisterPushTokenParams, RenameDeviceParams, RevokeDeviceParams, SelectRequestParams,
    SetSubscriptionParams, ChannelParams, SignerBackend, SubmitOptionParams,
};

const REFRESH_EVENT_LIMIT: usize = 500;

impl NodClientRuntime {
    pub async fn enroll(&mut self, params: EnrollParams) -> Result<ClientState> {
        let normalized_url = normalize_base_url(&params.base_url);
        let profile_id = profile_id_for(&normalized_url);
        // Provision the device key from the active backend: a software key the
        // store will persist, or a Secure Enclave key the host already holds.
        let (device_signing_key, software_key) =
            self.provision_device_signing_key(&profile_id)?;
        let api = crate::api::NodApi::new(&normalized_url, None)?;
        let response = api
            .enroll(EnrollDeviceRequest {
                code: &params.code.trim().to_ascii_uppercase(),
                device_name: params.device_name.trim(),
                platform: params
                    .platform
                    .unwrap_or_else(DevicePlatform::current_desktop),
                native_app_id: params.native_app_id.as_deref(),
                push_provider: params.push_provider.as_deref(),
                push_token: params.push_token.as_deref(),
                signing_key: Some(&device_signing_key),
                attestation: params.attestation.as_ref(),
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
            // Only the software backend persists a private key; the Secure
            // Enclave key stays in the host's hardware, never in the store.
            if let Some(software_key) = &software_key {
                self.store
                    .save_signing_key(&mut persisted, &profile_id, software_key)
                    .await?;
            }
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
        // Drop the host-held hardware key (no-op for the software backend, whose
        // key lives in the store and is cleared below). Done outside the store
        // lock so the foreign callback isn't held across the mutex.
        if let SignerBackend::Foreign(backend) = self.signer_backend() {
            backend.remove(server_id)?;
        }
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
        let requests = api.requests(None, Some(REFRESH_EVENT_LIMIT)).await?;
        let candidates = {
            let mut reducer = self.reducer.lock().await;
            reducer.set_notification_delivery_mode(current_user.notification_delivery.mode);
            reducer.apply_refresh(Some(current_user.user), devices, channels, requests)
        };
        self.emit_notifications(candidates).await;
        self.emit_state().await;
        Ok(self.state().await)
    }

    pub async fn submit_option(&mut self, params: SubmitOptionParams) -> Result<Request> {
        let text = params.text.as_deref().and_then(trimmed_text);
        let signature = self
            .decision_signature(DecisionSignatureInput {
                request_id: &params.request_id,
                option_id: &params.option_id,
                text,
            })
            .await?;
        let request = self
            .api()
            .await?
            .submit_option(SubmitOptionRequest {
                request_id: &params.request_id,
                option_id: &params.option_id,
                text,
                signature: signature.as_ref(),
            })
            .await?;
        self.reducer
            .lock()
            .await
            .apply_request_update(request.clone());
        if request.status != RequestStatus::Pending {
            self.emit_message(NodClientMessage::NotificationRemoved {
                request_id: request.id.clone(),
            })
            .await;
        }
        self.emit_state().await;
        Ok(request)
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

    /// Register/refresh the APNs push token across every enrolled server (the
    /// same token applies to all). Mirrors NodKit's `registerPushToken`.
    pub async fn register_push_token(
        &mut self,
        params: RegisterPushTokenParams,
    ) -> Result<ClientState> {
        let servers = { self.persisted.lock().await.servers.clone() };
        for server in &servers {
            let token = {
                let persisted = self.persisted.lock().await;
                self.store.load_token(&persisted, &server.id)
            };
            let api = crate::api::NodApi::new(&server.base_url_string, token)?;
            api.update_push_token(&params.provider, &params.native_app_id, &params.token)
                .await?;
        }
        self.refresh().await
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

    pub async fn select_request(&mut self, params: SelectRequestParams) -> Result<ClientState> {
        self.reducer.lock().await.state.selected_request_id = Some(params.request_id);
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

    /// Provision the device signing key for a newly enrolling profile. Returns
    /// the public `DeviceSigningKey` to register with the server, plus the
    /// software key to persist (`Some` for the software backend, `None` for the
    /// Secure Enclave — its key never leaves the host's hardware).
    fn provision_device_signing_key(
        &self,
        profile_id: &str,
    ) -> Result<(DeviceSigningKey, Option<StoredSigningKey>)> {
        match self.signer_backend() {
            SignerBackend::Software => {
                let key = StoredSigningKey::generate();
                let device_signing_key = key.device_signing_key()?;
                Ok((device_signing_key, Some(key)))
            }
            SignerBackend::Foreign(backend) => {
                let provisioned = backend.provision(profile_id)?;
                let device_signing_key = DeviceSigningKey {
                    key_id: provisioned.key_id,
                    algorithm: nod_proto::DECISION_SIGNING_ALGORITHM.to_string(),
                    public_key: provisioned.public_key,
                };
                Ok((device_signing_key, None))
            }
        }
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
