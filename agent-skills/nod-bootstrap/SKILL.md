---
name: nod-bootstrap
description: Help a user stand up, configure, enroll, and verify a self-hosted Nod instance from binaries or Docker while explaining issuer tokens, channels, clients, APNs, and smoke tests accurately.
---

# Nod Bootstrap

Use this skill when an agent is helping someone install or verify a self-hosted
Nod server.

## Workflow

1. Choose the deployment path:
   - Prebuilt binary for the simplest laptop, desktop, or small-box install.
   - Docker or Compose when the user already operates services that way.
2. Create and protect the admin token.
3. Start the server privately on `127.0.0.1:8767` or behind a private tunnel.
4. Open `/admin`, create users/channels if needed, and mint enrollment codes.
5. Enroll clients against the server URL.
6. Create issuer tokens with the least scopes needed by each automation.
7. Send a first request to the `default` channel.
8. Verify health, sync, decision signing, and issuer read/wait behavior.
9. Explain APNs honestly:
   - macOS, Windows, and TUI can receive while connected over WebSocket sync.
   - iOS foreground sync works while connected.
   - iOS background and lock-screen push needs APNs.
   - Do not claim content-private push relay or fully private notification
     content unless the current deployment is actually configured that way.

## References

- Read `references/bootstrap-workflow.md` for command templates and exact
  operator steps.
- For request payload design after bootstrap, use the `nod-notification-author`
  skill folder.

## Output Standard

When helping a user bootstrap Nod, produce:

- The chosen path and why it fits.
- Commands with placeholders for secrets.
- The server URL clients and issuers should use.
- A first request curl command.
- Verification steps and the APNs caveat for mobile background push.

