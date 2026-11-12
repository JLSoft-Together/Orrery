# PLAN v97 — Làm lại UI + motion app điện thoại Orrery Remote

Chủ nhân chốt 2026-11-12 (AskUserQuestion):
- **Phạm vi:** tất cả màn: danh sách máy (home), ghép máy, Monitor (list session + chi tiết), Workspace (tab + pane), terminal, xem file / diff, cài đặt (gồm khoá app), màn khoá, thông báo push. Dùng chung 1 bộ component: thẻ, chip trạng thái, nút, sheet, dialog, empty state, banner kết nối.
- **Phong cách:** khớp Orrery desktop.
  - Tối = Ngân hà (`galaxy`, `apps/web/src/themes.css` `[data-theme='galaxy']`); sáng = `orrery-light`.
  - Accent cam dịu; màu theo agent như pane desktop: Claude cam, Codex tím, shell teal.
  - Apple HIG, dày thông tin nhưng không rối mắt, không gradient, không blur.
  - Theo chế độ sáng / tối của máy.
  - Status = icon + chữ + màu, không chỉ màu (`.planning/COLOR-SYSTEM.md`).
- **Motion:** mọi thay đổi đều có hiệu ứng, như rule desktop:
  - Danh sách: dòng thêm / xoá / đổi tên đều animate; chip trạng thái chuyển màu + icon mượt.
  - Chuyển màn và mở sheet kiểu spring.
  - Kết nối: chấm nhịp nhẹ khi đang nối.
  - Màn khoá: hiện dần + rung nhẹ.
  - Thời lượng 150–300 ms; tôn trọng "giảm chuyển động" của máy.
  - Terminal không có hiệu ứng làm chậm output.
- **Quy trình:** mockup ảnh trước (chưa code Flutter), chủ nhân chọn / duyệt, rồi mới code.

## Bước
1. WP-28: mockup HTML tĩnh 1 file, mỗi màn 1 khung điện thoại 360×800 dp, sáng + tối.
   - Xuất PNG từng màn + 1 trang tổng.
   - Phần motion là demo chạy được trong HTML + bảng spec (thuộc tính, thời lượng, curve).
   - Màn nào có ≥ 2 hướng thì làm 2 phương án A / B cho chủ nhân chọn.
2. Chủ nhân duyệt → ghi lựa chọn vào đây.
3. Code Flutter theo cụm, mỗi cụm có video A21s:
   - cụm 1: theme + component chung;
   - cụm 2: home / ghép / khoá;
   - cụm 3: Monitor / Workspace / terminal;
   - cụm 4: viewer / cài đặt / thông báo.
4. Dọn danh sách máy trùng tên (bản đã thu hồi / offline / mới).

## Ràng buộc
- Không comment code; file ≤ ~400 dòng; vi + en; giữ hiệu năng terminal (gom ghi theo frame, WP-25).
- A21s là máy yếu: animation chỉ dùng transform / opacity, không vẽ lại cả list.

## Chủ nhân duyệt mockup (2026-11-12, `docs/design/v97-mobile-ui/`)
- Home: A (danh sách máy → vào máy, gộp bản trùng + "Dọn bản cũ").
- Monitor: mặc định A (thẻ), có nút đổi view mode sang B (nhóm theo trạng thái, dòng gọn); nhớ lựa chọn.
- Workspace: mặc định A (1 pane + chip), có nút đổi view mode sang B (lưới Mission Control); nhớ lựa chọn.
- Còn lại "ok" = giữ như mockup: chip trạng thái theo mockup, radius 8 px, terminal / code luôn nền tối, push "Trả lời" chỉ mở pane; motion theo bảng spec trong mockup.
