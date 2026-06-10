# Nod Bootstrap Workflow

## Choose A Path

Use a prebuilt binary when the user wants the shortest path and does not need
container management. Use Docker when they already run services with Docker or
want a managed volume.

Nod is designed to stay private. Prefer localhost, Tailscale, or a private
reverse proxy with TLS and WebSocket upgrade support for `/api/v1/sync`.

## Path A: Prebuilt Binary

Download the right server archive from the latest release, unpack it, then start
with an admin token.

macOS or Linux:

```bash
tar -xzf nod-server-*.tar.gz
mkdir -p ~/nod
cd ~/nod
NOD_ADMIN_TOKEN=$(openssl rand -hex 24)
printf "%s\n" "$NOD_ADMIN_TOKEN" > admin-token.txt
chmod 600 admin-token.txt
NOD_ADMIN_TOKEN="$NOD_ADMIN_TOKEN" /path/to/nod-server
```

Windows PowerShell:

```powershell
Expand-Archive nod-server-*-x86_64-pc-windows-msvc.zip -DestinationPath $HOME\nod
cd $HOME\nod
$env:NOD_ADMIN_TOKEN = -join ((1..48) | ForEach-Object { '{0:x}' -f (Get-Random -Max 16) })
$env:NOD_ADMIN_TOKEN | Out-File -Encoding ascii admin-token.txt
.\nod-server.exe
```

State lives in `.nod/` next to the working directory unless `NOD_DATA_DIR` or
`NOD_DATABASE_URL` is configured.

## Path B: Docker

```bash
docker run -d --name nod \
  -e NOD_ADMIN_TOKEN=change-me \
  -p 127.0.0.1:8767:8767 \
  -v nod-data:/data \
  ghcr.io/batteryshark/nod-server:latest
```

For the repository Compose helper:

```bash
server/nod-server/scripts/nod-compose up -d --build
```

The helper creates ignored secret files when needed and stores server data in
the configured Docker volume.

## First Five Minutes

1. Open `http://localhost:8767/admin`.
2. Log in with the admin token.
3. Use the admin UI to create users and channels as needed. A `default` channel
   and `owner` user are seeded.
4. Mint an enrollment code for the user.
5. Open a Nod client, point it at the server URL, and enter the code.
6. Create an issuer token for the automation.

Admin API equivalents:

```bash
curl -sS -X POST "$NOD_BASE_URL/api/v1/admin/issuer-tokens" \
  -H "Authorization: Bearer $NOD_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"agents","scopes":["requests:write","requests:read"]}'
```

```bash
curl -sS -X POST "$NOD_BASE_URL/api/v1/admin/users/owner/enrollment-codes" \
  -H "Authorization: Bearer $NOD_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"expires_in_seconds":600}'
```

To create a channel:

```bash
curl -sS -X POST "$NOD_BASE_URL/api/v1/admin/channels" \
  -H "Authorization: Bearer $NOD_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":"deploys","name":"Deploys"}'
```

Issuer scopes can be broad or channel-specific:

- `requests:write`, `requests:read`, `requests:cancel`
- `requests:write:deploys`, `requests:read:deploys`,
  `requests:cancel:deploys`

Issuer tokens can only cancel their own pending requests and need cancel scope.

## Send The First Request

```bash
curl -sS -X POST "$NOD_BASE_URL/api/v1/requests" \
  -H "Authorization: Bearer $NOD_ISSUER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"channel_id":"default","title":"Deploy to prod?","summary":"v1.0.0 is ready","options":[{"id":"approve","label":"Ship it","kind":"approve"},{"id":"reject","label":"Hold","kind":"reject"}]}'
```

The request should appear on enrolled clients. The decision response is stored
on the server and can be read by the issuer.

## Verify A Deployment

```bash
curl -fsS "$NOD_BASE_URL/health"
server/nod-server/scripts/nod-smoke "$NOD_BASE_URL" "$NOD_ADMIN_TOKEN"
```

The smoke script enrolls a throwaway device, creates a request, receives it over
WebSocket sync, submits a signed decision, and removes what it created.

## Reaching Phones And Other Machines

The server speaks HTTP on port `8767`. Keep it private.

Tailscale example:

```bash
tailscale serve --bg --set-path /nod 8767
```

Use the resulting HTTPS URL when enrolling clients and configuring issuers.

Reverse proxy requirements:

- Terminate TLS at the proxy.
- Proxy to `127.0.0.1:8767`.
- Support WebSocket upgrade for `/api/v1/sync`.

## APNs And TestFlight

Everything except iOS background push can work without APNs if clients are
connected over WebSocket sync. iOS background and lock-screen notifications
need APNs configured with Apple Developer credentials.

There are two APNs routes:

- Direct APNs from the server with local Apple credentials.
- Standalone APNs relay over mTLS, keeping Apple credentials outside the main
  server process.

TestFlight, App Store, and Apple Developer Enterprise builds use Apple's
production App Attest environment. Do not describe APNs as providing private
content by itself. Use redacted request notifications for lock-screen safety.

