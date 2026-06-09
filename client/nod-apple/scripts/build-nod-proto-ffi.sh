#!/usr/bin/env bash
# Build the nod-proto-ffi static libraries for Apple platforms, generate the
# UniFFI Swift bindings, and assemble:
#   Frameworks/nod_proto_ffiFFI.xcframework   (the linked Rust crypto)
#   Sources/NodProtoFFI/nod_proto_ffi.swift   (the generated Swift wrapper)
#
# Re-run this whenever nod-proto's signing contract changes. Both outputs are
# committed so a plain `swift build` / Xcode build works without a Rust
# toolchain present.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APPLE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${APPLE_DIR}/../.." && pwd)"

CRATE="nod-proto-ffi"
LIB="libnod_proto_ffi.a"
MODULE="nod_proto_ffiFFI"
SWIFT="nod_proto_ffi.swift"

# Each Apple platform slice is lipo'd from its arches so the xcframework covers
# Apple Silicon and Intel for macOS and the simulator (iOS devices are arm64).
MAC_ARM="aarch64-apple-darwin"
MAC_X64="x86_64-apple-darwin"
IOS_ARM="aarch64-apple-ios"
SIM_ARM="aarch64-apple-ios-sim"
SIM_X64="x86_64-apple-ios"

cd "${REPO_ROOT}"

echo "==> Building ${CRATE} static libs (release)"
for target in "${MAC_ARM}" "${MAC_X64}" "${IOS_ARM}" "${SIM_ARM}" "${SIM_X64}"; do
  rustup target add "${target}" >/dev/null
  cargo build -p "${CRATE}" --release --target "${target}"
done

echo "==> Generating Swift bindings"
cargo build -p "${CRATE}" --release
GEN_DIR="$(mktemp -d)"
cargo run -q -p "${CRATE}" --bin uniffi-bindgen -- generate \
  --library "${REPO_ROOT}/target/release/libnod_proto_ffi.dylib" \
  --language swift --out-dir "${GEN_DIR}"

HDR_DIR="${GEN_DIR}/headers"
mkdir -p "${HDR_DIR}"
cp "${GEN_DIR}/${MODULE}.h" "${HDR_DIR}/"
cp "${GEN_DIR}/${MODULE}.modulemap" "${HDR_DIR}/module.modulemap"

echo "==> Fusing universal static libs"
STAGE="$(mktemp -d)"
mkdir -p "${STAGE}/macos" "${STAGE}/ios" "${STAGE}/ios-sim"
lipo -create "target/${MAC_ARM}/release/${LIB}" "target/${MAC_X64}/release/${LIB}" \
  -output "${STAGE}/macos/${LIB}"
cp "target/${IOS_ARM}/release/${LIB}" "${STAGE}/ios/${LIB}"
lipo -create "target/${SIM_ARM}/release/${LIB}" "target/${SIM_X64}/release/${LIB}" \
  -output "${STAGE}/ios-sim/${LIB}"

echo "==> Assembling xcframework"
XCF="${APPLE_DIR}/Frameworks/${MODULE}.xcframework"
rm -rf "${XCF}"
mkdir -p "${APPLE_DIR}/Frameworks"
xcodebuild -create-xcframework \
  -library "${STAGE}/macos/${LIB}" -headers "${HDR_DIR}" \
  -library "${STAGE}/ios/${LIB}" -headers "${HDR_DIR}" \
  -library "${STAGE}/ios-sim/${LIB}" -headers "${HDR_DIR}" \
  -output "${XCF}"

echo "==> Placing generated Swift wrapper"
DEST="${APPLE_DIR}/Sources/NodProtoFFI"
mkdir -p "${DEST}"
cp "${GEN_DIR}/${SWIFT}" "${DEST}/${SWIFT}"

rm -rf "${GEN_DIR}" "${STAGE}"
echo "==> Done: ${XCF}"
