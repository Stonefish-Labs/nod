#!/usr/bin/env bash
# Generate the Swift + TypeScript types from the Rust source of truth using
# typeshare, so the clients can't drift from nod-proto / nod-client-core:
#   - Swift  (NodKit):  nod-proto wire types          -> Generated/NodProtoTypes.swift
#   - TS (nod-desktop): nod-proto + nod-client-core    -> src/dto/generated.ts
#
# Re-run whenever an annotated Rust type changes. Requires the typeshare CLI
# (`cargo install typeshare-cli`). Both outputs are committed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

# Reference projection only (NOT compiled into NodKit): typeshare emits
# snake_case Swift, which clashes with NodKit's camelCase Codable models, so the
# Swift client keeps its hand-written wire types. This file documents the
# canonical shapes the hand-written models must track.
SWIFT_OUT="client/nod-apple/Generated/NodProtoTypes.swift"
TS_OUT="client/nod-desktop/src/dto/generated.ts"

mkdir -p "$(dirname "${SWIFT_OUT}")" "$(dirname "${TS_OUT}")"

echo "==> Swift (nod-proto wire types) -> ${SWIFT_OUT}"
typeshare nod-proto/src --lang swift --output-file "${SWIFT_OUT}"

echo "==> TypeScript (nod-proto + nod-client-core view types) -> ${TS_OUT}"
typeshare nod-proto/src client/nod-client-core/src --lang typescript --output-file "${TS_OUT}"

echo "==> Done"
