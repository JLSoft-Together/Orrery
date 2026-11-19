## WP-1a-fix: sửa lỗi review core-disk

- Giao SAU khi WP-1b-host xong (WP đó đang sửa `serve.rs`, `index.rs` để thêm `disk.forget`).
- Nguồn: review sonnet 15/11 của commit fcf8e0c3. Sửa theo thứ tự ưu tiên, mỗi lỗi có test tái hiện khi test được.
- Phạm vi: `apps/core/crates/core-disk/**`, `.code_index/core-disk.md`. Model: sonnet (opus nếu vòng 1 fail).

## Danh sách

1. HIGH `serve.rs:302`: `input.lines()` gặp UTF-8 lỗi thì `break` làm chết serve. Đọc `read_until(b'\n')` + `from_utf8_lossy`, bỏ qua dòng lỗi.
2. MEDIUM `serve.rs:303`: dòng không phải object / có `id` mà thiếu `channel` hợp lệ / `id` không phải u64 bị bỏ không trả lời, promise phía host treo. Có `id` thì trả `{id, ok:false, error}`.
3. MEDIUM `walk/mod.rs:295` + `:415`: panic trong job làm `busy` kẹt, scan treo; `join().unwrap_or_default()` nuốt mất listing. Guard drop giảm `busy` + cờ ERROR, truyền panic ra.
4. MEDIUM `serve.rs:254`: `scan()` / `build()` panic thì slot `scan` không về None, `disk.scan` báo busy mãi, không có `disk:done`. Guard dọn slot, set done, emit lỗi.
5. MEDIUM `walk/win.rs:137`: thư mục cloud (OneDrive placeholder, `RECALL_ON_DATA_ACCESS` / `RECALL_ON_OPEN`, cloud reparse tag) vẫn bị đi vào, provider có thể tải listing qua mạng. Gắn `CLOUD`, không đi vào.
6. MEDIUM `index.rs:137`: `as u32` tràn khi pool > 4 GiB hoặc > 4G node. Kiểm limit trong `push`, dừng scan với lỗi / cờ truncated.
7. MEDIUM `index.rs:302`: merge quét lại thư mục con thì xoá cả index của cha (C:\Users\x thay C:\). Chỉ thay subtree.
8. LOW `walk/mac.rs:47`: cấp buffer 256 KiB mỗi thư mục. Dùng lại 1 buffer mỗi worker.
9. LOW `walk/mac.rs:66`: break khi `len == 0` / vượt biên mà không set `failed`. Set `failed = true`.
10. LOW `walk/mac.rs:93`: `s + l` tràn với offset âm / lớn. `checked_add` + `get(..)`.
11. LOW `walk/win.rs:173`: tên file UTF-16/8 lỗi bị `lossy` thành U+FFFD rồi `join` ra path không tồn tại, cả subtree bị bỏ. Giữ `OsString` / UTF-16 gốc để dựng path con.
12. LOW `walk/mod.rs:350`: exclude path chỉ áp cho thư mục. Áp cho file.
13. LOW `walk/mod.rs:353`: thư mục bị đổi thành link giữa lúc list và open vẫn bị đi theo. `O_NOFOLLOW` cho job không phải root; Windows kiểm lại reparse attr.
14. LOW `walk/win.rs:156`: file có cloud tag mà không có bit attr vẫn bị gọi `GetCompressedFileSizeW` / `CreateFileW`. Coi `is_cloud_tag(tag)` là cloud.
15. LOW `store.rs:157`: `zstd::decode_all` không giới hạn output. Streaming decoder + `take(limit)`.
16. LOW `store.rs:137`: tên tmp cố định, không fsync. Tên tmp riêng mỗi lần, khoá save, `sync_all` trước rename.
17. LOW `serve.rs:318`: thoát loop không join thread scan / request, mất reply + `disk:done`. Join với timeout ngắn.

## Tiêu chí

- [ ] 17 mục sửa hoặc ghi lý do không sửa; mục 1-7 có test.
- [ ] `cargo test -p core-disk -j 18` pass, `cargo clippy -p core-disk --all-targets` sạch, `cargo build -p core-bin -j 18` sạch.
- [ ] Không comment trong code.
- Báo cáo: `.planning/wp/WP-1a-fix-report.md`.

## Nhật ký
- 1 audit xong, bat dau sua
- code 1-17 xong, dang test
- xong, bao cao da ghi
