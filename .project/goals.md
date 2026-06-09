# Goals

## Original Goal

Ship Nod: a multi-client decision-request / approval system. A server issues structured "requests" to "channels"; recipient devices receive push/websocket notifications and submit cryptographically signed decisions. Clients: a Rust TUI, a Tauri desktop app, and native Swift macOS/iOS apps. An APNs relay forwards pushes to Apple. (Pre-launch; no deployed clients or DBs.)

## Current Goal (this effort)

Before publishing, **make the system coherent and maintainable**: eliminate the duplication and drift that had accumulated across the server and four client surfaces. Concretely:

- One source of truth for the wire protocol and the decision-signing crypto (the `nod-proto` crate), depended on by the server and every Rust client.
- Share the security-critical signing path into the Swift apps so there is exactly one implementation (UniFFI), not a parallel Swift reimplementation.
- Generate the Swift + TypeScript wire/view types from Rust so clients can't drift (typeshare).
- Finish the `source -> channel` domain rename consistently everywhere.
- Co-locate the APNs relay for single-box deploys without losing the scale-out option.

## Pivots

- 2026-06-09: #8 scope shifted from "run typeshare and adopt everywhere" to "wire typeshare + fix the discovered desktop bug, defer literal full adoption" after hitting typeshare output-idiom clashes. The user then directed a **parallel fork** to push full adoption (with the wire-semantics change) "if it makes the code easier to maintain with less churn."

## Accepted Scope

- `nod-proto` workspace crate; server + clients rewired onto it; frozen protocol-freeze vectors.
- UniFFI signing crate (`nod-proto-ffi`) consumed by Swift `NodKit`.
- typeshare annotations + generation script; generated projections as reference.
- `source -> channel` rename across Rust, Swift, and the desktop TS frontend.
- Relay as a library with in-process + remote (mTLS) providers.

## Deferred Scope

- Literal typeshare adoption (clients import generated types directly) — in the parallel fork.
- watchOS slice in the xcframework (Rust watchOS is tier-3).
- Release-hosted xcframework binaryTarget.
- Committing the milestone (awaiting user direction).

## Success Criteria

- A protocol/crypto change is a single edit in `nod-proto`, mechanically reflected everywhere.
- Green builds + tests across Rust (`cargo test --workspace`), Swift (`swift test`, `xcodebuild` NodMac/NodIOS), and desktop (`tsc`, `vitest`).
- No misleading/legacy names; no backcompat shims.

## Open Goal Questions

- Is "full literal adoption" (generated types imported directly) worth the wire-semantics change vs. keeping generated projections as a drift reference? (Fork is testing this.)
