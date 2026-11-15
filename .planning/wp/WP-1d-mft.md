## WP-1d-mft: Quét siêu nhanh NTFS bằng MFT (UAC, helper tự thoát)

- Mục tiêu: trên Windows, ổ NTFS, nút "Quét siêu nhanh" (backend ở WP này, UI chỉ thêm 1 nút) chạy `orrery-core disk mft` bản **elevated** qua UAC, đọc MFT (`FSCTL_ENUM_USN_DATA` hoặc đọc `$MFT` thô qua `FSCTL_GET_NTFS_VOLUME_DATA` + `ReadFile` volume), dựng cây path + size (allocated, `$DATA` non-resident size; hardlink tính 1 lần; file cloud-only không mở), gửi kết quả về tiến trình `orrery-core disk` thường qua **named pipe** do tiến trình thường tạo (tên ngẫu nhiên + token 32 byte một lần, ACL chỉ user hiện tại), rồi thoát. Index nạp vào engine như kết quả scan thường (merge theo root ổ). Đọc `.planning/PLAN-v101-disk-manager.md` (Engine, Q24), `.code_index/core-disk.md`, `.planning/wp/WP-1a-fix-report.md`.
- Bảo mật (bắt buộc): helper elevated chỉ nhận đúng 3 tham số (ký tự ổ, tên pipe, token); không đọc stdin, không chạy lệnh nào khác, không ghi file; kiểm ký tự ổ `[A-Z]`; nối pipe xong gửi token đầu tiên, sai token thì thoát; timeout 10 phút; tiến trình thường kiểm PID client của pipe (`GetNamedPipeClientProcessId`) là tiến trình nó vừa launch. Launch bằng `ShellExecuteExW` verb `runas` với đúng path binary hiện tại. Người dùng bấm Không ở UAC → trả lỗi `elevation-declined`, không thử lại.
- Không phải NTFS / không phải Windows / ổ mạng → op trả `unsupported`, UI ẩn nút.
- Op mới: `disk.mft {drive}` (tiến trình thường: tạo pipe, launch helper, nhận dữ liệu stream, progress `disk:progress`, xong `disk:done`). Bridge: `diskMapFastScan({ drive })` trong preload + ipc `ccm:diskmap-fastscan` (sửa `apps/desktop/src/diskMap/ipc.ts`, `preload.ts`, shared type), web thêm nút trong `DiskScanBar` khi volume `fs === 'NTFS'` và không phải ổ mạng, kèm dòng giải thích cần quyền admin (i18n `disk-v101.ts`).
- Phạm vi file: `apps/core/crates/core-disk/**`, `apps/core/Cargo.toml` / `Cargo.lock` (nếu cần feature `windows-sys`), `apps/desktop/src/diskMap/**`, `apps/desktop/src/preload.ts`, `packages/shared/src/diskMap.ts`, `apps/web/src/components/disk/DiskScanBar.vue`, `apps/web/src/lib/diskMap.ts`, `apps/web/src/stores/diskMap.ts`, `apps/web/src/i18n/disk-v101.ts`, `.code_index/core-disk.md`, `.code_index/desktop.md`.
- **Phối hợp WP-1c-search** (đang sửa `components/disk/**`, `stores/diskMap.ts`, `lib/diskMap.ts`, `i18n/disk-v101.ts`): làm phần Rust + desktop trước; phần web chỉ làm sau khi file `.planning/wp/WP-1c-search-report.md` xuất hiện (WP-1c xong). Nếu đến cuối mà WP-1c chưa xong → không sửa web, ghi patch cần thêm vào báo cáo.
- Model: opus (helper chạy quyền admin, FFI volume thô).

## Tiêu chí nghiệm thu

- [ ] Test Rust: parse record MFT từ buffer mẫu (record thật chụp từ volume test hoặc dựng tay đúng format: FILE header, fixup array, attribute $STANDARD_INFORMATION / $FILE_NAME (namespace Win32 vs DOS, chọn Win32) / $DATA resident + non-resident), dựng path từ parent ref, hardlink 1 lần, record bị xoá bỏ qua; kiểm tham số helper; token sai thì thoát.
- [ ] Chạy thật trên máy (UAC hiện, người duyệt): **chỉ chạy khi có người bấm**. Agent KHÔNG được tự bật UAC lúc chủ nhân không ngồi máy: thay vào đó viết script `scripts/mft-smoke.mjs` để chủ nhân chạy tay sau, in thời gian + số entry + so sánh tổng size với scan thường trên cùng ổ. Ghi "CHƯA KIỂM: cần chủ nhân bấm UAC" nếu không chạy.
- [ ] Test chạy được không cần admin: helper chế độ `--fixture <file>` đọc MFT từ file mẫu, đi đủ đường pipe + token + nạp index (E2E thường, không UAC).
- [ ] `cargo test -p core-disk -j 8`, `cargo clippy -p core-disk --all-targets -j 8` sạch; `npm test -w @ccm/desktop -- diskMap` pass; typecheck desktop + shared + web sạch.
- [ ] Không comment trong code.

- Nhìn thấy được: có (1 nút) — chụp nếu làm phần web.
- Báo cáo: `.planning/wp/WP-1d-mft-report.md`.

## Nhật ký
- 15/11: đọc brief + core-disk + host desktop, chốt thiết kế (helper dựng Index, gửi zstd qua pipe, fixture = subcommand riêng từ chối khi elevated). WP-1c chưa xong: web để sau.
- 15/11: Rust xong: mft/{record,tree,wire,sample,win}.rs, op disk.mft, cli mft-helper/mft-fixture; cargo test 62 pass, clippy sạch.
- 15/11: desktop xong (cleanFastScan, ccm:diskmap-fastscan, preload diskMapFastScan, shared DiskFastScan*), test diskMap 37 pass gồm 1 test orrery-core thật + fixture; scripts/mft-smoke.mjs chạy thử Z: (FAT32) -> unsupported.
- 15/11: web xong sau khi WP-1c báo DONE (nút Quét siêu nhanh + dòng cần quyền admin, store fastScan, i18n), web 2368 test pass, E2E v101-diskmap-fast 13/13, ảnh docs/qa/v101/WP-1d; code_index cập nhật; viết báo cáo.
