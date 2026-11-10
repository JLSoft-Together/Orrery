# WP-v87-4-T5b report — hook / status line / DeepSeek outbox files in Rust (`CCM_CORE_SIDEFILES`)

Báo cáo cũ của Codex liệt kê nhiều file không hề tồn tại; bản này viết lại theo trạng thái thật (đối chiếu `git status`).

## Đã làm
- Rust `apps/core/crates/core-transcript/src/sidefiles/` (thay `sidefiles.rs` 473 dòng của Codex, tách 8 file ≤ ~200 dòng): `common` / `hook` / `status` / `dir` / `outbox` / `outtail` / `mod` / `tests`. Ops `sidefiles.hooks|statusline {dir, now}` và `sidefiles.outbox {file, started, offset, skipping, fromEnd?, maxLine?}` (stateless, 1 MiB window). Sweep `KEEP_MS` (hooks 24 h, status line 7 d) chỉ trong đúng `dir` của request, chỉ tên `<uuid>.json`, dir phải tuyệt đối.
- Lỗi tìm thấy trong bản dở của Codex và đã sửa: `TOOL_RE` dùng Unicode alnum (TS chỉ ASCII); tên file `.JSON` (regex có cờ `i`); `deniedCount` `2.0`; outbox `at` trả chuỗi gốc thay vì `toISOString()`; `timestamp` hỏng không rơi về `created_at`; `schema_version` `1.0`; workspace đo byte thay vì UTF-16 và dùng `Path::is_absolute` theo platform; outbox đọc cả file mỗi lần và kẹt vĩnh viễn với dòng > max; thiếu `maxLine` / `fromEnd` / `skipping` / truncate; invalid UTF-8 làm hỏng cả file.
- TS seam (file mới): `sideFileSource.ts` (`CCM_CORE_SIDEFILES=rust|node`, mặc định `node`, `DirReader` / `OutboxTail` / `SideFileSources`, `createSideFileSources`), `nodeSideFiles.ts` (`HookReader` / `StatusLineReader` / `NodeOutboxTail`), `coreSideFiles.ts` (`CoreSideFiles` child lazy 30 s, lỗi → Node vĩnh viễn + `reportCoreFallback('sidefiles')` 1 lần; `CoreOutboxTail` giao lại cho Node qua `JsonlTailer.resumeAt` không mất / không lặp). Xoá `sideFiles.ts` của Codex.
- Sửa: `coreFallback.ts` (kind `sidefiles`), `tailer.ts` (`resumeAt`), `monitor.ts` (4 dòng: import / 2 field / 2 chỗ dựng, + `MonitorDeps.sideFiles`), `deepseek/monitor.ts` (dựng `outbox(...)`, callback nhận sự kiện đã parse, + `DeepseekMonitorDeps.sideFiles`). `monitor.ts` +1 dòng import, `deepseek/monitor.ts` −2 dòng.
- Desktop: `main.ts` `CORE_READERS.sidefiles` + `CORE_READERS_NODE_DEFAULT` + `noteCoreFallback`, `strings.ts` `coreFallbackSidefiles` (en + vi): notification + dòng Diagnostics (mặc định node), cùng cách `codex`.
- Docs: `CLAUDE.md` 1 mục env, `.code_index/{core,collector,_meta}.md`, plan §2.5 (protocol + khác biệt biên).

## File đổi (khớp `git status`)
- Mới: `apps/core/crates/core-transcript/src/sidefiles/*.rs`, `apps/collector/src/{sideFileSource,nodeSideFiles,coreSideFiles}.ts`, `apps/collector/test/sidefiles-parity{,-corpus}.test.ts|.ts`.
- Sửa: `apps/core/crates/core-transcript/src/{lib,serve}.rs`, `apps/collector/src/{coreFallback,tailer,monitor,deepseek/monitor}.ts`, `apps/desktop/src/{main,strings}.ts`, `CLAUDE.md`, `.code_index/{core,collector,_meta}.md`, `.planning/PLAN-v87-step4-core-transcript.md`.
- Test chạy làm đổi line ending của snapshot `fwp-host.test.ts.snap`: đã `git checkout` lại. `PROGRESS-2026-11-09-v87.md` đã sửa từ trước, không phải của WP này.

## Test (số thật)
- `cargo test --locked -j 4`: 142 pass / 0 fail (7 test mới `sidefiles::tests`); `cargo clippy --locked -j 4 -- -D warnings`: sạch cả host và `--target aarch64-apple-darwin`.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 492 pass / 0 fail / 3 skip (44 file pass, 1 skip); `sidefiles-parity.test.ts` 16 test. `npm test -w @ccm/desktop -- --maxWorkers=4`: 1524 pass / 18 skip (130 file). Typecheck collector + desktop: sạch.
- Parity (core debug build): 13 lượt đọc thư mục (63 record), 113 lượt đọc outbox (14848 event), 46 chuỗi ngày, 2 kịch bản monitor (`Monitor.pollExtras` limits; `DeepseekMonitor` start + 3 lượt đọc, snapshot store sau mỗi bước) khớp TS ↔ Rust.
- Kịch bản: `hook-bridge.mjs` / `statusline-bridge.mjs` thật ghi file rồi đọc 2 đường; bộ body lạ cho hooks và status line; fixture thô (JSON hỏng, rỗng, mảng, null, BOM, invalid UTF-8, key trùng, đúng 4096 / 4097 byte, > 4096, tên sai / `.tmp` / `.JSON`); file cũ 23 h / 25 h / 6 d / 8 d + thư mục bystander không bị đụng; file biến mất / đổi / quay lại giữa các lượt; outbox 3 seed × 3000 dòng ghi từng mẩu ngẫu nhiên, truncate, `fromEnd`, xoá + tạo lại file, dòng > max, dòng dở; file không tồn tại; core chết giữa chừng → Node tiếp không mất / không lặp + đúng 1 notification; không có bin → Node + 1 notification.

## Khác biệt biên còn lại (plan §2.5)
- `Date.parse` nhận 11 dạng Rust từ chối (offset không dấu `:`, `z` / `t` thường, `2026-1-1`, năm mở rộng, `Oct 29 2026`, RFC 1123, `10/29/2026`, `2026/10/29`); list ghim trong test (`KNOWN_DATE_DIFFS`); bridge luôn ghi `toISOString()`. Không có dạng nào Rust nhận mà Date.parse từ chối.
- Chỉ đọc code, chưa có test: số ngoài f64 (`1e999`) làm hỏng cả file / dòng ở Rust; `timestamp` > 8.64e15 ms: TS throw `RangeError`, Rust bỏ `at`; `payload.status` = `constructor`.
- Rust sắp xếp record theo tên file (Node theo `readdir`), không cache mtime theo file (đọc lại mỗi poll).

## Chưa làm / ghi chú
- Cờ vẫn mặc định `node`; chưa đo CPU-s (thư mục nhỏ, chủ yếu lợi ở outbox tail); chưa chạy trên app thật. Collector rust mode dùng 1 child `orrery-core transcript` chung cho `Monitor` + `DeepseekMonitor` (theo `bin`), riêng với child codex / tracker.
- Không đụng app Orrery đang chạy; thư mục thật không được đọc hay ghi, mọi test dùng thư mục tạm.

STATUS: DONE
