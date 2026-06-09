# Project Tracker — Nod

## Current State

- Current objective: Before publishing, make the system *make sense* — collapse protocol/crypto/type duplication across the Rust server, Rust clients, Swift apps, and TS desktop into single sources of truth, and finish the `source -> channel` rename everywhere.
- Build state: **All green.** Rust workspace (20 test suites); Swift `NodMac` + `NodIOS` build, `swift test` 24; desktop `tsc` clean + 16 vitest tests pass.
- Status: Centralization milestone **committed** as `f9714dd Hard Reboot` (on `b6765f4 Rework`); the final #8 typeshare-resolution tweaks (`models.ts`, `generate-types.sh`, `.gitignore`) + this tracker land alongside it. All 12 workstreams resolved. #8 full literal typeshare adoption was **evaluated and reverted** in this tree (typeshare's optional-everything types + snake_case Swift made the client code noisier, not cleaner — see decisions.md); landed on hand-written client types + a typeshare drift-diff + the desktop `source→channel` bug fix.
- Latest session: [2026-06-09-1241](sessions/2026-06-09-1241-session.md)
- Main constraints: "Burn the boats" — no backcompat/migration/legacy code (see `.claude` memory). Security is paramount (decision signing). typeshare/UniFFI output idioms clash with hand-written client idioms.
- Key learning: Xcode 26.5 + Swift 6.3.2 are available locally — Swift/iOS builds and tests can be run here directly. typeshare 1.13 has no camelCase-Swift / TS-union / `usize` support, which shaped the adoption strategy.
- Immediate next action: None blocking — centralization is committed (`Hard Reboot`); this tracker + the final #8 tweaks land on top. Push only when explicitly asked.

## Active Workstreams

- [completed] #8 typeshare codegen — Rust-side contract + git-ignored drift-diff projection + desktop `source→channel` bug fix. Full literal adoption evaluated and **rejected** (made client code noisier; failed the maintainability bar).
- [completed] #9 UniFFI crypto into Swift, #11 relay in-process provider, #10 source→channel rename, #1–#7 nod-proto centralization, #12 Docker build.
- See [workstreams.md](workstreams.md).

## Important Paths

- `nod-proto/`: single source of truth for wire types + decision-signing crypto (P-256, frozen vectors).
- `nod-proto-ffi/`: UniFFI crate exposing the signing contract to Swift (one Rust impl, no Swift reimplementation).
- `server/nod-apns-relay/`: relay library + bin; `ApnsDelivery`/`RelayPolicy`/`AppleApnsProvider`.
- `server/nod-server/crates/nod-server/`: axum server; `push.rs` (PushRoute), `apns_relay.rs` (remote + in-process providers).
- `client/nod-client-core/`: shared Rust client; `models.rs` view types serialized to the desktop frontend.
- `client/nod-apple/`: Swift `NodKit` lib + `NodMac`/`NodIOS`; `NodSigningKeyStore` signs via UniFFI.
- `client/nod-desktop/`: Tauri desktop — `src-tauri/` (Rust) + `src/` (React/TS, consumes `nod-client-core` view types over IPC).
- `scripts/generate-types.sh`, `client/nod-apple/scripts/build-nod-proto-ffi.sh`: codegen + xcframework build.
- `ARCHITECTURE_NOTES.md`: mini-ADRs (gitignored).

## Recent Changes

- 2026-06-09: #8 typeshare infra wired + desktop `source→channel` frontend bug fixed (was broken against the renamed backend). Tracker created.
- 2026-06-09: #9 UniFFI signing shared into Swift NodKit; #11 relay in-process provider + collapsed duplicate wire DTOs; #10 Swift rename verified across NodKit/NodMac/NodIOS.

## Current Questions And Ideas

- Resolved: `skip_serializing_if` + full TS adoption was tried and reverted — the optional-everything types forced defensive `?.` noise across the UI (worse, not better). Rust stays the contract; clients keep hand-written types reconciled by a typeshare drift-diff.
- Idea: Release-hosted `.xcframework` (URL + checksum binaryTarget) so Swift builds without a Rust toolchain.
- Evaluation (later): Is NodKit too Swift-heavy? Push most client *logic* into `nod-client-core` (UniFFI) and keep Swift only for Secure Enclave / SwiftUI / notifications / attestation. See [ideas.md](ideas.md) + [questions.md](questions.md).

## Navigation

- [Goals](goals.md)
- [Workstreams](workstreams.md)
- [Attempts](attempts.md)
- [Artifacts](artifacts.md)
- [Decisions](decisions.md)
- [Learnings](learnings.md)
- [Ideas](ideas.md)
- [Questions](questions.md)
- [Theories](theories.md)
- [Next Actions](next-actions.md)
- [Sessions](sessions/)
