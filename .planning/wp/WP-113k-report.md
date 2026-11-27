## WP-113k — PARTIAL (Dart xong và test xanh; Kotlin/manifest CHƯA KIỂM compile)

### Đã đổi
Dart (`apps/mobile`):
- Mới `lib/push/waiting_update.dart` (parse `w`/`wl`/`k:"clear"`, cắt nhãn 24 ký tự theo code point, tối đa 2 nhãn, count cap 999), `waiting_sink.dart` (`WaitingSink`, `NoWaitingSink`, `ChannelWaitingSink` kênh `dev.ccm.orrery.remote/waiting`), `push_pause.dart` (`PushPause`, `readPausedUntil`, kv `push.pausedUntil`), `pause_prompt.dart` (`PausePromptHost` + dialog "Tạm dừng thông báo 1 giờ?" / "Bật lại").
- Sửa `push_present.dart` (apply sink, bỏ qua clear, bỏ qua khi tạm dừng), `push_service.dart` (`pauseOneHour`, `resumeNow`, `pausedUntil`, `pauseRequests`, mute cả 3 group + `pauseUntil` trong `push.register` khi tạm dừng), `push_background.dart` (clear không hiện; tạm dừng hết hạn thì mới hiện), `app/bootstrap.dart` (+`ChannelWaitingSink`), `main.dart` (bọc `PausePromptHost`), ARB en + vi, `app_localizations*.dart` sinh lại.
- Test mới `test/waiting_test.dart` (20 test).

Android native (`android/app/src/main`):
- Mới `WaitingStore.kt` (SharedPreferences `orrery_waiting`: count, labels, at, pausedUntil; `refresh` cập nhật widget + `TileService.requestListeningState`; 1 alarm inexact `AlarmManager.set(RTC)` tại `at + 30 phút` hoặc `pausedUntil`, không poll), `OrreryWidgetProvider.kt` (2x2, "Mở app để cập nhật" khi chưa có dữ liệu / cũ > 30 phút, "lúc HH:mm", dòng tạm dừng, chạm mở app), `OrreryTileService.kt` (ACTIVE khi N > 0 và còn mới; phụ đề "N chờ bạn" / "Tạm dừng đến HH:mm" / "Mở app để cập nhật"; API 34+ `startActivityAndCollapse(PendingIntent)`, thấp hơn dùng Intent), `WaitingPushReceiver.kt` (nhận `com.google.android.c2dm.intent.RECEIVE`, đọc extras `w`/`wl`/`k` khi app nền hoặc đã bị kill).
- Sửa `MainActivity.kt` (kênh `waiting`: `ready`/`apply`/`setPausedUntil`; bắt `QS_TILE_PREFERENCES` rồi gọi `pause` sang Dart, giữ pending tới khi Dart `ready`), `AndroidManifest.xml` (receiver widget, receiver c2dm, service tile `BIND_QUICK_SETTINGS_TILE` + ACTIVE_TILE, intent-filter `QS_TILE_PREFERENCES` trên MainActivity).
- Res mới: `layout/widget_waiting.xml`, `xml/widget_info.xml`, `drawable/widget_background.xml`, `values-night/colors.xml` (widget_*); sửa `values/colors.xml`, `values/strings.xml`, `values-vi/strings.xml`. Màu accent = `notification_accent`, có chế độ tối.

### Tiêu chí
- [x] Test Dart: parse `w`/`wl`/`clear`, thiếu field, nhãn dài bị cắt, sink được gọi.
- [x] `flutter analyze` sạch. CHƯA KIỂM: compile Kotlin / merge manifest (không build APK theo luật).
- [x] Không comment (Dart, Kotlin, XML), chuỗi en + vi, không em dash, không dấu chấm giữa (`arb_no_dots_test` xanh).

### Audit
- Không có `shared_preferences` / `home_widget` trong pubspec và background isolate của FCM không có MethodChannel của MainActivity. Cách chọn (lý do):
  1. Cập nhật store khi push tới lúc app nền / bị kill: receiver native `WaitingPushReceiver` (không cần plugin, không đụng pubspec). Subclass `FlutterFirebaseMessagingService` bị loại vì `RemoteMessage` có thể không nằm trên compile classpath của app, cần sửa `build.gradle.kts`.
  2. Foreground: Dart `PushPresenter` gọi `WaitingSink` qua MethodChannel (ghi đè cùng giá trị, idempotent; `WaitingStore.apply` bỏ qua bản có `at` cũ hơn).
  3. Trạng thái tạm dừng: nguồn thật là kv Dart `push.pausedUntil` (isolate nền đọc được qua `FlutterSecureKv`), phản chiếu sang native để tile / widget hiện chữ.
- Tạm dừng thực hiện bằng cách dùng cơ chế mute sẵn có: mute cả 3 group trong `push.register` + trường mới `pauseUntil` (epoch ms), cộng chặn hiện local push data-only khi bg. Tự bỏ: Timer trong app, kiểm lúc app mở / resume / khi push tới.

### Review (edge case)
- Thiếu field / app desktop cũ: không có `w` thì không đụng store; sau 30 phút widget / tile tự chuyển "Mở app để cập nhật" nhờ alarm inexact 1 lần (và mỗi lần render đều tính lại theo giờ hiện tại).
- Số không bao giờ bịa: `count < 0`, không phải số, hoặc `at` ở tương lai (đổi giờ máy) đều coi là chưa có dữ liệu.
- `k:"clear"`: ghi count 0, không hiện notification (cả foreground và bg).
- Isolate nền: không phụ thuộc channel, xem Audit.
- API 34+: `startActivityAndCollapse(PendingIntent IMMUTABLE)`; API < 29 không có subtitle nên nhãn ghép "Orrery, <phụ đề>".
- Giới hạn đã biết: push mang khối `notification` (chế độ không nội dung) do hệ thống tự hiện khi app nền, app không chặn được local; chỉ chặn được nhờ server mute (cần desktop hiểu `mute` đủ group / `pauseUntil`, desktop cũ vẫn gửi). Desktop cũ không hiểu `pauseUntil` thì sau 1 giờ vẫn bị mute tới khi app mở / resume lần kế tiếp (app gửi lại `push.register` đã bỏ mute). Cần WP F5 desktop xử lý `pauseUntil` để tự bật lại đúng giờ.
- App đang khoá (LockGate): dialog tạm dừng hiện sau khi mở khoá.
- Hạn chế: `push_service.dart` 419 dòng (vượt ~400 nhẹ).

### Test
- `flutter analyze` (cwd `apps/mobile`) → No issues found.
- `flutter test --concurrency=2 test/waiting_test.dart test/push_test.dart test/push_present_test.dart test/push_ui_test.dart test/arb_no_dots_test.dart` → All tests passed (56); chạy lại `waiting_test.dart` sau `dart format` → 20 passed.
- CHƯA KIỂM: compile Kotlin, merge manifest, receiver c2dm thực sự nhận extras trên máy, alarm, tile.

### Nhìn
CHƯA NHÌN: widget 2x2 (sáng / tối, 3 trạng thái: có số, 0, cần mở app, + dòng tạm dừng), ô QS (ACTIVE / INACTIVE, phụ đề, giữ ô hiện dialog tạm dừng). Main kiểm trên A33 khi build.

### Hot file cần thêm
- Desktop (WP F5): ghi `w`, `wl` (JSON array chuỗi, hoặc xuống dòng) vào mọi push data, `k:"clear"` khi về 0; hiểu `pauseUntil` trong `push.register` (mute mọi group tới mốc đó).
- Nếu receiver c2dm không nhận được trên máy thật: cần sửa `android/app/build.gradle.kts` thêm `firebase-messaging` để subclass `FlutterFirebaseMessagingService`.
- `apps/mobile/.code_index/mobile.md`: thêm mục WP-113k (chưa ghi vì file ngoài phạm vi được liệt kê).
- Đã sửa thêm `main.dart` và `app/bootstrap.dart` (mỗi file vài dòng) để nối dialog và sink.

### Ngoài phạm vi
Không đụng Rust, web, `pubspec.yaml`, `pair_*`, `deep_link_host.dart`. Không commit.

### Commit message đề xuất
feat(mobile): home screen widget and quick settings tile for waiting sessions, with 1 hour pause
