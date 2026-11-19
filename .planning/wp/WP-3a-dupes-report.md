## WP-3a-dupes: DONE (kèm 1 cảnh báo ngoài phạm vi)

**Đã đổi**
- Rust `core-disk`: `src/dupes.rs` (mới), op `disk.dupes` + event `disk:dupes-progress` / `disk:dupes-done` trong `serve.rs` (Reporter nhận closure), `lib.rs`, `cli.rs` (`bench-dupes`), `walk/mod.rs` (chỉ đổi visibility `Excludes`, `wait_if_paused`), `Cargo.toml` + `Cargo.lock` (`blake3 = "1"`), `tests/dupes.rs` (13 test).
- Bridge: `packages/shared/src/diskMap.ts` (types), `desktop/src/diskMap/{map,host,ipc}.ts` (`cleanDupes`, `toDupesProgress/Done`, `ccm:diskmap-dupes`, `ccm:diskmap-dupes-cancel`, home không có trong index thì thử các root), `preload.ts` (khối riêng 4 dòng), `desktop/test/diskMap-dupes.test.ts`.
- Web: `lib/diskMap.ts` (bridge), `stores/diskDupes.ts` (+test), `components/disk/dupes/{model.ts,model.test.ts,DiskDupes.vue,DupeList.vue}`, `i18n/disk-v101.ts` (`diskDupes.*`, en + vi), `DiskView.vue` gắn tab `'dupes'` (additive, không đụng dòng của clean), E2E `desktop/e2e/v101-dupes.cjs`. `.code_index/core-disk.md` + `web.md` đã thêm mục.

**Tiêu chí 7/7** (test Rust, test store, lệnh test + clippy, bench, E2E, không comment / từ cấm).

**Bench thật (chỉ đọc)**, `F:\work`, level balanced (10 luồng / 20), min 1 MB, release: index 2 196 606 file, 51 011 ứng viên, hash 35 697 file, **79.02 GB đọc**, **103.6 s** (781 MB/s), CPU trung bình 3.4% máy (0.67 core), đỉnh 5.9%, peak RSS 342 MB, 1864 nhóm trùng (phí 42.78 GB), skipped 0.

**Audit / Review / Test (kết quả thật)**
- Edge: nhóm 300 bản + cap file/nhóm + budget tổng 50 000 file (test), file khoá (Windows `share_mode(0)` -> `skipped.locked`), file đổi size / mtime sau scan (`skipped.stale`), đổi giữa lúc hash (stat lại sau hash + đọc thiếu / thừa byte -> `changed`), path Unicode (Tiếng Việt, Nhật), index cũ hơn đĩa, hardlink (dedupe theo file id lúc hash, kể cả Windows không bật hardlink scan), cloud-only (cờ index + kiểm attribute RECALL/OFFLINE/reparse trước khi mở, không bao giờ mở), cancel (trước + giữa chừng).
- `cargo test -p core-disk -j 8` -> 16+13+19+5+24 pass, 0 fail. `cargo clippy -p core-disk -j 8 --all-targets` -> sạch.
- Web `npm test -w @ccm/web -- disk Dupes messages` -> 14 file, 129 test pass. `npm run typecheck -w @ccm/web` chỉ còn lỗi ngoài phạm vi: `packages/shared/src/index.ts(41)` `MoveRecord` export trùng giữa `agentData` và organize (WP-3b).
- Desktop `diskMap-dupes` + `diskMap-host/sizes` pass 25/25 lúc chạy (19:03), nhưng LẦN CHẠY CUỐI 7/8 file fail vì `apps/desktop/src/diskMap/guard.ts:151` đang lỗi cú pháp do session khác sửa dở (file cấm đụng, không phải của WP này). Cần chạy lại sau khi guard.ts sửa xong.
- E2E bản dev tách biệt (cổng 4483 / 9483, `CCM_E2E_DISKMAP_CONFIRM=1`, core build vào target-dir riêng nên không đụng orrery-core đang chạy): **20/20 pass**: 3 nhóm trùng đúng, decoy cùng size và file nhỏ không thành nhóm, bản cũ nhất tự giữ, không tick hết nhóm, radio đổi bản giữ, xoá 4 bản vào Thùng rác (còn lại 3 bản giữ + file khác nguyên vẹn), history `trash` count 4, Recycle Bin đã dọn, không scroll ngang ở 900px. E2E bắt được 1 bug (checkbox DOM tự nhảy khi store từ chối) đã sửa: disable ô của bản giữ cuối + đồng bộ lại `checked`.

**Nhìn:** có. `docs/qa/v101/WP-3a/` (01-before-search, 02-result-expanded, 03-keep-changed, 04-after-clean, 05-narrow-900). Đã xem 02 và 05: layout đúng, dark theme Galaxy, không tràn. Bản sửa summary sống (sau khi xoá) chụp lại trong lần E2E cuối.

**Hot file đã chạm (khối riêng, additive):** `preload.ts` (4 dòng sau `diskMapSizes`), `DiskView.vue` (import, `dupes` store, `dupesTab`, `dupesMode`, `setTab`, 1 dòng template). Không đụng `packages/shared/src/index.ts` (diskMap đã export sẵn).
**Cần thêm từ main:** (1) sửa lỗi `MoveRecord` ở shared/index (WP-3b) và `guard.ts` (WP-2b hoặc session khác) rồi chạy lại `npm test -w @ccm/desktop -- diskMap` + `npm run build -w @ccm/web`; (2) build thật cần `orrery-core.exe` mới (đang bị Orrery chạy khoá nên con build vào target-dir riêng); (3) macOS chưa kiểm (`file_id` unix, `lower_priority`).

**Commit message đề xuất:** `feat(disk): add duplicate file finder with BLAKE3 size, quick and full hash stages, hardlink and cloud-only safe, keep-oldest picks and trash or delete from the Disk tab`
