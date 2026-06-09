# Ideas

## Wire omits nulls via `skip_serializing_if`

- Status: exploring (active in the #8 parallel fork)
- Idea: Add `#[serde(skip_serializing_if = "Option::is_none")]` to the wire `Option<T>` fields so JSON omits absent fields instead of sending `null`.
- Why it might help: Makes the wire match typeshare's TS `?: T` / `undefined`, unblocking literal generated-type adoption in the desktop frontend without a `| null` mismatch; also smaller payloads.
- Evidence or inspiration: typeshare maps `Option<T>` to `?: T`, but the wire currently sends JSON `null` (desktop `tsc` errors on `string | undefined`).
- Risk or tradeoff: Broad change across `nod-proto` + `nod-client-core`; must confirm the server, Swift (`decodeIfPresent`), and TUI still decode absence; verify `request_digest` is unaffected (it reads structs, not raw JSON).
- Next validation step: The fork applies it + updates enum call sites, then `tsc`/`vitest`/`cargo test`.
- Related paths: `nod-proto/src/`, `client/nod-client-core/src/models.rs`.

## camelCase Swift from typeshare (or post-process)

- Status: parked
- Idea: Get camelCase Swift out of typeshare (config, fork, or a post-process pass) so NodKit could adopt generated wire types.
- Why it might help: Would let Swift drop hand-written wire structs and track Rust directly.
- Risk or tradeoff: typeshare has no camelCase mode today; post-processing is fragile. Low payoff since NodKit crypto is already shared via UniFFI.
- Next validation step: none committed.

## Release-hosted xcframework binaryTarget

- Status: candidate
- Idea: Publish `nod_proto_ffiFFI.xcframework` to a GitHub release and reference it by URL + checksum in `Package.swift`.
- Why it might help: Swift builds without a Rust toolchain or the local build script; repo stays lean.
- Evidence or inspiration: The xcframework is ~140 MB and currently gitignored/built-from-source.
- Next validation step: When cutting the first release.
- Related paths: `client/nod-apple/Package.swift`, `client/nod-apple/scripts/build-nod-proto-ffi.sh`.

## watchOS slice in the xcframework

- Status: parked
- Idea: Add a watchOS slice so a future watch target can link the signing crypto.
- Risk or tradeoff: Rust watchOS targets are tier-3 (need nightly + `-Z build-std`). No watch target exists yet (xcodeproj has only NodIOS/NodMac).
- Next validation step: When a watch app target is added.

## CI drift-check for generated vs hand-written types

- Status: candidate
- Idea: A CI step that runs `generate-types.sh` and fails if the committed projection (or, where types stay hand-written, a parity check) is stale.
- Why it might help: Guarantees the "source of truth" actually stays the source of truth.
- Next validation step: After #8 settles on its final shape.

## Thin Swift over a fat shared Rust core (NodKit on nod-client-core)

- Status: candidate (evaluate post-launch)
- Idea: Push most of NodKit's logic — API client, store/state, sync (websocket), inbox, models, notification policy, registration orchestration — down into the shared `nod-client-core` Rust crate (exposed via UniFFI, the way crypto already is), leaving Swift only for what must be platform-native: Secure Enclave signing, SwiftUI views, UserNotifications/APNs, App Attest / DeviceCheck, Keychain, app lifecycle.
- Why it might help: Collapses the largest remaining duplication (nod-client-core Rust ↔ NodKit Swift reimplement the same client logic), extending the centralization thesis past types+crypto. The Tauri desktop already proves the fat-core/thin-shell model (its logic IS nod-client-core; the frontend is thin UI). The UniFFI pipeline already exists from #9.
- Evidence or inspiration: #9 shared crypto via UniFFI cleanly; the desktop runs on nod-client-core; the "shared Rust core + thin native shell" pattern is proven (Mozilla application-services, Bitwarden, etc.).
- Risk or tradeoff: Stateful **async** UniFFI (sync/websocket + observers/callbacks driving SwiftUI) is much harder than stateless crypto fns; moving networking to `reqwest`/`rustls` loses URLSession's iOS system integration (background transfer, cellular/proxy, ATS); larger xcframework (tokio/reqwest/tungstenite cross-compiled to iOS); error/type marshaling + Rust-runtime↔main-actor threading. SE / UI / notifications / attestation must stay Swift regardless.
- Next validation step: Post-launch spike — prototype one slice (the API client OR the sync loop) through UniFFI to measure binding complexity, app-size delta, and the networking-integration tradeoff before committing to the sweep.
- Related paths: `client/nod-client-core/`, `client/nod-apple/Sources/NodKit/` (NodAPI, NodStore*, NodRequestInbox, sync), `nod-proto-ffi/`.
