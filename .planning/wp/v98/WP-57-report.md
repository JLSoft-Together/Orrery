# WP-57 báo cáo

## Đã xoá / sửa
- Xoá: `apps/mobile/ios/**` (41 file), `apps/mobile/rust_builder/ios/**`.
- Dart: `lib/app/bootstrap.dart` (`platform: 'android'`), `lib/remote/device_name.dart` (bỏ `_iosModels`, nhánh `iosInfo`, `dart:io`), `lib/remote/secure_kv.dart` (bỏ `IOSOptions`), `lib/security/lock_gate.dart` (cover khi `inactive` mọi nền tảng, vì chỉ còn Android), `lib/ui/theme/orrery_theme.dart` (bỏ `TargetPlatform.iOS`).
- Test: bỏ `ios machine id maps to a friendly name` (`test/device_name_test.dart`), `leaving the screen shows a cover on iOS` (`test/lock_ui_test.dart`, + import thừa).
- Config: `.metadata` (platform ios, unmanaged file pbxproj), `analysis_options.yaml` (`ios/**`), `.gitignore` (ios Flutter, GoogleService-Info.plist), `rust_builder/pubspec.yaml` (plugin ios).
- Docs: `docs/orrery-remote.md` (§1 bảng, §4 macOS, §7 App Links, §11 iPhone = web app + mục "Web app iPhone và relay: đánh đổi"), `docs/remote-integration.md` (§7.1, §7.2 push.register, deploy relay APPLE_APP_ID). Các bước cài web app / CA LAN ghi "sẽ bổ sung ở v98".
- Index: `apps/mobile/.code_index/mobile.md`, `.code_index/_meta.md`.

## Tham chiếu iOS còn giữ
- `rust/tests/common/commands.rs:118`: test hợp đồng `push.register` chấp nhận `"ios"`; hợp đồng giữ nguyên.
- `rust_builder/cargokit/**` (iOS target, podspec): mã vendor, không sửa.
- `rust_builder/macos|linux|windows`: không phải iOS.
- `pubspec.yaml` dòng 14-16 và `cupertino_icons`, `lib/l10n/app_localizations.dart`: comment mẫu Flutter / sinh tự động / icon font.
- `docs/remote-integration.md`: ghi `platform: "ios"` còn trong hợp đồng + giải thích.

## Test thật
`node apps/mobile/scripts/check.mjs`: gen-l10n OK, format OK, analyze sạch, flutter test +266 ~63 (skip) -0. Trước đó 268; giảm 2 là 2 test iOS đã xoá. Lưu ý: 3 file integration cần `rust/target/debug/rust_lib_orrery_remote.dll`, phải `cargo build` trong `apps/mobile/rust` trước (đã làm; thư mục target bị gitignore).

## Việc dọn sau
- Relay (WP-53, không đụng): route `apple-app-site-association`, `APPLE_APP_ID` trong `apps/relay/wrangler.jsonc` và `scripts/` deploy.
- Shell/desktop: `platform: "ios"` trong `push.register` còn được chấp nhận; xem lại khi WP-56 xong Web Push.
- `apps/mobile/pubspec.lock`/pubspec: `cupertino_icons` có thể bỏ nếu không dùng (không kiểm).
