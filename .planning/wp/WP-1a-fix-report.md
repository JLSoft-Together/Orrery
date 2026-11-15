## WP-1a-fix - DONE

| # | Sửa | Ghi chú |
|---|---|---|
| 1 | có | `serve`: `read_until` + `from_utf8_lossy`, test `serve_survives_invalid_utf8` |
| 2 | có | request có `id` mà id không phải u64 / thiếu channel trả `{id, ok:false, error}`, test `serve_answers_malformed_requests` |
| 3 | có | `WorkerGuard` trả busy + done khi panic, join lỗi trả `ScanError::Panicked`; test `worker_panic_ends_scan_with_error` (qua `scan_with` + lister) |
| 4 | có | `run_scan` bọc `catch_unwind`, `Reporter` có Drop, slot luôn về None; test `engine_recovers_after_scan_panic` |
| 5 | có | thư mục CLOUD (attr hoặc reparse tag) không đi vào; test `cloud_classification` (win) |
| 6 | có | `Builder` giới hạn u32 (`overflowed`), `ScanError::TooLarge`, `Index::can_merge`; test `builder_stops_at_limits` |
| 7 | có | `merge` graft subtree (`append_grafted`), cha điều chỉnh size; 2 test merge |
| 8 | có | buffer bulk thread_local dùng lại (mac, CHƯA KIỂM) |
| 9 | có | `failed = true` khi len == 0 / vượt biên (mac, CHƯA KIỂM) |
| 10 | có | `checked_add` + `get` + `try_from` (mac, CHƯA KIỂM) |
| 11 | có | `Raw.alt` + `alts: Vec<OsString>` giữ tên gốc cho path con (Windows, Unix, mac); test `ill_formed_utf16_names_are_still_walked` |
| 12 | có | exclude path áp cho file và link; test `exclude_path_applies_to_files` |
| 13 | có | Unix `O_NOFOLLOW` cho job không phải root (còn khe nhỏ ở fallback `read_dir`), Windows `GetFileAttributesW` kiểm reparse; CHƯA test được race |
| 14 | có | `classify` coi `is_cloud_tag(tag)` là cloud dù thiếu attr |
| 15 | có | `inflate` streaming + `take`, `MAX_DECODED` 4 GiB; test `inflate_respects_limit` |
| 16 | có | tmp riêng (pid + seq), `SAVE_LOCK`, `sync_all`, xoá tmp khi lỗi; test `concurrent_saves_stay_readable` |
| 17 | có | thoát loop cancel scan rồi đợi max 3 s request + scan; panic request trả internal error; test `serve_drains_scan_before_returning` |

Test:
- `cargo test -p core-disk -j 8` -> 9 unit + 23 integration pass, 0 fail
- `cargo clippy -p core-disk --all-targets -j 8` -> No issues found
- `cargo build -p core-bin -j 8` -> Finished (warning chỉ ở core-desk, không thuộc gói này)
- API đổi: `Raw.alt`, `DirCtx.root`, `Lister` / `scan_with` / `Engine::with_lister`, `Builder::with_limits`, `Index::can_merge`, `ScanError::{Panicked, TooLarge}`; protocol NDJSON giữ nguyên, chỉ thêm reply lỗi cho request hỏng.
- Chưa kiểm: mac.rs (mục 8, 9, 10, 13 phần mac) và race link mục 13. `.code_index/core-disk.md` đã cập nhật.

Commit message đề xuất: fix(core): harden core-disk serve loop, scan panics, cloud dirs, merge of nested rescans and index store
