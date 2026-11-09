# PLAN v87 — vỏ Tauri: khảo sát + phương án (2026-11-09)

Nguồn: khảo sát chỉ đọc (sonnet) trên `apps/desktop/src`, `apps/web/src`, `apps/collector`. Số đếm là grep thô.

## Kết luận
- Electron main không chỉ là vỏ: ~42k dòng TS, 277 `ipcMain.handle`, preload ~300 method (devices/adb, run, preview, layout, github, telegram/gramjs, extensions, MCP `orrery`, PtyBroker…). Collector thêm 12.7k dòng.
- Bước 3–5 của PLAN-v87 chỉ thay collector. Sau bước 5 vẫn còn ~40k dòng Node của main → Tauri "không Node" (20–35 MB như §6 bước 6) cần port thêm phần main, ước nhiều tháng.
- PLAN-v87 §6 bước 6 cần sửa: thêm 6a (tách main thành Node sidecar), 6b (vỏ Tauri), mục tiêu size thực tế.

## Phương án
| | A: Tauri + Node sidecar | B: đợi port hết sang Rust |
|---|---|---|
| Setup Windows (ước) | 45–60 MB | 20–35 MB |
| Cài xong (ước) | 150–200 MB | < 100 MB |
| Thời gian tới bản đầu | vài tuần sau spike | sau bước 5 + port main (nhiều tháng) |
| Đi tiếp | chuyển dần module sidecar → Rust (bước 3–5 rồi telegram, devices…), hết module thì bỏ Node | — |

## Vướng phải giải trước khi dựng vỏ
1. `<webview>` + `capturePage` (BrowserPane, FilePane, `preview/ipc.ts:965`, dust `main.ts:1488`): Tauri không có tag; child webview (unstable) + plugin chụp ảnh riêng (WebView2 `CapturePreview`, WKWebView `takeSnapshot`).
2. `webRequest.onBeforeSendHeaders` gắn `X-CCM-App` (`main.ts:814`): Tauri không có → đổi xác thực app-key (token trong URL / subprotocol / IPC).
3. Migrate `localStorage` (160 chỗ / 59 file, origin `127.0.0.1:<CCM_PORT>`) và `safeStorage` (DPAPI / Keychain → keyring plugin).
4. `titleBarOverlay` đổi màu theo theme: tự vẽ nút cửa sổ.
5. `powerMonitor`, `powerSaveBlocker`: viết Rust.
6. E2E: WebView2 có CDP (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`), macOS WKWebView không → E2E macOS cần cách khác.
7. Bridge `statusline-bridge.mjs` / `hook-bridge.mjs` đang chạy bằng exe Electron (`ELECTRON_RUN_AS_NODE`) → binary Rust nhỏ hoặc Node sidecar; config cũ của người dùng trỏ vào exe cũ.

## Spike trước (1–2 ngày)
- Child webview Tauri thay `<webview>`: load trang ngoài, chụp ảnh.
- WebView2 + xterm WebGL 8 pane.
- Size thật Node SEA sidecar + Tauri shell trống trên Windows.

## Chủ nhân chốt (2026-11-09)
- Tauri **không** kèm Node sidecar: đợi port hết sang Rust (bước 3–5 + phần main ~40k dòng), mục tiêu Setup ~20–35 MB.
- Bước 3 (`core-procs`) và spike Tauri chạy song song (không chung file).
- Test giao Codex (main vẫn tự kiểm kết quả).

## Kết quả spike (WP-v87-6-spike, 2026-11-09, Windows 11, Tauri 2.12.5)
| Đo | Tauri | Electron hiện tại |
|---|---|---|
| exe / NSIS Setup trống | 3.23 MB / 1.13 MB (chưa kèm WebView2 bootstrapper) | Setup 104.7 MB |
| Ước Setup vỏ + `apps/web/dist` (11.3 MB nén) | ~12–13 MB + core Rust | — |
| RAM rảnh, renderer thật (không backend) | 112.6 MB private | ~206 MB (274 tổng − collector 68), cách đo có thể lệch |
| 8 xterm WebGL 20 s | 58.6 fps, p95 16.9 ms, không context loss | — |
- Child webview (`add_child`, feature `unstable`): load, resize / move theo layout, chụp PNG (`CapturePreview`), chặn điều hướng + popup, storage riêng — đều chạy.
- Header tự vẽ + CDP (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`) chạy. Chưa kiểm: kéo cửa sổ, snap layout flyout, overlay HTML bị child che.
- Rủi ro: child storage riêng = environment WebView2 thứ 2 (+~100 MB, 6 process, không vào CDP) → BrowserPane chọn chung dir (E2E được, mất cô lập) hoặc riêng; child HWND che popover / menu HTML; `add_child` unstable → ghim version; `webview2-com` 0.39 / `windows` 0.62 phải khớp tauri; macOS cần plugin `takeSnapshot`; header `X-CCM-App` chưa có lời giải; CI cache `tauri-cli` (~25 phút build).
- Spike code: `spikes/tauri-shell/`, ảnh `docs/qa/v87-6-spike/`, báo cáo `.planning/wp/WP-v87-6-spike-report.md`.
