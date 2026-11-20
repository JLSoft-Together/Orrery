# PLAN mac E2E: cổng CDP giả cho WKWebView (2026-11-20)

Mục tiêu: chạy được bộ E2E `apps/desktop/e2e/*.cjs` (driver `cdp.cjs`) với shell Tauri trên macOS, trước khi release (chủ nhân: "#4 #5 phải test ok trên mac").

## Vì sao
WebView2 (Windows) có `--remote-debugging-port`; WKWebView không có CDP. `cdp.cjs` chỉ cần `/json` + 1 WebSocket nói JSON CDP, nên shell bản debug tự mở cổng giả.

## Thiết kế
- `apps/shell/src/e2e_cdp.rs`, chỉ macOS, chỉ khi `config::devtools_allowed()` (bản debug hoặc đủ biến E2E) và có `CCM_E2E_CDP_PORT`.
- `127.0.0.1:<port>` HTTP `/json` → 1 target page (url của cửa sổ chính); `127.0.0.1:<port+1>` WebSocket.
- Lệnh:
  - `Runtime.evaluate`: `webview.eval` bọc async, kết quả về qua `invoke('ccm', 'ccm:e2e-result')` → `Shell.replies` (timeout 20 s).
  - `Page.reload` → `location.reload()`; `Page.loadEventFired` khi `PageLoadEvent::Finished` của trang app.
  - `Page.captureScreenshot` → `native::capture` (WKWebView snapshot).
  - `Input.dispatchMouseEvent` / `dispatchKeyEvent` / `insertText`: dựng DOM event trong trang (pointer + mouse + click/dblclick, focus khi mousedown, wheel tự cuộn, keydown + `execCommand('insertText')`, Tab chuyển focus). Không dùng CGEvent để không giành chuột / bàn phím của chủ nhân.
  - `Emulation.setDeviceMetricsOverride` / `Browser.setWindowBounds` → đổi inner size cửa sổ (logical); clear → trả size cũ.
  - `Emulation.setEmulatedMedia` `prefers-color-scheme` → `set_theme`.
  - `Page.addScriptToEvaluateOnNewDocument` → chạy lại mỗi lần trang app load xong (gần đúng).
  - Không hỗ trợ: `Input.dispatchDragEvent`, `Input.imeSetComposition` (trả lỗi).
- `cdp.cjs`: trên darwin + shell Tauri thêm env `CCM_E2E_CDP_PORT=<DBG>`.

## Đánh đổi
- Event dựng trong JS có `isTrusted=false`: phần lớn handler Vue / xterm vẫn chạy; hành vi mặc định của trình duyệt (chọn chữ bằng kéo, menu native) không có. E2E dựa vào các thứ đó ghi vào danh sách "không chạy trên Mac".

## Bước
1. Module + hook (`main.rs`, `window.rs` page load, `ccm.rs` kênh `ccm:e2e-result`, `state.rs` `expect`).
2. `cdp.cjs` env.
3. Chạy toàn bộ E2E trên Mac, sửa lỗi app thật / ghi lý do test không hợp Mac.
4. Build `.dmg`, cài, test lại.
