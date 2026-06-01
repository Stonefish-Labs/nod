# Nod Desktop

Tauri 2 desktop client for Nod on Windows and Linux.

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

## Test

```bash
npm run typecheck
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

The frontend talks to the Rust runtime through typed wrappers in `src/commands.ts`.
Do not call Tauri `invoke` directly from React components.
