# Kết quả QA v87 trên macOS (2026-11-09)

Máy: MacBook (Darwin 27.0.0, 10 core, 24 GB), Rust 1.99.1 (rustup cài mới), Node v26.12.1.

## Test tự động

| Bộ | Trước sửa | Sau sửa |
|---|---|---|
| `cargo test` (apps/core) | 1 fail: `core-transcript` `dedupes_globally_and_keeps_resume_copies_after_delete` | pass hết |
| collector | 11 fail (parity tracker Node ↔ Rust + recovery) | 465 pass / 3 skip |
| desktop | 8 fail (5 file) | 1515 pass / 27 skip, typecheck sạch |

## Lỗi chỉ có trên mac đã sửa

1. Thứ tự đọc thư mục: Rust `std::fs::read_dir` trên APFS trả thứ tự thô, Node (libuv `scandir`) sort theo byte trên unix. Lệch parity thật: message id trùng giữa 2 file usage gán khác file; thứ tự subagent trong tracker khác Node. Sửa: `read_dir_sorted` trong `jobs.rs` (unix sort byte, Windows giữ thứ tự OS như libuv), dùng cho `list_jsonl`, `find_transcript`, `discover`.
2. Symlink `/tmp`, `/var` → `/private/...`: so đường dẫn đã realpath với root chưa resolve → `denied` / `missing` sai. Sửa trong code (người dùng mac thật có thể gặp): `permRules.ts` `writeAllow`, `agentStatus/watch.ts` `readStatus`, `fileWrite.ts` `writeTextFile` (realpath roots bên trong), `logRead.ts` (file không tồn tại: so thêm root gốc để trả `missing`). Kiểm containment giữ nguyên.
3. `shim.test.ts`: đường dẫn `C:\` chỉ assert trên win32 (lỗi test).

## Còn treo

- `cargo clippy -D warnings` trên mac: `core-procs/src/kinfo.rs:21` `chunks_exact` hằng số (lint mới, file chỉ build trên mac; CI không chạy clippy). Chưa sửa.
- `cargo fmt --check` lệch cả repo (không dùng rustfmt) — bỏ qua.
- QA tay 12 bước trong `QA-v87-mac.md`: chờ chủ nhân chạy `npm run desktop`.
