# PLAN v87 bước 6 — vỏ Tauri, không Node (2026-11-10)

Nguồn: `PLAN-v87-rust-core.md` §6 bước 6, `PLAN-v87-tauri-options.md` (khảo sát + spike), `DECISIONS.md` 2026-11-09 (Tauri **không** Node sidecar). Chủ nhân 2026-11-10: "cứ làm cả", làm trọn trên Windows, con tự chốt, test giao Codex, xong Windows chủ nhân sang Mac test.

## 1. Mục tiêu

- Bỏ Electron + Node khỏi bản cài: vỏ Tauri v2 (WebView2 / WKWebView) + `orrery-core` Rust. Setup Windows ≤ 35 MB, RAM rảnh ≤ 200 MB tổng.
- Renderer Vue giữ nguyên; chỉ thay lớp `window.ccmDesktop` (preload) bằng shim gọi Tauri `invoke` / `listen`, cùng tên method + event.
- Mọi tính năng hiện có chạy được (trừ Office: khoá như chế độ rust hiện tại; DeepSeek: đang tắt, port sau cùng).

## 2. Cách làm: 2 giai đoạn

**6a — port main sang Rust khi vẫn chạy trong Electron.** Crate mới `core-desk` (thư viện): `dispatch(channel, args) -> Result<Value>` + sink event. Electron chạy nó qua `orrery-core desk` (NDJSON stdin/stdout, cùng kiểu `serve`); handler `ccm:*` của từng module chuyển sang forward vào Rust sau cờ `CCM_DESK_<MODULE>=rust|node` (mặc định `rust` khi module qua parity). E2E CDP hiện có kiểm từng module — không cần chờ vỏ mới.

**6b — vỏ Tauri.** App Tauri link thẳng `core-desk` + `core-serve` + `core-pty` (một process Rust + WebView), `invoke("ccm", {channel, args})` gọi cùng `dispatch`. Shim `window.ccmDesktop` sinh từ danh sách channel của preload. Electron giữ song song tới khi 6c đạt, rồi bỏ.

## 3. Con chốt (thay chủ nhân, ghi lại để đổi được)

1. **Xác thực app-key**: Tauri mở `http://127.0.0.1:<port>/?k=<key>` → core-http đổi key lấy cookie `HttpOnly; SameSite=Strict`, redirect bỏ query. WS / fetch dùng cookie. Electron vẫn dùng header như cũ. Giữ origin `127.0.0.1:4317` nên key `localStorage` không đổi tên.
2. **localStorage / sessionStorage**: profile WebView2 khác Chromium của Electron → bản Electron cuối (6a) ghi `CCM_DATA_DIR/webstorage-export.json` mỗi lần thoát; Tauri lần đầu nạp file đó vào `localStorage` trước khi app Vue chạy (script init).
3. **safeStorage**: crate `keyring` (Windows Credential Manager, macOS Keychain). Migrate: Rust đọc khoá `os_crypt` trong `Local State` của Electron (DPAPI trên Windows, Keychain "Orrery Safe Storage" trên mac), giải ciphertext cũ, ghi vào keyring. Hỏng thì báo nhập lại token, không crash.
4. **`<webview>`**: child webview Tauri (`unstable`, ghim version tauri 2.12.x). BrowserPane + FilePane preview dùng chung data dir với cửa sổ chính (CDP thấy được, đổi lại mất cô lập storage — chấp nhận, như spike). Overlay HTML mở thì ẩn child. Chụp ảnh: WebView2 `CapturePreview` (Windows), `takeSnapshot` (mac).
5. **Header**: tự vẽ nút min / max / close theo theme (spike đã chạy), `data-tauri-drag-region`; mac dùng `titleBarStyle: Overlay` + traffic light.
6. **Telegram MTProto (upload lớn)**: crate `grammers`; Bot API phần còn lại bằng `reqwest`.
7. **Bridge statusline / hook**: binary Rust nhỏ `orrery-bridge` (thay `ELECTRON_RUN_AS_NODE`); `claudeHooks` tự sửa entry `settings.json` cũ trỏ exe Electron.
8. **sql.js** → `rusqlite` (bundled). **node-pty** fallback, **Node collector** fallback, **utilityProcess**: bỏ ở Tauri (cờ `CCM_CORE=node` / `CCM_CORE_PTY=node` chỉ còn ở bản Electron).
9. **Power**: `SetThreadExecutionState` / `IOPMAssertion`, on-battery qua `GetSystemPowerStatus` / IOKit — viết trong `core-desk`.
10. **Updater**: port flow riêng (`update/`) sang Rust, gói NSIS / dmg của Tauri.
11. **E2E**: Windows giữ CDP qua `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`, `cdp.cjs` thêm chế độ `--shell=tauri`. macOS: checklist tay của chủ nhân.
12. **Office**: khoá trên Tauri (như `CCM_CORE=rust`). **DeepSeek** (đang tắt): port cuối, không chặn.

## 4. Gói việc

### 6a — port module (mỗi WP: Rust + forward + twin test TS↔Rust + E2E module đó)
| WP | Module (dòng TS) | Ghi chú |
|---|---|---|
| D0 | khung `core-desk` + `orrery-core desk` + forward trong main + cờ + mẫu twin test; port `backup`, `buildClean`, `libs` làm mẫu | mở đường |
| D1 | `claudeHooks`, `bridge` → `orrery-bridge`, `extensions`, `hooks`, `statusline`, `editors`, `appearance`, `layouts`, `pages`, `restore` | |
| D2 | `github` (+ keyring + migrate safeStorage), `disk`, `power`, `agentStatus`, `resources`, `capture` (phần file), `phone`, `attention` | |
| D3 | `preview/` git + file workspace (6.4k, 84 handler) | lớn nhất, có thể tách 2 |
| D4 | `run/` (5.4k) + `flutterPreview/` (1.3k) | DONE, mặc định rust từ D4b (chờ E2E; `CCM_DESK_RUN=node` / `CCM_DESK_FLUTTER_PREVIEW=node` quay về TS): 23 kênh `ccm:run-*` + 5 kênh `ccm:fwp-*` sang `core-desk`; D4b nối TS consumer qua `run/facade.ts` `RunFacade` (mirror `ccm:run-event` + `desk:run-state` + `desk:run-settings`); xem `WP-v87-6-D4-report.md`, `WP-v87-6-D4b-report.md` |
| D5 | `devices/` (3.7k, sql.js → rusqlite) + `apk/` + `releaseCheck/` | PARTIAL (chờ E2E, mặc định node; `CCM_DESK_APK|RELEASE_CHECK|DEVICES=rust` bật): `apk` 3 kênh + `releaseCheck` 4 kênh + `devices` 23 kênh (20 `ccm:dev-*` + 3 `ccm:avd-*`) sang `core-desk`; D5b đã port nốt `dev-rec-*`, `devfs-*`, `perf-*`, `appdata-*` (sql.js → `rusqlite` bundled) cùng cờ `CCM_DESK_DEVICES` (`.planning/wp/WP-v87-6-D5-report.md`, `WP-v87-6-D5b-report.md`) |
| D6 | `layout/` (3.6k, JVM daemon spawn + download + model Gradle) | PARTIAL (chờ E2E, mặc định node; `CCM_DESK_LAYOUT=rust` bật): D6 = 5 kênh `ccm:layout-{status,download,render,warmup}` + `ccm:drawable-render` + `desk:layout-*` sang `core-desk/src/layout/` (JDK, AppCDS, daemon JVM, download pin, cache render). D6b = model service: 3 kênh `ccm:layout-model-{status,refresh,pick}` + `desk:layout-after-build`, `ModelSvc` (resolve / pick app, cache `<hash>.json` cùng format TS, Gradle init script cùng lệnh, source model + AAR cache, loose kit, watch res / code), `model.prepare` chạy trong Rust, vỏ Tauri không Node render đủ. Còn: E2E + chạy `gradlew` thật; vỏ Tauri phải gọi `desk:layout-after-build` khi Run pane `running`. Báo cáo `.planning/wp/WP-v87-6-D6-report.md`, `WP-v87-6-D6b-report.md` |
| D7 | `telegram/` (4.1k + grammers) | PARTIAL (2026-11-10, chưa E2E): Bot API sang `core-desk/src/telegram/` (15 kênh `ccm:telegram-*` + `desk:tg-*`, poll, lệnh, nút, 429, pin / icon / summary), mặc định node (`CCM_DESK_TELEGRAM=rust` để bật); xem `WP-v87-6-D7-report.md`. **D7b (2026-11-11, chưa E2E):** MTProto (file lớn, tài khoản user) sang Rust `core-desk/src/telegram/mt/` bằng `grammers-mtsender` 0.12 (session riêng `telegram-mtproto.json` 0600, sealed; tài khoản GramJS cũ phải đăng nhập lại, báo 1 lần qua host `telegram.notice`); `CCM_TELEGRAM_MTPROTO=node` giữ đường GramJS qua host; xem `WP-v87-6-D7b-report.md` |
| D8 | `pty/` MCP `orrery` + `mcpTools` + `orreryApi` + `mcpDesk` + `agentData` (bỏ host.ts Node) | PARTIAL (2026-11-11, chưa E2E; Electron mặc định node, `CCM_DESK_MCP=rust` / `CCM_DESK_AGENT_DATA=rust` bật): `core-desk/src/mcp/` (JSON-RPC, 19 tool, server HTTP 127.0.0.1, registry token / ticket cho Tauri, kênh `desk:mcp-handle`), `mcp_desk/` (`desk:mcp-desk`, `desk:mcp-screenshot`, deps host / native), `agent_data/` (9 kênh `ccm:agentdata-*`); vỏ Tauri `apps/shell/src/mcp.rs` + `pty.rs` env / cờ MCP. Thiếu ở Tauri: `resources.report` (list_servers / free_port), dừng collector quanh move agentData. Báo cáo `.planning/wp/WP-v87-6-D8-report.md` |
| D9 | `update/`, `deepseek/` | DONE (chờ E2E, mặc định node; `CCM_DESK_UPDATE=rust` / `CCM_DESK_DEEPSEEK=rust` bật): 9 kênh `ccm:update-*` + `desk:update-start`, 9 kênh `ccm:deepseek-*`; autoUpdater / cài / relaunch / dialog qua host `update.*` / `deepseek.confirm` (`.planning/wp/WP-v87-6-D9-report.md`) |

**D0 xong (2026-11-10)**, báo cáo `.planning/wp/WP-v87-6-D0-report.md`:
- Hợp đồng `core-desk`: kênh = đúng tên `ccm:*` (args = mảng tham số IPC), kiểm frame (`isAppFrame`) ở vỏ; phần chỉ vỏ làm được (dialog, localStorage trang, confirm có text i18n, thùng rác, dữ liệu main như `resReport` / tài khoản GitHub, relaunch) đi qua gọi ngược `ctx.host("<module>.<việc>", params)` — Electron trả bằng `hosts` của `DeskRouter.route`, Tauri (T0) trả trực tiếp trong process. Text i18n chưa sang Rust: Rust gửi dữ liệu thô (số thư mục, size, dòng), vỏ dựng câu.
- Event Rust → cửa sổ: phải có trong `DESK_EVENTS` (`forward.ts`).
- Boot (`applyPending`) và `writeAuto` của update vẫn TS ở Electron (desk chưa chắc đã lên lúc boot); Rust có sẵn `desk:backup-apply-pending` / `desk:backup-write-auto` cho Tauri / D9.
- HTTP trong desk: `ureq` 3 + rustls + platform verifier (dùng lại được cho D2 / D7 / D9). `orrery-core` release 5,0 → 8,1 MB; nếu cần gọn cho mục tiêu Setup ≤ 35 MB thì đổi sang `native-tls` (SChannel / Security.framework) ở 6b.

**D2 xong phần code (2026-11-10, chưa E2E)**, báo cáo `.planning/wp/WP-v87-6-D2-report.md`: 7 module `github`, `phone`, `capture`, `power`, `disk`, `agentStatus`, `resources` (`ccm:res-kill`) trong `DESK_NODE_DEFAULT` (bật bằng `CCM_DESK_<MODULE>=rust`). Token / URL chỉ seal / unseal qua host `keychain.*` (safeStorage ở vỏ; keyring + migrate safeStorage để T2). Event nền qua `Desk::emitter()` (vỏ Tauri phải gọi `desk.set_sink`). Thuần vỏ, không port: `attention.ts` (dock badge / bounce), `capture.ts` `dustSize` + `ccm:capture-rect` / `ccm:capture-guest` (T1), `resources/deviceUse.ts` (post collector), `resources/agentPorts.ts` (MCP, D8).

### 6b — vỏ Tauri
| WP | Việc |
|---|---|
| T0 | `apps/shell` Tauri: cửa sổ, header, tray, menu, single-instance, deep link `agent-monitor://`, autostart, notification / toast, zoom, cookie auth, link `core-serve` + `core-pty` + `core-desk`, shim `ccmDesktop` sinh tự động |
| T1 | child webview BrowserPane / FilePane + capture + forward-key + ẩn khi overlay; dust capture |
| T2 | DONE phần code (2026-11-11, chưa `tauri build` / E2E): migrate localStorage (export Electron + đọc leveldb) + safeStorage (keyring master key, giải `v10` Electron), `tauri.release.json` (NSIS currentUser + WebView2 bootstrapper, dmg arm64 / x64, identifier `dev.ccm.monitor`, sidecar), hook NSIS gỡ Electron cũ, updater host Tauri, `dist-tauri.mjs` + CI, devtools chỉ debug / E2E, `cdp.cjs --shell=tauri`; `.planning/wp/WP-v87-6-T2-report.md` |
| T3 (Codex) | nghiệm thu: E2E đủ bộ Tauri trên Windows, size, RAM / CPU, so Electron |

Ghi chú T0 (2026-11-10, `.planning/wp/WP-v87-6-T0-report.md`): `apps/shell` chạy core-serve trong process (stdin của process thay bằng pipe vì `core_serve::run` đọc lệnh từ stdin; nên thêm `run_with(cfg, control)` vào core-serve ở WP sau) và spawn `orrery-core` cho terminal (không link `core-pty`: stdin đã dùng, `serve()` private, job object cả process). `core-desk` chưa link (D0 đang làm): kênh chưa port trả `not ported`. Cookie auth xong trong core-http. E2E Tauri: p2 15/15, agent-map 34/36 (2 lỗi clipboard ở menu node).

Ghi chú T1 (2026-11-10, `.planning/wp/WP-v87-6-T1-report.md`): vỏ link thẳng `core-desk` (`Desk::standard`, kênh `ccm:*` chưa có ở vỏ → desk, host callback trong `apps/shell/src/hosts.rs`; `desk:backup-apply-pending` chạy lúc boot). `<webview>` = child webview Tauri qua `webview.js` (vá `createElement('webview')`, bám vị trí, ẩn khi overlay HTML che) + `guest.js` (event / exec / phím) + command `guest` (capability riêng `guest-*`, không gọi được `ccm`). Chụp WebView2 `CapturePreview` (child + dust), mac `takeSnapshot` viết theo `cfg` nhưng chưa biên dịch (máy Windows thiếu `cc` cho objc). Clipboard: quyền `ClipboardRead` cho trang app → agent-map 34/34. E2E riêng `apps/shell/e2e/t1.cjs` 33/33. Còn chặn: `browser.cjs` / `tabs-cwd.cjs` cần preview server + quyền thư mục (D3), `v74` cần inspector Node (thay bằng `CCM_E2E_CONFIRM` + t1).

### 6c — chốt
- Đổi bản phát hành sang Tauri, bỏ `apps/desktop` Electron + `apps/collector` sau khi chủ nhân test Mac (checklist trong `.planning/CHECKLIST-v87-step6-mac.md`).

## 5. Cổng mỗi WP
- `cargo test` + clippy `-D warnings`; twin test TS↔Rust cùng input = cùng output (handler module); `npm test` desktop / web; typecheck; E2E bộ liên quan (Codex chạy, main tự kiểm).
- Không comment trong code, file ≤ ~400 dòng, chỉ Windows + macOS (code mac viết theo `cfg`, chủ nhân test).

## 6. Không được phá
- App Orrery thật của chủ nhân (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`): mọi lần chạy thử dùng port + data + userData tạm.
- Protocol WS / `packages/shared` không đổi. Hard rules sanitize / 127.0.0.1 / app key / không gọi LLM giữ nguyên.
