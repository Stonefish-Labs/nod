use super::support::*;

#[tokio::test]
async fn user_can_manage_own_registered_devices() {
    let app = TestApp::new().await;
    app.create_user("paul", "Paul").await;
    app.create_user("maya", "Maya").await;
    let (phone_id, phone_token) = app
        .enroll_device_for_user("Paul's Phone", "ios", Some("paul"))
        .await;
    let (mac_id, mac_token) = app
        .enroll_device_for_user("Paul's MacBook", "macos", Some("paul"))
        .await;
    let (maya_id, _maya_token) = app
        .enroll_device_for_user("Maya's Phone", "ios", Some("maya"))
        .await;

    let (status, me) = app
        .request(Method::GET, "/api/v1/users/me", Some(&phone_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["user"]["id"], "paul");
    assert_eq!(me["current_device"]["id"], phone_id);
    assert_eq!(me["current_device"]["is_current"], true);

    let (status, listed) = app
        .request(
            Method::GET,
            "/api/v1/users/me/devices",
            Some(&phone_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["devices"].as_array().unwrap().len(), 2);
    assert!(listed["devices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|device| device["id"] == mac_id));
    assert!(!listed["devices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|device| device["id"] == maya_id));

    let (status, renamed) = app
        .request(
            Method::PUT,
            &format!("/api/v1/users/me/devices/{mac_id}"),
            Some(&phone_token),
            Some(json!({ "name": "Paul's Mac Studio" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(renamed["device"]["name"], "Paul's Mac Studio");
    assert_eq!(renamed["device"]["is_current"], false);

    let (status, forbidden_revoke) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/users/me/devices/{maya_id}"),
            Some(&phone_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{forbidden_revoke}");

    let (status, revoked_sibling) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/users/me/devices/{mac_id}"),
            Some(&phone_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{revoked_sibling}");
    let (status, rejected_sibling) = app
        .request(Method::GET, "/api/v1/sources", Some(&mac_token), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{rejected_sibling}");

    let (status, revoked_current) = app
        .request(
            Method::DELETE,
            &format!("/api/v1/users/me/devices/{phone_id}"),
            Some(&phone_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{revoked_current}");
    let (status, rejected_current) = app
        .request(Method::GET, "/api/v1/sources", Some(&phone_token), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{rejected_current}");
}

#[tokio::test]
async fn admin_can_delete_source_and_related_request_history() {
    let app = TestApp::new().await;
    let (_device_id, device_token) = app.enroll_device("Phone", "ios").await;
    let issuer_token = app.issuer_token().await;

    let (status, source) = app
        .request(
            Method::POST,
            "/api/v1/admin/sources",
            Some("admin-test-token"),
            Some(json!({
                "id": "accidental",
                "name": "Accidental",
                "icon": "bell",
                "color": "#ff5500"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{source}");

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(json!({
                "source_id": "accidental",
                "title": "Delete me",
                "summary": "Temporary",
                "options": [
                    { "id": "approve", "label": "Approve", "kind": "approve" },
                    { "id": "reject", "label": "Reject", "kind": "reject" }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let request_id = created["request_id"].as_str().unwrap();

    let (status, deleted) = app
        .request(
            Method::DELETE,
            "/api/v1/admin/sources/accidental",
            Some("admin-test-token"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{deleted}");

    let (status, sources) = app
        .request(
            Method::GET,
            "/api/v1/admin/sources",
            Some("admin-test-token"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{sources}");
    assert!(!sources["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["id"] == "accidental"));

    let (status, device_sources) = app
        .request(Method::GET, "/api/v1/sources", Some(&device_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{device_sources}");
    assert!(!device_sources["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["id"] == "accidental"));

    let (status, missing_request) = app
        .request(
            Method::GET,
            &format!("/api/v1/requests/{request_id}"),
            Some("admin-test-token"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing_request}");
}

#[tokio::test]
async fn deleting_missing_source_returns_not_found() {
    let app = TestApp::new().await;

    let (status, response) = app
        .request(
            Method::DELETE,
            "/api/v1/admin/sources/not-here",
            Some("admin-test-token"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{response}");
}

#[tokio::test]
async fn admin_can_create_test_request_without_issuer_token() {
    let app = TestApp::new().await;
    let (_device_id, device_token) = app.enroll_device("Phone", "ios").await;

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/admin/test-requests",
            Some("admin-test-token"),
            Some(json!({
                "source_id": "default",
                "title": "Admin test",
                "summary": "Created from the admin panel",
                "body_markdown": "**Admin** test body",
                "fields": [{ "label": "Environment", "value": "local" }],
                "links": [{ "label": "Runbook", "url": "https://example.com" }],
                "options": [
                    { "id": "approve_notes", "label": "Approve with notes", "kind": "approve_with_text", "text_placeholder": "Notes" },
                    { "id": "reject_reason", "label": "Reject with reason", "kind": "reject_with_text", "text_placeholder": "Reason" }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["deduped"], false);
    assert_eq!(created["request"]["title"], "Admin test");
    assert_eq!(created["request"]["options"][0]["requires_text"], true);
    assert_eq!(created["request"]["options"][1]["requires_text"], true);

    let (status, listed) = app
        .request(Method::GET, "/api/v1/requests", Some(&device_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["requests"][0]["id"], created["request_id"]);
}

#[tokio::test]
async fn enrollment_request_option_flow_updates_decision() {
    let app = TestApp::new().await;
    let (device_id, device_token) = app.enroll_device("Phone", "ios").await;
    let issuer_token = app.issuer_token().await;

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(json!({
                "source_id": "default",
                "title": "Deploy approval",
                "summary": "Approve production deploy?",
                "body_markdown": "**Production** deploy is waiting.",
                "options": [
                    { "id": "approve", "label": "Approve", "kind": "approve" },
                    {
                        "id": "reject_reason",
                        "label": "Reject with reason",
                        "kind": "reject_with_text",
                        "text_placeholder": "Why?"
                    }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let request_id = created["request_id"].as_str().unwrap();

    let (status, listed) = app
        .request(Method::GET, "/api/v1/requests", Some(&device_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["requests"].as_array().unwrap().len(), 1);

    let (status, resolved) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/options/approve"),
            Some(&device_token),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{resolved}");
    assert_eq!(resolved["request"]["status"], "resolved");
    assert_eq!(
        resolved["request"]["decision"]["actor_device_id"],
        device_id
    );

    let (status, decision) = app
        .request(
            Method::GET,
            &format!("/api/v1/requests/{request_id}/decision"),
            Some(&issuer_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{decision}");
    assert_eq!(decision["status"], "resolved");
    assert_eq!(decision["decision"]["option_id"], "approve");
}

#[tokio::test]
async fn v1_signed_decision_records_verified_signature() {
    let app = TestApp::new().await;
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
    let key_pair =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    let key_id = "test-signing-key";
    let public_key = URL_SAFE_NO_PAD.encode(key_pair.public_key().as_ref());

    let (status, enrollment) = app
        .request(
            Method::POST,
            "/api/v1/admin/users/owner/enrollment-codes",
            Some("admin-test-token"),
            Some(json!({ "expires_in_seconds": 600 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{enrollment}");
    let code = enrollment["code"].as_str().unwrap();
    let (status, enrolled) = app
        .request(
            Method::POST,
            "/api/v1/enroll",
            None,
            Some(json!({
                "code": code,
                "device_name": "Phone",
                "platform": "ios",
                "signing_key": {
                    "key_id": key_id,
                    "algorithm": "p256_ecdsa_sha256",
                    "public_key": public_key
                }
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{enrolled}");
    let device_id = enrolled["device_id"].as_str().unwrap();
    let device_token = enrolled["token"].as_str().unwrap();
    let issuer_token = app.issuer_token().await;

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(json!({
                "source_id": "default",
                "title": "Approve deploy",
                "summary": "Production deploy is waiting",
                "options": [{ "id": "approve", "label": "Approve", "kind": "approve" }]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let request_id = created["request_id"].as_str().unwrap();
    let request_digest = created["request"]["request_digest"].as_str().unwrap();
    let nonce = "nonce-1";
    let signed_at = "2026-05-31T12:00:00.000Z";
    let text = "ship it";
    let payload = decision_payload(DecisionPayload {
        request_id,
        request_digest,
        option_id: "approve",
        option_kind: "approve",
        user_id: "owner",
        device_id,
        key_id,
        nonce,
        signed_at,
        text,
    });
    let signature = URL_SAFE_NO_PAD.encode(key_pair.sign(&rng, payload.as_bytes()).unwrap());

    let (status, resolved) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/options/approve"),
            Some(device_token),
            Some(json!({
                "text": text,
                "signature": {
                    "key_id": key_id,
                    "algorithm": "p256_ecdsa_sha256",
                    "nonce": nonce,
                    "signed_at": signed_at,
                    "request_digest": request_digest,
                    "signature": signature
                }
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{resolved}");
    let decision = &resolved["request"]["decision"];
    assert_eq!(decision["signature"]["verified"], true);
    assert_eq!(decision["signature"]["request_digest"], request_digest);
    assert_eq!(decision["actor_device_id"], device_id);
}

#[tokio::test]
async fn per_user_decision_resolution_tracks_each_user_independently() {
    let app = TestApp::new().await;
    app.create_user("paul", "Paul").await;
    app.create_user("maya", "Maya").await;
    let (_paul_device_id, paul_token) = app
        .enroll_device_for_user("Paul's Phone", "ios", Some("paul"))
        .await;
    let (_maya_device_id, maya_token) = app
        .enroll_device_for_user("Maya's Phone", "ios", Some("maya"))
        .await;
    let issuer_token = app.issuer_token().await;

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(json!({
                "source_id": "default",
                "recipients": ["paul", "maya"],
                "decision_resolution": "per_user",
                "title": "Review",
                "summary": "Each user can decide",
                "options": [
                    { "id": "approve", "label": "Approve", "kind": "approve" },
                    { "id": "reject", "label": "Reject", "kind": "reject" }
                ]
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let request_id = created["request_id"].as_str().unwrap();

    let (status, paul_resolved) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/options/approve"),
            Some(&paul_token),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{paul_resolved}");
    assert_eq!(paul_resolved["request"]["status"], "resolved");
    assert_eq!(
        paul_resolved["request"]["decision"]["actor_user_id"],
        "paul"
    );

    let (status, paul_requests) = app
        .request(Method::GET, "/api/v1/requests", Some(&paul_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{paul_requests}");
    assert_eq!(paul_requests["requests"][0]["status"], "resolved");

    let (status, maya_requests) = app
        .request(Method::GET, "/api/v1/requests", Some(&maya_token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{maya_requests}");
    assert_eq!(maya_requests["requests"][0]["status"], "pending");

    let (status, aggregate) = app
        .request(
            Method::GET,
            &format!("/api/v1/requests/{request_id}/decision"),
            Some(&issuer_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{aggregate}");
    assert_eq!(aggregate["status"], "pending");
    assert_eq!(aggregate["decisions"].as_array().unwrap().len(), 1);
    assert_eq!(aggregate["pending_recipients"], json!(["maya"]));

    let (status, maya_resolved) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/options/reject"),
            Some(&maya_token),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{maya_resolved}");
    assert_eq!(
        maya_resolved["request"]["decision"]["actor_user_id"],
        "maya"
    );

    let (status, aggregate) = app
        .request(
            Method::GET,
            &format!("/api/v1/requests/{request_id}/decision"),
            Some(&issuer_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{aggregate}");
    assert_eq!(aggregate["status"], "resolved");
    assert_eq!(aggregate["decisions"].as_array().unwrap().len(), 2);
    assert_eq!(aggregate["pending_recipients"], json!([]));
}

#[tokio::test]
async fn device_can_dismiss_request_without_options() {
    let app = TestApp::new().await;
    let (_device_id, device_token) = app.enroll_device("Phone", "ios").await;
    let issuer_token = app.issuer_token().await;

    let (status, created) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(json!({
                "source_id": "default",
                "title": "FYI",
                "summary": "No decision needed",
                "body_markdown": "This card should be dismissible."
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["request"]["options"].as_array().unwrap().len(), 0);
    let request_id = created["request_id"].as_str().unwrap();

    let (status, resolved) = app
        .request(
            Method::POST,
            &format!("/api/v1/requests/{request_id}/options/dismiss"),
            Some(&device_token),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{resolved}");
    assert_eq!(resolved["request"]["status"], "resolved");
    assert_eq!(resolved["request"]["decision"]["option_id"], "dismiss");
    assert_eq!(resolved["request"]["decision"]["option_kind"], "dismiss");
    assert_eq!(resolved["request"]["decision"]["option_label"], "Dismiss");
}

#[tokio::test]
async fn request_list_limit_caps_handled_but_keeps_all_pending() {
    let app = TestApp::new().await;
    let (_device_id, device_token) = app.enroll_device("Phone", "ios").await;
    let issuer_token = app.issuer_token().await;

    for index in 0..3 {
        let (status, created) = app
            .request(
                Method::POST,
                "/api/v1/requests",
                Some(&issuer_token),
                Some(json!({
                    "source_id": "default",
                    "title": format!("Handled {index}"),
                    "summary": "Already handled"
                })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{created}");
        let request_id = created["request_id"].as_str().unwrap();
        let (status, resolved) = app
            .request(
                Method::POST,
                &format!("/api/v1/requests/{request_id}/options/dismiss"),
                Some(&device_token),
                Some(json!({})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{resolved}");
    }

    for index in 0..2 {
        let (status, created) = app
            .request(
                Method::POST,
                "/api/v1/requests",
                Some(&issuer_token),
                Some(json!({
                    "source_id": "default",
                    "title": format!("Pending {index}"),
                    "summary": "Needs attention"
                })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{created}");
    }

    let (status, listed) = app
        .request(
            Method::GET,
            "/api/v1/requests?limit=1",
            Some(&device_token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let requests = listed["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 3, "{listed}");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request["status"] == "pending")
            .count(),
        2
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request["status"] != "pending")
            .count(),
        1
    );
}

#[tokio::test]
async fn dedupe_returns_existing_pending_request() {
    let app = TestApp::new().await;
    let issuer_token = app.issuer_token().await;
    let payload = json!({
        "source_id": "default",
        "title": "Same thing",
        "summary": "Only one card",
        "dedupe_key": "agent:permission:123",
        "options": [{ "id": "approve", "label": "Approve", "kind": "approve" }]
    });

    let (status, first) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(payload.clone()),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (status, second) = app
        .request(
            Method::POST,
            "/api/v1/requests",
            Some(&issuer_token),
            Some(payload),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{second}");

    assert_eq!(first["request_id"], second["request_id"]);
    assert_eq!(second["deduped"], true);
}
