# Workstreams

## nod-proto centralization (wire types + signing crypto)

- Status: completed
- Goal: One Rust crate owning the wire DTOs and the canonical decision-signing crypto (`request_digest`, `decision_signing_payload`, P-256 sign/verify), depended on by the server and every Rust client.
- Current state: `nod-proto/` exists; server + `nod-client-core` + TUI + desktop-tauri rewired onto it. Standardized on pure-Rust `p256` (off `ring`) so it cross-compiles to Apple.
- Current evidence: `cargo test --workspace` → 20 suites green; frozen vectors in `nod-proto/src/signing.rs`.
- Recent learning: pure-Rust `p256` was the enabler for the later UniFFI cross-compile.
- Next action: none (stable).
- Stop condition: done.
- Related paths: `nod-proto/`, `server/nod-server/crates/nod-server/src/signing.rs`, `client/nod-client-core/src/signing.rs`.

## Frozen protocol-freeze corpus + client digest recompute

- Status: completed
- Goal: Lock the canonical signing bytes so a refactor can't silently invalidate existing signatures; let clients independently recompute the digest (defense-in-depth).
- Current state: Frozen vectors for `request_digest` (`97e2edc5…`) and `decision_signing_payload` (`text_sha256:bef4261f…`). Client recomputes + fails on mismatch.
- Related paths: `nod-proto/src/signing.rs` (tests), `nod-proto-ffi/src/lib.rs` (frozen vector through the FFI), `client/nod-client-core/src/signing.rs`.

## #9 UniFFI — share signing crypto into Swift

- Status: completed
- Goal: One Rust implementation of the canonical signing bytes behind Swift; no parallel Swift reimplementation.
- Current state: `nod-proto-ffi` crate exposes `request_digest`/`decision_signing_payload`/`verify_payload`/`validate_public_key` via UniFFI; xcframework built (universal arm64+x86_64 macOS/sim + arm64 device); `NodSigningKeyStore` builds the payload through `NodProtoFFI`. Secure Enclave still signs.
- Current evidence: `swift test` 24 green; `xcodebuild` NodMac + NodIOS succeed; `cargo test -p nod-proto-ffi` 4 green.
- Related paths: `nod-proto-ffi/`, `client/nod-apple/scripts/build-nod-proto-ffi.sh`, `client/nod-apple/Package.swift`, `client/nod-apple/Sources/NodKit/NodSigningKeyStore.swift`.

## #11 Relay — library split + in-process provider

- Status: completed
- Goal: Co-located (single-box) APNs delivery with no HTTP/mTLS hop, keeping the standalone relay for scale-out.
- Current state: `InProcessApnsProvider` embeds the relay's `AppleApnsProvider`; remote `ApnsRelayProvider` keeps mTLS. Duplicate wire DTOs (server ↔ relay) collapsed onto the relay crate via a shared `build_relay_request`. Route renamed `RemotePushRoute -> PushRoute` (`ApnsRelay`|`ApnsDirect`); selection by config presence, both-configured is a hard error.
- Current evidence: workspace tests green; clippy + fmt clean.
- Related paths: `server/nod-server/crates/nod-server/src/{push.rs,apns_relay.rs}`, `server/nod-apns-relay/src/{lib.rs,relay.rs}`, `ARCHITECTURE_NOTES.md` §3.

## #10 source → channel rename

- Status: completed (Rust + Swift); desktop-frontend gap found and fixed under #8
- Goal: Consistent domain naming everywhere.
- Current state: Rust + Swift verified (NodKit/NodMac/NodIOS build + `swift test`). The desktop **TS frontend** had not been renamed (still `source`) while its backend emitted `channel` — fixed during #8 (incl. `SourceSubscriptions.tsx -> ChannelSubscriptions.tsx`).
- Related paths: server `db/channels.rs`, `models/channel.rs`; `client/nod-desktop/src/**`.

## #8 typeshare codegen (Swift + TypeScript)

- Status: completed
- Goal: Generate client types from the Rust source of truth so they can't drift — pursued *only where it makes the code clearer*.
- Final state: `#[typeshare]` annotations on `nod-proto` wire types + `nod-client-core` view types; `scripts/generate-types.sh` emits a **git-ignored** `client/nod-desktop/src/dto/generated.ts` to **diff** against the hand-written `src/dto/models.ts`. Clients import the hand-written types, not the generated ones; Swift is not generated. The real win: the desktop `source→channel` frontend bug was fixed (incl. `SourceSubscriptions.tsx -> ChannelSubscriptions.tsx`).
- Why not full adoption: tried it; typeshare's optional-everything (from `#[serde(default)]`) forced defensive `?.` across the UI, and snake_case Swift / TS enums added friction — net noisier, failing the user's "easier to maintain / less churn" bar. **Reverted.** (See decisions.md.)
- Evidence: `tsc` clean + 16 vitest tests; `cargo` workspace green with the inert `typeshare` deps.
- Stop condition: met — Rust is the documented contract, drift is caught by the generated diff, clients read cleanly, desktop is green.
- Related paths: `nod-proto/src/{request,decision,notification}.rs`, `client/nod-client-core/src/models.rs`, `scripts/generate-types.sh`, `client/nod-desktop/src/dto/`.

## Commit the milestone

- Status: completed
- Goal: Land the centralization diff.
- Current state: Committed as `f9714dd Hard Reboot` on `main` (the ~118-file bulk), with the final #8 typeshare-resolution tweaks + the `.project/` tracker committed alongside. Not pushed (push only when asked).
- Next action: none (push on request).
- Related paths: whole repo.
