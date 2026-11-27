## WP-113i — DONE (cần main thêm 2 dòng hot file, xem dưới)

### Đã đổi
Desktop (Rust, `apps/shell/src/remote/`):
- `fs_put.rs` (mới, 243 dòng) + `fs_put_tests.rs` (mới): op `fs.put`. Nạp bằng `#[path]` trong `viewer_map.rs` (không đụng `mod.rs`).
- `viewer_map.rs`: `Arg::Put`, `SPECS` 37 -> 38 (`s("fs.put", FS_PUT, Arg::Put)`), `put_args` (kiểm root / `up` / `seq <= 64` / `last` / `b64 <= 512 KB`).
- `viewer.rs`: thêm 1 nhánh `FS_PUT => fs_put::reply(...)` + 1 dòng import.
- Field id của upload đặt tên `up` (không phải `id` như brief) vì `command_of` cắt `id` (đó là id request). Phone gửi `fs.put {root, up, seq, last, b64}`.
- Reply: chunk giữa `{ok, done:false, next}`; chunk cuối `{ok, done:true, path, name}`; lỗi `{ok:false, error: invalid|denied|order|expired|size|busy|image|io}` (`denied` đi qua `outcome` thành error code `denied`).
- Bảo mật: root phải qua guard viewer có sẵn (probe `ccm:list-dir {root, dir:root}` trả mảng, chỉ ở chunk seq 0); desktop `control()` tắt thì từ chối; từ chối root có `..`, UNC, NUL, root không phải thư mục (không tự tạo root); `.orrery` và `inbox` không được là symlink / file; tên file do desktop đặt `yyyyMMdd-HHmmss[-n].jpg|png` (UTC, `create_new` không ghi đè, hậu tố `-n`); đuôi lấy từ magic bytes (`image::guess_format`, chỉ JPEG / PNG, còn lại `image`); buffer theo `(owner, up)`, max 4 upload mở mỗi owner, max 8 MB, max 64 chunk, hết hạn 60s im lặng, mọi lỗi xoá buffer; không log nội dung.

Mobile (`apps/mobile`):
- `lib/media/`: `image_prep.dart` (đọc kích thước JPEG / PNG, nén: cạnh dài > 2048 thì thu nhỏ bằng `dart:ui` ra PNG, > 8 MB từ chối), `image_upload.dart` (chia chunk 384 KB raw = 512 KB base64, gửi tuần tự, map lỗi), `inbox_ignore.dart` (đọc `.gitignore`, thêm dòng `.orrery/inbox/` giữ eol + base mtime, qua `fs.read` / `fs.write` sẵn có có confirm), `dictation.dart` (engine trừu tượng + `PlatformSpeechEngine` bọc `speech_to_text`), `media_flow.dart` (chọn ảnh bằng `image_picker` kèm maxWidth/Height 2048 + quality 85, chống gửi 2 ảnh cùng lúc), `media_prefs.dart`, `media_providers.dart`.
- `lib/ui/chat/chat_media.dart` (mới): `PhotoButton`, `MicButton` (giữ để nói, thả để dừng, chữ điền vào ô nhập, KHÔNG tự gửi), `ComposerMedia`.
- `chat_composer.dart`: thêm tham số `media`, `draft` / `setDraft` / `addPath`, 2 nút (chỉ thêm). `input_bar.dart` không có trong repo (composer là `chat_composer.dart`), nên không đụng.
- `chat_screen.dart` (không có trong danh sách phạm vi nhưng bắt buộc để nối): thêm `_media()` + truyền `media:`. Nút ảnh chỉ hiện khi hello caps có `fsput` (desktop cũ không có thì ẩn; nếu vẫn gọi thì lỗi `denied` hiện thông báo).
- `pubspec.yaml` / `pubspec.lock`: `speech_to_text ^7.5.0`, `image_picker ^1.2.4`.
- `AndroidManifest.xml`: thêm `RECORD_AUDIO` (CAMERA đã có) và `<intent><action android.speech.RecognitionService/>` trong `<queries>`. Chỉ thêm 2 khối, không sửa dòng khác.
- ARB en + vi: 27 key `media*` (thêm tay ở cuối 2 file ARB, `flutter gen-l10n` đã chạy; `tool/gen_arb.mjs` KHÔNG cập nhật nên chạy lại script đó sẽ mất các key `clip*` và `media*`, đã lệch từ trước).

### Tiêu chí
- [x] cargo `fs.put`: ghép chunk, sai thứ tự, thiếu chunk, > 8 MB, root ngoài guard / có `..` / không tồn tại / `.orrery` là file bị từ chối, tên do desktop đặt, không ghi đè (hậu tố), buffer hết hạn 60s, view-only từ chối trước khi chạm guard.
- [x] Dart: nén (cạnh dài <= 2048) + chunk <= 512 KB base64; mic điền chữ không gửi; gợi ý gitignore chỉ gửi `fs.write` khi bấm "Thêm" (Bỏ qua thì không đọc / ghi gì, và không hỏi lại cho root đó).
- [x] `flutter analyze` toàn app sạch; không comment; ARB en + vi, không em dash / dấu chấm giữa (`arb_no_dots_test` xanh).
- [~] cargo remote xanh: 355 pass, 1 fail ngoài phạm vi (xem Hot file).
- [x] file <= 400 dòng (test tách `media_test.dart` 219, `media_ui_test.dart` 364, `support/media_fakes.dart` 114).

### Audit
Diff chỉ nằm trong file được phép + 3 chỗ phải nối: `viewer.rs`, `chat_screen.dart`, ARB / manifest / pubspec. Không đụng `mod.rs`, `ctx.rs`, `push*`, `offer_apk`, `run_log*`, `clip*`, `danger*`, `pair*`, `loopback*`, `lib/push|install|ui/run`, Kotlin, `file_paths.xml`.

### Review (edge case)
- Từ chối mic: `init` false + không có quyền -> toast "Microphone access is off" (có test). Từ chối camera: `PlatformException` code chứa `denied` -> toast camera.
- Không có gói offline: `PlatformSpeechEngine.offlineFor` = Android SDK >= 33 và `locales()` (timeout 2s, vì plugin không trả lời khi on-device không có) có ngôn ngữ đó; không thì hỏi 1 lần "dùng online?", chỉ lưu khi bấm Cho phép (bấm không thì lần sau hỏi lại, để không kẹt vĩnh viễn; nếu main muốn lưu cả lựa chọn từ chối thì sửa `MediaPrefs`).
- Mất kết nối giữa chunk: dừng gửi, báo "Lost the connection", desktop tự xoá buffer sau 60s; không tự gửi lại chunk (gửi lại chunk đã nhận sẽ thành lỗi `order`), người dùng bấm gửi lại ảnh.
- 2 ảnh cùng lúc: phone chặn bằng `busy`; nếu 2 thiết bị / 2 luồng thì mỗi `up` có buffer riêng (có test), max 4 upload mở mỗi kết nối.
- HEIC / PNG lớn: `image_picker` Android resize + re-encode ra JPEG / PNG (HEIC thành JPEG); nếu bytes vẫn là định dạng lạ -> `unsupported`; PNG > 2048 cạnh -> thu nhỏ bằng `dart:ui` ra PNG (không có bộ mã hoá JPEG trong `dart:ui`; nên desktop nhận cả `.png`, khác brief chỉ `.jpg`).
- Desktop cũ không có op: lookup fail -> reply `denied` -> thông báo "does not accept photos"; thêm nữa nút ảnh ẩn khi thiếu cap `fsput`.
- Chữ nhận dạng không log; ảnh không log; đường dẫn trả về là tuyệt đối trên desktop và được chèn vào ô nhập (có nháy kép nếu có khoảng trắng), người dùng tự bấm gửi.

### Test
- `cd apps/shell; cargo test --locked -j 4 fs_put` -> 15 passed, 0 failed.
- `cd apps/shell; cargo test --locked -j 4 remote` -> 355 passed, 1 failed, 3 ignored. Fail: `remote::viewer_events::run_log::tests::ops::the_run_ops_are_mapped_and_every_op_name_is_unique` tại `run_log_tests.rs:123` (`assert_eq!(SPECS.len(), 37)`), file cấm đụng; chỉ cần đổi 37 -> 38.
- `cd apps/mobile; flutter pub get` ok; `flutter gen-l10n` ok; `flutter analyze` (toàn app) -> No issues found.
- `flutter test --concurrency=2 test/media_test.dart test/media_ui_test.dart` -> 38 passed (gồm test thu nhỏ ảnh thật bằng `dart:ui`).
- `flutter test --concurrency=2 test/chat_ui_test.dart test/arb_no_dots_test.dart test/chat_ui_model_test.dart test/clip_test.dart test/viewers_test.dart` -> 50 passed.

### Nhìn: CHƯA NHÌN
Hai nút mới nằm trong ô nhập chat (ảnh bên trái ô chữ, mic bên phải trước nút gửi), bottom sheet chọn camera / thư viện, hộp thoại hỏi online speech và hộp thoại gitignore chưa xem trên máy / emulator. CHƯA KIỂM trên máy thật: nhận dạng giọng nói thật (on-device vs online, quyền mic, `locales()` trả lời thế nào), chụp / chọn ảnh thật, gửi ảnh qua LAN / relay thật tới desktop thật, bố cục 4 nút trên màn hẹp (360 dp).

### Hot file cần thêm
1. `apps/shell/src/remote/ctx.rs`: `pub const DESKTOP_CAPS: [&str; 3] = ["devinfo", "checklist", "clip"];` -> `pub const DESKTOP_CAPS: [&str; 4] = ["devinfo", "checklist", "clip", "fsput"];` (không có cap này nút ảnh ở phone sẽ ẩn). Soát test nào assert danh sách caps.
2. `apps/shell/src/remote/run_log_tests.rs:123`: `assert_eq!(SPECS.len(), 37);` -> `38`.
3. `docs/remote-integration.md`: op `fs.put {root, up, seq, last, b64}` (chunk raw <= 384 KB, b64 <= 512 KB, file <= 8 MB, <= 64 chunk, buffer 60s, chỉ ghi `<root>/.orrery/inbox/<UTC yyyyMMdd-HHmmss>[-n].jpg|png`, cap `fsput`, lỗi `order|expired|size|busy|image|io|denied|invalid`).
4. `.code_index/lookup.md` + `.code_index/_meta.md`: `remote/fs_put.rs::Puts/run/save`; `apps/mobile/.code_index/mobile.md`: mục `lib/media/*` + `ui/chat/chat_media.dart`.
5. `tool/gen_arb.mjs` đã lệch ARB (thiếu key `clip*` và `media*`); nếu còn dùng thì phải bổ sung, không thì bỏ.

### Ngoài phạm vi
- Sửa `chat_screen.dart` (cần để truyền `media`).
- Desktop dùng UTC cho tên file (không có thư viện múi giờ trong crate).
- Phone nhận cả PNG (không chỉ JPEG) vì `dart:ui` không mã hoá JPEG.

### Commit message đề xuất
feat(remote): voice dictation and photo upload from the phone into a pane

Hold-to-talk mic fills the message box without sending (offline recognizer first, one-time online consent). Photos are shrunk to 2048 px, split into 512 KB base64 chunks and sent over the new fs.put op, which the desktop writes only under <project>/.orrery/inbox/ with a desktop-chosen name after the viewer root guard; the path is inserted into the message and an optional .gitignore rule is written only when the user taps Add.
