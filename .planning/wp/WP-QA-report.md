## WP-QA — DONE

6 mục sửa:
1. Mode bar Ổ đĩa: `DiskView.vue` class `max-w-full flex-wrap gap-y-0.5` (xuống 2 dòng ở 760px, 1 dòng ở 1400px, không cuộn ngang, aria/phím mũi tên giữ). `SegmentedControl.vue` thêm `data-value` (ngoài phạm vi components/disk nhưng 1 dòng, cần cho E2E).
2. E2E chọn tab theo `data-value`: v101-diskclean, diskclean-system, sdk-move, organize, dupes, apps-ram, diskmap-ui.
3. Ô ngày: `DiskSearch.vue` input text, `lib/diskMap.ts` thêm `datePattern/formatDateInput/parseDateInput` (vi dd/mm/yyyy, en yyyy-mm-dd, nhận ISO, ngày sai thì hoàn lại).
4. Cleanable: `junkOf` (useDiskSizes.ts) bỏ mục khoá (`lockReason` + `clean.open`).
5. Chuyển ổ: menu chuột phải Duyệt (`DiskTree.vue`) + Explorer (`contextMenu.ts` `disk-move`, `ExplorerTree/Panel`, `useDiskNav.moveFolder`); IPC `from-clean` nhận `open: true` (service.fromClean chỉ nhận path tuyệt đối, vẫn qua `block()` + confirm native); history action `move` (`diskMove/service.ts`, `diskMap/history.ts`, `packages/shared` diskMap.ts + diskMove.ts, `DiskHistory.vue`, i18n en/vi trong `disk-v101.ts`), `totalFreed` không cộng `move`.
6. `orrery-core` build vào scratchpad `qa-target` (3m01s), `desk-mcp.twin` xanh. Binary Orrery đang chạy ở `%LOCALAPPDATA%\Orrery`, build vào `apps/core/target` bị auto-mode chặn nên dùng scratchpad.

E2E (17 script, 16 yêu cầu + `v101-qa.cjs` mới): v100-menu-bar 15 PASS, click 25 PASS (lần 1 fail 2 check mouse mode do chuột thật, chạy lại sạch), queue 17, toast 14; v101-diskmap-host 18, ui 30, fast 13, projsize 17, diskclean 22, diskclean-system 26, dupes 20, organize 27, sdk-move 28 (thêm check history `move`), apps-ram 26, snap 36, mcp-disk 26; v101-qa 19 (mode bar 760/1400, ô ngày vi, menu move từ Duyệt). Tất cả PASS. Thùng rác không còn item E2E.

Lệnh: `npm run typecheck` sạch; `npm test -w @ccm/web` 2464 pass (275 file); `npm test -w @ccm/desktop` 1993 pass, 1 fail; `cargo test -p core-disk -p core-desk -j 8` pass (326+...); `npm run qa -- --no-e2e` 9/9 pass.
Lỗi có từ trước: `desk-agent-data.twin` EPERM symlink, `fs.symlinkSync` thuần trong tmp cũng EPERM (không có quyền symlink), file test không bị sửa; không dùng worktree.
Test mới: junkOf locked, date locale, contextMenu disk-move, DiskView data-value + wrap, diskMap totalFreed bỏ move, history accept move, service ghi history move, ipc from-clean open.
Sửa kỳ vọng E2E v101-projsize (menu thêm "Move to another drive", tooltip rail không "Cleanable" khi project mở).
Nhìn: `docs/qa/v101/QA/mode-bar-760.png` (2 dòng, RAM xuống dòng 2) và `mode-bar-1400.png` (1 dòng) đã xem, đúng. `docs/qa/v101/RESULT.md` đã ghi.
Chưa làm: Tauri shim chưa có `from-clean open` (luồng Tauri); build desktop `apps/desktop/stage` + `apps/web/dist` đã rebuild lại.

Commit message đề xuất:
fix(disk): wrap the Disk mode bar, locale date fields, count only unlocked cleanable size, add move to another drive from Browse and Explorer with its own history entry, select E2E tabs by value
