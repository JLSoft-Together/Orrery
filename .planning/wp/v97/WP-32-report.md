# WP-32 báo cáo: cụm 4 viewer, Cài đặt, state

## Đợt — Viewer
- Code: giếng tối cả sáng lẫn tối (như mockup), số dòng mờ, màu cú pháp dark, tìm trong file giữ nguyên. Diff: giếng tối, hunk xanh dương nhạt, dòng thêm / xoá tô xanh / đỏ, chip "N thêm / N xoá" đếm từ diff thật. File: dòng đường dẫn mono, footer "dart · 20 dòng · chỉ đọc" + chip Đã lưu / Chưa lưu, banner conflict dùng NoticeCard + nút kit. Logic sửa file / git không đổi.
- Ảnh: docs/qa/v97/WP-32/file-*, diff-* (light, dark).

## Đợt — Cài đặt
- Tab thứ 4 "Cài đặt" ở thanh dưới (`settings_body.dart`): Bảo mật (switch khoá app + chọn thời gian bằng segmented kit, nhãn ngắn), Thông báo (PushTile kit, ẩn khi không có push service), Giao diện (ngôn ngữ / chủ đề: hiện "Theo máy tính" chỉ đọc), Về app (tên + máy đang kết nối). `LockControls` tách ra dùng chung với `LockSettingsPage` (nút khoá ở Home vẫn còn). `AppLockScope` đưa `AppLock` xuống mà không sửa main.dart.
- Ảnh: settings-light / dark.

## Đợt — State empty / error
- `ErrorState` / `EmptyState` / lỗi inline của `AsyncPanel` dùng kit (NoticeCard, OrreryButton, vòng tròn raised) nên mọi màn viewer (Explorer, Changes, Log, Run, File, Diff) đổi theo. Ảnh: error-*, empty-*. Offline giữ ConnectionBanner của WP-31 (Workspace / Monitor).

## Đợt — Tách home_page
- home_page 420 → 133 dòng; `machine_card.dart` 203, `home_duplicates.dart` 110. Không đổi hành vi (test home cũ pass).

## Đã đổi
Mới: lib/app/{app_lock_scope,machine_card,home_duplicates}.dart, lib/ui/workspace/settings_body.dart, tool/arb_cluster4.mjs, test/wp32_test.dart, wp32_shots_test.dart, docs/qa/v97/WP-32/*.
Sửa: viewers/{code_view,diff_view,diff_page,file_page,async_panel}, security/lock_settings_page, push/push_providers, workspace/workspace_page, app/{home_page,deep_link_host}, tool/gen_arb.mjs + ARB sinh lại, test lock_ui / lock_shots / viewers / ui_viewer_integration (key `lock-timeout-<index>`, "Saved" tìm trong SnackBar), .code_index/mobile.md.

## Tiêu chí n/m
- File / diff / Cài đặt / state khớp mockup, ảnh sáng + tối: đạt (có chỗ khác bên dưới); thông báo Android: chưa (xem dưới). 3/4 mục ảnh.
- Thanh dưới có Cài đặt, home_page ≤ ~400: đạt.
- analyze sạch, 267 test pass (concurrency 4): đạt.
- vi + en, không comment, index cập nhật: đạt; file_page 422 dòng (hơi quá ~400).

## Test thật
flutter analyze: No issues found. flutter test --concurrency=4: All tests passed (267, 63 skip do tag / shots). Mới: settings tab có / không lock, chip đếm diff, retry của ErrorState. Lỗi tự bắt khi nhìn ảnh: nhãn segmented bị xuống dòng (thêm nhãn ngắn `lockShort*`).

## Nhìn
Đã mở file-dark, file-light, diff-light, settings-dark, settings-light, error-dark, empty-light. Bố cục, màu, chip khớp mockup sau khi sửa nhãn segmented.

## Khác mockup
- Notification Android CHƯA làm: màu accent cần thêm `com.google.firebase.messaging.default_notification_color` vào AndroidManifest.xml (ngoài phạm vi, main quyết định); nút "Trả lời / Mở pane" không làm được vì desktop gửi payload có khối `notification` do hệ thống hiện, app không vẽ action.
- Cài đặt: mockup có 3 switch theo loại thông báo + "Đóng kết nối khi khoá" + ngôn ngữ / chủ đề đổi được; hiện chỉ có 1 switch push (desktop quyết định nội dung), kết nối luôn đóng khi khoá, ngôn ngữ / chủ đề theo máy tính (chỉ đọc). Không có số phiên bản app (chưa có package_info_plus).
- File: không có tab Tệp | Diff và nút copy ở AppBar (chưa chắc đường dẫn tương đối cho `git.diff` từ file). Dòng diff dài xuống dòng, không cắt ngang như mockup.
- Mục 6 (Ctrl / Alt dính, Hoạt động gần đây) chưa làm, tránh đụng đường nhập terminal / không có dữ liệu.
- Lưới B không đổi (không thuộc gói).

## Commit message đề xuất
feat(mobile): dark code and diff viewers with line counts, Settings tab (app lock, notifications, appearance, about), kit-styled error and empty states, split home page
