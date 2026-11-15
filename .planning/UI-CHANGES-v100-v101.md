# UI thay đổi v100 + v101 (nhánh `feat_clean_memory`, gửi orrery-e6 cho v102)

Nhánh `feat_clean_memory`, base `6d7054be` → HEAD `60438421`, chưa merge `master`.
Xem file: `git diff 6d7054be..feat_clean_memory -- <file>` hoặc `git show feat_clean_memory:<file>`.
Ảnh QA: `docs/qa/v100/WP-0b..0e/`, `docs/qa/v101/WP-*/`, `docs/qa/v101/QA/` (mode bar 760px + 1400px). Plan: `.planning/PLAN-v100-click-menu.md`, `.planning/PLAN-v101-disk-manager.md`.
i18n: `apps/web/src/i18n/click-menu-v100.ts`, `disk-v101.ts`, `disk-clean-v101.ts`, `disk-organize-v101.ts`, `disk-apps-v101.ts` (import trong `en.ts` / `vi.ts`).

## 1. Màn MỚI

### v101 view Ổ đĩa (view lớn, mở từ Settings > Data > Ổ đĩa)
- `components/disk/DiskView.vue`: khung chính, thanh mode 9 tab (Duyệt, Tìm kiếm, File lớn, File cũ, Dọn dẹp, File trùng, Sắp xếp, App, RAM) dùng `SegmentedControl`, wrap khi hẹp (data-value cho E2E).
- `DiskScanBar.vue` (chọn ổ / thư mục, Quét, Quét siêu nhanh MFT, tiến độ), `DiskVolumeRow.vue` (dòng ổ + thanh dung lượng).
- Duyệt: `DiskTree.vue` (cây size, menu chuột phải: mở, xoá, chuyển sang ổ khác), `DiskTreemap.vue`, `DiskSunburst.vue` (canvas, `useCanvas.ts`), `DiskTip.vue` (tooltip).
- Tìm kiếm / File lớn / File cũ: `DiskSearch.vue` (ô tìm + filter size, đuôi, ngày dạng text theo locale), `DiskResults.vue` (list ảo), `DiskActions.vue` (thanh hành động khi chọn), `DiskHistory.vue` (lịch sử xoá / move).
- Dọn dẹp: `clean/DiskClean.vue`, `CleanGroup.vue` (nhóm: dev, hệ thống, SDK, Docker), `CleanRow.vue` (mục khoá vì project đang mở), `CleanOneTap.vue` (Dọn 1 chạm).
- Chuyển ổ: `move/DiskMove.vue` (dialog chọn ổ đích, verify, rollback, move back).
- File trùng: `dupes/DiskDupes.vue`, `DupeList.vue`.
- Sắp xếp: `organize/OrganizeTab.vue`, `OrganizeRules.vue`, `OrganizeJunk.vue`, `OrganizeHistory.vue` (Undo).
- App: `apps/DiskApps.vue`, `AppRow.vue`, `AppLeftovers.vue`.
- RAM: `ram/DiskRam.vue`.
- Dialog xoá / confirm là dialog native của OS (không phải UI web).

### Settings
- `settings/SettingsDiskMap.vue`: trang Settings > Data > Ổ đĩa (nút mở view, quét nền, snapshot, Telegram report, cảnh báo ổ đầy gộp từ v77). Entry trong `settings/index.ts`, link từ `SettingsData.vue`.

### v100 click menu (Workspace)
- `workspace/TermMenuBar.vue`: thanh nút dưới pane khi agent đang hỏi menu chọn (mỗi lựa chọn 1 nút, Submit cho multiSelect).
- `workspace/MenuQueueChip.vue`: chip "Chờ trả lời" (số pane đang hỏi, popover list, Ctrl+Alt+Q).
- Trong xterm: hover / click / double click dòng menu (`composables/useTermMenuMouse.ts`, highlight overlay trong `TermPane.vue`).
- Toast OS có nút trả lời: `apps/desktop/src/menuToast.ts` (ngoài web, toast Windows / macOS).

## 2. Màn SỬA

- `App.vue`: view `disk` (async `DiskView`); yêu cầu Build clean cũ của v77 (`ui.buildClean`) giờ mở tab Dọn dẹp của Ổ đĩa.
- `AppHeader.vue`: thêm mục nav header "Ổ đĩa" (icon `HardDrive`, `ui.setView('disk')`).
- `HeaderStatusPopover.vue`, `DiskBadge.vue`: bấm dòng ổ / badge mở view Ổ đĩa; nút Dọn dẹp, dòng "Cleanable" (chỉ cộng mục không khoá).
- `ProjectStrip.vue`, `workspace/rail/RailClusterView.vue`: tooltip project thêm size project + rác dọn được.
- `CommandPalette.vue`: lệnh nav "Mở Ổ đĩa".
- `workspace/TermPane.vue`: gắn TermMenuBar + chuột trên menu.
- `workspace/WorkspaceView.vue`: gắn MenuQueueChip.
- `workspace/mission/MissionCard.vue`: card đang hỏi hiện các lựa chọn, bấm để trả lời.
- `workspace/ExplorerTree.vue`, `ExplorerPanel.vue`: menu chuột phải thư mục "Chuyển sang ổ khác".
- `settings/SettingsEditor.vue` (công tắc `clickMenu` / `menuToast`), `SettingsNotify.vue` (bỏ embed `DiskSettings` cũ, cảnh báo ổ đầy dời sang Settings > Data > Ổ đĩa), `ui/SegmentedControl.vue` (thêm data-value).
- Dialog phím tắt (`lib/shortcuts.ts`): mục Ctrl+Alt+Q.

## 3. Lưu ý cho v102

- Chỉ dùng token + utility có sẵn (`ccm-panel`, `ccm-seg`, `ccm-chip`, ...), không màu cứng; treemap / sunburst lấy màu từ token qua `getComputedStyle`.
- Thanh mode 9 tab hẹp ở 760px: hiện đang wrap 2 dòng, v102 có thể đổi sang menu "Thêm".
- Canvas treemap / sunburst vẽ lại khi đổi theme (`useCanvas.ts` MutationObserver trên `class` / `data-theme` / `style` / `data-stars` của root).
