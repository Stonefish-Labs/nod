use super::support::*;

#[tokio::test]
async fn issuer_token_can_cancel_own_pending_request() {
    let app = TestApp::new().await;
    let owner_token = app
        .issuer_token_with_scopes([
            "requests:write:default",
            "requests:read:default",
            "requests:cancel:default",
        ])
        .await;
    let read_write_token = app
        .issuer_token_with_scopes(["requests:write:default", "requests:read:default"])
        .await;
    let other_cancel_token = app
        .issuer_token_with_scopes(["requests:read:default", "requests:cancel:default"])
        .await;

    let payload = json!({
        "channel_id": "default",
        "title": "Cancel me",
        "summary": "No longer needed",
        "dedupe_key": "cancel:pending:123",
        "options": [{ "id": "approve", "label": "Approve", "kind": "approve" }]
    });
    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&owner_token),
            Some(payload.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let request_id = created["request_id"].as_str().unwrap();

    let (status, rejected) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/cancel"),
            Some(&read_write_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{rejected}");

    let (status, rejected) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/cancel"),
            Some(&other_cancel_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{rejected}");

    let (status, cancelled) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/cancel"),
            Some(&owner_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["request"]["status"], "cancelled");
    assert_eq!(cancelled["request"]["decision"], Value::Null);

    let (status, decision) = app
        .request(
            Method::GET,
            &format!("/api/v1/requests/{request_id}/decision"),
            Some(&owner_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{decision}");
    assert_eq!(decision["status"], "cancelled");

    let (status, duplicate) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&owner_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{duplicate}");
    assert_eq!(duplicate["deduped"], false);
    assert_ne!(duplicate["request_id"], request_id);
}

#[tokio::test]
async fn admin_can_cancel_any_pending_request() {
    let app = TestApp::new().await;
    let issuer_token = app.issuer_token().await;
    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(json!({
                "channel_id": "default",
                "title": "Admin cancel",
                "summary": "Withdrawn by admin"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let request_id = created["request_id"].as_str().unwrap();

    let (status, cancelled) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/cancel"),
            Some("admin-test-token"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["request"]["status"], "cancelled");

    let (status, rejected) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/cancel"),
            Some("admin-test-token"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{rejected}");
}

#[tokio::test]
async fn issuer_token_can_be_scoped_to_one_channel() {
    let app = TestApp::new().await;
    let scoped_token = app
        .issuer_token_with_scopes([
            "requests:write:steam-wishlist-notifier",
            "requests:read:steam-wishlist-notifier",
        ])
        .await;

    let (status, channel) = app
        .request(
            Method::POST,
            "/api/v1/admin/channels",
            Some("admin-test-token"),
            Some(json!({
                "id": "steam-wishlist-notifier",
                "name": "Steam Wishlist Notifier",
                "emoji": "🎮"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{channel}");

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&scoped_token),
            Some(json!({
                "channel_id": "steam-wishlist-notifier",
                "title": "Wishlist test",
                "summary": "Allowed channel"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");

    let request_id = created["request_id"].as_str().unwrap();
    let (status, decision) = app
        .request(
            Method::GET,
            &format!("/api/v1/requests/{request_id}/decision"),
            Some(&scoped_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{decision}");

    let (status, rejected) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&scoped_token),
            Some(json!({
                "channel_id": "default",
                "title": "Wrong channel",
                "summary": "Should not be allowed"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{rejected}");
}
