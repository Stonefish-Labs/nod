use anyhow::{anyhow, Result};

use crate::{
    api::NodApi,
    models::{DecisionSignature, Event, ServerProfile},
    signing::{DecisionSigningRequest, StoredSigningKey},
};

use super::NodClientRuntime;

pub(super) struct DecisionSignatureInput<'a> {
    pub event_id: &'a str,
    pub action_id: &'a str,
    pub text: Option<&'a str>,
}

struct SelectedServer {
    profile: ServerProfile,
    signing_key: Option<StoredSigningKey>,
}

impl NodClientRuntime {
    pub(super) async fn api(&self) -> Result<NodApi> {
        let profile = self.selected_server_profile().await?;
        let persisted = self.persisted.lock().await;
        let token = self.store.load_token(&persisted, &profile.id);

        NodApi::new(&profile.base_url_string, token)
    }

    pub(super) async fn decision_signature(
        &self,
        input: DecisionSignatureInput<'_>,
    ) -> Result<Option<DecisionSignature>> {
        let selected_server = self.selected_server_with_signing_key().await?;
        let Some(signing_key) = selected_server.signing_key else {
            return Ok(None);
        };
        let event = self.loaded_event(input.event_id).await?;
        let user_id = selected_server
            .profile
            .user_id
            .as_deref()
            .ok_or_else(|| anyhow!("selected server is missing user identity"))?;
        let device_id = selected_server
            .profile
            .device_id
            .as_deref()
            .ok_or_else(|| anyhow!("selected server is missing device identity"))?;

        signing_key
            .sign_decision(DecisionSigningRequest {
                event: &event,
                action_id: input.action_id,
                text: input.text,
                user_id,
                device_id,
            })
            .map(Some)
    }

    async fn selected_server_with_signing_key(&self) -> Result<SelectedServer> {
        let profile = self.selected_server_profile().await?;
        let persisted = self.persisted.lock().await;
        let signing_key = self.store.load_signing_key(&persisted, &profile.id);

        Ok(SelectedServer {
            profile,
            signing_key,
        })
    }

    async fn selected_server_profile(&self) -> Result<ServerProfile> {
        self.reducer
            .lock()
            .await
            .selected_server()
            .cloned()
            .ok_or_else(|| anyhow!("no selected server"))
    }

    async fn loaded_event(&self, event_id: &str) -> Result<Event> {
        self.reducer
            .lock()
            .await
            .state
            .events
            .iter()
            .find(|event| event.id == event_id)
            .cloned()
            .ok_or_else(|| anyhow!("event {event_id} is not loaded"))
    }
}
