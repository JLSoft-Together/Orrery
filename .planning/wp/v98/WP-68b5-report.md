# WP-68b5 báo cáo: QA web app Sương kính + sửa lệch + bundle relay

Ngày 2026-11-20, máy mới (Windows, 20 luồng, 64 GB). Brief: `.planning/wp/v98/WP-68b5.md`.

## 1. E2E (2 engine)
- `npm run e2e -w @ccm/pwa -- --relayPort=8795 --inspectorPort=9355`: **96 / 96 pass, 0 fail** (WebKit + Chromium), chạy 2 lượt liền, 0 vi phạm CSP.
- Sửa test: `apps/pwa/e2e/lib/mist-ws-steps.mjs` ngưỡng phím dock `44 x 40` thành `43.5 x 39.5` (phím `left` đo 43.99997 px do làm tròn subpixel, không phải lỗi bố cục).
- Máy mới cần thêm: `cargo install wasm-bindgen-cli --version 0.2.130` (bản cũ 0.2.129 lệch crate, `ensure-wasm.mjs` hỏng làm `src/gen/` rỗng), `npx playwright install webkit`, `npm install` trong `apps/relay` (cờ `--workspaces=false`).

## 2. So ảnh với mockup
Bảng chi tiết: `.planning/wp/v98/WP-68b5-compare.md` (14 cặp). Đã sửa:
- `tone-*` scoped rò xuống component con, tô màu tên session / agent / chip: `MonitorView.vue` (`gtone-*`), `SessionView.vue` (`subtone-*`).
- Ô chữ project tone 4 mất nền vì `--ccm-success` không tồn tại: `styles/base.css`, `components/IpadRail.vue` dùng `--ccm-done`.
- Chip Relay / LAN thành nền nhạt không viền: `components/LinkChip.vue`.
- Nút màn "Bật khoá" đẩy xuống đáy: `App.vue` `.view:not(.fill)` flex column (E2E 96 / 96 sau khi sửa, không vỡ bố cục).

Lệch còn lại, cần gói riêng: Monitor iPad dạng lưới thẻ + tab con trong rail; thanh context trên hàng session; Limit ẩn segmented khi có banner; Detail nút terminal ở header; Workspace B lưới 2 cột.

## 3. Đo cuộn (rAF, 1.8 s)
| Engine | Terminal 64 KiB | Cuộn danh sách dài |
|---|---|---|
| Chromium | p50 6.9 ms, p95 7 ms, max 7.1 ms | p50 6.9 ms, p95 7 ms |
| WebKit (Playwright, Windows) | p50 15 ms, p95 18 ms, max 39 ms | p50 155 ms, p95 215 ms (12 frame) |

WebKit headless trên Windows vẽ bằng phần mềm nên số cuộn danh sách không đại diện cho Safari iOS (lượt 12 máy cũ: 206 ms). Cần đo trên iPhone thật (thêm vào `.planning/QA-mac-checklist.md`).

## 4. Test Rust / Android
- `cargo test --bin orrery-shell remote::`: 218 pass, 3 ignored.
- `disk_organize`: 8 / 8 pass trên máy mới (fail cũ ở máy cũ, nghi đồng hồ / mtime máy cũ, không lặp lại).
- `node apps/mobile/scripts/check.mjs`: analyze OK; `flutter test` pass hết sau khi build `apps/mobile/rust` (máy mới chưa có `rust_lib_orrery_remote.dll`, 3 file integration fail ở `setUpAll`). Android parse trường `os` không lỗi.
- `relay_setup` 29 / 29 pass với bundle mới.

## 5. Android release trên Galaxy A33 (SM-A336E)
- `flutter build apk --release --target-platform android-arm64` (38.6 MB). Máy mới cần `kotlin.incremental=false` trong `apps/mobile/android/gradle.properties` (pub cache ổ C, project ổ F, lỗi `Could not close incremental caches`).
- Cài, mở: không crash. Khoá app bật từ dữ liệu cũ, máy chưa có khoá màn hình, đã tắt khoá app (chủ nhân cho phép). Màn khoá làm `FLAG_SECURE` nên screencap đen là đúng.
- Ghép LAN với bản dev cách ly (`apps/shell/e2e/v97-device.cjs`, cổng 4421 / remote 4331 / điều khiển 4556): **đạt**, LAN 15 ms, tên máy "Samsung SM-A336E" (lỗi WP-22 #5 đã hết).
- Workspace khớp bản dev (1 tab, 1 pane shell); terminal PowerShell nhận lệnh, Enter, ^C, lỗi đỏ hiện đúng; `1..3000` không đổi kích thước: L2924 đến L3000 liền mạch, cuộn vuốt mượt.
- **Lỗi (WP-22 #3 tái hiện):** `1..2000` rồi ẩn bàn phím 1 s sau Enter (pane đổi số hàng giữa lúc output chảy): buffer điện thoại mất `line-1963` đến `line-1984` (22 dòng), vuốt lên vẫn thiếu. Lần chạy không resize thì không mất. Nghi resize ConPTY giữa burst (ConPTY vẽ lại viewport, dòng đã cuộn qua giữa 2 khung bị bỏ) hoặc phía `apps/mobile/lib/ui/terminal/terminal_pane.dart` khi nhận snapshot sau resize. Chưa tái hiện lần 2 (2 lần thử Back thoát ra Workspace thay vì ẩn bàn phím). Desktop xterm của pane không có scrollback để so.
- Nhỏ: thẻ pane ở Workspace phone chỉ hiện "Rảnh", không có xem trước terminal.
- `adb input text` nhân đôi `%`; tiếng Việt không gõ được qua adb (không phải lỗi app).

## 6. Build web app + bundle relay
- `npm run build -w @ccm/pwa` OK; `CCM_PWA_DIST=apps/pwa/dist node scripts/relay-bundle.mjs`: hash **`dc9ab3569380568a`**, 1 848 106 byte (gzip 683 429), ghi `apps/shell/resources/relay/worker.js(on)`.
- Ảnh cũ bị E2E ghi đè đã `git checkout --`.

## Test macOS / iPhone (gom lượt cuối)
- Đo cuộn danh sách dài Monitor / Workspace trên Safari iPhone thật (WebKit Windows không đại diện).
- Màn lock Face ID (UA test Windows chỉ hiện "Bật khoá màn hình").
- Kiểm 4 sửa bố cục ở mục 2 trên iPhone + iPad.
