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
| D4 | `run/` (5.4k) + `flutterPreview/` (1.3k) | |
| D5 | `devices/` (3.7k, sql.js → rusqlite) + `apk/` + `releaseCheck/` | |
| D6 | `layout/` (3.6k, JVM daemon spawn + download) | |
| D7 | `telegram/` (4.1k + grammers) | |
| D8 | `pty/` MCP `orrery` + `mcpTools` + `orreryApi` + `mcpDesk` + `agentData` (bỏ host.ts Node) | |
| D9 | `update/`, `deepseek/` | |

### 6b — vỏ Tauri
| WP | Việc |
|---|---|
| T0 | `apps/shell` Tauri: cửa sổ, header, tray, menu, single-instance, deep link `agent-monitor://`, autostart, notification / toast, zoom, cookie auth, link `core-serve` + `core-pty` + `core-desk`, shim `ccmDesktop` sinh tự động |
| T1 | child webview BrowserPane / FilePane + capture + forward-key + ẩn khi overlay; dust capture |
| T2 | migrate localStorage + safeStorage, đóng gói NSIS (WebView2 bootstrapper download), icon, ký; `cdp.cjs --shell=tauri` |
| T3 (Codex) | nghiệm thu: E2E đủ bộ Tauri trên Windows, size, RAM / CPU, so Electron |

### 6c — chốt
- Đổi bản phát hành sang Tauri, bỏ `apps/desktop` Electron + `apps/collector` sau khi chủ nhân test Mac (checklist trong `.planning/CHECKLIST-v87-step6-mac.md`).

## 5. Cổng mỗi WP
- `cargo test` + clippy `-D warnings`; twin test TS↔Rust cùng input = cùng output (handler module); `npm test` desktop / web; typecheck; E2E bộ liên quan (Codex chạy, main tự kiểm).
- Không comment trong code, file ≤ ~400 dòng, chỉ Windows + macOS (code mac viết theo `cfg`, chủ nhân test).

## 6. Không được phá
- App Orrery thật của chủ nhân (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`): mọi lần chạy thử dùng port + data + userData tạm.
- Protocol WS / `packages/shared` không đổi. Hard rules sanitize / 127.0.0.1 / app key / không gọi LLM giữ nguyên.
