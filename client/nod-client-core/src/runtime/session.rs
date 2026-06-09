use anyhow::{anyhow, Result};

use crate::{
    api::NodApi,
    models::{DecisionSignature, Request, ServerProfile},
    signing::{
        build_decision_signature, DecisionSigningRequest, DeviceSigner, ForeignDeviceSigner,
    },
};

use super::{NodClientRuntime, SignerBackend};

pub(super) struct DecisionSignatureInput<'a> {
    pub request_id: &'a str,
    pub option_id: &'a str,
    pub text: Option<&'a str>,
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
        let profile = self.selected_server_profile().await?;
        let Some(signer) = self.device_signer_for(&profile).await? else {
            return Ok(None);
        };
        let request = self.loaded_request(input.request_id).await?;
        let user_id = profile
            .user_id
            .as_deref()
            .ok_or_else(|| anyhow!("selected server is missing user identity"))?;
        let device_id = profile
            .device_id
            .as_deref()
            .ok_or_else(|| anyhow!("selected server is missing device identity"))?;

        build_decision_signature(
            signer.as_ref(),
            DecisionSigningRequest {
                request: &request,
                option_id: input.option_id,
                text: input.text,
                user_id,
                device_id,
            },
        )
        .map(Some)
    }

    /// Resolve the device signer for a profile from whichever backend is active.
    /// `None` means the profile has no key and its decisions cannot be signed.
    pub(super) async fn device_signer_for(
        &self,
        profile: &ServerProfile,
    ) -> Result<Option<Box<dyn DeviceSigner>>> {
        match self.signer_backend() {
            SignerBackend::Software => {
                let persisted = self.persisted.lock().await;
                Ok(self
                    .store
                    .load_signing_key(&persisted, &profile.id)
                    .map(|key| Box::new(key) as Box<dyn DeviceSigner>))
            }
            SignerBackend::Foreign(backend) => {
                let Some(key) = backend.signing_key(&profile.id)? else {
                    return Ok(None);
                };
                Ok(Some(Box::new(ForeignDeviceSigner {
                    backend: backend.clone(),
                    profile_id: profile.id.clone(),
                    key,
                }) as Box<dyn DeviceSigner>))
            }
        }
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
