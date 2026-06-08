use nod_client_core::models::Request;

pub(super) fn selected_id_after<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    current: Option<&str>,
    delta: isize,
) -> Option<String> {
    let ids: Vec<_> = ids.into_iter().collect();
    if ids.is_empty() {
        return None;
    }

    let current_index = current
        .and_then(|id| ids.iter().position(|candidate| *candidate == id))
        .unwrap_or_default();
    Some(ids[moved_index(current_index, ids.len(), delta)].to_string())
}

pub(super) fn moved_index(current: usize, count: usize, delta: isize) -> usize {
    let last_index = count.saturating_sub(1);
    current.saturating_add_signed(delta).min(last_index)
}

pub(super) fn previous_index(current: usize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    current.saturating_sub(1)
}

pub(super) fn next_index(current: usize, count: usize) -> usize {
    current.saturating_add(1).min(count.saturating_sub(1))
}

pub(super) fn request_matches(request: &Request, query: &str) -> bool {
    request.title.to_lowercase().contains(query)
        || request.summary.to_lowercase().contains(query)
        || request.body_markdown.to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_id_after_empty_list_returns_none() {
        assert_eq!(selected_id_after([], Some("current"), 1), None);
    }

    #[test]
    fn moved_index_stays_in_bounds() {
        assert_eq!(moved_index(0, 3, -1), 0);
        assert_eq!(moved_index(1, 3, 1), 2);
        assert_eq!(moved_index(2, 3, 1), 2);
    }
}
