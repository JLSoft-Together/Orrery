## WP-0d: trả lời menu từ Mission Control + hàng chờ "Chờ trả lời"

- Mục tiêu (`.planning/PLAN-v100-click-menu.md` phase 0d):
  1. Mission Control (`apps/web/src/components/workspace/mission/`): card của pane đang hiện menu (parser `termMenu.ts` confident) hiện câu hỏi (dòng tiêu đề phía trên menu nếu parser lấy được, không thì tên loại menu) + các nút đáp án (dùng lại `TermMenuBar` hoặc tách component con dùng chung), click là chọn y như thanh nút dưới pane (cùng đường gửi phím + đọc lại màn trước khi gửi, `useTermMenu` / `termMenuKeys`). Phím 1-9 của Mission Control đang dùng để chọn card: giữ nguyên, không đổi.
  2. Hàng chờ "Chờ trả lời": danh sách mọi pane (mọi project, mọi tab) đang hiện menu, sắp theo thời gian bắt đầu hỏi; chỗ hiện: chip đếm trên header Workspace / rail (theo chỗ hợp với code hiện có, ví dụ cạnh `AttentionPopover.vue`), mở ra popover list (tên project, tab, câu hỏi, thời gian chờ), click nhảy tới pane. Phím tắt nhảy tới câu kế tiếp (chọn tổ hợp chưa dùng, kiểm `lib/keymap*` / chỗ đăng ký phím, thêm vào danh sách phím tắt nếu repo có).
- Nguồn trạng thái: cần biết pane nào đang hiện menu khi pane không hiển thị (tab khác). Xem `useTermMenu.ts` hiện chỉ chạy khi pane hiện: mở rộng để đọc buffer xterm của pane ẩn với throttle thấp (vd 1 lần/giây, chỉ pane agent đang ở trạng thái `asking` / idle theo store hiện có) hoặc dùng status `asking` có sẵn trong store agents để lọc trước rồi mới parse. Không parse mọi pane mỗi frame.
- Đọc `.planning/wp/WP-0b-report.md`, `.planning/wp/WP-0c-report.md`, `.planning/COORD-2026-11-15-v100-v102.md` (chỉ token + utility có sẵn), CLAUDE.md hard rules.
- Phạm vi file: `apps/web/src/components/workspace/mission/**`, `apps/web/src/components/workspace/TermMenuBar.vue` (tách phần dùng chung nếu cần), mới `apps/web/src/stores/menuQueue.ts` (+test) hoặc composable tương đương, `apps/web/src/composables/useTermMenu.ts` (+test), component chip/popover hàng chờ (mới) + chỗ gắn (header Workspace / rail), `apps/web/src/i18n/click-menu-v100.ts`, E2E `apps/desktop/e2e/v100-menu-queue.cjs`.
- Cấm đụng: `apps/web/src/components/disk/**`, `stores/diskMap.ts`, `lib/diskMap.ts`, `i18n/disk-v101.ts` (WP-1c / WP-1d), `apps/desktop/src/**`, `apps/core/**`, `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `App.vue`, `AppHeader.vue`.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] Test store hàng chờ: thêm khi menu xuất hiện, bỏ khi menu biến mất / pane đóng, thứ tự, nhảy kế tiếp vòng lại.
- [ ] Mission Control card có nút đáp án, click gửi phím đúng (test component với buffer giả).
- [ ] `npm test -w @ccm/web` pass; `npm run typecheck -w @ccm/web` sạch.
- [ ] E2E trên **bản dev tách biệt** (`apps/desktop/e2e/cdp.cjs`, cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): 2 pane ở 2 tab in màn menu fixture, chip đếm = 2, popover list đủ, phím tắt nhảy pane, trả lời từ Mission Control gửi đúng phím, chip giảm. Chụp `docs/qa/v100/WP-0d/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-0d-report.md`.

## Nhật ký
- 17:52 code: useTermMenu seen/observe, stores/menuQueue, MenuQueueChip, MissionCard strip, chord Ctrl+Alt+Q; tiếp: test + typecheck
- 17:53 test mới pass; chạy full + typecheck
- 17:56 e2e v100-menu-queue.cjs viết xong; build web+desktop rồi chạy
