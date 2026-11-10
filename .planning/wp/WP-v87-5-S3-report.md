# WP-v87-5-S3 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S3.md`, dòng S3 plan `PLAN-v87-step5-core-store.md`. Không commit / push, không sửa `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`, không đụng app Orrery đang chạy (test dùng thư mục tạm, không bind cổng).

## Đã làm

Module mới `apps/core/crates/core-store/src/persist/` (mỗi file < 310 dòng, không comment), bản 1:1 của các store nhỏ ở `apps/collector/src`:

- `json_file.rs` = `json-file.ts`: compact JSON, `<file>.tmp` + rename, mkdir, đọc lỗi = không có state, debounce do chủ chạy (`schedule(now)` / `next_due` / `poll(now, serialize)` / `flush` / `flush_pending`, giống `Store::poll`, không có timer bên trong).
- `aliases.rs` (alias max 80, notes max 140, ghi pretty ngay), `favorites.rs` (+ `paths.rs`: `resolve` / `basename` / `is_absolute` / `join` cho Win và Posix, `order`, sort legacy bằng `collate.rs` = `locale_cmp`), `history.rs` (max 1000, 10 s), `snooze.rs` (+ `snoozed_until`, 500 ms), `response.rs` (10 s), `timeline.rs` (30 s, roll nửa đêm), `forecast.rs` (`forecast_window` + `LimitHistory`, 60 s), `daystart.rs`, `datadir.rs`, `notify_file.rs` (`notify.json`: toast, locale, kinds, quiet, stuckMinutes, idleMinutes, key; sanitizer của `shared/notify.ts`; logic toast để S7).
- `localtime.rs` (`Zone::Local` / `Fixed`, `parse_iso` kiểu `Date.parse` ISO; sao từ `core-transcript/src/time.rs` để `core-store` không phụ thuộc transcript), `jsstr.rs` (JS whitespace, UTF-16 slice, `js_round`).
- Stores nhận `Clock` + `Zone` tiêm được; `Cargo.toml` core-store thêm `windows-sys` (`Win32_Foundation`, `Win32_System_Time`) / unix `libc` — đã có trong lock, không crate mới.
- Cổng parity: bin `src/bin/persist-cli.rs` + `apps/collector/test/persist-cross.test.ts` (14 test): từng store 2 chiều Node ghi → Rust đọc, Rust ghi → Node đọc, deep-equal (aliases, notes, favorites, history, snooze, response, timeline, limits + forecast probe, `notify.json` qua `Toaster`, daystart, datadir 4 case). Test tự skip khi chưa build `persist-cli` (hoặc `CCM_PERSIST_CLI`).
- Test Node cũ `favorites` / `daystart` / `datadir` giữ nguyên, pass; tương ứng Rust: unit trong `favorites.rs` / `daystart.rs` / `datadir.rs`.
- Plan §3.4, `.code_index/{core,_meta}.md`.

## File

- Mới: `apps/core/crates/core-store/src/persist/{mod,json_file,jsstr,localtime,paths,collate,aliases,favorites,history,snooze,response,timeline,forecast,daystart,datadir,notify_file}.rs`, `apps/core/crates/core-store/src/bin/persist-cli.rs`, `apps/collector/test/persist-cross.test.ts`.
- Sửa: `apps/core/crates/core-store/Cargo.toml`, `apps/core/crates/core-store/src/lib.rs` (`pub mod persist;`), `apps/core/Cargo.lock` (chung với S2: phần của S3 chỉ là cạnh `windows-sys` / `libc` cho core-store), `.planning/PLAN-v87-step5-core-store.md` (§3.4), `.code_index/core.md`, `.code_index/_meta.md`.
- Không đụng `src/store/**`. `replay.rs`, `apps/core/Cargo.toml`, `crates/core-http/` trong git status là của S2.

## Test (số thật)

- `cargo test --locked -j 4 -p core-store`: 66 passed (lib 39 = S1 6 + persist 33, golden 4, replay 3, shared_ts 5, store 15), 0 failed. Chạy cả workspace `cargo test --locked -j 4` (lúc đó có cả file của S2): mọi binary test ok.
- `cargo clippy --locked -j 4 -p core-store --all-targets -- -D warnings`: sạch. Clippy cả workspace lỗi ở `core-http` (WP S2, `type_complexity`), không thuộc S3.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 47 file passed + 1 skipped, 514 passed, 3 skipped (trước S3: 500; +14 `persist-cross`).
- `npm run typecheck -w @ccm/collector`: sạch.

## Khác biệt biên (đã biết, không ảnh hưởng nội dung file)

- Lỗi ghi file chỉ `eprintln`; ghi alias / notes / favorites chạy đồng bộ trong lệnh gọi (Node async), file nhỏ.
- `Date.parse` chỉ hiểu ISO (như `core-transcript`); chuỗi ngày kiểu khác → coi là không hợp lệ.
- `history.json`: `SessionRecord` dùng `u64` nên phần lẻ của token / compactions bị cắt (Node giữ số lẻ). Thực tế luôn nguyên.
- Thứ tự key trong object ghi ra có thể khác Node (nội dung bằng nhau); id dạng số nguyên thuần không bị JS sắp lại key.
- `localeCompare` cho sort favorites legacy = `locale_cmp` xấp xỉ ICU cho ASCII.
- Mốc nửa đêm local dùng offset hệ thống từng khe 15 phút; ngày chuyển DST có thể lệch vài phút so với ICU.

## Việc cho WP sau

- S2 / S4: dựng các store qua `Zone::Local` + `system_clock()`, gọi `next_flush_at()` / `poll(now)` của từng store trong vòng tokio (cùng nhịp với `Store`), `flush` / `save` khi shutdown. `SnoozeStore.prune` phải xếp sau flush của `Store` (như S1 ghi).
- S7: dùng `NotifyFile` + `NotifyPrefs` (đã có sanitizer, `kinds` giữ thứ tự `TOAST_KINDS`); key sinh bằng hàm truyền vào `NotifyFile::new` (chưa có crate random trong lock).
- `CCM_DATA_DIR` env chưa gói vào Rust (`config.ts` là việc S2 / S10); `default_data_dir` đã có.

STATUS: DONE
