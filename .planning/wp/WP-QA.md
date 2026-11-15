## WP-QA: sửa tồn + QA tổng v100 / v101 trên bản dev tách biệt

- Mục tiêu: dọn các việc nhỏ còn treo trong `.planning/PROGRESS-2026-11-15-click-disk.md` (cột Ghi chú) rồi chạy QA tổng.
- Sửa:
  1. Thanh mode của view Ổ đĩa có 9 tab (Duyệt, Tìm kiếm, File lớn, File cũ, Dọn dẹp, File trùng, Sắp xếp, App, RAM) bị cắt ở 760px: cho xuống dòng gọn hoặc gom bớt vào menu "Thêm" khi hẹp (đo bằng container width như pane header đang làm), không cuộn ngang, giữ keyboard / aria. Chỉ token + utility có sẵn.
  2. E2E `apps/desktop/e2e/v101-diskclean.cjs` và `v101-diskclean-system.cjs` chọn tab theo vị trí (tab cuối) nên hỏng khi thêm tab: chọn theo value / `data-*` attribute; rà các E2E v100 / v101 khác cùng kiểu.
  3. Ô ngày trong filter tìm kiếm (Sửa sau / Sửa trước) hiện `mm/dd/yyyy` theo OS: hiện theo locale app (vi `dd/mm/yyyy`), có thể dùng input text + parse hoặc component ngày có sẵn trong repo.
  4. "Cleanable" (size rác dọn được) ở rail / Monitor đang cộng cả mục bị khoá vì project đang mở: tách hoặc chỉ cộng mục không khoá.
  5. Lối vào "Chuyển sang ổ khác" mới có ở tab Dọn dẹp: thêm vào menu chuột phải tab Duyệt (thư mục) + Explorer Workspace (thư mục); lịch sử dọn ghi action `move` riêng (không chỉ dòng xoá bản cũ).
  6. Rebuild `orrery-core` vào target chính nếu không bị khoá (`cargo build -p core-bin --release -j 8`; bị khoá bởi Orrery đang chạy thì build vào target-dir trong scratchpad và trỏ `CCM_CORE_BIN`, không xoá / thay binary đang chạy) để `desk-mcp.twin.test.ts` chạy với binary mới.
- QA tổng (sau khi sửa):
  - `npm run typecheck`, `npm test` (vitest `--maxWorkers=6` nếu chạy tay), `cargo test -p core-disk -p core-desk -j 8`.
  - `npm run qa -- --no-e2e` từ repo root.
  - Toàn bộ E2E v100 + v101 trên bản dev tách biệt, mỗi script một cổng riêng (không chạy song song quá 3): `v100-menu-bar`, `v100-menu-click`, `v100-menu-queue`, `v100-menu-toast`, `v101-diskmap-host`, `v101-diskmap-ui`, `v101-diskmap-fast`, `v101-projsize`, `v101-diskclean`, `v101-diskclean-system`, `v101-dupes`, `v101-organize`, `v101-sdk-move`, `v101-apps-ram`, `v101-snap`, `v101-mcp-disk`.
  - Ghi `docs/qa/v101/RESULT.md`: bảng script → PASS/FAIL (số check), lỗi + cách sửa; test fail có sẵn từ trước (vd `desk-agent-data.twin` EPERM symlink) ghi riêng là "có từ trước", kiểm bằng `git stash` **KHÔNG được dùng** → kiểm bằng cách chạy test đó trên commit `6d7054be` qua `git worktree add` trong scratchpad nếu cần chứng minh, rồi `git worktree remove`.
- Phạm vi file: `apps/web/src/components/disk/**`, `components/workspace/ExplorerTree.vue` / `ExplorerPanel.vue` / `lib/explorer/contextMenu.ts` (mục 5), `components/workspace/rail/**` + `ProjectStrip.vue` (mục 4), stores / lib disk*, i18n `disk-*.ts`, `apps/desktop/src/diskMove/**` + `diskMap/history.ts` (mục 5), `apps/desktop/e2e/v100-*.cjs`, `v101-*.cjs`, `docs/qa/v101/RESULT.md`.
- Cấm đụng: `apps/shell/**`, `apps/core/crates/core-desk/src/mcp/registry.rs`, `apps/desktop/e2e/{browser,p6,shell-agent,tier2,v70-tools,whats-new}.cjs`, `.code_index/_meta.md`, `.planning/DECISIONS.md`, `.planning/RELEASE-4.0.6.md`, `CLAUDE.md`, `docs/**` trừ `docs/qa/v101/RESULT.md` (WP-DOC đang viết docs), `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] 6 mục sửa xong (hoặc lý do), mỗi mục có test hoặc check E2E.
- [ ] Typecheck sạch, test toàn bộ xanh trừ lỗi có từ trước (đã chứng minh), `npm run qa -- --no-e2e` xanh.
- [ ] 16 E2E v100 / v101 PASS trên bản dev tách biệt (cổng riêng, không kill Orrery/Electron/orrery-core đang chạy, không dùng 4317 / 5173, không `npm run desktop:dev`); `docs/qa/v101/RESULT.md` đủ.
- [ ] Ảnh thanh mode ở 760px + 1400px vào `docs/qa/v101/QA/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".
- Báo cáo: `.planning/wp/WP-QA-report.md`.

## Nhật ký
- 20:13 mục 1,3,4 sửa code (DiskView wrap + data-value, ô ngày text theo locale, junkOf bỏ item khoá)
- 20:15 mục 5 sửa code (menu Browse + Explorer, action move, fromClean open); mục 2 e2e chọn theo data-value
- 20:16 thêm test mục 3,4,5; typecheck sạch; chạy test
- 20:21 viết e2e v101-qa.cjs (mode bar 760/1400, ô ngày, menu move); build web+desktop xong; chờ cargo scratchpad
- 20:22 core build xong vào scratchpad qa-target; chạy test desktop + E2E đợt 1
- 20:25 E2E v100-menu-bar 15, click 25 (lần 1 fail 2 do chuột thật di qua, lần 2 sạch), queue 17
- 20:28 toast 14, host 18, ui 30, fast 13, diskclean 22; projsize sửa kỳ vọng (menu move, junk khoá)
- 20:30 16/16 E2E PASS + v101-qa 19; chạy qa/cargo/typecheck cuối
