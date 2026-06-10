# Changelog

## v1.0.0 — first public release

- Self-hosted decision server: structured requests, channels, multi-user /
  multi-device enrollment, shared and per-user resolution, expiration,
  cancellation, dedupe, callbacks, audit logs, embedded admin panel.
- Decisions signed on-device with P-256 keys (Secure Enclave on Apple
  hardware), verified against the immutable request snapshot, with nonce
  replay rejection.
- Clients: native macOS app (notarized DMG), iOS via TestFlight, Windows
  desktop app (Tauri, unsigned MSI), terminal UI.
- Apple push via an in-process APNs route or a credential-isolating mTLS
  relay; WebSocket sync + local notifications everywhere else.
- Self-contained server binaries for macOS/Linux/Windows and a Docker image
  on GHCR.

Full notes: [docs/release-notes/v1.0.0.md](docs/release-notes/v1.0.0.md)
