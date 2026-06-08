use anyhow::{anyhow, Result};

use crate::{
    api::NodApi,
    models::{DecisionSignature, Request, ServerProfile},
    signing::{DecisionSigningRequest, StoredSigningKey},
};

use super::NodClientRuntime;

pub(super) struct DecisionSignatureInput<'a> {
    pub request_id: &'a str,
    pub option_id: &'a str,
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
        let request = self.loaded_request(input.request_id).await?;
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
                request: &request,
                option_id: input.option_id,
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

    async fn loaded_request(&self, request_id: &str) -> Result<Request> {
        self.reducer
            .lock()
            .await
            .state
            .requests
            .iter()
            .find(|request| request.id == request_id)
            .cloned()
            .ok_or_else(|| anyhow!("request {request_id} is not loaded"))
    }
}
