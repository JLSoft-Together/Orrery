# WP-v87-5-S6 — báo cáo

## Đã làm

Crate mới `apps/core/crates/core-sources` (port Rust của resources sampler + tree + idle, sensitive rules + watch, prompt-search). Không regex crate: mọi regex JS viết tay (kể cả thứ tự backtracking của `daemonOf`). Gọi thẳng `core-procs` / `core-store` / `core-transcript`, không qua pipe NDJSON.

- `mask.rs`: `mask_secrets` (= `shared/secretMask.ts`).
- `resources/`: `procs.rs` (`ProcRow`, `ProcReader`, `CoreReader` trên `core_procs`), `daemon.rs`, `tree.rs`, `idle.rs`, `ports.rs`, `fixture.rs` (`CCM_RES_FIXTURE`), `sampler.rs` (report là `serde_json::Value` đúng shape `ResourceReport`), `runner.rs` (`Runner` thread bật / tắt như `setActive`, `busy_counts` + `BusyTracker` = message `busy` chỉ khi đổi).
- `sensitive/`: `settings.rs` (sanitizer + `rule_group`), `paths.rs`, `classify.rs` (`classify_touch`; `touch_of` / `command_hits` dùng lại từ `core-transcript` T2), `watch.rs` (`SensitiveWatch`, `sensitive.json` qua `JsonFile` 300 ms, `next_due` / `poll` / `flush`).
- `prompt_search/`: `text.rs`, `nfc.rs` + `nfc_table.rs` (sinh từ `String.normalize`), `scan.rs` (chunk 64 KiB, dòng max 2 MiB), `searcher.rs` (`start` / `cancel` / `drop_client` / `open`).
- Bin `sources-cli` (JSON stdin) làm harness parity.
- Bản Node `sensitive/rules.ts` mà `coreTracker` còn gọi: chưa bỏ (cấm sửa `apps/collector/src`); bản Rust đã sẵn để S10 / S12 nối và gỡ.

## File đổi (khớp `git status`, phần của S6)

- Mới: `apps/core/crates/core-sources/**` (Cargo.toml, `src/{lib,mask}.rs`, `src/resources/*`, `src/sensitive/*`, `src/prompt_search/*`, `src/bin/sources-cli.rs`, `tests/{resources,sensitive,prompt_search}.rs`), `apps/collector/test/sources-cross.test.ts`, `.planning/wp/WP-v87-5-S6-report.md`.
- Sửa: `apps/core/Cargo.toml` (thêm `crates/core-sources` vào members; dòng members có thêm `core-monitor` của S4), `apps/core/Cargo.lock` (entry `core-sources`; phần còn lại của diff lock là của S4), `.code_index/{core,collector,_meta}.md`, `.planning/PLAN-v87-step5-core-store.md` (dòng S6 + §3.5).
- Không phải của S6 nhưng đang có trong git status: `core-transcript/src/tracker/{mod,subs}.rs`, `apps/core/crates/core-monitor/` (S4), `.planning/PROGRESS-…`, `RELEASE-v2.3.0.md`, `wp/WP-v87-5-S4.md`, `docs/qa/*`.
- Dep mới: chỉ `memchr = "2"` (đã có trong lock nhờ `core-transcript`, dùng cho lọc byte từng dòng).

## Test (số thật)

- `cargo test --locked -j 4 -p core-sources`: 48 passed, 1 ignored (đo máy thật) = unit 27 + `resources.rs` 8 + `sensitive.rs` 7 + `prompt_search.rs` 6. Cả workspace `cargo test --locked -j 4`: 0 failed (lúc chạy có cả crate của S4 trong cây).
- `cargo clippy --locked -j 4 --all-targets -- -D warnings` (cả workspace): sạch.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 48 file passed + 1 skipped, 524 passed, 3 skipped (trước S6: 514; +10 `sources-cross`). `sources-cross` chạy lặp 12 lần liên tiếp: 0 lỗi (một lỗi flaky lúc đầu = `mtimeMs` lệch 1 ms, đã sửa bằng đúng công thức libuv).
- `npm run typecheck -w @ccm/collector`: sạch.
- Không build dist, không chạm app đang chạy (cổng 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm` không đụng; mọi test dùng thư mục tạm). `~/.claude` không đọc.

## Kết quả parity

`apps/collector/test/sources-cross.test.ts` (Node vs Rust trên cùng input, 0 khác biệt, deep-equal):

1. `daemonOf` 34 command line (gradle / kotlin / dart / flutter / emulator, các nhánh backtracking).
2. `maskSecrets` × 3 platform.
3. `classifyTouch` WIN + POSIX × 4 bộ settings (mặc định, glob + ignore, không project, tắt nhóm) trên file và lệnh.
4. `buildTree` 4 ngữ cảnh × 3 rừng process (+ `portsByPid`).
5. `ResourceSampler` 5 bước (idle / forgotten, orphan, emulator + `deviceUsed`, outside ports, port conflict, session kết thúc).
6. `SensitiveWatch`: quyết định check / ignore / update, file `sensitive.json` Node ghi = Rust ghi, đọc file bẩn / hỏng.
7. Prompt: `promptSearchTerms`, `snippetOf`, `promptTextOf`.
8. `PromptSearcher` trên thư mục projects tạm: 8 truy vấn × 2 platform (days, project, title, mask, session con bị bỏ).

Test Rust chuyển 1:1 từ `resources.test.ts` (không gồm phần Toaster, là S7), `sensitive.test.ts` (không gồm phần Toaster phone), `prompt-search.test.ts` (không gồm validation WS, đã ở `core-http`).

CPU sampler: 1 pass Rust trên máy thật (debug) ~20 ms; không spawn PowerShell / `ps` như `cimReader`. Chưa đo CPU tổng của Node vs Rust trong app (cần S10).

## Khác biệt biên

- Surrogate lẻ (cắt giữa ký tự astral trong snippet, `\ud83d` trong dòng jsonl) → U+FFFD (Node giữ surrogate lẻ). Test so sau `toWellFormed()`.
- NFC chỉ gộp ký tự + dấu kết hợp U+0300–036F (Latin / Hy Lạp / Cyrillic, có sắp lại ccc); không có Hangul / singleton.
- JSON sâu > 128 bị bỏ dòng; `Date.parse` chỉ ISO; pid fixture phải là số nguyên; `sanitize_ignore` giữ thứ tự chèn (JS đẩy key số lên trước).
- `CoreReader` không có đường lui PowerShell / `ps` (đã ở cùng process): `core_procs::table()` lỗi → `read()` trả `None` → không có report (S10 quyết định thông báo / lui Node).
- `same_or_under` của prompt-search dùng path style của host (như `path.resolve`), chỉ phần so hoa-thường theo `platform`.

## Việc cho WP sau

- S10: dựng `Runner` với `sample()` (lấy `terminals` / `agents` / `jobs` từ `Store`, `idle_minutes` từ `NotifyFile`, `conflicts` từ tracker, `ports` = `core_procs::ports()` → `ports_by_pid` hoặc `FixtureReader::ports()`); đẩy report vào `Store::set_resources` + message `{type:"resources", report}` ra main; `BusyTracker::changed(busy_counts(..))` sau mỗi flush store; `PromptSearcher` chạy trên worker thread (mỗi client 1 search, `open_folder` = RPC `host.open`).
- S4 / S10: gọi `SensitiveWatch::check` từ tracker Rust (truyền `Touch` từ `core-transcript`), `root_of` = repo root của store; sau đó gỡ `sensitive/rules.ts` khỏi `coreTracker` (S12).
- S7: `resources` report đã đủ trường để `observeIdle` Rust dùng (`servers[].forgotten`, `idleSinceMs`, `daemon`).

STATUS: DONE
