# Decisions

## nod-proto is the single source of truth (workspace crate)

- Date: prior session
- Rationale: The server and four client surfaces had drifted (e.g., the server verified with `ring`, clients signed with `p256`; wire types duplicated). One crate in a single Cargo workspace means one `Cargo.lock`, one dependency graph, and a protocol change becomes one mechanical edit.
- Tradeoffs: All consumers must live in (or depend on) the workspace; adds a shared crate boundary.
- Rejected alternatives: Per-repo copies kept in sync by convention (the status quo that drifted).
- Reconsideration trigger: If a client must ship independently of the workspace.

## Standardize signing on pure-Rust `p256` (off `ring`)

- Date: prior session
- Rationale: `p256` cross-compiles cleanly to the Apple targets, enabling the same crypto in Swift via UniFFI. `from_sec1_bytes` also does full point validation (stricter than the old `ring` path).
- Reconsideration trigger: A measured perf gap that matters.

## "Burn the boats" — no backcompat / migration / legacy code

- Date: prior session (persisted in `.claude` memory `nod-burn-the-boats-no-backcompat`)
- Rationale: Pre-launch greenfield; no deployed clients or DBs. Legacy-name handling and fallbacks are "ghosts" that waste code and mislead.
- Consequences: Deleted `reset_legacy_config`, retired-column migrations, and legacy-rejection tests (Rust + the Swift twin `testLegacyServerContractKeysDoNotDecode`). Lenient decoding (no `deny_unknown_fields` on inbound wire types) chosen over strict.
- Reconsideration trigger: First real external client ships.

## #9 UniFFI lives in ONE crate (`nod-client-ffi`) — SUPERSEDED the dedicated `nod-proto-ffi`

- Date: 2026-06-09
- Decision: There is exactly one UniFFI crate / xcframework / Swift module (`nod-client-ffi` → `nod_client_ffiFFI.xcframework` → `NodClientFFI`). It exposes both the decision-signing contract (direct `nod-proto` dep) and the client logic (via `nod-client-core`). `nod-proto` itself stays a pure protocol crate (no `uniffi`).
- History: First built as a dedicated `nod-proto-ffi` crate (signing only). When `nod-client-ffi` was added for client logic, having TWO UniFFI xcframeworks broke the `.xcodeproj` app build — Xcode flattens each binaryTarget's `Headers/module.modulemap` into one `include/` and they collide ("Multiple commands produce module.modulemap"). `swift build` tolerates it; the app build does not. Resolution: deleted `nod-proto-ffi`, merged its surface into `nod-client-ffi`. Burn-the-boats — no second FFI crate kept around.
- Tradeoffs: the one FFI crate links the whole `nod-client-core` dep tree (tokio/reqwest/keyring) even for code that only needs signing. Acceptable: it's an Apple-only artifact and the app links it all anyway.
- Rule going forward: NEVER add a second UniFFI crate/xcframework — extend `nod-client-ffi`. (See learnings: "Two UniFFI xcframeworks collide … RESOLVED by one FFI crate".)
- Rejected alternatives: per-module header subdirs in each xcframework (fragile clang module resolution); feature-gate `uniffi` inside `nod-proto` (pollutes the canonical crate).

## #9 xcframework is built from source, not committed

- Date: 2026-06-09
- Rationale: The universal static archive is large; building from source (via `build-nod-client-ffi.sh`) is leaner and more auditable for crypto (review source, not a binary blob).
- Tradeoffs: A fresh checkout needs the Rust toolchain + script before the Swift app builds.
- Rejected alternatives: Commit the binary (repo bloat); leave unbuilt (breaks `swift build`).
- Reconsideration trigger: Cutting releases → switch to a release-hosted URL+checksum `binaryTarget` (see ideas).

## #9 generated UniFFI Swift shim builds in Swift 5 language mode

- Date: 2026-06-09
- Rationale: UniFFI 0.28's generated `var initializationResult` violates Swift 6 strict concurrency. Per-target `.swiftLanguageMode(.v5)` isolates the shim; NodKit and the apps stay Swift 6.

## #11 Relay route selected by config presence, mutually exclusive

- Date: 2026-06-09
- Rationale: Local Apple creds (`NOD_APNS_DIRECT_*`) → in-process; relay URL + mTLS (`NOD_APNS_RELAY_*`) → remote. Configuring both is a hard startup error, not a silent precedence rule.
- Tradeoffs: In-process mode means the server process holds the APNs `.p8` (acceptable for single-operator/single-box; user chose this over keeping the relay always isolated).

## #8 typeshare: Rust is the contract; clients keep hand-written types (full adoption evaluated and REJECTED)

- Date: 2026-06-09 (final)
- Decision: `#[typeshare]` annotations stay on the Rust wire types (nod-proto) + view types (nod-client-core). `scripts/generate-types.sh` emits a **git-ignored** `client/nod-desktop/src/dto/generated.ts` whose only job is to be **diffed** against the hand-written `src/dto/models.ts` to catch drift. The clients import the **hand-written** types, not the generated ones. Swift is not generated at all.
- Why not import the generated types (this is the load-bearing learning): typeshare's output idioms clash with clean client code, and the clash makes the code *worse*, not better —
  - It maps every `#[serde(default)]` field (which exists for *lenient decode*) to a TS **optional**, but the backend *always* populates those fields. Importing the generated types forces defensive `?.` / `?? []` at every `request.fields`, `request.links`, … across the components — more noise, not less.
  - It emits TS `enum`s where the UI compares string literals, and **snake_case Swift** that clashes with NodKit's camelCase `Codable`.
  - The hand-written `models.ts` encodes the *practical* contract (always-present fields are required) → clean direct access. That is a better frontend contract than the literal Rust projection.
- Evaluated and rejected: **full literal adoption** — `models.ts` re-exports `generated.ts` + `#[serde(skip_serializing_if="Option::is_none")]` so JSON omits nulls (→ matches typeshare `undefined`) + enum-member call sites. It was actually tried in this tree and **reverted**: it made the desktop noisier and failed the user's explicit bar ("only if it makes the code easier to maintain / more understandable with less churn").
- The real #8 win: the desktop frontend was **broken** — still `source`/`source_id` while the rewired backend emits `channel`. Completed source→channel across the frontend (incl. `SourceSubscriptions.tsx` → `ChannelSubscriptions.tsx`); `tsc` clean, 16 vitest tests pass.
- Reconsideration trigger: typeshare gains camelCase Swift + TS string-unions + `| null` for options; or the team decides the defensive-access cost is worth a single imported source of truth (then do `skip_serializing_if` + import `generated.ts`).

## Apple apps move onto `nod-client-core` (logic) — keep SwiftUI (native UI)

- Date: 2026-06-09
- Decision: Kill the biggest remaining duplication — NodKit re-implements the API client, store, sync, request/notification rules, and models that `nod-client-core` already owns. Move that *logic* into `nod-client-core` (shared by TUI + desktop + Apple). **Keep SwiftUI**: the dedup target is the logic, not the UI; two thin renderers (SwiftUI + React) over one Rust core preserves native quality.
- Two decisions deliberately separated: (a) dedup the logic → **YES**; (b) drop SwiftUI / full-Tauri-on-Apple → **NO** (only buys one fewer thin UI at the cost of native QoL). They are independent.
- Mechanism: the single `nod-client-ffi` UniFFI crate wraps `nod-client-core` AND carries the signing contract (it absorbed `nod-proto-ffi` — see #9). Desktop keeps `nod-client-core` as a direct Rust dep (no UniFFI). Hexagonal ports-and-adapters: the core defines capability ports; each host provides adapters. The existing `NodClientRuntime` + `RpcRequest`/`NodClientMessage` is the seam to expose.
- Progress: (1) scaffolded + iOS-cross-compile-proven; the FIRST swap is LIVE — `NodServerAddress` delegates to `NodClientFFI` (Rust), `NodSigningKeyStore` builds the payload via `NodClientFFI`. (2) The async `NodClientRuntime` is now exposed + proven from Swift — `NodClient` (UniFFI, `async_runtime = "tokio"`) with a `NodClientObserver` foreign callback and a JSON `request`/`start` surface; a Swift test drives it end to end. (3) The Secure-Enclave `DeviceSigner` port is in: nod-client-core has a `DeviceSigner` trait + one `build_decision_signature` path + a `ForeignSigner` port + `SignerBackend::{Software,Foreign}` (TUI/desktop stay on software keys, unchanged); `NodClient::new(observer, signer)` mandates a `NodDeviceSigner` callback so Apple signs in hardware with no software fallback. A fake-SE signature verifies via `nod_proto`. NodMac + NodIOS build green throughout. Remaining for full migration: NodKit implements `NodDeviceSigner` over `SecureEnclave.P256`, routes enroll/submit/sync/state through `NodClient`, and deletes the duplicated Swift client.
- Genuinely-Swift adapters (fewer than first listed): Secure Enclave signer, App Attest, UserNotifications + APNs token, SwiftUI. **Keychain stays in Rust** — `nod-client-core` already uses the `keyring` crate with `apple-native`.
- Cost/risk (honest): the *hard* part of UniFFI — async (tokio↔Swift), a stateful interface object, foreign-trait callbacks (SE/attest/notify), a state-observer bridge, a bigger xcframework, iOS lifecycle (Swift drives connect/disconnect; APNs covers background). Proven in industry (Signal/1Password embed Rust+tokio on iOS) but a multi-step project, not one task.
- Incremental path (each shippable): (1) move the API client behind the FFI; (2) move sync + state store (Swift becomes a thin observer); (3) NodKit = SwiftUI + 3 adapters. Started by scaffolding `nod-client-ffi`.
- Reconsideration trigger: if maintaining SwiftUI stops being worth its native polish → revisit full Tauri (needs no UniFFI: src-tauri uses `nod-client-core` directly).
