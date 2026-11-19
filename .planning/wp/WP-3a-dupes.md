## WP-3a-dupes: tìm file trùng

- Mục tiêu (PLAN v101 Q8, Q30 + "File trên cloud"): tab **File trùng** trong view Ổ đĩa.
- Engine (Rust `apps/core/crates/core-disk`): op `disk.dupes { under?: string[], minSize = 1 MiB, excludes? }` chạy nền như scan (progress `disk:progress` kiểu riêng hoặc event `disk:dupes-progress`, cancel / pause dùng chung cơ chế, level tài nguyên), lấy ứng viên **từ index** (không walk lại): file thường, size ≥ minSize, không cloud-only (không bao giờ mở file cloud), mặc định chỉ dưới thư mục user (home), bỏ `node_modules`, `.git`, thư mục hệ thống / app (`AppData`, `Library`, `Program Files`, `Windows`...), hardlink cùng file id coi là một (không phải trùng). Thuật toán: nhóm theo size → hash 64 KiB đầu + 64 KiB cuối (BLAKE3) → hash đủ BLAKE3 streaming (buffer tái dùng, IO priority thấp, số luồng theo level) chỉ cho nhóm còn ≥ 2. Trả `{ groups: [{ size, hash, files: [{path, mtime}] }], wasted, ms }` sort theo `wasted = size * (n-1)` giảm dần; cap số nhóm trả về (vd 5000) + tổng.
- Bridge: `ccm:diskmap-dupes` / `diskMapDupes(req)`, `ccm:diskmap-dupes-cancel`, event progress; type ở `packages/shared/src/diskMap.ts`.
- UI: tab mới trong `DiskView.vue` (1 chỗ gắn) + component `apps/web/src/components/disk/dupes/**`: nút Tìm, progress (giai đoạn: nhóm size / hash nhanh / hash đủ, số byte đã đọc), danh sách nhóm ảo (tên, số bản, size mỗi bản, phí X GB), mở nhóm thấy từng bản (path rút gọn có tooltip, ngày sửa, Mở trong Explorer/Finder). **Tick sẵn bản thừa**: giữ bản cũ nhất (mtime nhỏ nhất), hoà thì giữ path ngắn nhất; người dùng đổi được (radio "giữ bản này"); luôn còn ít nhất 1 bản không tick trong mỗi nhóm (chặn ở store). Thanh hành động: tổng sẽ giải phóng + Vào Thùng rác / Xoá hẳn qua `diskMapDelete` (guard canonical, dialog native có sẵn). Filter: thư mục, loại, size min.
- Đọc `.planning/PLAN-v101-disk-manager.md`, `.code_index/core-disk.md`, `.planning/wp/WP-1a-fix-report.md`, `.planning/wp/WP-2sec-report.md`, `.planning/COORD-2026-11-15-v100-v102.md`.
- Phạm vi file: `apps/core/crates/core-disk/**` (+ `Cargo.toml` thêm `blake3`, `Cargo.lock`), `apps/desktop/src/diskMap/**` (thêm kênh, đọc lại file trước Edit), `apps/desktop/src/preload.ts` (khối riêng), `packages/shared/src/diskMap.ts`, `apps/web/src/components/disk/dupes/**`, `DiskView.vue` (gắn tab), `apps/web/src/stores/diskDupes.ts` (+test), `apps/web/src/lib/diskMap.ts`, `apps/web/src/i18n/disk-v101.ts`, E2E `apps/desktop/e2e/v101-dupes.cjs`.
- Cấm đụng: `apps/desktop/src/diskClean/**`, `components/disk/clean/**`, `stores/diskClean.ts`, `lib/diskClean.ts`, `i18n/disk-clean-v101.ts`, `packages/shared/src/diskClean.ts` (WP-2b); `apps/desktop/src/organize/**`, `components/disk/organize/**`, `stores/organize.ts` (WP-3b); `diskMap/guard.ts`, `diskMap/delete.ts` (không đổi hành vi guard); `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `apps/shell/**`.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] Test Rust: nhóm đúng trên cây tạm (cùng nội dung khác tên; cùng size khác nội dung đầu; khác nội dung chỉ ở giữa file → phân biệt ở hash đủ; hardlink không tính trùng; cloud-only không mở (giả lập cờ); file < minSize bỏ; cancel giữa chừng).
- [ ] Test store: tick sẵn bản thừa theo luật, không cho tick hết 1 nhóm, tổng giải phóng đúng.
- [ ] `cargo test -p core-disk -j 8` + clippy sạch; `npm test -w @ccm/desktop -- diskMap`, web test (`--maxWorkers=6`) pass; typecheck sạch.
- [ ] Bench thật (chỉ đọc) trên một thư mục lớn của máy ở level balanced, ghi số file đọc, GB hash, thời gian, CPU vào báo cáo.
- [ ] E2E trên **bản dev tách biệt** (cổng riêng, `CCM_E2E_DISKMAP_CONFIRM=1` chỉ có tác dụng bản dev, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): thư mục tạm có 3 nhóm trùng, tab hiện đúng, tick sẵn đúng, xoá bản thừa vào Thùng rác, dọn Thùng rác sau E2E. Chụp `docs/qa/v101/WP-3a/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-3a-dupes-report.md`.

## Nhật ký
- 18:50 Rust engine dupes.rs + serve op disk.dupes + 12 test tests/dupes.rs xanh. Tiếp: clippy, bridge, store, UI, bench, E2E.
- 19:00 Bridge xong: shared diskMap.ts types, diskMap/{map,host,ipc}.ts, preload block, test desktop diskMap-dupes (25 xanh). Tiếp: web lib, store, UI, i18n, DiskView tab, bench, E2E.
- 19:05 Web: lib/diskMap bridge, components/disk/dupes/{model,DupeList,DiskDupes}, stores/diskDupes, i18n diskDupes, tests (model+store 24 xanh), web typecheck sạch. Tiếp: gắn tab DiskView, bench, E2E, nhìn.
- 19:25 E2E v101-dupes 20/20, bench F:\work 79 GB / 103 s. Tiếp: gate cuối + báo cáo.
