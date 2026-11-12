# WP-29 báo cáo — Đợt 1: theme + kit

## Đã đổi file
- apps/mobile/lib/ui/theme/orrery_colors.dart (mới), orrery_shape.dart (mới), spring_transition.dart (mới), orrery_theme.dart (viết lại `buildOrreryTheme`, `StatusColors` đổi giá trị)
- apps/mobile/lib/ui/kit/: kit.dart, status_chip.dart, pulse_dot.dart, orrery_card.dart, buttons.dart, overlays.dart, connection_banner.dart, states.dart, orbit_mark.dart, animated_items.dart, shake.dart (tất cả mới, mỗi file < 230 dòng)
- apps/mobile/test/kit_test.dart (15 test), kit_shots_test.dart (mới), workspace_model_test.dart:105-116 (theme không còn nhận accent từ palette desktop)
- apps/mobile/.code_index/mobile.md (mục WP-29 đợt 1), .planning/wp/v97/WP-29.md (nhật ký)
- docs/qa/v97/WP-29/kit-light.png, kit-dark.png

## Tiêu chí
- Home / Ghép / Khoá khớp mockup: CHƯA (đợt 2, theo lệnh main).
- Motion: chip 220 / 150 ms, list 220 vào / 150 ra, sheet 300 spring / 210 đóng, page 300 / 210, shake 240, pulse 1200, lock fade 250; tắt hết khi `disableAnimations` (có test): 1/2 (phần kit xong, màn chưa).
- analyze sạch, `flutter test --concurrency=4` 254 pass: đạt. vi + en: kit nhận string từ caller, chưa thêm ARB. Không comment, file ≤ 400 dòng, index cập nhật: đạt.

## Audit / Review
- Theme cố định orrery-light / galaxy theo bảng token; palette gửi từ desktop chỉ còn đọc `mode` (đổi hành vi so với trước, test cập nhật).
- StatusColors: working đổi từ xanh dương sang cam, asked sang vàng theo mockup; ảnh hưởng các màn cũ dùng `statusColors` (monitor_page, permission_bar, home).
- Lỗi đã gặp và sửa: `AnimatedItems` gọi setState trong didUpdateWidget khi giảm chuyển động (dùng postFrame); `AnimatedSize` Duration.zero assert layout (bỏ AnimatedSize khi giảm chuyển động).
- `AnimatedItems` dùng SizeTransition (đổi layout cho vài dòng) để các dòng dưới dịch lên; opacity / transform cho phần còn lại.
- Độ trễ trong `LinkPill` / banner mockup (12 ms, 84 ms): LinkState chưa có số đo, kit chỉ hiện khi caller truyền `latencyMs`, không bịa. Cần nguồn đo ping nếu muốn hiện.

## Test thật
`flutter analyze` sạch; `flutter test --concurrency=4` all passed (254, 29 skip do tag / shots). kit_test: token theme sáng/tối, radius 8, lerp, chip đổi màu/icon/chữ, pulse, list thêm/xoá/giảm chuyển động, đổi tên, banner, LinkPill, confirm dialog, sheet, shake, button busy, empty.

## Nhìn
Mở kit-light.png / kit-dark.png so mockup (home-a, pair-qr, error): chip viền 38% không nền, thẻ line mỏng, nút cam, banner / notice tint 10%, orbit mark đúng màu agent. Khớp.

## Ngoài phạm vi
Không đụng ui/terminal, pty_stream, remote/*. Home / Ghép / Khoá, ARB, machines.dart: chưa làm (đợt 2).

## Commit message đề xuất
feat(mobile): Orrery light and Galaxy theme tokens, spring page transition, and a shared UI kit (chip, card, button, sheet, dialog, banner, empty state, animated list, shake) with reduce-motion support

## Đợt 2 — Home A + Ghép máy
- Đổi: apps/mobile/lib/app/home_page.dart (viết lại: tiêu đề, nhóm "Đang kết nối" / "Bản trùng tên", thẻ máy dùng OrreryCard + LinkPill + chip đang chạy / cần bạn / rảnh, AnimatedItems thêm / xoá, AnimatedRename, empty state orbit, "Dọn N bản cũ" có confirm), app/home_groups.dart (mới, `groupMachines`: mỗi tên giữ 1 bản: đang ready > chưa thu hồi > pairedAt mới nhất; còn lại vào "Bản trùng tên"), app/pair_page.dart (tab Quét QR | Nhập mã, khung QR có 4 góc cam, thẻ info mã hoá đầu cuối, lỗi ghép thành NoticeCard; tab Nhập mã + ô `pair-code` chỉ ở debug), ui/kit/segmented.dart (mới), tool/arb_home.mjs (mới, gộp vào gen_arb.mjs) + app_{en,vi}.arb + app_localizations*.dart sinh lại.
- Không hiện ms (chưa có nguồn đo). Dọn bản cũ gọi `MachinesNotifier.forget` từng bản, không sửa machines.dart.
- Test: test/home_wp29_test.dart (grouping + dọn có confirm / huỷ), pair_page_test.dart (đổi sang bấm tab Nhập mã, kiểm thẻ lỗi), support/ui_fakes.dart (Harness `extraLinks`, override link / view cho máy phụ), wp29_shots_test.dart. analyze sạch, 256 test pass.
- Nhìn: docs/qa/v97/WP-29/home-{light,dark}.png, home-empty-*, pair-qr-*, pair-code-*, pair-error-*: khớp home-a / pair-qr / pair-code / error của mockup (khác: không có dòng "Windows 11 · Orrery 2.3.0" vì app chưa có dữ liệu OS / version; ô mã 8 ký tự rời không làm vì Pairing chỉ nhận chuỗi QR).

## Đợt 2 — Màn khoá + cài đặt khoá
- Đổi: apps/mobile/lib/security/lock_gate.dart (LockScreen: OrbitMark, FadeIn 250 ms, ShakeBox ±6 px 240 ms + haptic chỉ khi `AuthOutcome.failed`, nút vân tay tròn, OrreryButton cho các nút phụ; logic giữ nguyên, key giữ nguyên), lock_settings_page.dart (OrreryCard + SectionLabel, OrreryDialog), test/lock_wp29_test.dart (fade + shake chỉ khi failed), wp29_shots_test.dart.
- Nhìn: docs/qa/v97/WP-29/lock-{light,dark}.png, lock-settings-{light,dark}.png khớp mockup lock (khác: không có dòng "Tự khoá sau 1 giờ" dưới đáy, nhãn vân tay dùng "Mở khoá").
- analyze sạch, `flutter test --concurrency=4` pass toàn bộ.
- Tiêu chí còn lại: motion theo spec (có test), vi + en (ARB sinh lại), không comment, file < 400 dòng, index cập nhật.
