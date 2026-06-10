# Nod Desktop

Tauri 2 desktop client for Nod. Windows is the shipped platform — a zipped,
unsigned `Nod.exe` users place wherever they want (no installer; Linux bundles
are configured but not yet released). It runs fine on macOS for
development, but macOS users get the native app in `client/nod-apple` instead —
that one signs decisions with the Secure Enclave.

## Development

Works on macOS, Windows, or Linux:

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

The release workflow ships the bare exe (`npm run tauri build -- --no-bundle`)
zipped; `windows-exe.yml` builds the same artifact on demand for VM testing,
and `scripts/build-windows-exe` cross-compiles it locally from macOS.

## Test

```bash
npm run typecheck
npm test
npm run drift-check
cargo test --manifest-path src-tauri/Cargo.toml
```

`drift-check` regenerates the typeshare projection (`src/dto/generated.ts`,
git-ignored) from the Rust `#[typeshare]` types and compares it against the
hand-written contract in `src/dto/models.ts` — names must agree; deliberate
divergences are documented in `scripts/check-drift.mjs`. It needs
`cargo install typeshare-cli` once.

The frontend talks to the Rust runtime through typed wrappers in `src/commands.ts`.
Do not call Tauri `invoke` directly from React components.
