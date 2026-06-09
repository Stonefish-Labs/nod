# Attempts Ledger

## 2026-06-09 — Relay in-process provider + collapse duplicate wire DTOs (#11)

- Intent: Co-located APNs delivery with no HTTP/mTLS hop; kill the duplicated relay wire contract.
- Method: Export the relay's library surface; add `RelayPolicy::sanitize` (anyhow) for library callers; add `InProcessApnsProvider` (embeds `AppleApnsProvider`); shared `build_relay_request`; rename `RemotePushRoute -> PushRoute` (+`ApnsDirect`); config presence selection in `push.rs`; server config gains `NOD_APNS_DIRECT_*`.
- Evidence: `cargo test --workspace` green; clippy + fmt clean; admin API/HTML `remote_push_route -> push_route`.
- Result: Success.
- Learning: Relay was already a lib; the duplicate DTOs were the real drift risk.
- Follow-up: none.
- Related paths: `server/nod-server/crates/nod-server/src/{push.rs,apns_relay.rs,config.rs}`, `server/nod-apns-relay/src/{lib,relay}.rs`.

## 2026-06-09 — UniFFI signing into Swift (#9)

- Intent: One Rust implementation of the canonical signing bytes behind Swift.
- Method: `nod-proto-ffi` crate (proc-macro UniFFI, in-crate `uniffi-bindgen`); `build-nod-proto-ffi.sh` builds slices, generates bindings, assembles the xcframework; `Package.swift` binaryTarget + `NodProtoFFI` target; `NodSigningKeyStore` builds the payload via `NodProtoFFI`.
- Evidence: `cargo test -p nod-proto-ffi` 4 green (incl. frozen vector through FFI); `swift test` 24 green; `xcodebuild` NodMac + NodIOS BUILD SUCCEEDED.
- Result: Success, after two fixes (below).
- Learning: see learnings.md (Swift 5 mode; universal slices).
- Follow-up: watchOS slice; release-hosted xcframework.
- Related paths: `nod-proto-ffi/`, `client/nod-apple/`.

## 2026-06-09 — UniFFI fix: Swift 6 strict concurrency

- Intent: Compile the generated Swift shim.
- Method: `.swiftLanguageMode(.v5)` on the `NodProtoFFI` SPM target.
- Evidence: error `var 'initializationResult' is not concurrency-safe` → resolved.
- Result: Success.

## 2026-06-09 — UniFFI fix: iOS simulator x86_64 link failure

- Intent: Make `xcodebuild -scheme NodIOS -sdk iphonesimulator` link.
- Method: lipo arm64+x86_64 into universal macOS + simulator xcframework slices.
- Evidence: `ld: … found architecture 'arm64', required architecture 'x86_64'` → BUILD SUCCEEDED.
- Result: Success.

## 2026-06-09 — Verify #10 rename on the Swift side

- Intent: Confirm the source→channel rename compiles across all Apple targets.
- Method: `swift build`, `xcodebuild -scheme NodIOS -sdk iphonesimulator`, `swift test`.
- Evidence: NodKit/NodMac/NodIOS build; `swift test` 24 (after deleting the ghost `testLegacyServerContractKeysDoNotDecode`, the Swift twin of the Rust legacy-rejection tests).
- Result: Success.
- Learning: NodIOS-only files are excluded from the SPM package, so `xcodebuild` is needed to cover them.

## 2026-06-09 — typeshare annotate + generate (#8)

- Intent: Generate Swift + TS types from the Rust source of truth.
- Method: `#[typeshare]` on `nod-proto` wire types + `nod-client-core` view types; `serialized_as` for chrono + the `usize` map; `scripts/generate-types.sh`.
- Evidence: Generation succeeded after the `usize -> u32` override; Rust workspace stayed green.
- Result: Partial — generation works; literal adoption blocked by typeshare idioms (enums, `undefined` vs `null`, snake_case Swift).
- Learning: see learnings.md (typeshare idioms; TS source is nod-client-core).
- Follow-up: Parallel fork: `skip_serializing_if` + enum call-site updates for full TS adoption; Swift generation dropped.
- Related paths: `nod-proto/src/`, `client/nod-client-core/src/models.rs`, `scripts/generate-types.sh`, `client/nod-desktop/src/dto/`.

## 2026-06-09 — Fix the desktop source→channel frontend bug (under #8)

- Intent: The TS frontend was broken (`source` vs backend `channel`).
- Method: Case-preserving `source -> channel` rename across `client/nod-desktop/src` (no `resource` collisions); `SourceSubscriptions.tsx -> ChannelSubscriptions.tsx`.
- Evidence: At the time of the rename (with a hand-written `models.ts`), `tsc` clean + 16 vitest green. NOTE: the tree has since moved to the fork's `models.ts` re-export, so `tsc` currently fails on enum/undefined adoption issues pending in the fork.
- Result: The naming bug is fixed; the adoption-mode build is red here pending the fork.
- Related paths: `client/nod-desktop/src/**`.
