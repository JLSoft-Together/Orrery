# WP-58a report: QA on PC, real dev desktop + local relay + web app

## Environment (commands)
- Relay: `cd apps/relay && npx wrangler dev --local --port 8787 --persist-to D:/tmp/wp58a/relay-state` (no login, no deploy).
- Web app: `cd apps/pwa && npx vite preview --port 5181 --host 127.0.0.1` (built dist, base `/app/`).
- Desktop: `apps/shell/target/debug/orrery-shell.exe` rebuilt with `cargo build -j 4` (old exe was from 2026-11-12). Launched by `apps/pwa/e2e/live/deskd.mjs <name> <collectorPort> <cdpPort> <lanPort> <ctrlPort> [cancel]` (uses `apps/desktop/e2e/cdp.cjs`): temp data dir `D:/tmp/wp58a/out-<name>/run/data`, `remote/config.json` with `relay.url = http://127.0.0.1:8787`, `CCM_E2E_CONFIRM=ok` (auto-approves the native pair box; `cancel` for the deny daemon), separate keyring service, `CLAUDE_CONFIG_DIR` = `node scripts/demo-fixture.mjs --sessions=5 --subagents=2` root, `CCM_VERIFY_PROCESSES=0`, `CCM_CODEX=0`. Three daemons: a (4399/4320), b (4400/4321), c deny (4401/4322). The real Orrery.exe (port 4317) was untouched.
- Driver scripts (new, `apps/pwa/e2e/live/`): `lib.mjs`, `desk.cjs`, `deskd.mjs`, `s1..s12.mjs`, `s2.mjs`+`viewer.mjs` (main scenario, `node e2e/live/s2.mjs chromium|webkit [phases]`). Screenshots: `docs/qa/v98/WP-58a/`.
- Pairing: paste code in the web app, native box auto-approved. QR camera path not retried (covered by e2e).

## Cases
| Id | Case | Result |
|---|---|---|
| P1 | Pair via relay (paste), Chromium + WebKit, desktop lists device connected via relay | DAT |
| H1 | Home card, link chip ready/Relay | DAT |
| H2 | Home with 2 real desktops (A and B) | DAT (both are the same host name, so one is listed under "Bản trùng tên" by design) |
| M1/M2 | Monitor A (5 fixture sessions, filters), B, session detail | DAT (both browsers) |
| W1/W2 | Workspace shows real panes; open a shell pane from the phone, runs on desktop with right cwd | DAT |
| T1 | xterm live, line input, output | DAT |
| T2 | Ctrl once/locked/off, Alt once | DAT |
| T3 | Direct typing | DAT |
| T2b | `^C` button interrupts `sleep` / `ping -t` | LOI, not fixed (see bug 2) |
| A1/A2 | Rename tab, close pane + Undo (desktop pane reopened) | DAT |
| V1-V3 | Viewer files, edit + save (written to disk), real conflict (file changed on disk) banner + Reload | DAT (V2 flaked once: save did not return in 15 s, not reproduced in 4 later runs) |
| V4 | Diff tab lists changes, stage, commit (git log confirms) | DAT |
| V6 | Discard: phone confirm then desktop confirm shown on phone, file reverted | DAT |
| V7 | Run tab with no config: "Chưa có phiên chạy nào" | DAT (no Run config in the repo, start/stop/reload CHƯA THỬ on real desktop) |
| D1 | Pairing denied on desktop: phone shows denied + retry, desktop saves nothing | DAT |
| R1 | Relay killed then restarted: chips go reconnecting at once, back to ready 8 s after relay up; desktop itself needs up to ~60 s (backoff) | DAT |
| G1 | WP-59: 20 bad tokens from one IP, then 4429 blocked, row appears on desktop guard list, unblock-all clears it | DAT |
| X1 | Revoke from desktop: card disappears from phone within 1 s | DAT (WebKit too) |
| Push | Chromium (headful, persistent profile): subscribe to FCM, `push.register` ok, desktop device `push: web`, "Kèm nội dung" on/off reaches desktop (`pushContent` true/false) | DAT |
| Push delivery | Real notification arriving, content on/off text | CHƯA THỬ (no episode source with the fixture; 150 s wait saw nothing) |
| LAN | Pair / connect over LAN | CHƯA THỬ: web app dials `wss://ip:4320`, desktop listens plain `ws://` (CSP/plan assume a private CA, not built). Every attempt fails then falls back to relay |
| View-only | hello `control:false` / `denied` | CHƯA THỬ: real desktop always sends `control:true`, no switch (docs); covered by e2e peer |
| Dark/iPad | Screenshots iPad + phone dark, WebKit, no horizontal overflow | DAT |

## Bugs found
1. FIXED (small): pairing blocked by the relay (close 4429) or bad/used ticket (4401) showed generic "Ghép thất bại". Now `pairErrorKey('4429')` gives `blocked` (new i18n `pair.err.blocked`, en + vi), `'4401'` gives `expired`. Files: `apps/pwa/src/lib/qrInput.ts`, `apps/pwa/src/i18n/{en,vi}.ts`, test in `apps/pwa/src/lib/logic.test.ts`.
2. NOT FIXED: `^C` (`\x03`) does not interrupt a running command in a pane on Windows (PowerShell `sleep`, `ping -t`); the desktop's own Ctrl+C via CDP also did not. Suspect `apps/core/crates/core-pty/src/pty_win.rs` (ConPTY flags, win32 input mode). Needs a core fix; outside this WP scope.
3. NOT FIXED (large): when the collector state is big (demo fixture left running about 1 h: agents ~850 KB + activity ~290 KB), the phone snapshot exceeds `MAX_MESSAGE` = 1 MiB (`core-remote-proto/frame.rs`, `Secure::frame` returns false). The phone then stays on "Chưa có dữ liệu" forever (also `ws.get` and `push.register` time out), no error shown. Needs a trimmed / paged snapshot or a visible error. Reproduced only with that inflated fixture; it went away after restarting with a fresh fixture.
4. Observation, not fixed: web app tries `wss://<lan>` first on every connect and reconnect when a LAN host is stored. It can never succeed against the current desktop, so on a real phone away from home it can cost `lanBudgetMs` = 3 s per attempt before the relay.
5. Observation: WebKit fresh context showed the "Có bản mới" update bar on first load against the preview server (screenshot `webkit-ipad-dark-workspace.png`). Not investigated.
6. Local `wrangler dev` closes a rejected phone socket (4401 / 4429) about 10 s late; real Chrome flows were not affected in tests. Likely a local runtime effect.

## Tests re-run
- `npm test -w @ccm/pwa`: 189 passed / 0 failed (18 files).
- `npm run typecheck -w @ccm/pwa`: clean.
- `node e2e/run.mjs --no-peer-build` in `apps/pwa` (same as `npm run e2e`): 80 pass / 0 fail (WebKit + Chromium).
- `cargo test remote::` and relay vitest: not run, no shell or relay source touched. Shell debug exe was rebuilt (`cargo build -j 4`), `permissions/autogenerated` restored with git checkout.

## Still to try on real devices
iPhone / iPad Safari and installed PWA: camera QR scan, WebAuthn lock, real Web Push delivery (content on/off), keyboard + KeyBar feel, LAN once a TLS story exists, A21s browser, update bar false positive on WebKit, large snapshot behaviour, real Run configs (start / reload / stop).

## Cleanup
Daemons, wrangler / workerd, vite preview, fixture generator stopped. Leftover: Windows Credential Manager entries for services `Orrery E2E 58a a|b|c` (can be deleted with `cmdkey`), temp dir `D:/tmp/wp58a`. Nothing committed.

## Ghi chú main (2026-11-19)
- `apps/pwa/e2e/live/` không commit (gitignore): script gắn đường dẫn máy này (`D:/tmp/wp58a`, đường dẫn repo tuyệt đối), chỉ để chạy lại trên máy này. Muốn thành bộ QA chung thì phải bỏ đường dẫn cứng trước.
