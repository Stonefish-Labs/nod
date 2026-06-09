# Learnings

## Xcode + Swift toolchain is available locally

- Date: 2026-06-09
- Status: active
- Learned: Xcode 26.5 / Swift 6.3.2 / all SDKs are installed. `swift build`, `swift test`, and `xcodebuild` for NodMac and the iOS simulator all run here. (An earlier claim that Xcode wasn't available was wrong.)
- Evidence: `xcodebuild -version` → Xcode 26.5; `swift test` → 24 tests; `xcodebuild -scheme NodIOS -sdk iphonesimulator … CODE_SIGNING_ALLOWED=NO` → BUILD SUCCEEDED.
- Implication: Swift/iOS correctness can be verified directly; no need to hand verification to the user.
- Related paths: `client/nod-apple/`.

## typeshare 1.13 output idioms

- Date: 2026-06-09
- Status: active
- Learned: typeshare (1) has no camelCase mode — Swift properties come out snake_case; (2) emits TS `enum`s, not string-literal unions; (3) maps `Option<T>` to `?: T` (TS `undefined`), not `| null`; (4) rejects `usize` ("Unsupported type"). chrono fields need `#[typeshare(serialized_as = "String")]`; maps need `serialized_as = "HashMap<String, u32>"`.
- Evidence: Generated Swift used `request_id`/`option_kind`; generated TS used `export enum` + `field?: string`; `usize` errored at `models.rs:134`.
- Implication (decided): Importing the generated types is a **net negative**. typeshare marks every `#[serde(default)]` field optional, but the backend always populates them — so the UI would need defensive `?.` / `?? []` everywhere (noisier, not clearer). So the generated types are a **drift-diff reference**, not imported; clients keep hand-written types that encode the practical (always-present) contract. Full literal adoption + `skip_serializing_if` was tried in this tree and **reverted** (see decisions.md).
- Related paths: `nod-proto/src/{request,decision,notification}.rs`, `client/nod-client-core/src/models.rs`.

## The desktop TS frontend consumes nod-client-core view types over Tauri IPC

- Date: 2026-06-09
- Status: active
- Learned: The desktop frontend does not decode server JSON directly — it decodes `nod-client-core`'s view types (`ClientState`, `Channel`, `ServerProfile`, `UserDevice`) serialized across the Tauri boundary. So the TS typeshare source is `nod-client-core` (+ `nod-proto` for embedded wire types), while the Swift source is `nod-proto`.
- Evidence: `dto/models.ts` mirrors `ClientState`/`ServerProfile`; the IPC types are defined in `client/nod-client-core/src/models.rs` (not in `src-tauri`).
- Implication: "typeshare for Swift + TS" is two sources, not one annotate-and-generate.

## The desktop frontend had a real source→channel drift bug

- Date: 2026-06-09
- Status: active
- Learned: The src-tauri Rust backend was renamed to `channel` (`pending_counts_by_channel`, `select_channel`), but the TS frontend still used `source`/`source_id` (66 hits / 9 files). So the desktop was broken against its own backend — the #10 rename never reached the frontend.
- Evidence: `grep` for `channel`/`source` in `src-tauri` vs `src`; the rename + `tsc` fixed the field/type mismatches.
- Implication: Renames must cover the TS frontend, not just Rust + Swift. typeshare adoption would have prevented this.

## The Swift signing private key never leaves the Secure Enclave

- Date: 2026-06-09
- Status: active
- Learned: NodKit signs with a Secure Enclave P-256 key (non-exportable), so the *signing* must stay in Swift. What is drift-prone and worth sharing is the *canonical byte construction* (`request_digest`, `decision_signing_payload`) — which is what UniFFI now provides.
- Evidence: `NodSigningKeyStore.swift` uses `SecureEnclave.P256.Signing.PrivateKey`; payload now built by `NodClientFFI` (the single consolidated FFI module — see the resolved modulemap-collision entry).
- Implication: UniFFI's value here is "what gets signed + verification," not "do the signing."

## The relay was already a library; the duplicate wire contract was the risk

- Date: 2026-06-09
- Status: active
- Learned: `nod-apns-relay` already exposed a lib + `ApnsDelivery` trait. The real drift risk was a hand-maintained copy of the relay wire request in BOTH the server and the relay. Collapsing it onto the relay crate (shared `build_relay_request`) removed the duplication.
- Related paths: `server/nod-server/crates/nod-server/src/apns_relay.rs`, `server/nod-apns-relay/src/relay.rs`.

## UniFFI xcframework needs universal slices for the iOS simulator

- Date: 2026-06-09
- Status: active
- Learned: An arm64-only xcframework links for `swift build` (Apple Silicon macOS) but fails `xcodebuild -scheme NodIOS` because the iOS target also builds x86_64 simulator. Fix: lipo arm64+x86_64 for the macOS and simulator slices (iOS device stays arm64).
- Evidence: `ld: warning: ignoring file … found architecture 'arm64', required architecture 'x86_64'` → resolved after universal slices.
- Related paths: `client/nod-apple/scripts/build-nod-proto-ffi.sh`.

## nod-client-core cross-compiles to iOS; its address helpers match NodKit byte-for-byte

- Date: 2026-06-09
- Status: active
- Learned (the key de-risk for "Apple apps onto nod-client-core"): `nod-client-ffi` (wrapping the *whole* `nod-client-core` — tokio, reqwest/rustls, keyring `apple-native`) **cross-compiles cleanly to `aarch64-apple-ios`**. And the first shared logic exposed (`normalize_base_url` / `profile_id_for` / `display_name_for`) produces **identical output to NodKit's hand-written `NodServerAddress`** (parity test). So the big-dep-tree-on-iOS risk is retired and the first swap is behavior-preserving.
- Evidence: `cargo build -p nod-client-ffi --target aarch64-apple-ios` → Finished; `nod-client-ffi` test `matches_nodkit_server_address_vectors` passes.
- Implication: the remaining work (async `NodClientRuntime` + foreign-trait callbacks + xcframework + NodKit swaps) is now known-feasible engineering, not an open risk. Start the NodKit swaps with `NodServerAddress` (pure, parity-proven), then the API client.
- Related paths: `nod-client-ffi/`, `client/nod-client-core/src/api.rs`.

## Full RPC parity: every NodKit NodAPI method now maps to a runtime RPC (no capability gaps)

- Date: 2026-06-09
- Status: active
- Learned: enumerating NodKit's `NodAPI` surface against the runtime's RPC methods is the way to prove the migration is capability-complete. The mapping: enroll→enroll, currentUser/channels/requests→refresh, devices→list_devices, renameDevice→rename_device, revokeDevice→revoke_device, submit→submit_option, clear→clear_channel, updateSubscription→set_subscription, updateDevicePreferences→set_notification_preference, websocketURL→connect_sync. After enroll parity, the ONLY remaining gap was `updatePushToken` (refreshing the APNs token on already-enrolled devices) — now closed with an `update_push_token` API method + a `register_push_token` RPC that mirrors NodKit's behavior (updates EVERY enrolled server, since the same APNs token applies to all). `NodRuntimeClient.registerPushToken` exposes it.
- Implication: there are no capability gaps left between NodKit's hand-written client and the shared runtime. Deleting `NodAPI`/`NodStore*`/`SyncClient` is now purely the mechanical view-rewire — no missing behavior to discover mid-cut.
- Evidence: `cargo test --workspace` 150 green; NodKit builds; NodMac/NodIOS green.
- Related paths: `client/nod-client-core/src/api.rs` (`update_push_token`), `client/nod-client-core/src/runtime/{rpc,workflows}.rs` (`register_push_token`), `client/nod-apple/Sources/NodKit/NodRuntimeClient.swift`.

## Enroll parity: the runtime's enroll now carries App Attest + push (the real migration blocker)

- Date: 2026-06-09
- Status: active
- Learned (why NodKit's enroll couldn't just move onto the runtime): NodKit's `register` does MORE than nod-client-core's `enroll` — it attaches App Attest attestation and the APNs push token / provider / native_app_id. The runtime's enroll only sent code+device_name+platform+signing_key, so deleting NodKit's enroll would have REGRESSED App Attest and push (a security regression). That capability gap — not the view wiring — was the true blocker to deleting the duplicated Swift client.
- Fix (purely additive, no server change): the server's enroll endpoint ALREADY accepts `native_app_id`/`push_provider`/`push_token`/`attestation`; only the client was under-sending. Extended `EnrollDeviceRequest` (api), `EnrollParams` (rpc), and `enroll` (workflows) to thread them through. Attestation is forwarded as an opaque `serde_json::Value` — nod-client-core doesn't parse it; the canonical attestation shape stays the server's contract. App Attest is a *pre-enroll* native step (host builds the blob, passes it in the enroll params), NOT a mid-flow foreign callback like signing — so no new callback was needed. Swift `NodRuntimeClient.enroll` gained the matching optional params.
- Implication: the runtime enroll is now at feature parity with NodKit's. The view rewire + deletion of `NodAPI`/`NodStore*`/`SyncClient` is now unblocked on capability grounds — the app can enroll (with attestation+push), submit (SE-signed), and sync entirely through `NodRuntimeClient`.
- Evidence: `cargo test --workspace` 150 green; the `EnrollDeviceRequest` serialization test still holds (new fields `skip_serializing_if = none`); NodKit builds; NodMac/NodIOS green.
- Related paths: `client/nod-client-core/src/api.rs`, `client/nod-client-core/src/runtime/rpc.rs`, `client/nod-client-core/src/runtime/workflows.rs`, `client/nod-apple/Sources/NodKit/NodRuntimeClient.swift`.

## nod-client-core silently dropped device attestation + has_signing_key (latent bug, fixed)

- Date: 2026-06-09
- Status: active
- Learned: comparing NodKit's `NodUserDevice` to nod-client-core's `UserDevice` surfaced a real data-loss bug — the **server emits** `has_signing_key: bool` and `attestation: DeviceAttestationSummary` on every device, NodKit captured them, but **nod-client-core's `UserDevice` omitted both**, so the TUI + desktop could never show signing-key / attestation status, and `ClientState.devices` would have failed to decode into NodKit after the migration. The drift was invisible because serde silently ignores unknown fields.
- Fix (single-source the wire types): moved `DeviceAttestationSummary` + `DeviceAttestationStatus` into `nod-proto` (typeshare'd, `verified_at` → `Option<String>`); the server now re-exports them (`pub use nod_proto::…`, its server-only record/verification types stay local); nod-client-core's `UserDevice` gained `has_signing_key` + `attestation` (`#[serde(default)]` for forward-safety). One definition, server + clients + generated Swift/TS aligned.
- Lesson: when two sides hand-maintain a "the same" wire type, diff them field-by-field before bridging — `#[serde(default)]`/ignore-unknown hides drift until something downstream needs the missing field. typeshare's drift-diff is the guard, but only if the source type is complete.
- Also fixed while proving populated decode: NodKit's runtime bridge was decoding with a plain `JSONDecoder()`; the wire models have ISO-8601 `Date` fields, so it MUST use `JSONDecoder.nod` (the custom date strategy). Empty state hid it; a populated state would have thrown.
- Evidence: `cargo test --workspace` 150 green; Swift `NodRuntimeStateDecodingTests` decodes a fully-populated `ClientState` (device with verified attestation + signing key, ISO dates, request with options) into `NodRuntimeState`. NodMac/NodIOS green.
- Related paths: `nod-proto/src/attestation.rs`, `server/.../models/attestation.rs`, `client/nod-client-core/src/models.rs`, `client/nod-apple/Sources/NodKit/NodRuntimeState.swift`.

## NodKit's runtime bridge: NodRuntimeClient drives nod-client-core, decodes ClientState

- Date: 2026-06-09
- Status: active
- Learned: NodKit can consume the shared runtime's view state directly. `NodRuntimeClient` (`@MainActor ObservableObject`) owns a `NodClient`, forwards `NodClientMessage` events onto the main actor as `@Published` state, and sends RPCs via `client.request(json)` — the same JSON-RPC the desktop speaks. `NodRuntimeState` is a Codable mirror of `ClientState` that REUSES NodKit's existing wire models (`NodChannel`/`NodUser`/`NodUserDevice`/`NodRequest`, which already have snake_case keys matching the shared `nod-proto` shapes); only `ServerProfile` needed a dedicated snake_case type (`NodRuntimeServerProfile`) because NodKit's persisted `NodServerProfile` is camelCase. `NodRuntimeMessage` decodes the `{kind,payload}` envelope.
- Swift 6 concurrency gotchas: UniFFI 0.28 does NOT mark generated objects `Sendable`, but they wrap thread-safe Rust (`Arc<Mutex>`), so `extension NodClient: @unchecked Sendable {}` (plain, NOT `@retroactive` — same SwiftPM package) is correct and required to `await` its methods off the main actor. The injected signer must be typed `any NodDeviceSigner & Sendable` (the SE signer + test stubs are `@unchecked Sendable`). The observer callback fires on Rust's pump thread → an `@unchecked Sendable` bridge decodes and hops to `@MainActor`.
- Evidence: `NodRuntimeClientTests` starts the real runtime, the `ready`+`state` envelopes decode into `NodRuntimeState` on the main actor (fresh hermetic store → not registered, no servers), and a serverless `refresh` round-trips as a thrown `NodRuntimeError.rpc`. NodMac + NodIOS green.
- Remaining for #16 (the last big step): rewire the 9 SwiftUI views from `NodStore` to `NodRuntimeClient`, then DELETE the duplicated Swift client (`NodAPI`, `NodStore*`, `SyncClient`, and the now-unused model surface). This is mechanical but touches every view — a focused dedicated pass.
- Related paths: `client/nod-apple/Sources/NodKit/{NodRuntimeClient,NodRuntimeState}.swift`.

## NodKit's Secure Enclave signer satisfies the Rust verify contract (migration keystone)

- Date: 2026-06-09
- Status: active
- Learned: the one genuinely-native adapter the whole Apple-onto-core migration hinges on — Secure Enclave signing — is implemented and *proven against Rust*. `SecureEnclaveDeviceSigner` (NodKit) implements the FFI `NodDeviceSigner` callback over NodKit's existing `NodSigningKeyStore`, keyed by server profile id (account `decisionSigningKey.<profileId>`). The runtime builds the canonical payload in Rust and hands raw bytes to `sign(profileId:payload:)`; Swift only does the hardware signature. Added two store helpers: `existingSigningKey(account:)` (load-only, for "is this enrolled?") and `signPayload(_:account:)` (sign already-canonical bytes).
- Evidence (the proof that matters): `SecureEnclaveDeviceSignerTests` — provision → sign canonical bytes → **`NodClientFFI.verifyPayload` (i.e. `nod_proto::verify_payload`) accepts the signature**, a tampered payload is rejected, and remove drops the key. Hermetic: a software P-256 key stands in for the SE (real ECDSA; only the key's residence differs in production). NodMac + NodIOS green.
- Error bridging: callback failures map to the FFI's `SignerCallbackError.Failed` so the runtime never sees an opaque "unexpected callback error."
- Remaining for #16 (the large app-layer piece, NOT yet done): construct `NodClient(observer:signer:)` in the app with `SecureEnclaveDeviceSigner`, decode `NodClientMessage` envelopes to drive SwiftUI, route enroll/submit/sync/state through `NodClient.request(json)`, then DELETE NodKit's duplicated client (`NodAPI`, `NodStore*`, `SyncClient`, models). Keep only native adapters (SE, App Attest, UserNotifications/APNs, SwiftUI).
- Related paths: `client/nod-apple/Sources/NodKit/SecureEnclaveDeviceSigner.swift`, `client/nod-apple/Sources/NodKit/NodSigningKeyStore.swift`.

## Secure Enclave signing as a `DeviceSigner` port — no SE regression on Apple

- Date: 2026-06-09
- Status: active
- Learned: the runtime can sign decisions with a *host-owned hardware key* without nod-client-core ever seeing a private key, so the Apple apps keep their non-exportable Secure Enclave keys while using all of the shared client. The seam is a small trait, not a rewrite.
- Design (three layers): (1) `DeviceSigner` trait in nod-client-core is *just the primitive* — `key_id`/`algorithm`/`public_key`/`sign(bytes)`; the security-critical orchestration (digest recompute, option resolution, nonce/timestamp, canonical `nod-proto` payload) lives once in `build_decision_signature(&dyn DeviceSigner, …)`. (2) `ForeignSigner` port (provision/signing_key/sign/remove, keyed by profile id) is the host backend; `SignerBackend::{Software,Foreign}` on the runtime selects it — TUI/desktop pass nothing (software keys in the Store, unchanged), Apple injects `Foreign`. enroll provisions via the backend and persists a private key ONLY for software; forget removes the hardware key; submit resolves a signer per profile. (3) `nod-client-ffi` exposes `NodDeviceSigner` as a UniFFI `callback_interface` and a `ForeignSignerBridge` adapts it to the core port; `NodClient::new(observer, signer)` mandates a signer so Apple has no software path to regress onto.
- Why the primitive/orchestration split matters: only `sign(bytes)` crosses into hardware (or across the FFI). The bytes the SE signs are built in Rust and verified (the digest-recompute defense-in-depth) identically for every platform — there is exactly one decision-signing implementation.
- Evidence: nod-client-core `foreign_signer_path_produces_verifiable_signature` — a fake SE backend signs through `build_decision_signature` and the signature **verifies via `nod_proto::verify_payload`** over the exact captured payload. nod-client-ffi builds host + `aarch64-apple-ios`; generated Swift: `init(observer:signer:)`, `protocol NodDeviceSigner`, `struct NodDeviceKey`, `enum SignerCallbackError`. NodMac + NodIOS green.
- Remaining: NodKit implements `NodDeviceSigner` over `SecureEnclave.P256.Signing` (it already has the SE key store) and moves enroll/submit/sync onto `NodClient`, then deletes the duplicated Swift client.
- Related paths: `client/nod-client-core/src/signing.rs`, `client/nod-client-core/src/runtime/{session,workflows}.rs`, `nod-client-ffi/src/runtime.rs`.

## The async `NodClientRuntime` drives from Swift via UniFFI (tokio↔Swift proven)

- Date: 2026-06-09
- Status: active
- Learned (retires the last hard unknowns for "Apple onto nod-client-core"): the whole async runtime is now drivable from Swift. The three risks the decision doc flagged — tokio↔Swift async, foreign-trait callbacks, and the state-observer bridge — are all proven, in actual Swift, not just Rust.
- Mechanism: `nod-client-ffi` exposes a `NodClient` UniFFI object with `#[uniffi::export(async_runtime = "tokio")]`. That attribute wraps every exported future in `uniffi::deps::async_compat::Compat`, which (a) gives nod-client-core's tokio I/O (reqwest, the sync websocket) an ambient tokio runtime under the *foreign* (Swift) async caller, and (b) keeps detached `tokio::spawn` tasks alive on async-compat's global runtime — that's what lets the outbox-pump task and `connect_sync`'s background task survive between calls. Requires `uniffi`'s `tokio` feature (`features = ["cli", "tokio"]`) which pulls `uniffi_core/tokio` → `async-compat`. Generated Swift: `init(observer:) async throws`, `func request(requestJson:) async -> String`, `func start() async`, `protocol NodClientObserver { func onMessage(message:) }`.
- Transport choice: JSON strings, NOT a typed UniFFI surface per method. `request(json) -> json` carries the exact `RpcRequest`/`RpcResponse` envelopes the desktop already speaks over Tauri IPC; events arrive as serialized `NodClientMessage` (`{kind,payload}`). So there is ONE client protocol and Swift decodes the same `ClientState`/`Request` shapes the desktop does (already `#[typeshare]`'d) — no second hand-maintained method surface to drift, and adding an RPC method needs no FFI change.
- Evidence: `cargo test -p nod-client-ffi` `ffi_runtime_starts_and_round_trips_state_rpc` (hermetic: `NOD_CLIENT_CORE_STATE_DIR` temp + `NOD_CLIENT_CORE_INSECURE_TOKEN_STORE`); iOS cross-compile clean; **Swift `NodClientRuntimeFFITests` passes** — a Swift `NodClientObserver` received `ready`+`state` from Rust's detached pump and `request`/`start` round-tripped. NodMac + NodIOS build green with the runtime embedded.
- DEFERRED (security, deliberate): decision signing inside the runtime still uses nod-client-core's *software* `StoredSigningKey`. Apple must NOT regress off the Secure Enclave, so NodKit is NOT yet migrated onto `NodClient` for enroll/submit. Next: a `DeviceSigner` port in nod-client-core + a second foreign callback so Swift signs in the Secure Enclave; only then does NodKit's enroll/submit move onto the runtime. The transport is proven first so that work builds on a green base.
- Related paths: `nod-client-ffi/src/runtime.rs`, `client/nod-apple/Tests/NodKitTests/NodClientRuntimeFFITests.swift`, `client/nod-client-core/src/runtime/`.

## Two UniFFI xcframeworks collide on `module.modulemap` — RESOLVED by one FFI crate

- Date: 2026-06-09
- Status: resolved
- Learned: With BOTH `nod_proto_ffiFFI.xcframework` and `nod_client_ffiFFI.xcframework` as SPM binaryTargets, `swift build` / `swift test` are fine, but the `.xcodeproj` app build (NodMac/NodIOS) fails — "Multiple commands produce .../include/module.modulemap". Xcode flattens every binaryTarget's `Headers/module.modulemap` into one `include/`, so the two collide.
- Resolution (the "make the system make sense" / burn-the-boats answer): **collapse the two FFI crates into one.** `nod-proto-ffi` was deleted; `nod-client-ffi` now exposes BOTH surfaces — the decision-signing contract (it took a direct `nod-proto` dep) and the client logic (via `nod-client-core`, which already depends on `nod-proto`, so no version skew). One crate → one `nod_client_ffiFFI.xcframework` → one `module.modulemap` → one Swift module (`NodClientFFI`). No collision possible, and there is now a single place the Apple app reaches shared Rust.
- Outcome (all green): `cargo test -p nod-client-ffi` 5/5 (frozen signing vector + address parity); `swift build`/`swift test` green; **`xcodebuild` NodMac AND NodIOS both BUILD SUCCEEDED.** The `NodServerAddress` swap is now LIVE (delegates to `NodClientFFI.normalizeBaseUrl`/`profileIdFor`/`displayNameFor`); `NodSigningKeyStore` calls `NodClientFFI.decisionSigningPayload`. NodKit no longer depends on a separate signing module.
- Implication for future FFI growth: keep adding `#[uniffi::export]` surface to the single `nod-client-ffi` crate — do NOT spawn a second FFI crate/xcframework (that reintroduces the collision). The async `NodClientRuntime` + capability callbacks (task #13) go here too.
- Related paths: `nod-client-ffi/src/lib.rs`, `client/nod-apple/scripts/build-nod-client-ffi.sh`, `client/nod-apple/Package.swift`, `client/nod-apple/Sources/NodKit/{NodServerAddress,NodSigningKeyStore}.swift`.
