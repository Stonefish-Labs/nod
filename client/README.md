# Nod Client

Native clients and local client runtime for Nod.

## Projects

- `nod-client-core`: Rust library and stdio binary that own enrollment, persistence, API calls, sync, signing, and local state.
- `nod-desktop`: Tauri 2 + React desktop client for Windows and Linux.
- `nod-tui`: Ratatui + Crossterm terminal client for headless and tmux-heavy workflows.
- `nod-apple`: Native SwiftUI macOS and iOS clients.

## Prerequisites

- Rust stable
- Node.js 20 or newer
- npm
- Tauri platform prerequisites:
  - Windows: Microsoft C++ Build Tools and WebView2 runtime
  - Linux: WebKitGTK and system tray/appindicator packages for the target distro

## Build

From `client/`:

```bash
cargo build --manifest-path ./nod-client-core/Cargo.toml --release
cargo build --manifest-path ./nod-tui/Cargo.toml --release
cd nod-desktop
npm install
npm run tauri build
```

For development:

```bash
cd nod-desktop
npm install
npm run tauri dev
```

The desktop app and TUI both link `nod-client-core` directly. The stdio binary remains available for future headless experiments that prefer process IPC:

```bash
cargo run --manifest-path ./nod-client-core/Cargo.toml
cargo run --manifest-path ./nod-tui/Cargo.toml
```

## Test

```bash
cargo test --manifest-path ./nod-client-core/Cargo.toml
cargo test --manifest-path ./nod-tui/Cargo.toml
cargo test --manifest-path ./nod-desktop/src-tauri/Cargo.toml
cd nod-desktop
npm run typecheck
npm test
```

## Notes

Windows and Linux use local desktop notifications while Nod is running or minimized to tray. Remote push for Windows/Linux is intentionally out of scope; autostart is available through the Tauri backend so the local sync channel can be kept alive.

`nod-apple` remains the Apple-native client for macOS and iOS. `nod-desktop` is bundled for Windows and Linux only.
