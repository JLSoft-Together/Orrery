## WP-1: Spike terminal Flutter + toolchain FRB (Orrery Remote)
- Mục tiêu: có khung app Flutter `apps/mobile` (Orrery Remote, `dev.ccm.orrery.remote`) chạy được trên Android thật, trả lời được câu hỏi "xterm.dart có đủ dùng không" bằng số đo + ảnh, và chứng minh chuỗi build Rust → Android qua flutter_rust_bridge chạy được.
- Spec item: PLAN-v97-mobile.md T3, T9, H3, review #22, #25.
- Phạm vi file: chỉ `apps/mobile/**` (tạo mới), `docs/qa/v97/WP-1/**`, file này (Nhật ký), báo cáo. | Cấm đụng: mọi thứ khác, đặc biệt `apps/core/**`, `apps/shell/**`, `apps/web/**`, root `package.json`.
- Phụ thuộc: — | Song song với: WP-2, WP-3 (Rust ở `apps/core`, cùng máy ⇒ giới hạn tài nguyên bên dưới) | Worktree: không.
- Skill bắt buộc: `flutter-tips-tricks` (đọc trước khi code), `flutter-code-indexer` (tạo `apps/mobile/.code_index/` tối thiểu ở cuối).
- Model: sonnet.

### Việc
1. `flutter create --org dev.ccm.orrery --project-name orrery_remote --platforms android,ios apps/mobile`; applicationId / bundle id `dev.ccm.orrery.remote`; tên hiển thị "Orrery Remote". minSdk 24.
2. Android: `network_security_config` cho phép cleartext (base-config, app này) — LAN dùng `ws://` + Noise; `android:allowBackup="false"` + `dataExtractionRules` chặn backup; quyền `INTERNET`, `CHANGE_WIFI_MULTICAST_STATE`, `ACCESS_WIFI_STATE`, `CAMERA`. iOS Info.plist: `NSLocalNetworkUsageDescription`, `NSBonjourServices` = `_orrery._tcp`, `NSCameraUsageDescription`, ATS `NSAllowsLocalNetworking` (chưa build iOS, chỉ cấu hình).
3. Rust qua flutter_rust_bridge (bản 2.x mới nhất): crate `apps/mobile/rust` (workspace Cargo riêng, KHÔNG thêm vào `apps/core`), 1 hàm mẫu `proto_version() -> u32` trả 1 và `echo(bytes) -> bytes`; gọi từ Dart, hiện trên màn spike. Dùng cargokit / tích hợp Gradle mặc định của FRB (không cần `cargo-ndk` nếu FRB không đòi; cần thì `cargo install cargo-ndk`). NDK có sẵn: `$ANDROID_HOME/ndk/28.2.13676358` (hoặc bản FRB yêu cầu). Target Rust Android đã cài.
4. Màn spike "Terminal": package `xterm` (pub.dev, 4.x). Nút nạp fixture: file asset ~256 KiB output ANSI thật của Claude Code TUI (tự tạo: chạy `claude --help`, `git log --color=always -200`, chuỗi truecolor `\x1b[38;2;r;g;bm`, ký tự tiếng Việt có dấu, ký tự rộng ⬛/emoji, alt screen `\x1b[?1049h`). Đo: thời gian nạp 256 KiB, FPS khi cuộn (Flutter DevTools hoặc `FrameTiming`), RAM; chụp ảnh. Khung nhập dòng dưới terminal (TextField, gõ tiếng Việt bằng Gboard / Samsung Keyboard Telex) + nút Gửi ghi vào terminal (local echo) + thanh phím phụ Esc / Tab / Ctrl-C / ↑ ↓ ← → / Enter (chỉ ghi chuỗi escape vào terminal để xem).
5. Phương án dự phòng (chỉ làm nếu xterm.dart hỏng nặng — vỡ màu, vỡ dấu tiếng Việt, < 30 FPS cuộn): màn thứ 2 xterm.js trong `webview_flutter`, cùng fixture, cùng số đo. Ghi rõ đã làm hay không.
6. Build + cài: `flutter build apk --debug` (chạy qua `pwsh -File C:\Users\dauph\bin\run-capped.ps1 -Exe flutter -ArgList build,apk,--debug` nếu được; Gradle `--max-workers=4`), `adb -s 192.168.1.217:43169 install -r …`. Máy: Samsung Galaxy A21s (SM-A217F, Android 12, 720×1600) qua adb Wi-Fi. Trước khi dùng máy: kiểm `device.lock` (nếu có file `D:\work\device.lock` hoặc app khác đang foreground của project khác thì dừng, ghi vào báo cáo); không đổi setting máy.
7. Chụp ảnh màn spike (terminal đã nạp fixture, đang cuộn, khung nhập có chữ tiếng Việt) vào `docs/qa/v97/WP-1/` bằng `adb exec-out screencap -p`.

### Tiêu chí nghiệm thu
- [ ] `apps/mobile` có app id `dev.ccm.orrery.remote`, tên "Orrery Remote"; `flutter analyze` sạch; `flutter test` pass (≥ 1 test widget màn spike).
- [ ] Gọi Rust qua FRB chạy trên máy thật (ảnh màn hiện `proto_version = 1`).
- [ ] Bảng số đo xterm.dart: thời gian nạp 256 KiB, FPS cuộn, RAM; đánh giá truecolor / tiếng Việt / ký tự rộng / alt screen (đạt / lỗi + ảnh).
- [ ] Khung nhập dòng gõ được tiếng Việt Telex, Gửi đưa đúng chuỗi UTF-8 vào terminal.
- [ ] Cleartext + allowBackup + quyền Android + key Info.plist đã cấu hình.
- [ ] Ảnh trong `docs/qa/v97/WP-1/` chụp SAU lần sửa code cuối.
- [ ] Kết luận rõ: dùng xterm.dart / xterm.js WebView / cần native, kèm lý do.

### Lệnh test
`cd apps/mobile && flutter analyze && flutter test --concurrency=4`

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/v97/WP-1-report.md`

### Nhật ký
- 12/11: tạo apps/mobile (flutter create, appId, manifest, plist), FRB integrate + proto_version/echo, màn spike + fixture 256KiB, analyze sạch, test pass. Tiếp: build APK + đo trên máy.
- 12/11: build+cài APK OK; máy khoá PIN, dừng ở bước đo/chụp. Xem WP-1-report.md
