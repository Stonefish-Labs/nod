# Nod APNs Relay

Standalone mTLS Apple APNs relay for Nod. It accepts APNs relay requests from
`server/nod-server` and keeps Apple credentials outside the main server.

## Endpoints

- `GET /health`
- `POST /v1/notifications`

All endpoints are HTTPS-only and require a trusted client certificate.
Notification targets must include `native_app_id`, and the relay rejects
requests whose native app id does not match its configured APNs bundle id.

## Configuration

```bash
NOD_APNS_RELAY_BIND=127.0.0.1:8768
NOD_APNS_RELAY_SERVER_CERT_PATH=/secrets/relay-server.crt
NOD_APNS_RELAY_SERVER_KEY_PATH=/secrets/relay-server.key
NOD_APNS_RELAY_CLIENT_CA_CERT_PATH=/secrets/relay-client-ca.crt

NOD_APNS_RELAY_TEAM_ID=...
NOD_APNS_RELAY_KEY_ID=...
NOD_APNS_RELAY_BUNDLE_ID=com.yourname.Nod
NOD_APNS_RELAY_PRIVATE_KEY_PATH=/secrets/AuthKey_....p8
NOD_APNS_RELAY_ENVIRONMENT=production
```

The server certificate must include a SAN matching the hostname used by
`NOD_APNS_RELAY_URL` in `server/nod-server`. Client certificates must chain to
`NOD_APNS_RELAY_CLIENT_CA_CERT_PATH`.

Run locally:

```bash
cargo run
```
