# Relay TLS Test Fixtures

The certificate and key files in this directory are generated locally and
ignored by git so private-key material never lands in the repository.

Run this from the repository root before running tests that need relay TLS
identity fixtures:

```bash
server/nod-server/crates/nod-server/tests/fixtures/relay-tls/generate
```
