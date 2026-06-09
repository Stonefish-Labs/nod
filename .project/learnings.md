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
- Evidence: `NodSigningKeyStore.swift` uses `SecureEnclave.P256.Signing.PrivateKey`; payload now built by `NodProtoFFI`.
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
