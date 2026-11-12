# WP-22 RESULT: QA thật trên Galaxy A21s (SM-A217F, 720x1600, debug APK) với Orrery dev Tauri
nBáo cáo đầy đủ + ảnh: docs/qa/v97/WP-22/RESULT.md. File đổi: apps/mobile/lib/app/pair_page.dart, apps/mobile/test/pair_page_test.dart, apps/shell/e2e/v97-device.cjs.

Ngày 2026-11-12. Điện thoại 192.168.1.217 (subnet .1), PC 192.168.3.27 (subnet .3): định tuyến được, LAN ghép + nối bình thường, không bị firewall chặn (cổng dev 4331 / 4332, không dùng 4320 để khỏi đụng app thật). Dev Tauri chạy bằng `apps/shell/e2e/v97-device.cjs` (CDP, `CCM_DATA_DIR` tạm, keyring service riêng `Orrery E2E v97 device`, HTTP điều khiển 127.0.0.1:4556).

| # | Mục | Kết quả | Ảnh |
|---|---|---|---|
| 0 | Ô "Nhập mã ghép" (kDebugMode) + test widget | ĐẠT (`flutter test test/pair_page_test.dart` pass, build APK debug OK) | |
| 1 | Cài APK, mở app, màn đầu | ĐẠT | 01-first-screen.png |
| 2 | Ghép LAN (IP) + native confirm, máy vào list | ĐẠT | 02-paired-lan.png |
| 3 | Workspace khớp store Pinia, mở pane shell từ điện thoại | ĐẠT (1 → 2 pane khớp store; desktop vẫn ở workspace) | 03-workspace.png, 03b-pane-opened.png |
| 4 | Terminal: echo, độ trễ, cuộn 2000 dòng, tiếng Việt / box-drawing | ĐẠT có 2 lỗi nhỏ (xem dưới) | 04a-d |
| 5 | Viewer: Explorer, mở file, Changes, Monitor | ĐẠT | 05a-d |
| 6 | Relay | ĐẠT (LAN chặn bằng cách đổi cổng listener dev 4331 → 4332) | 06a, 06b |
| 7 | Push (setup, test, done, remove) | LỖI chặn: `remotePushSetup` trả `keyring`, không tới được test thông báo | |
| 8 | Thu hồi | ĐẠT (điện thoại hiện "Máy đã thu hồi điện thoại này" trong ~6 s, qua relay) | 08a, 08b |

## Số đo mục 4 (debug APK, A21s, LAN)
- Enter → output hiện trên điện thoại: **~0.10 – 0.17 s** (screenrecord + ffmpeg, độ phân giải 33 ms; frame 1.93 s lệnh đã vào terminal, frame 2.00 s có output).
- Cuộn trong lúc `1..2000` chảy (vuốt liên tục 8 vòng): SurfaceFlinger latency của SurfaceView Flutter, 126 frame trong 2.65 s = **47.5 fps**; khoảng giữa 2 frame p50 16.7 ms, p90 33.4 ms, p99 33.5 ms; **13/125 khoảng > 33 ms (10%)**, 1 khoảng > 50 ms, max 66.6 ms. Có giật nhẹ, chưa mượt 60 fps. Lưu ý: bản debug (JIT), chưa đo bản release/profile.
- `dumpsys gfxinfo` trả 0 frame (Flutter vẽ qua SurfaceView, không qua HWUI) nên không có jank % từ gfxinfo.
- Tiếng Việt ("Xin chào ạếữ") hiển thị đúng; `1..3000` không vuốt: 3000 dòng liên tục, không mất dòng (04d).

## Lỗi
1. **Push setup luôn lỗi `keyring` trên Windows (chặn mục 7).** `apps/shell/src/remote/push_setup.rs` ghi `fcm_files::stored(...)` (`client_email` + `private_key`, ~1.8 nghìn ký tự) vào keychain qua crate `keyring`, Windows Credential Manager lưu UTF-16 và max blob 2560 byte (~1280 ký tự). Đã đo bằng `CredWrite`: 1280 ký tự OK, 1281 ký tự lỗi 1783; 1800 ký tự lỗi. Service account Firebase thật không ghi được. Tái hiện: `remotePushSetup(google-services.json, service account)` trong app dev Windows. Hướng sửa: chia secret thành nhiều entry, hoặc lưu file mã hoá DPAPI / ghi PEM theo DER (~1.2 KB) hoặc nén. Chưa thử `remotePushTest`, thông báo trên điện thoại, push `done`.
2. **Box-drawing lệch dọc trong ô** (`┌┐` thấp, `└┘` cao, `│` kéo dài) trong terminal điện thoại, chữ `█▓` đúng (04c-vietnamese-box.png). Nghi font fallback / đo metric ô trong terminal pane của app mobile (`apps/mobile/lib/ui/terminal/terminal_surface.dart`, `terminal_fonts.dart`).
3. **Nghi mất dòng khi vuốt cuộn lúc output đang chảy**: `1..2000` hiện `1962 → 1981`, `1987 → 1990` (04a-scroll2000.png, 04c). Không lặp lại khi không vuốt (04d). Cần tái hiện: chạy `1..2000`, vuốt lên xuống liên tục ngay lúc đang chảy. Nghi `apps/mobile/lib/ui/terminal/terminal_pane.dart` / `terminal_surface.dart` (stream pty → xterm).
4. Lệnh dài gõ vào ô nhập khi bàn phím chưa mở đôi lúc bị nhập thẳng vào terminal mà không Enter; gửi ô trống / chỉ khoảng trắng không làm gì (có thể đúng thiết kế, ghi nhận).
5. Tên thiết bị ghép hiện "Android phone" thay vì model (SM-A217F). Nhẹ.
6. Trạng thái badge "LAN" ở danh sách máy giữ nguyên khi desktop tắt / bật remote lại (không thấy chuyển "đang kết nối lại"); chưa xác nhận là lỗi.

## Việc cần chủ nhân
- Quyết định cách lưu service account Firebase trên Windows (lỗi 1), rồi chạy lại mục 7 (đã sẵn script: `POST http://127.0.0.1:4556/push-setup` trong `v97-device.cjs`, đọc file service account trong Node, không in ra).
- Relay `orrery-relay.jinvu.workers.dev` giữ 1 machine id dev đã tự đăng ký lúc QA (không có API gỡ đăng ký); vô hại nhưng có thể xoá phía Cloudflare nếu muốn.
- Không cần bấm Allow firewall: LAN chạy được qua định tuyến subnet.

## Dọn
Dev app tắt (không còn `orrery-shell` / `msedgewebview2` của QA; `orrery-core` và `Orrery.exe` còn lại là của chủ nhân), điện thoại về Home, `show_touches` đã trả 0, app Orrery Remote vẫn cài (đã "Gỡ máy" sau thu hồi), `remotePushRemove` đã gọi (không có gì lưu). Thư mục `D:\tmp` có sẵn của chủ nhân, file `m4.mp4` do QA ghi nhầm vào đó đã xoá.
