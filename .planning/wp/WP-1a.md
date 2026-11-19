## WP-1a: Rust engine quét ổ đĩa + index + tìm kiếm (chưa có UI)

- Mục tiêu: crate Rust mới trong `apps/core` quét thư mục/ổ thật nhanh, không làm đơ máy, giữ index trong RAM + lưu đĩa, tìm theo tên + filter cực nhanh, xuất ops để main Electron gọi ở phase 1b. Đọc trước `.planning/PLAN-v101-disk-manager.md` (mục Engine quét + index, Không làm đơ máy, File trên cloud, Phase 1a).
- Phạm vi file: crate mới `apps/core/crates/core-disk/**`; được sửa `apps/core/Cargo.toml` (workspace member + deps) và `apps/core/crates/core-bin/src/main.rs` (thêm subcommand `disk`); `apps/core/Cargo.lock`. Ghi chú kiến trúc: `.code_index/core-disk.md` (mới).
- Cấm đụng: `apps/desktop/**`, `apps/web/**`, crate khác (đọc được để theo mẫu: `core-desk` nhất là `build_clean/scan.rs`, `core-procs`, cách `core-desk::stdio` nói NDJSON với main).
- Phụ thuộc: không. Song song với WP-0a (TS, không chung file).
- Model: opus (kiến trúc index dùng cho mọi phase sau).

## Yêu cầu

1. Walk song song: Windows dùng `FindFirstFileExW` + `FindExInfoBasic` + `FIND_FIRST_EX_LARGE_FETCH` (lấy size/attr/mtime ngay trong entry, không stat thêm); macOS `getattrlistbulk` (fallback `readdir`+`lstat`). Pool luồng tự viết hoặc crate nhẹ (rayon / crossbeam deque); số luồng theo mức `light|balanced|max` (balanced = nửa số nhân, light = 2). Hạ priority: Windows `THREAD_MODE_BACKGROUND_BEGIN` (IO + CPU thấp), macOS `setiopolicy_np` + nice. Không follow symlink/junction; không đi qua mount point khác ổ trừ khi được bảo.
2. Size trên đĩa (allocated): Windows dùng compressed/allocation size khi rẻ, hardlink chỉ tính 1 lần (file id) khi bật; macOS `st_blocks*512`. File cloud-only (Windows `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` / `RECALL_ON_OPEN` / `OFFLINE`, macOS `SF_DATALESS`) gắn cờ `cloud`, size trên đĩa 0, **không bao giờ mở nội dung**.
3. Index: cấu trúc gọn cho hàng triệu entry (vd mảng node: parent idx u32, name offset vào một string pool, size u64, mtime, flags; con lưu khoảng liên tục), size thư mục cộng dồn. Lưu đĩa nén (zstd hoặc lz4) dưới thư mục do caller truyền (`CCM_DATA_DIR/disk/`), có version header; load lại nhanh.
4. Tìm kiếm: tên chứa chuỗi (không phân biệt hoa thường, có bỏ dấu tiếng Việt), glob `*.mp4`, filter size min/max, mtime trước/sau, loại (ext group: image/video/audio/doc/archive/installer/code), chỉ file/chỉ thư mục, giới hạn trong thư mục con; trả top N theo size hoặc theo tên, kèm tổng số khớp. Chạy song song trên pool khi index lớn. Mục tiêu < 50ms trên 2 triệu entry.
5. Ops (NDJSON qua stdio, theo đúng kiểu `core-desk` đang dùng; subcommand `orrery-core disk`): `disk.scan {roots, level, excludes}` (stream progress theo batch ≤ 10 lần/giây: số file, số byte, thư mục đang đi), `disk.pause`, `disk.resume`, `disk.cancel`, `disk.children {path, sort, limit}` (con trực tiếp + size, cho treemap/cây), `disk.search {...}`, `disk.save`, `disk.load`, `disk.volumes` (danh sách ổ + total/free + loại FS, ổ mạng đánh dấu). Lỗi quyền / path dài (Windows `\\?\`) → đếm và bỏ qua, không dừng.
6. CLI bench: `orrery-core disk bench <path> [--level ..]` in thời gian quét, số entry, RAM index, thời gian 5 câu search mẫu.

## Tiêu chí nghiệm thu

- [ ] `cargo test -p core-disk` pass: test trên cây thư mục tạm (size cộng dồn đúng, hardlink 1 lần, symlink không follow, exclude, cancel giữa chừng, save/load round-trip, search từng filter, bỏ dấu tiếng Việt).
- [ ] `cargo build -p core-bin` sạch, `cargo clippy -p core-disk` không warning mới.
- [ ] Bench chạy thật trên 1 thư mục lớn của máy (vd `C:\Users\dauph` hoặc `F:\work`), ghi số vào báo cáo (entry, giây, MB RAM, ms search). Không quét cả ổ C: lúc máy đang dùng nếu chưa có level light.
- [ ] Khi đang quét ở level balanced, máy vẫn mượt (CPU tổng của tiến trình ≤ ~50%); ghi số đo.
- [ ] `.code_index/core-disk.md` mô tả cấu trúc index, ops, invariant (không mở file cloud-only, không follow link).
- [ ] Không comment trong code.

- Lệnh test: `cd apps/core && cargo test -p core-disk && cargo build -p core-bin`.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-1a-report.md`.

## Nhật ký
- 15/11 WP-1a: đọc brief + plan + mẫu core-desk, chốt thiết kế (walk pool std thread, index mảng node 48B, zstd, NDJSON kiểu core-desk).
- 15/11 WP-1a: viết xong crate core-disk (fold, index, walk win/unix/mac, search, store zstd, volumes, serve NDJSON, cli bench) + nối core-bin.
- 15/11 WP-1a: cargo test -p core-disk 16 pass, clippy sạch, build core-bin debug.
- 15/11 WP-1a: bench F:\work + C:\Users\dauph xong, viết .code_index/core-disk.md, test/clippy/build lại.
