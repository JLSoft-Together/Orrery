## WP-1b: Spike — cửa sổ Tauri ẩn xuống tray có bóp timer của trang không
- Mục tiêu: trả lời bằng số đo: trên Windows, khi cửa sổ `main` của Orrery (Tauri, WebView2) bị `hide()` vào tray > 5 phút, `setTimeout` / `setInterval` 200 ms trong trang trễ bao nhiêu; và cách nào giữ đúng nhịp (WebLock pending, `additional_browser_args` `--disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows`, AudioContext im lặng, hoặc dời timer sang Rust). Chốt 1 cách cho WP-6 / WP-7.
- Spec item: PLAN-v97-mobile.md H1, H6; review #2.
- Phạm vi file: script đo tạm trong `scripts/spike/v97-throttle/**` (tạo mới, xoá được sau), báo cáo. KHÔNG sửa code app (`apps/**`) — nếu cần thử cờ WebView2 thì dùng biến môi trường `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` khi chạy bản dev, hoặc chạy app có sẵn rồi đo qua CDP.
- Phụ thuộc: — | Song song được với: WP-4, WP-9 | Worktree: không.
- Model: sonnet.

### Cách đo (gợi ý)
- Chạy bản dev Tauri theo `.code_index/shell.md` (debug, `CCM_PORT` trống → 4419, data dir `%TEMP%/orrery-shell-dev`) — KHÔNG đụng app thật đang chạy của chủ nhân (port 4317, `%LOCALAPPDATA%\ccm`). Nếu bản dev chưa build: build debug `apps/shell` (`cargo build -j 3`) được phép trong spike này; không build release.
- Mở CDP (WebView2 `--remote-debugging-port`, qua `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`) → inject script ghi `performance.now()` mỗi tick 200 ms vào `localStorage` / gửi về; ẩn cửa sổ (đóng = vào tray), chờ 6 phút, hiện lại, đọc độ trễ (p50 / max) theo phút.
- Lặp với từng cách giữ nhịp. Ghi kết quả bảng.

### Tiêu chí nghiệm thu
- [ ] Bảng số đo: không làm gì / WebLock / cờ WebView2 / AudioContext — độ trễ p50, max theo phút 0–6.
- [ ] Kết luận 1 cách cho WP-6 / WP-7, kèm mã cần thêm (mô tả, không sửa app).
- [ ] App thật của chủ nhân không bị đụng (ghi rõ port / data dir đã dùng).
- [ ] Dọn process dev sau khi đo.

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-1b-report.md`

### Nhật ký
