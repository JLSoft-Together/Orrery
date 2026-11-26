## WP-113h2 — PARTIAL (xong code + test; "điền sẵn ô nhập" chờ hot file chat; CHƯA KIỂM compile Kotlin)

### Đã đổi
- MỚI `apps/mobile/lib/install/`: `apk_offer.dart` (parse + kiểm offer: chỉ http IP LAN private, size 1..1GiB, sha 64 hex, exp), `apk_io.dart` (ApkSource HttpClient, ApkInstaller + MethodChannel `dev.ccm.orrery.remote/install`), `apk_flow.dart` (luồng tải/kiểm/cài), `apk_install_sheet.dart`, `apk_install_host.dart` (nghe tap push, giữ khi app lock).
- push: MỚI `lib/push/push_apk.dart`; sửa `push_notice.dart` (kind `apk`, payload mang offer), `push_seal.dart` (đọc `apk` trong seal), `local_notifier.dart` (nút "Tải và cài"), `push_service.dart` (stream `apkTaps`, `takeInitialApk`; file đã 419 dòng từ trước, thêm ~35), `push_present.dart` + `push_background.dart` (apk không bị phone-pause như `open`), `main.dart` (bọc `ApkInstallHost`).
- MỚI `lib/ui/run/`: `run_log_model.dart`, `run_log_page.dart`, `crash_send.dart`. Sửa `lib/ui/viewers/run_widgets.dart` + `run_page.dart` (nút vào trang log).
- Android: MỚI `ApkInstaller.kt`, `ApkFileProvider.kt`, `res/xml/apk_file_paths.xml` (đặt tên riêng tránh đụng plugin); manifest chỉ thêm `REQUEST_INSTALL_PACKAGES` + `<provider>`; `MainActivity.kt` thêm 1 dòng register.
- ARB en + vi sửa tay (gen_arb.mjs đã lỗi thời), `flutter gen-l10n` đã chạy.
- Test MỚI: `test/apk_install_test.dart`, `apk_sheet_test.dart`, `run_log_test.dart`.

### Tiêu chí
- [x] parse push apk (seal + tap payload + FCM hiện bởi hệ thống thì mở seal khi chạm); sha sai -> lỗi, xoá file, KHÔNG gọi installer; đúng -> gọi channel; ngoài LAN -> "Cần cùng Wi-Fi"; run log parse + tô crash (`FATAL EXCEPTION`, `Unhandled Exception`, `E/flutter`, kèm stack tiếp theo).
- [~] "Gửi crash cho agent": chọn pane agent cùng cwd, đặt draft vào `crashDraftProvider`, copy clipboard, mở pane, toast "chưa gửi gì". KHÔNG tự gửi. Ô nhập chưa tự điền vì `ChatScreen` không có API nhận draft (cấm sửa `lib/ui/chat/*`) -> tạm dùng clipboard.
- [x] analyze sạch; ARB en+vi; không comment; file < 400 dòng (trừ push_service đã 419 từ trước). Kotlin/manifest CHƯA KIỂM compile.

### Audit
- Chỉ nhận URL `http://` IP private v4 (10/172.16-31/192.168), tối đa 3, không userinfo. File tải về chỉ trong cache `apk_offer/`; Kotlin kiểm canonical path nằm trong thư mục đó + đuôi .apk trước khi cấp URI.
- Payload thông báo chứa ticket URL (cục bộ trên máy, sống ngắn theo `exp`).
- App lock đang khoá -> giữ offer, không tải/cài đến khi mở khoá.

### Review (edge case)
- Ticket hết hạn: exp cục bộ (+2 phút bù lệch giờ) hoặc HTTP 401/403/404/410 -> "Liên kết đã hết hạn".
- Tải đứt giữa chừng / thân ngắn hơn size / idle 15s -> interrupted, xoá file, bấm "Thử lại" được. Dài hơn size hoặc Content-Length lệch -> lỗi size.
- Không đủ dung lượng: kiểm `StatFs` trước (size + 16MB) và bắt ENOSPC khi ghi.
- Chưa cho "cài app không rõ nguồn": kiểm trước khi tải -> nút "Mở cài đặt"; quay lại app tự thử lại. Installer trả notAllowed cũng về trạng thái này.
- App bị kill khi tải: file dở nằm `cache/apk_offer`; quét file > 1 giờ lúc mở app, và xoá hết khi bắt đầu lần tải mới.
- Huỷ giữa chừng: về idle, xoá file dở (đã sửa lỗi Windows giữ handle khi test).
- Desktop cũ không có op `run.log`: lỗi `denied` -> "Máy tính này chưa hỗ trợ Run log, cập nhật Orrery" (CHƯA KIỂM với desktop thật, phản hồi op lạ suy từ `phone_ws.rs`).
- Hạn chế: crash prompt dùng log đã mask phía desktop (ring buffer); trang log kéo bằng `run.log since` mỗi khi có `run.event` log (debounce 250ms), tối đa 2000 dòng.

### Test
- `flutter analyze` -> No issues found.
- `flutter test --concurrency=2 test/push_actions_test.dart test/push_test.dart test/push_present_test.dart test/push_seal_test.dart test/push_ui_test.dart test/viewers_test.dart test/arb_no_dots_test.dart test/deep_link_test.dart test/apk_install_test.dart test/apk_sheet_test.dart test/run_log_test.dart` -> 127 passed.

### Nhìn
CHƯA NHÌN: sheet cài APK (tiến độ, lỗi), trang Run log (dòng crash tô đỏ, nút gửi crash), thông báo có nút "Tải và cài"; chưa chạy trên máy thật.

### Hot file cần thêm
- `apps/mobile/lib/ui/chat/chat_screen.dart` (WP khác): trong initState/sau frame đầu `final d = ref.read(crashDraftProvider.notifier).take(widget.paneKey); if (d != null) _composer.currentState?.fill(d);` (import `../run/crash_send.dart`); xong thì bỏ bước `Clipboard.setData` trong `crash_send.dart` và đổi chuỗi `runLogCrashCopied`. Với pane terminal (không chat) cần tương tự ở `input_bar`.
- `.code_index/mobile.md`: thêm `lib/install/*` (luồng cài APK offer, channel `dev.ccm.orrery.remote/install`), `lib/ui/run/*`, kind push `apk`, `push_apk.dart`.
- `.planning/QA-mac-checklist` không liên quan; thêm vào checklist máy thật Android: push apk, tải LAN, quyền cài app không rõ nguồn, trình cài, trang Run log.

### Ngoài phạm vi
Không đụng Rust, web, pubspec (không cần: dùng `cryptography` có sẵn cho SHA-256, `path_provider` có sẵn), lib/ui/chat, security, checklist, clip, WP-113i. ARB/manifest có WP-113i sửa song song, chỉ chèn dòng của gói này.

### Commit message đề xuất
feat(mobile): download and install an offered APK over LAN and add a Run log page with crash highlighting

Handle the apk push kind (seal or system-shown), verify size and SHA-256 before handing the file to the Android installer through a FileProvider, and add a Run log page backed by run.log and run.watch that tints crashes and prepares a crash prompt without sending it.
