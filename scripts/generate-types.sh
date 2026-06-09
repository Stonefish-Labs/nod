#!/usr/bin/env bash
# Generate the desktop frontend's TypeScript wire/view types from the Rust source
# of truth (nod-proto wire types + nod-client-core view types) via typeshare, so
# the frontend can't drift from Rust. Output -> client/nod-desktop/src/dto/generated.ts
# (committed; src/dto/models.ts re-exports it). Re-run whenever an annotated Rust
# type changes. Requires the typeshare CLI (`cargo install typeshare-cli`).
#
# Swift is intentionally not generated: typeshare emits snake_case, which clashes
# with NodKit's camelCase Codable models, and NodKit's security-critical crypto is
# already shared from Rust via UniFFI (nod-client-ffi). NodKit's wire types stay
# hand-written and are verified by its build + tests; nod-proto is the contract.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

TS_OUT="client/nod-desktop/src/dto/generated.ts"
mkdir -p "$(dirname "${TS_OUT}")"

echo "==> TypeScript (nod-proto + nod-client-core view types) -> ${TS_OUT}"
typeshare nod-proto/src client/nod-client-core/src --lang typescript --output-file "${TS_OUT}"

echo "==> Done"
