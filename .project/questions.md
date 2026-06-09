# Questions

## Should the wire omit nulls (`skip_serializing_if`) globally?

- Status: open (being tested in the #8 fork)
- Why it matters: Determines whether the desktop can import typeshare-generated types directly (TS `undefined`) vs. keeping a hand-written `| null` mirror. Affects the server, Swift, and TUI serialization.
- Current evidence: typeshare emits `?: T`; wire currently sends `null`; desktop `tsc` fails on the mismatch.
- Evidence needed: Full `cargo test --workspace` + `swift test` + desktop `tsc`/`vitest` after the change.
- Related paths: `nod-proto/src/`, `client/nod-client-core/src/models.rs`.
- Answer: (pending fork)

## How and when to commit the 118-file milestone?

- Status: open
- Why it matters: The entire centralization effort is uncommitted on `b6765f4 Rework`; risk of loss / hard to review. The #8 fork complicates ordering.
- Current evidence: `git status` → 112 M / 3 RM / 3 ??.
- Evidence needed: User decision on granularity (single milestone vs per-workstream) and whether to wait for the #8 fork to merge.
- Answer: (pending user)

## Should NodKit ever adopt generated Swift wire types?

- Status: deferred
- Why it matters: Decides whether Swift wire types stay hand-written (current) or track Rust via codegen.
- Current evidence: typeshare emits snake_case Swift (clashes with NodKit camelCase); crypto already shared via UniFFI; NodKit wire types are build/test-verified.
- Evidence needed: A camelCase-Swift path (see ideas) or acceptance of snake_case.
- Answer: For now, no — Swift wire types stay hand-written; `nod-proto` is the contract.

## Is NodKit's Swift footprint too heavy vs. sharing nod-client-core? (evaluate later)

- Status: open (post-launch evaluation)
- Why it matters: Decides the macOS/iOS architecture direction — how much client *logic* is duplicated in Swift vs. shared from the Rust core. Large maintenance + drift implications; this is the logical next step of the centralization thesis (types + crypto are centralized; client logic is not).
- Current evidence: NodKit reimplements API/store/sync/inbox/models in Swift that `nod-client-core` already implements in Rust; the desktop shares `nod-client-core` successfully; crypto is already shared via UniFFI (#9). The Tauri desktop is the fat-core/thin-shell proof.
- Evidence needed: A spike measuring UniFFI **async**/observer binding complexity, xcframework size delta, and the URLSession-vs-`reqwest` iOS integration tradeoff.
- Related paths: `client/nod-apple/Sources/NodKit/`, `client/nod-client-core/`, `nod-proto-ffi/`.
- Answer: Pending evaluation. Leaning: worth pursuing post-launch for the *logic* layer; keep Secure Enclave / SwiftUI / UserNotifications / App Attest / Keychain native regardless. See [ideas.md](ideas.md) "Thin Swift over a fat shared Rust core."

## In-process APNs key custody — acceptable long-term?

- Status: answered
- Why it matters: `InProcessApnsProvider` means the server process holds the APNs `.p8`.
- Answer: Accepted by the user for single-operator/single-box ("co-location without mTLS when same instance"); the remote mTLS relay remains for isolation/scale-out. Configuring both is a hard error.
- Related paths: `server/nod-server/crates/nod-server/src/{config.rs,push.rs}`.
