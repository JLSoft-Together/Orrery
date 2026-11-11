## WP-1 — PARTIAL (đo trên máy CHƯA KIỂM)
- Đã đổi: apps/mobile/** (spike xterm, FRB proto_version/echo, fixture 262383 B, cấu hình Android/iOS, .code_index/mobile.md), docs/qa/v97/WP-1/00-lock.png.
- Tiêu chí: 3/7 đạt. Chưa đạt: Rust trên máy thật, bảng số đo, Telex, ảnh sau sửa cuối, kết luận terminal.
- CHƯA KIỂM: A21s có app project khác (com.wejoy.ragdollplayground) ở foreground suốt lúc con kiểm; theo luật thiết bị dùng chung con không cài/mở/chụp, không force-stop. Đã kiểm lần 5 phút vẫn vậy; hết giờ chờ (30 phút) mà chưa rảnh thì main xếp lại.
- Số đo xterm.dart: chưa có trên máy. Chỉ có số PC trong flutter test: write 256 KiB = 55 ms, frame đầu 665 ms, 6451 dòng.
- Kết luận terminal: CHƯA KẾT LUẬN (chưa làm xterm.js WebView).
- Test: flutter analyze sạch; flutter test --concurrency=4 pass; build debug OK (718s); build `--profile` arm64 OK (182s, app-profile.apk 45 MB) tại apps/mobile/build/app/outputs/flutter-apk/app-profile.apk, CHƯA cài.
- Việc còn lại khi máy rảnh: adb install -r app-profile.apk, mở màn spike (nút trên màn chính), Nạp fixture, Đo cuộn, gõ Telex + Gửi, đọc log tag flutter "SPIKE", chụp ảnh vào docs/qa/v97/WP-1/.
- Commit message đề xuất: Add Orrery Remote Flutter spike app with xterm terminal and flutter_rust_bridge toolchain
