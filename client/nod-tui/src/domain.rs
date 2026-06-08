use nod_client_core::models::{
    ClientState, OptionKind, Request, RequestOption, RequestStatus, Source,
};

pub fn total_pending_count(state: &ClientState) -> usize {
    state.pending_counts_by_source.values().sum()
}

pub fn pending_count_for(source: &Source, state: &ClientState) -> usize {
    state
        .pending_counts_by_source
        .get(&source.id)
        .copied()
        .unwrap_or_default()
}

pub fn subscribed_sources(state: &ClientState) -> Vec<&Source> {
    state
        .sources
        .iter()
        .filter(|source| source.subscribed)
        .collect()
}

pub fn ordered_requests(requests: &[Request]) -> Vec<&Request> {
    let mut ordered: Vec<_> = requests.iter().collect();
    ordered.sort_by(|left, right| {
        status_rank(&left.status)
            .cmp(&status_rank(&right.status))
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });
    ordered
}

pub fn selected_source(state: &ClientState) -> Option<&Source> {
    state
        .selected_source_id
        .as_deref()
        .and_then(|id| state.sources.iter().find(|source| source.id == id))
        .or_else(|| state.sources.iter().find(|source| source.subscribed))
        .or_else(|| state.sources.first())
}

pub fn selected_request(state: &ClientState) -> Option<&Request> {
    state
        .selected_request_id
        .as_deref()
        .and_then(|id| state.requests.iter().find(|request| request.id == id))
        .or_else(|| ordered_requests(&state.requests).into_iter().next())
}

pub fn selected_server_id(state: &ClientState) -> Option<&str> {
    state
        .selected_server_id
        .as_deref()
        .or_else(|| state.servers.first().map(|server| server.id.as_str()))
}

pub fn option_requires_text(option: &RequestOption) -> bool {
    option.requires_text
        || matches!(
            option.kind,
            OptionKind::ApproveWithText | OptionKind::RejectWithText
        )
}

pub fn option_for_kind<'a>(request: &'a Request, kind: OptionKind) -> Option<OptionChoice<'a>> {
    request
        .options
        .iter()
        .find(|option| option.kind == kind)
        .map(OptionChoice::from_option)
        .or_else(|| default_dismiss_option(request, &kind))
}

pub fn first_text_option(request: &Request) -> Option<OptionChoice<'_>> {
    request
        .options
        .iter()
        .find(|option| option_requires_text(option))
        .map(OptionChoice::from_option)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionChoice<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub placeholder: Option<&'a str>,
    pub requires_text: bool,
}

impl<'a> OptionChoice<'a> {
    fn from_option(option: &'a RequestOption) -> Self {
        Self {
            id: &option.id,
            label: &option.label,
            placeholder: option.text_placeholder.as_deref(),
            requires_text: option_requires_text(option),
        }
    }
}

fn default_dismiss_option<'a>(request: &'a Request, kind: &OptionKind) -> Option<OptionChoice<'a>> {
    if *kind != OptionKind::Dismiss || !request.options.is_empty() {
        return None;
    }

    // The core signer recognizes this implicit option for optionless requests,
    // so the TUI can offer a consistent dismiss key without server metadata.
    Some(OptionChoice {
        id: "dismiss",
        label: "Dismiss",
        placeholder: None,
        requires_text: false,
    })
}

fn status_rank(status: &RequestStatus) -> u8 {
    match status {
        RequestStatus::Pending => 0,
        RequestStatus::Resolved => 1,
        RequestStatus::Expired => 1,
        RequestStatus::Cancelled => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{client_state, request_with_status};
    use nod_client_core::models::RequestStatus;

    #[test]
    fn ordered_requests_keep_pending_before_handled() {
        let resolved = request_with_status("resolved", "default", RequestStatus::Resolved);
        let pending = request_with_status("pending", "default", RequestStatus::Pending);
        let requests = vec![resolved, pending];

        let ordered = ordered_requests(&requests);

        assert_eq!(ordered[0].id, "pending");
        assert_eq!(ordered[1].id, "resolved");
    }

    #[test]
    fn selected_request_falls_back_to_first_ordered_request() {
        let mut state = client_state();
        state.requests = vec![
            request_with_status("handled", "default", RequestStatus::Resolved),
            request_with_status("pending", "default", RequestStatus::Pending),
        ];

        let selected = selected_request(&state).map(|request| request.id.as_str());

        assert_eq!(selected, Some("pending"));
    }
}
