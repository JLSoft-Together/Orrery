## WP-2c-projsize: size project trong Workspace + Monitor

- Mục tiêu (PLAN v101 "Size project trong Workspace"): lấy size từ index `core-disk` (không quét riêng; chưa có index thì không hiện gì, không tự quét):
  1. Explorer của Workspace (`ExplorerTree.vue` / `ExplorerPanel.vue`): thư mục hiện size bên phải tên (chữ nhỏ, màu ink phụ), chỉ khi index có path đó; menu chuột phải thêm "Mở trong Ổ đĩa" (mở view Ổ đĩa tab Duyệt tại path đó) và "Dọn dẹp project này" (tab Dọn dẹp lọc theo project, dùng `useDiskCleanStore().openFor(dir)` đã có từ WP-2a).
  2. Rail / group project của Workspace: tooltip hoặc dòng phụ size project + rác dọn được (nếu đã có kết quả scan Dọn dẹp gần nhất cho project đó, không tự quét).
  3. Monitor: card / dòng project hiện size + rác dọn được theo cùng nguồn.
- Bridge: dùng `diskMapChildren` / `diskMapStatus` (đã có). Nếu cần lấy size nhiều path một lần cho Explorer (tránh gọi từng thư mục), thêm op Rust `disk.sizes {paths}` → `{path: size|null}` trong `apps/core/crates/core-disk` + kênh `ccm:diskmap-sizes` + preload `diskMapSizes` + type shared; cache phía web theo `scannedAt`.
- Hiệu năng: không gọi IPC mỗi lần render; batch theo thư mục đang mở, debounce, cache Map theo path + version index.
- Đọc `.planning/PLAN-v101-disk-manager.md`, `.planning/wp/WP-1b-host-report.md`, `.planning/wp/WP-2a-devjunk-report.md`, `.planning/COORD-2026-11-15-v100-v102.md` (chỉ token có sẵn), `.code_index/web.md`, `.code_index/core-disk.md`.
- Phạm vi file: `apps/web/src/components/workspace/ExplorerTree.vue`, `ExplorerPanel.vue`, component rail / group (tìm trong `components/workspace/rail/**`), component Monitor project (tìm), mới `apps/web/src/composables/useDiskSizes.ts` (+test), `apps/web/src/lib/diskMap.ts` (thêm hàm sizes), `apps/web/src/i18n/disk-v101.ts`; nếu thêm op: `apps/core/crates/core-disk/**`, `apps/desktop/src/diskMap/**`, `preload.ts` (1 dòng), `packages/shared/src/diskMap.ts`.
- Cấm đụng: `apps/desktop/src/diskClean/**`, `components/disk/clean/**`, `stores/diskClean.ts`, `lib/diskClean.ts`, `i18n/disk-clean-v101.ts`, `packages/shared/src/diskClean.ts` (WP-2b đang làm), `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `apps/shell/**`.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] Test composable: batch + cache + bỏ khi index đổi version; không index → không gọi.
- [ ] Nếu thêm op Rust: `cargo test -p core-disk -j 8` pass, clippy sạch.
- [ ] `npm test -w @ccm/web -- --maxWorkers=6` pass; typecheck web + desktop + shared sạch.
- [ ] E2E trên **bản dev tách biệt** (cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): quét thư mục tạm có project giả, mở project đó trong Workspace, Explorer hiện size thư mục, chuột phải "Mở trong Ổ đĩa" nhảy đúng path. Chụp `docs/qa/v101/WP-2c/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-2c-projsize-report.md`.

## Nhật ký
- 18:35 Rust op disk.sizes + test thêm xong, chạy cargo
- Desktop kênh ccm:diskmap-sizes + preload + shared type xong
- Web: composable useDiskSizes, ExplorerTree/Panel, contextMenu, rail tip, ProjectStrip hint, i18n xong; còn test + typecheck + E2E
- tests: web useDiskSizes (8) + contextMenu; desktop diskMap-sizes; guard/delete test fail không phải của WP này (guard.ts đang bị sửa bởi session khác)
- cargo test core-disk 24 pass + clippy sạch; build release core-bin xong; đang build web + desktop cho E2E v101-projsize.cjs (cổng 4475/CDP 9475)
