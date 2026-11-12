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
