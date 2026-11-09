## WP-v87-6-spike: spike vỏ Tauri 2 (Windows)
- Bối cảnh: `.planning/PLAN-v87-tauri-options.md` (khảo sát + chủ nhân chốt: Tauri không kèm Node, port hết sang Rust). `.planning/PLAN-v87-rust-core.md` §6 bước 6. Rust workspace hiện có `apps/core` (đọc `.code_index/core.md`), không đụng.
- Mục tiêu: trả lời bằng số đo, trên Windows 11 (WebView2 có sẵn):
  1. Size: app Tauri 2 trống (1 cửa sổ, tray, 1 command) — exe release + NSIS installer (`tauri build` bundle nsis). Ghi MB.
  2. Child webview thay `<webview>` (BrowserPane / preview): trong cửa sổ chính, nhúng 1 child webview (`WebviewBuilder` + `Window::add_child`, feature `unstable`) load trang ngoài (vd `https://example.com`), resize / di chuyển theo vùng layout, chụp ảnh nội dung child webview ra PNG (WebView2 `ICoreWebView2::CapturePreview` qua `with_webview`). Báo cái nào chạy, cái nào không.
  3. Chặn điều hướng / popup của child webview (`on_navigation`, new-window), cô lập storage (data directory riêng cho child nếu được).
  4. Load renderer thật: build `apps/web` (dùng `apps/web/dist` có sẵn nếu đã có; không thì `npm run build -w @ccm/web`) vào Tauri qua `frontendDist` hoặc phục vụ local; mở được trang, đo RAM process WebView2 + shell lúc rảnh so với Electron (ước từ `docs/perf/`). Không cần backend: lỗi kết nối WS là chấp nhận được, chỉ đo vỏ.
  5. xterm WebGL: trang test nhỏ trong spike mở 8 terminal xterm.js + `@xterm/addon-webgl` (dùng package trong `node_modules` của repo), ghi chữ liên tục, đo FPS / CPU WebView2, có context loss không.
  6. Header tuỳ biến: `decorations: false` + nút cửa sổ tự vẽ, drag region, đổi màu theo theme; kiểm Windows snap layout có còn.
  7. CDP: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>` → `http://127.0.0.1:<port>/json` liệt kê được target (để dùng lại `apps/desktop/e2e/cdp.cjs`).
- Phạm vi file: thư mục mới `spikes/tauri-shell/**` (Cargo project riêng, KHÔNG thêm vào workspace `apps/core`), `.gitignore` (chỉ thêm `spikes/tauri-shell/target/`, `spikes/tauri-shell/gen/` nếu cần), báo cáo. Cấm đụng mọi file khác. Không thêm dependency vào `package.json` của repo; cần Tauri CLI thì dùng `cargo install tauri-cli --locked` (ghi vào báo cáo) hoặc `cargo tauri` nếu đã có.
- Model: sonnet.
- Build: được `cargo build/run --release -j 4` trong `spikes/tauri-shell`, `cargo tauri build` (bundle nsis), `npm run build -w @ccm/web` một lần nếu thiếu dist. Chạy qua PowerShell (Git Bash có `link` coreutils che link.exe).
- An toàn máy: không chạm app Orrery đang chạy (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); WebView2 data dir của spike đặt trong thư mục tạm; dọn process chỉ theo pid tự spawn, không `taskkill /IM`.
- Tiêu chí nghiệm thu:
  - [ ] Bảng số: size exe + installer, RAM rảnh shell + WebView2 (renderer thật), FPS/CPU 8 xterm WebGL.
  - [ ] Child webview: load / resize / chụp ảnh / chặn điều hướng — từng mục chạy / không, kèm đoạn code mấu chốt.
  - [ ] Header tự vẽ + snap layout, CDP — chạy / không.
  - [ ] Kết luận rủi ro cho bước 6 + ảnh chụp (`docs/qa/v87-6-spike/`).
- Nhìn thấy được: có (ảnh cửa sổ spike) → chụp vào `docs/qa/v87-6-spike/`.
- Báo cáo: `.planning/wp/WP-v87-6-spike-report.md`

## Nhật ký
- 15:00 bắt đầu: cargo install tauri-cli ^2 --locked -j 4 (nền), dist web có sẵn, xterm + addon-webgl có trong node_modules
- 19:xx viết xong spikes/tauri-shell (Cargo.toml, tauri.conf.json, src/main.rs, ui/*.html); chờ tauri-cli cài xong rồi cargo build
- tauri-cli 2.12.4 đã cài (cargo install tauri-cli ^2 --locked -j 4, ~25 phút); cargo tauri icon từ apps/web/dist/icon-512.png; bắt đầu cargo build --release -j 4
- cargo build --release OK (sửa webview2-com 0.39 + windows 0.62 khớp tauri 2.12.5); exe 3.23 MB; chạy auto test shell
- auto test shell, real renderer, xterm 8 WebGL, CDP đã chạy (số trong báo cáo); chạy cargo tauri build (nsis)
- nsis build OK (Setup 1.13 MB); dọn temp; báo cáo ghi xong, DONE
