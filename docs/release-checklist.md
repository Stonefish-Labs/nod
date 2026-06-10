# Release Checklist

Per-surface verification before a release cut, plus the cut runbook itself.
Check items off per release; date the section header when a full pass
completes. Automated coverage is noted next to each item — "scripted" items
re-verify on every test run, the rest need a human pass.

## Green gate (run before everything)

```bash
cargo test --workspace
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cd client/nod-apple && swift test
cd client/nod-desktop && npm run typecheck && npm test && npm run drift-check
server/nod-server/scripts/nod-smoke
```

## Server

- [x] `scripts/nod-smoke` — health, admin provisioning, P-256 enrollment, sync
      WebSocket, issuer request, signed decision, two-recipient fanout,
      cleanup (scripted; runs in `cargo test -p nod-server`)
- [x] `docker compose up` serves `/health` and `/admin` (verified 2026-06-10:
      image built, container serves embedded admin page, no asset files)
- [ ] `scripts/nod-smoke URL TOKEN` against the production deployment

## TUI — executed 2026-06-10 (sandboxed: `NOD_CLIENT_CORE_STATE_DIR` + insecure token store, tmux-driven)

- [x] Enrollment form → enroll against local server → main screen (scripted
      analog: `smoke_test.rs` event-loop test)
- [x] Live request arrives over the sync WebSocket and renders with options
- [x] `a` approve → server records P-256-signed decision, `verified: true`
- [x] `a` on an `approve_with_text` option opens the notes editor; submitted
      text lands in the decision (bug found + fixed here: the hinted key did
      not reach `*_with_text` options; regression test added in `domain.rs`)
- [x] `r` reject → signed, verified
- [x] Server restart while TUI open → reconnects, next request arrives live
- [x] TUI restart → profile and request history persist, no re-enrollment
- [x] Resize to 60x20 → layout stays sane
- [ ] Keyring (non-insecure) enrollment path — deliberately manual: automated
      runs must not write the real macOS keychain; normal daily use covers it
- [ ] Visual pass in your daily terminals (Terminal.app, iTerm2): colors,
      glyphs, bell behavior — subjective "is it what we want"

## Windows desktop (Tauri) — run in the Windows VM with the CI-built MSI

- [ ] VM reaches host server: `http://<host-ip>:8767/health` from the VM
      browser (Parallels/VMware shared net → host LAN IP; UTM → `10.0.2.2`)
- [ ] `Get-FileHash` of the downloaded MSI matches the published SHA-256
- [ ] SmartScreen "More info → Run anyway" flow — screenshot it (feeds
      deploy docs + release notes)
- [ ] Install → app launches; tray icon shows the nod head (first visual
      check on Windows); tooltip + Show/Quit menu work, light + dark taskbar
- [ ] Enroll to the host server; request arrives; Windows toast shows
- [ ] Approve with text; reject; verify both on the server
- [ ] Quit + relaunch → still enrolled (Credential Manager)
- [ ] VM reboot → still enrolled; autostart behaves as configured
- [ ] Server restart while app open → sync reconnects
- [ ] Uninstall removes the tray entry; reinstall works

## macOS app — run from the stapled DMG, fresh copy in /Applications

- [ ] First launch passes Gatekeeper with no override (notarized build)
- [ ] Menu-bar nod-head glyph renders in light + dark mode; pending count
- [ ] Enroll, receive, decide (Secure Enclave signing), notification + sound
- [ ] Quit + relaunch → still enrolled
- [ ] `spctl --assess --type execute -vv Nod.app` reports Notarized Developer ID

## iOS — TestFlight build

- [ ] Install from TestFlight on a physical device
- [ ] APNs push arrives with the app backgrounded
- [ ] Approve from the notification and in-app; verify on server

## Cut runbook (v1.0.0)

1. Green gate above, all sections checked for the surfaces shipping
2. Push main; CI green
3. `git tag -a v1.0.0 -m "Nod 1.0.0" && git push origin v1.0.0` → draft
   release builds artifacts + SHA256SUMS
4. Quick MSI reinstall sanity in the VM
5. `client/nod-apple/scripts/release-macos` → `gh release upload v1.0.0 Nod-1.0.0.dmg`
6. Append the DMG line to SHA256SUMS, `gh release upload v1.0.0 SHA256SUMS --clobber`
   (CI checksums do not cover the locally built DMG)
7. Publish the draft; verify every download link from a browser; `shasum -c`
   one artifact per OS family
8. `docker run ghcr.io/batteryshark/nod-server:v1.0.0` boots, `/health` answers
9. Walk docs/deploy.md Path A using the published release on a clean account
