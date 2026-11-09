## WP-v87-6-spike — DONE
Spike `spikes/tauri-shell` (Tauri 2.12.5, WebView2, Rust 1.97.1) biên dịch và chạy được cả 7 mục. Mục chưa kiểm hết được ghi rõ ở "Chưa kiểm".

### Bảng số (Windows 11, máy chủ nhân đang dùng song song)
| Mục | Số đo |
|---|---|
| exe release (strip, lto, opt s) | 3.23 MB (đã nhúng `ui/`) |
| NSIS Setup (`cargo tauri build --bundles nsis`, webviewInstallMode skip) | 1.13 MB |
| `apps/web/dist` (renderer thật) | 22.0 MB raw, 11.3 MB zip |
| RAM rảnh shell trống (1 cửa sổ, 1 environment) | 78.0 MB private, 7 process (shell 3.4 + WebView2 75) |
| RAM rảnh renderer thật (`apps/web/dist`, không backend, hiện onboarding) | 112.6 MB private (max 118.7), 7 process, CPU 0.6% 1 core |
| Electron cùng loại (docs/perf/v87-step1-win32.json, idle-monitor) | 274.3 MB TOTAL (main 77 + renderer 66 + gpu 49 + collector 68 + util) |
| 8 xterm WebGL (xterm 6.0.0 + addon-webgl 0.19.0), 20s | 58.6 fps, frame p95 16.9 ms, max 33.5 ms, context loss 0, webglOk 8/8, ANGLE D3D11 Intel UHD 630 |
| CPU cả cây lúc 8 xterm | 54.7% của 1 core, RAM 226 MB private |
Ghi chú số: RAM đo bằng `WorkingSetPrivate` (CIM) qua `scripts/measure.ps1`, có thể lệch cách đo của `perf-bench.mjs`. Setup thực tế còn cộng core Rust + resource; ước vỏ + renderer ~12-13 MB.

### Child webview (`WebviewBuilder` + `Window::add_child`, feature `unstable`)
| Mục | Kết quả |
|---|---|
| Load trang ngoài `https://example.com` | Chạy |
| Resize / di chuyển (`set_position`/`set_size`) | Chạy: 600x380 -> 400x300 đọc lại đúng |
| Theo vùng layout (ResizeObserver trang chính -> command `child_bounds`) | Chạy: resize cửa sổ, maximize đều kéo child theo (684x644, 1604x976 px) |
| Chụp ảnh (WebView2 `CapturePreview` qua `with_webview`) | Chạy: PNG đúng kích thước child (600x380, 400x300, 684x644), chỉ nội dung child |
| Chặn điều hướng (`on_navigation`) | Chạy: `location.href` sang host lạ bị chặn, child giữ nguyên URL |
| Chặn popup (`on_new_window` -> `Deny`) | Chạy: `window.open` trả null, `target=_blank` bị deny |
| Cô lập storage (`data_directory` riêng) | Chạy: child A (dir mặc định) và B (dir riêng) cùng origin, B đọc `localStorage` trống (prior=null) |

### Header + snap / CDP
- Header tự vẽ (`decorations:false`, `data-tauri-drag-region`, nút min/max/close, đổi màu theo class theme): hiển thị đúng (ảnh). Maximize/unmaximize bằng API chạy.
- Style cửa sổ sau khi bỏ decoration: `0x14CF0000`, còn `THICKFRAME`, `MAXIMIZEBOX`, `MINIMIZEBOX`, `CAPTION`, `SYSMENU`. Win+mũi tên và resize vẫn dựa vào các cờ này.
- CDP: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>` -> `/json` liệt kê target (`http://tauri.localhost/`). Chạy.
- Chưa kiểm: kéo cửa sổ bằng drag region, snap layout flyout khi hover nút max tự vẽ (cần `WM_NCHITTEST` HTMAXBUTTON, cần chuột thật; không giả lập input trên máy đang dùng), bấm nút Theme, hành vi overlay HTML đè lên child.

### Rủi ro cho bước 6
1. Child có `data_directory` khác main = environment WebView2 riêng (thêm ~6 process, +100 MB). Child không đặt dir lấy `%LOCALAPPDATA%\<identifier>\EBWebView`, KHÔNG phải dir của main. Hai environment cùng `--remote-debugging-port`: chỉ environment đầu bind port, child ở environment thứ hai biến mất khỏi CDP (đã thấy). Chung dir với main (CDP thấy child, đã kiểm) thì mất cô lập storage. `cdp.cjs` E2E cho BrowserPane phải chọn một.
2. Child là HWND native nằm trên webview chính: popover, menu, dialog HTML của renderer sẽ bị child che; cần ẩn/thu child khi mở overlay (chưa kiểm, suy ra từ kiến trúc).
3. API `add_child` / `WebviewBuilder` là `unstable`; `on_new_window` có sẵn ở 2.12.5. Ghim phiên bản tauri.
4. Chụp ảnh phải gọi `webview2-com` + `windows` đúng phiên bản của tauri (0.39.1 / 0.62; chọn 0.38 / 0.61 làm lỗi biên dịch hai kiểu `windows` khác nhau); chạy trên UI thread bằng `wait_for_async_operation`. macOS cần plugin riêng (`takeSnapshot`), chưa làm.
5. `<webview>` tag bị thay bằng child do Rust điều khiển: toàn bộ API `capturePage`, `will-navigate`, `did-*` của BrowserPane/FilePane phải đi qua command + event mới.
6. Header X-CCM-App (`webRequest`) chưa thử, không có API tương đương; vẫn theo PLAN mục 2.
7. `cargo install tauri-cli --locked` ~25 phút build ở `-j 4`; CI cần cache. NSIS tải `nsis-3.13` + `nsis_tauri_utils` từ GitHub lần đầu.
8. Size Setup 1.13 MB chưa gồm WebView2 bootstrapper (máy Win11 có sẵn; Win10 cũ cần `embedBootstrapper`/`downloadBootstrapper`).

### Ảnh (`docs/qa/v87-6-spike/`)
`shell-main.png` (header tự vẽ + child example.com), `child-webview-capture.png` (684x644 từ CapturePreview), `child-webview-capture-600x380.png`, `real-renderer.png`, `xterm-8-webgl.png`.

### Lệnh đã chạy
`cargo install tauri-cli --version ^2 --locked -j 4` · `cargo tauri icon ..\..\apps\web\dist\icon-512.png` · `cargo build --release -j 4` · `cargo tauri build --bundles nsis` (CARGO_BUILD_JOBS=4) · chạy exe với `SPIKE_MODE=shell|real|xterm`, `SPIKE_AUTO=1`, `SPIKE_OUT`, `SPIKE_TMP`, `SPIKE_CHILD_DIR=main`, CDP port 9444-9448 · `scripts/measure.ps1`, `scripts/shot.ps1`. Static server nội bộ phục vụ `apps/web/dist` tại `/`, `node_modules` tại `/__nm/`, `ui/` tại `/__ui/`.

File đụng: `spikes/tauri-shell/**` (Cargo.toml, build.rs, tauri.conf.json, capabilities/, icons/, src/main.rs, ui/*.html, scripts/*.ps1), `.gitignore` (+2 dòng target/gen), `docs/qa/v87-6-spike/*`, báo cáo, nhật ký brief. Không commit. Process spike đã kill theo pid, `%LOCALAPPDATA%\dev.ccm.spike` và thư mục temp spike đã xoá.
