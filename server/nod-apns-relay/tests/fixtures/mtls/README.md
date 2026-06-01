# mTLS Test Fixtures

The certificate and key files in this directory are generated locally and
ignored by git so private-key material never lands in the repository.

Run this from the repository root before running tests that need the APNs relay
mTLS fixtures:

```bash
server/nod-apns-relay/tests/fixtures/mtls/generate
```
