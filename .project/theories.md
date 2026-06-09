# Theories / Hypotheses

## Full literal typeshare adoption reduces net churn and confusion

- Status: testing (the #8 parallel fork)
- Confidence: medium
- Claim: Having the desktop import typeshare-generated types directly (with `skip_serializing_if` so the wire matches `undefined`, plus enum-member call sites) is more maintainable and less confusing than a hand-written `models.ts` mirror that can silently drift.
- Evidence for: The frontend already drifted once (`source` vs `channel`) and broke; a generated single source prevents recurrence.
- Evidence against: typeshare idioms (enums, `undefined`) force consumer churn (enum comparison sites) and a broad wire change; the hand-written mirror is small and readable.
- Validation step: Fork applies it; compare diff size + clarity, and confirm green `tsc`/`vitest`/`cargo test`/`swift test`.
- Outcome: pending.

## Swift wire-type drift is low-risk without codegen

- Status: plausible
- Confidence: medium-high
- Claim: NodKit's hand-written wire types won't dangerously drift from `nod-proto`, because the security-critical crypto is shared via UniFFI, the frozen vectors pin the signing bytes, and a type-shape drift surfaces as a decode failure in `swift test`.
- Evidence for: `swift test` decodes a full `NodRequest`; the canonical bytes come from Rust via `NodProtoFFI`; signing path is not duplicated.
- Evidence against: A purely additive/cosmetic wire field could go unmodeled in Swift silently (lenient decode ignores unknowns) — but that's forward-compatible by design.
- Validation step: A periodic parity check or CI drift-check (see ideas) would raise confidence.
- Outcome: pending.

## In-process delivery removes the mTLS hop without weakening validation

- Status: supported
- Confidence: high
- Claim: `InProcessApnsProvider` can skip HTTP/mTLS yet keep the exact same `RelayPolicy` (bundle-id pinning + field validation) the standalone relay enforces.
- Evidence for: `InProcessApnsProvider` calls `RelayPolicy::sanitize` before `AppleApnsProvider.send`; shared `build_relay_request`; workspace tests green; a bundle-id-pinning test exists.
- Evidence against: none observed.
- Outcome: implemented and green.
