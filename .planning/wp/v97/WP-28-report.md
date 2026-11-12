# WP-28 báo cáo — Mockup UI + motion Orrery Remote

Trạng thái: xong, chờ chủ nhân chọn phương án. Không đụng `apps/**`, không git.

## File
- Trang tổng (offline, 1 file): `docs/design/v97-mobile-ui/index.html` (sinh từ `src/*` bằng `node build.mjs`; chụp lại: `bash shoot.sh`).
- PNG: `docs/design/v97-mobile-ui/png/` — `<id>-light.png` + `<id>-dark.png` (360×800), `overview.png` (cả trang), `section-motion|components|tokens|questions.png`.
- Xem 1 màn: `index.html?s=<id>&t=light|dark`.

## Danh sách màn (id → mỗi màn có -light / -dark)
- home-a, home-b — Home (A: danh sách máy, gộp bản trùng tên + dọn; B: vào thẳng máy, sheet đổi máy)
- pair-qr, pair-code — ghép máy (QR + nhập mã, máy mDNS)
- mon-a, mon-b — Monitor (A: thẻ; B: nhóm theo trạng thái, dòng gọn)
- detail — chi tiết session
- ws-a, ws-b — Workspace (A: 1 pane + chip; B: lưới Mission Control)
- term, term-pick — terminal (thanh phím phụ) + sheet chọn pane
- file, diff — xem file / diff
- settings — khoá app, thời hạn khoá, đóng kết nối khi khoá, thông báo, giao diện
- lock — màn khoá
- push — thông báo Android
- empty, offline, error — trống / mất kết nối / mã ghép hết hạn + relay timeout

## Phương án A / B
- Home: A rõ LAN / relay + dọn bản trùng ngay màn, nhưng thêm 1 chạm; B nhanh với 1–2 máy, nhưng bản trùng khó thấy.
- Monitor: A nhiều thông tin (tool đang chạy), ~4 session / màn; B thấy 6+ session, nhóm Chờ bạn lên đầu, ít chỗ hiện tool.
- Workspace: A thấy nhiều output 1 pane, pane khác chỉ tên; B thấy mọi pane cùng lúc (khớp Mission Control), output chỉ 3 dòng.

## Motion
7 demo chạy được (list thêm / xoá / đổi tên, chip, push / pop, sheet spring, chấm nối, màn khoá, pull to refresh) + bảng spec (phần tử, kích hoạt, thuộc tính transform / opacity / màu, 150–300 ms, curve → `Curves.*` / `SpringDescription`, khi giảm chuyển động) + checkbox mô phỏng giảm chuyển động. Mục "Motion" trong `index.html`.

## Bảng token mapping
Mục "Token desktop → Flutter" trong `index.html` (`png/section-tokens.png`): token `--ccm-*` sáng / galaxy → `ColorScheme` + `ThemeExtension<OrreryColors>` / `OrreryShape`, kèm bảng cỡ chữ → `TextTheme`.

## Điểm chưa chắc, cần chủ nhân chọn
1. Home A hay B. 2. Monitor thẻ hay dòng gọn. 3. Workspace 1 pane hay lưới.
4. Chip trạng thái: chỉ viền như token desktop (`--ccm-chip-fill: 0`, đang dùng) hay thêm nền tint 10–12%.
5. Radius nút / ô nhập: desktop 6 px, mockup nới 8 px (touch); card 10 px giữ nguyên.
6. Terminal / code ở chủ đề sáng: nền tối `well` (đang dùng) hay nền sáng.
7. Push: nút "Trả lời" chỉ mở app tại pane, không gõ từ notification.

## Đã tự kiểm
Mở từng PNG màn (38 ảnh chụp lại sau lần sửa cuối, đã xem ~20 đại diện cả sáng / tối + các section): không tràn, chữ có dấu đúng, không gradient / blur, status có icon + chữ. Dòng code / diff và tiêu đề dài bị cắt ở mép phải có chủ đích (ellipsis / cuộn ngang trong khối). Trang tổng không cuộn ngang ở viewport ≥ 477 px.
Hạn chế: Edge headless (máy không có Chrome / Playwright); demo motion chưa kiểm bằng tay trong trình duyệt có chuột, chỉ kiểm không lỗi JS khi tải.
