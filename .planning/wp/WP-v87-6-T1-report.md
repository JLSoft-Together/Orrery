# WP-v87-6-T1 — báo cáo (2026-11-10)

Vỏ Tauri `apps/shell` nối `core-desk`, `<webview>` của BrowserPane / FilePane chạy bằng child webview do Rust điều khiển, chụp ảnh (child + dust), clipboard. E2E riêng `apps/shell/e2e/t1.cjs` **33/33 PASS**, agent-map **34/34** (2 lỗi clipboard của T0 đã hết), p2 **14/14**.

## Đã làm

1. **Nối `core-desk`** (`desk.rs`, `hosts.rs`, `desk_text.rs`): crate link thẳng qua path. `desk::boot` chạy trước `Serve::start`: `Desk::standard(env)` + `desk:backup-apply-pending` (thay `backups.applyPending()` lúc boot của Electron). Command `ccm`: kênh vỏ tự làm trước, mọi kênh `ccm:*` khác → `Desk::dispatch_with` trong `spawn_blocking`; `desk:*` / kênh lạ không tới desk; desk chưa có kênh → `not ported: <channel>` như T0. Event desk → `emit_to(main)` (+ `compact` nếu mở). Kênh D0 (backup, buildClean, libs) và kênh D1 / D3 đăng ký vào `Desk::standard` tự có trên Tauri.
2. **Host callback** (`hosts::call`), trả lời theo đúng hợp đồng D0:
   | method | Tauri |
   |---|---|
   | `backup.readLocal` | `eval_with_callback(LOCAL_SNAPSHOT_JS)` trên webview gọi, max 3 s, lỗi → `{}` |
   | `backup.github` | `[]` (chờ D2 GitHub + keyring) |
   | `backup.saveDialog` / `backup.openDialog` | dialog plugin, filter json, cha = cửa sổ main; `CCM_E2E_BACKUP_FILE` thay dialog như Electron |
   | `backup.confirmApply` | message dialog `OkCancelCustom`, câu en/vi giống `strings.ts` |
   | `backup.relaunch` | `quitting` + `shutdown` (pty, serve) + `app.restart()` sau 200 ms |
   | `buildClean.running` / `buildClean.daemons` | `[]` (chờ resources / run được port) |
   | `buildClean.trash` | crate `trash`, chỉ path tuyệt đối |
   | `buildClean.confirm` | message dialog, trả 0 (đồng ý) / 1 như `showMessageBox` |
   E2E: `CCM_E2E_CONFIRM=ok|cancel` trả lời mọi confirm của desk và ghi câu hỏi vào `<data>/e2e-confirms.log` (Tauri không có inspector Node như v74).
3. **Child webview** (`guest.rs`, `guest_ipc.rs`, `guest.js`, `webview.js`):
   - Trang app: `webview.js` (trong init script, sau shim + header) vá `Document.prototype.createElement('webview')` (tên `webview` không hợp lệ cho custom element) và gắn API mà `BrowserPane.vue` / `FilePane.vue` gọi: `loadURL`, `getURL`, `getTitle`, `canGoBack/Forward`, `goBack/Forward`, `reload`, `reloadIgnoringCache`, `stop`, `setZoomFactor`, `executeJavaScript` (await promise, reject khi lỗi), `getWebContentsId`, `capturePage`, `open/close/isDevToolsOpened`, `focus`, thuộc tính `src`; event `did-start-loading`, `did-navigate`, `dom-ready`, `did-finish-load`, `did-stop-loading`, `did-navigate-in-page`, `page-title-updated`, `console-message`, `page-favicon-updated`. `apps/web` **không sửa**.
   - Vị trí / size bám element (`getBoundingClientRect`, × zoom trang); khung thiết bị dùng `transform: scale` → child nhận `scale` và zoom tương ứng (iPhone 390 px đúng). Ẩn child khi element không hiện hoặc overlay HTML che (`[role=dialog|alertdialog|menu|tooltip]`, `[data-context-menu]`, con trực tiếp của `body` ngoài `#app`). Web không có hook overlay chung (mỗi component Teleport riêng) nên dò theo DOM. Theo dõi: rAF 450–900 ms sau input / resize / transition + `setInterval` 250 ms, chỉ khi còn child.
   - Rust: label `guest-<n>`, chung data dir với cửa sổ chính (plan §3.4), `on_navigation` chỉ http(s) và không phải origin app, `on_new_window` loopback → điều hướng chính child, web khác → trình duyệt ngoài, luôn Deny (như `hardenGuest`). `on_page_load` / `on_document_title_changed` → event; `canBack/canForward` lấy từ WebView2 `CanGoBack/CanGoForward` (mac `WKWebView`). Trang tải lại → đóng hết child của cửa sổ đó; zoom trang đổi → đặt lại vị trí child.
   - Child → vỏ: command `guest` (capability riêng `guest` cho webview `guest-*` trên remote http(s), chỉ quyền `allow-guest`; child gọi `ccm` bị từ chối — có E2E). Dùng cho reply của `executeJavaScript` (nonce 16 byte, chỉ nhận từ đúng child), console error / warning (max 50 / s, 4000 ký tự), favicon, `pushState` / hash, phím: `workspaceChord` port từ `preview/keys.ts` → `ccm:forward-key`, F5 / Ctrl+R reload, F12 devtools.
4. **Capture** (`native.rs`, `capture.rs`, `shot.rs`): WebView2 `CapturePreview` (handler async trong `with_webview`, không chặn UI thread). `ccm:capture-guest` → PNG child vào clipboard (`arboard`); `capturePage()` → data URL; `ccm:capture-rect` (dust) = chụp webview chính, cắt `rect × zoom × px/dip`, thu theo `dust_size` (port `capture.ts`, ≤ 160k px), max 4 Mpx. Kết quả đi dạng base64; shim đổi lại `Uint8Array` (`BINARY` trong `gen-desk-shim.mjs`) vì IPC remote của Tauri biến bytes `Raw` thành mảng số. macOS: `takeSnapshotWithConfiguration` → TIFF → `NSBitmapImageRep` PNG, `canGoBack` qua objc2 (theo `cfg`).
5. **Clipboard**: `on_permission_request` cho trang app: `ClipboardRead` → Allow (WebView2 mặc định hỏi). `ccm:clipboard-image` (data URL PNG ≤ 20 MB, kiểm chữ ký) và `ccm:clipboard-image-file` (tuyệt đối, png/jpg/gif/webp ≤ 20 MB) giải mã rồi ghi pixel vào clipboard, như `preview/clipboardImage.ts`.
6. Sửa lỗi init script: `header.js` kết thúc `})()` không `;` → IIFE ghép sau bị gọi như tham số. Giờ mỗi phần kết thúc bằng `;` (có test).

## File

- Mới: `apps/shell/src/{desk,desk_text,hosts,guest,guest_ipc,native,capture,shot}.rs`, `apps/shell/src/{guest,webview}.js`, `apps/shell/e2e/t1.cjs`, `apps/shell/permissions/autogenerated/guest.toml` (tauri-build sinh).
- Sửa: `apps/shell/{Cargo.toml, Cargo.lock, build.rs}` (command `guest`; dep `core-desk`, `arboard`, `image`, `trash`, `webview2-com =0.39.1`, `windows =0.62.2`, mac `objc2*`, `block2`; tauri feature `devtools`), `apps/shell/src/{main,ccm,window}.rs`, `apps/shell/src/shim.js` (sinh lại), `scripts/gen-desk-shim.mjs` (`BINARY`), `.code_index/{shell,_meta}.md`, plan §4 ghi chú T1.
- Không đụng: `apps/core/**` (core-desk của D1 / D3), `apps/desktop/src`, `apps/desktop/e2e/cdp.cjs`, `apps/web`.
- Không comment trong code; file dài nhất `guest.rs` 380 dòng.

## Số test thật

- `cargo test -j 4` (apps/shell): **50 passed** (24 cũ + 26 mới). `cargo clippy -j 4 --all-targets -- -D warnings`: sạch. `node scripts/gen-desk-shim.mjs --check`: up to date.
- E2E Tauri (`CCM_E2E_SHELL=tauri`, port 4481, CDP 9481, data tạm, build debug):
  | Bộ | Kết quả | Ghi chú |
  |---|---|---|
  | `apps/shell/e2e/t1.cjs` (mới) | **33/33 PASS** | load / exec / reject / title / CDP thấy child / size bám element / console / in-page nav / back-forward / chặn `file:` + `window.open` / ẩn dưới menu rồi hiện lại / iPhone 390 px / captureGuest → clipboard `image/png` / capturePage / Ctrl+Shift+P trong child → quick open / captureRect 200×100 PNG + rect hỏng → null / backup export (có `ccm.locale` từ readLocal) + pick + apply → confirm (Cancel) / build-scan qua desk / `desk:*` không gọi được / child không gọi được `ccm` / reload đóng child cũ, mở lại / đóng pane đóng child |
  | `agent-map` | **34/34 PASS** | 2 check clipboard T0 fail nay PASS |
  | `p2` | **14/14 PASS** | |
  | `browser` | 2 PASS / 1 FAIL | dừng ở "page loaded": mở file local cần `ccm:preview-url` + preview server (D3 chưa port) |
  | `tabs-cwd` | 10 PASS / 7 FAIL | nhánh git của pane (T0 chưa port, `wt`), quyền thư mục preview / `preview-grants.json` (D3) |
  | `v74` | 0 / 1 FAIL | cần inspector Node của main Electron (`no main inspector`); phần backup đã kiểm ở t1 |
- Sau mỗi lượt không còn `orrery-shell.exe` (stop = `taskkill /T` theo pid con tự spawn).

## Khác biệt còn lại / rủi ro

- **Cookie app lộ cho trang loopback trong BrowserPane**: child chung data dir với cửa sổ chính; cookie `ccm_app` (HttpOnly, SameSite=Strict) theo host, không theo cổng → trang ở `127.0.0.1:<cổng khác>` (preview server, dev server) cùng "site". Request có `Origin` bị `is_local_request` chặn, nhưng request không `Origin` (thẻ `<img>`, điều hướng GET) vẫn mang cookie, vd `GET /snooze`. Electron không bị vì `persist:ccm-preview` tách cookie. Đề xuất (core-http, ngoài phạm vi T1): chỉ nhận cookie khi `Sec-Fetch-Site` là `same-origin` / `none`, hoặc data dir riêng cho child (+~100 MB, mất CDP).
- macOS: code `cfg(target_os = "macos")` (capture `takeSnapshot`, `canGoBack`) **chưa biên dịch** — `cargo check --target aarch64-apple-darwin` dừng ở build script `objc2-exception-helper` (máy thiếu `cc` cho Apple). Chữ ký đã đối chiếu với source `objc2-web-kit` / `objc2-app-kit` 0.3.2. Clipboard read permission trên mac: WebView2-only.
- `executeJavaScript` chỉ nhận biểu thức (mọi chỗ gọi trong web đều là biểu thức); câu lệnh rời → "script failed". `userGesture` bỏ qua. `did-fail-load` chưa có (WebView2 tự hiện trang lỗi). Console chỉ error / warning (đủ cho handoff).
- Overlay dò theo DOM: overlay mới không có role / không Teleport ra `body` sẽ bị child che → thêm role hoặc selector.
- Child rời DOM > 3 s thì đóng (KeepAlive giữ lâu hơn sẽ mở lại từ URL, mất state trang).
- Tauri feature `devtools` bật cả bản release (Electron cho mở devtools guest bằng F12).
- FilePane preview HTML dùng cùng adapter nhưng cần preview server (D3) mới kiểm được E2E.
- Không thêm callback host mới vào `core-desk`; không thiếu API host.

## An toàn máy

CPU 7–16 % trước mỗi việc nặng, `cargo -j 4`, mỗi lúc một việc. Không chạm port 4317 / `%APPDATA%\Agent Monitor` / `%LOCALAPPDATA%\ccm`, không đăng ký protocol, không commit / push / checkout, không build dist. E2E có ghi clipboard thật (như agent-map của Electron).

STATUS: DONE
