use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    models::{
        CardField, CardLink, Decision, DecisionRequest, DecisionResolution,
        DecisionSignatureRecord, OptionKind, RequestNotification, RequestOption, RequestStatus,
        UserDecision,
    },
    signing,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RequestView {
    pub id: String,
    pub request_id: String,
    pub source_id: String,
    pub recipients: Vec<String>,
    pub decision_resolution: DecisionResolution,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub fields: Vec<CardField>,
    pub links: Vec<CardLink>,
    pub image_url: Option<String>,
    pub notification: RequestNotification,
    pub dedupe_key: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub decision: Option<DecisionView>,
    pub decisions: Vec<UserDecisionView>,
    pub callback_url: Option<String>,
    pub options: Vec<RequestOption>,
    pub request_digest: Option<String>,
}

impl RequestView {
    pub(crate) fn from_request(request: &DecisionRequest) -> Self {
        Self {
            id: request.id.clone(),
            request_id: request.id.clone(),
            source_id: request.source_id.clone(),
            recipients: request.recipients.clone(),
            decision_resolution: request.decision_resolution.clone(),
            title: request.title.clone(),
            summary: request.summary.clone(),
            body_markdown: request.body_markdown.clone(),
            fields: request.fields.clone(),
            links: request.links.clone(),
            image_url: request.image_url.clone(),
            notification: request.notification.clone(),
            dedupe_key: request.dedupe_key.clone(),
            expires_at: request.expires_at,
            status: request.status.clone(),
            created_at: request.created_at,
            updated_at: request.updated_at,
            resolved_at: request.resolved_at,
            decision: request.decision.as_ref().map(DecisionView::from),
            decisions: request
                .user_decisions
                .iter()
                .map(UserDecisionView::from)
                .collect(),
            callback_url: request.callback_url.clone(),
            options: request.options.clone(),
            request_digest: signing::request_digest(request).ok(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RequestDecisionView {
    pub request_id: String,
    pub status: RequestStatus,
    pub decision: Option<DecisionView>,
    pub decisions: Vec<UserDecisionView>,
    pub decision_resolution: DecisionResolution,
    pub recipients: Vec<String>,
    pub pending_recipients: Vec<String>,
    pub request_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_out: Option<bool>,
}

impl RequestDecisionView {
    pub(crate) fn from_request(request: &DecisionRequest) -> Self {
        Self {
            request_id: request.id.clone(),
            status: request.status.clone(),
            decision: request.decision.as_ref().map(DecisionView::from),
            decisions: request
                .user_decisions
                .iter()
                .map(UserDecisionView::from)
                .collect(),
            decision_resolution: request.decision_resolution.clone(),
            recipients: request.recipients.clone(),
            pending_recipients: pending_recipients(request),
            request_digest: signing::request_digest(request).ok(),
            timed_out: None,
        }
    }

    pub(crate) fn mark_timed_out(&mut self) {
        self.timed_out = Some(true);
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CallbackPayload {
    pub request_id: String,
    pub source_id: String,
    pub status: RequestStatus,
    pub decision: Option<DecisionView>,
    pub decisions: Vec<UserDecisionView>,
    pub decision_resolution: DecisionResolution,
}

impl CallbackPayload {
    pub(crate) fn from_request(request: &DecisionRequest) -> Self {
        let decision_view = RequestDecisionView::from_request(request);
        Self {
            request_id: decision_view.request_id,
            source_id: request.source_id.clone(),
            status: decision_view.status,
            decision: decision_view.decision,
            decisions: decision_view.decisions,
            decision_resolution: decision_view.decision_resolution,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DecisionView {
    pub request_id: String,
    pub option_id: String,
    pub option_kind: OptionKind,
    pub option_label: String,
    pub text: Option<String>,
    pub actor_user_id: Option<String>,
    pub actor_device_id: Option<String>,
    pub signature: Option<DecisionSignatureRecord>,
    pub resolved_at: DateTime<Utc>,
}

impl From<&Decision> for DecisionView {
    fn from(decision: &Decision) -> Self {
        Self {
            request_id: decision.request_id.clone(),
            option_id: decision.option_id.clone(),
            option_kind: decision.option_kind.clone(),
            option_label: decision.option_label.clone(),
            text: decision.text.clone(),
            actor_user_id: decision.actor_user_id.clone(),
            actor_device_id: decision.actor_device_id.clone(),
            signature: decision.signature.clone(),
            resolved_at: decision.resolved_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UserDecisionView {
    pub user_id: String,
    pub decision: DecisionView,
}

impl From<&UserDecision> for UserDecisionView {
    fn from(decision: &UserDecision) -> Self {
        Self {
            user_id: decision.user_id.clone(),
            decision: DecisionView::from(&decision.decision),
        }
    }
}

fn pending_recipients(request: &DecisionRequest) -> Vec<String> {
    if request.decision_resolution != DecisionResolution::PerUser
        || request.status != RequestStatus::Pending
    {
        return Vec::new();
    }
    request
        .recipients
        .iter()
        .filter(|user_id| {
            !request
                .user_decisions
                .iter()
                .any(|decision| &decision.user_id == *user_id)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::models::{Decision, DecisionResolution, OptionKind, RequestStatus};

    #[test]
    fn callback_payload_uses_the_decision_projection() {
        let now = Utc::now();
        let decision = Decision {
            request_id: "request-1".to_string(),
            option_id: "approve".to_string(),
            option_kind: OptionKind::Approve,
            option_label: "Approve".to_string(),
            text: Some("ship it".to_string()),
            actor_user_id: Some("owner".to_string()),
            actor_device_id: Some("device-1".to_string()),
            signature: None,
            resolved_at: now,
        };
        let request = DecisionRequest {
            id: "request-1".to_string(),
            source_id: "deploys".to_string(),
            recipients: vec!["owner".to_string()],
            decision_resolution: DecisionResolution::Shared,
            title: "Deploy".to_string(),
            summary: "Deploy is waiting".to_string(),
            body_markdown: String::new(),
            fields: Vec::new(),
            links: Vec::new(),
            image_url: None,
            notification: Default::default(),
            dedupe_key: None,
            expires_at: None,
            status: RequestStatus::Resolved,
            created_at: now,
            updated_at: now,
            resolved_at: Some(now),
            decision: Some(decision),
            user_decisions: Vec::new(),
            callback_url: None,
            options: Vec::new(),
        };

        let callback = serde_json::to_value(CallbackPayload::from_request(&request)).unwrap();
        let projection = serde_json::to_value(RequestDecisionView::from_request(&request)).unwrap();

        assert_eq!(callback["decision"], projection["decision"]);
        assert_eq!(callback["decisions"], projection["decisions"]);
        assert_eq!(
            callback["decision_resolution"],
            projection["decision_resolution"]
        );
    }
}
