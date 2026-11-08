# HANDOFF 2026-11-09 — tối ưu RAM + CPU (PLAN-v86)

## Đang ở đâu
- 6 WP perf xong, nghiệm thu, **chưa commit** (rule chủ nhân). Message + file từng commit: `.planning/COMMITS.md` (mục 2026-11-09 perf).
- Test: collector 417, web 1823, desktop 1414 pass; `npm run typecheck` toàn repo exit 0.
- Chi tiết: `.planning/PROGRESS-2026-11-09-perf.md` (Bảng WP), báo cáo `.planning/wp/WP-perf-*-report.md`, plan `.planning/PLAN-v86-performance.md`.
- Cũng chưa commit: đổi link update feed/README sang repo `JLSoft-Together/Orrery` (repo đã rename; remote đã set-url).

## Chủ nhân cần kiểm trên app (CHƯA NHÌN)
1. Office mặc định tắt; Settings → General → "Office view" bật/tắt chạy ngay; toolbar map chỉ có nút Office khi bật.
2. Mở Monitor lần đầu (giờ là async chunk) không nháy trắng khó chịu.
3. Workspace nhiều tab, pane ẩn đang stream → chuyển tab: output đủ, không lệch kích thước, Claude TUI không treo.
4. Boot: window hiện sớm hơn, dashboard vào bình thường; restore backup vẫn đúng giao diện.

## Đã đổi hành vi
- Chip dev server chỉ còn ở pane shell (perf-2).
- Office tắt mặc định (DECISIONS 2026-11-09).

## Việc tiếp (chưa làm, theo PLAN-v86)
- Gõ phím: PTY host flush leading edge (`apps/desktop/src/pty/host.ts:44,191`) — chủ nhân chọn tập trung RAM/CPU nên chưa làm.
- Boot: intro không chờ cố định 2.45 s (`main.ts:~738`, `pages.ts:6`).
- Đo lại RAM/CPU trên app build mới (Windows) + macOS; ghi `docs/perf/`.
- Bỏ đổi line ending 2 file `.snap` trước khi commit.
