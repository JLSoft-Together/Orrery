# WP-v87-5-S4 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S4.md`, dòng S4 plan `PLAN-v87-step5-core-store.md` (§3.5 mới). Không commit / push, không sửa `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`, không đụng file của S6. Không chạm app Orrery đang chạy: mọi collector test chạy cổng trống + thư mục tạm, `~/.claude` / `~/.codex` không đọc tới (root tạm qua `CLAUDE_CONFIG_DIR`). CPU lúc chạy trace 3–11 %.

## Đã làm

1. **Crate mới `apps/core/crates/core-monitor`** (lib + bin `monitor-script`, `monitor-serve`), port 1:1 của `monitor.ts` và file nó dùng, gọi thẳng `core-transcript` / `core-procs` / `core-store` (không pipe NDJSON):
   - `registry.rs` (`registry.ts`), `process.rs` (`procStartMatches`, `isPidAlive`, `queryProcesses` từ `core_procs::table()`), `shared.rs` (`mapRegistryStatus`, `mainAgentStatus`, `contextWindowOf`, `usablePct`, `autocompactBufferPct` của `packages/shared`), `hints.rs`.
   - `tracker.rs` `MonTracker` = `CoreTracker` bỏ pipe: giữ `core_transcript::tracker::Tracker`, trang 4 MiB tới `Step::Done`, view đầy đủ sau mỗi sync có đổi (`view.rs` = `trackerView.ts`), `spans.rs` = `spanMirror.ts`, safety net 4 s, hook `SensitiveCheck` cho S6.
   - `peers.rs`, `jobs.rs`, `job_stop.rs` (`claude stop`, max 20 s, `is_daemon_running`), `catalog.rs` (`model-catalog.ts` + `claude-settings.ts`), `gitstat.rs` (coalesce theo cwd + 1 lượt chờ, NOT_A_REPO 5 phút, thư mục mất 30 s, git chạy trên thread), `repo.rs` (`RepoRoots`), `sidefiles.rs` (hooks / status line qua `core_transcript::sidefiles`, `latest_limits`), `watch.rs` (crate `notify`), `usage.rs` (worker usage index trên `core_transcript::jobs`).
   - `monitor/{mod,deps,reconcile,extras,publish,actions}.rs` `Monitor`: reconcile 5 s, verifyLive + hints 60 s, TTL, dismiss, extras (jobs, limits, hooks, cost, context, day roll), peers, diff, alias / note / favorites / history / spans / timeline / diagnostics. Timer là mốc giờ: chủ gọi `next_due()` / `tick(&mut Store)`.
   - (a) **Đọc lại subagent khi xong**: `core_transcript::tracker::Tracker.reread_finished` (+ `about_to_resolve`), mặc định tắt (Node collector qua `CoreTracker` giữ nguyên), `core-monitor` bật. Ghi vào plan §3.5 là cải tiến có chủ đích. Test `tests/reread.rs`: bật → span `ok`, đủ token (37); tắt → `interrupted`, thiếu token (7).
   - (b) **`Arc` như Node**: `diff` giữ `Arc` khi `sameDiff`; `deniedTools` lấy `Arc` của bản ghi hook (bản ghi chỉ thay khi `at` / stale đổi); `limitHit` giữ `Arc` cũ khi giá trị mới bằng nhau. Twin kiểm: poll lại / đổi note / đổi alias không sinh patch `diff` / `deniedTools` / `limitHit` thừa (strict = 0).
2. **Harness**: `script.rs` + bin `monitor-script` (lệnh NDJSON, đồng hồ giả, không watcher); `serve/` + bin `monitor-serve --dev` (core-http + thread monitor + usage + RepoRoots + snooze + notify settings + sensitive mặc định; chỉ là harness, `orrery-core serve` là S10).
3. **Cổng parity**:
   - `apps/collector/test/monitor-twin.ts` + `monitor-twin.test.ts`: Node `Monitor` (SessionTracker + side file Node, watcher tắt, batch 1 h) và `monitor-script` chạy trên **cùng một root fixture**, 21 bước: kịch bản `monitor.test` / `trace-scenario` (tool result, `toolSpans.get` ×3, alias / note, subagent lồng + fail, subagent xong, rename / waiting, `/clear`, end + history / timeline, TTL) + (c) context hints (settings `opus[1m]` + model catalog), hook denied + compacting, status line limits / cost / context, **limit hit**, jobs blocked + stop, **git diff thật** (repo tạm), phiên thứ 2 với subagent foreground, favorites (toggle / add / exists / invalid / notFound), pid chết + dismiss. 2 lượt (không git / có git): `--strict` = 0 và fold = 0.
   - Demo 30×6: `scripts/trace-capture.mjs --server=both --rust-out=` chạy Node collector và `monitor-serve` **cùng lúc trên một root sống** (data dir, cổng, WS riêng); trace Rust mang `mask: ['diagnostics.data.node']` (`trace-compare.mjs --mask=type.field` mới).
   - (d) Lượt c thêm `--ops`: `trace-replay` (Store S1) trên chuỗi op của Node = 0 → lỗi store và lỗi monitor tách được (Store khớp, monitor khớp).
4. Test Node → Rust: `peers.test` → `tests/peers.rs` (8); `job-actions.test` phần Monitor → `tests/job_actions.rs` (5), phần runner → unit `job_stop.rs` (3); `extras.test` phần jobs / model catalog / status line → unit `jobs.rs` / `catalog.rs` / `sidefiles.rs`; `context.test` → unit `shared.rs`; `gitstat.test` → unit `gitstat.rs`; `monitor.test` + `tracker-parity` (qua tracker trong monitor) → twin.
5. Docs: plan §3.5 + đánh dấu dòng S4, `.code_index/core.md` (core-monitor), `collector.md` (twin + `--server`), `_meta.md`.

## File (khớp `git status`)

- Mới: `apps/core/crates/core-monitor/` (`Cargo.toml`; `src/{lib,shared,hints,registry,process,view,spans,tracker,peers,jobs,job_stop,catalog,gitstat,repo,sidefiles,watch,usage,script}.rs`, `src/monitor/{mod,deps,reconcile,extras,publish,actions}.rs`, `src/serve/{mod,dispatch}.rs`, `src/bin/{monitor-script,monitor-serve}.rs`, `tests/{peers,job_actions,reread}.rs`), `apps/collector/test/monitor-twin.ts`, `apps/collector/test/monitor-twin.test.ts`, báo cáo này.
- Sửa: `apps/core/Cargo.toml` (thêm `"crates/core-monitor"`; `core-sources` là của S6), `apps/core/Cargo.lock` (thêm `core-monitor` + `notify` 8.2 và dep của nó, resolve `--offline` từ cache; phần `core-sources` là của S6), `apps/core/crates/core-transcript/src/tracker/{mod,subs}.rs` (+7/−1), `scripts/trace-capture.mjs` (`--server`, `--rust-out`, `--rust-bin`), `scripts/trace-compare.mjs` (`--mask`, `trace.mask`), `.planning/PLAN-v87-step5-core-store.md`, `.code_index/{core,collector,_meta}.md`.
- Không phải của S4: `.planning/PROGRESS-2026-11-09-v87.md`, `RELEASE-v2.3.0.md`, `wp/WP-v87-5-S6*`, `core-sources/`, `sources-cross.test.ts`, `docs/qa/*`.
- Dep crate mới: `notify` 8.2 (watch thư mục sessions / projects đệ quy trên Windows `ReadDirectoryChangesW` + macOS FSEvents, crate watch phổ biến nhất, có sẵn trong cache cargo). Còn lại đã có trong lock. Mọi file ≤ 346 dòng, không comment.

## Test (số thật)

- `cargo test --locked -j 4` (apps/core, cả workspace, gồm crate của S6): **359 passed, 0 failed, 6 ignored**; `core-monitor` 38 = unit 23 + peers 8 + job_actions 5 + reread 2; `core-transcript` 52 passed + 2 ignored (không đổi).
- `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/collector -- --maxWorkers=4`: **49 file passed + 1 skipped, 526 passed, 3 skipped** (gồm 2 test twin; `trace-golden`, `trace-replay`, `persist-cross` vẫn pass).
- `npm run typecheck -w @ccm/collector`: exit 0.

## Kết quả parity

| Kịch bản | Frame Node / Rust | fold | `--strict` |
|---|---|---|---|
| twin `monitor` (không git), 21 bước | bằng nhau từng bước | 0 | 0 |
| twin `monitor` + git diff thật | bằng nhau từng bước | 0 | 0 |
| demo 30×6 lượt a | 827 / 640 | 0 | không dùng |
| demo 30×6 lượt b | 866 / 667 | 0 | không dùng |
| demo 30×6 lượt c (+ `--ops`) | 844 / 652 | 0 | không dùng |
| lượt c: Node WS ↔ `store-replay` (S1) | 844 / 844 | 0 | 0 |

Thêm: Node a ↔ Node b = 0. Demo Rust ít frame hơn vì 2 process gộp batch 50 ms ở thời điểm khác nhau; `--strict` trên demo lệch ngay snapshot boot (Node đã có `usage.roots`, Rust chưa) vì 2 process đọc file ở thời điểm khác nhau — đúng giới hạn plan §3.1 (strict chỉ cho kịch bản trong process).

## Khác biệt biên (cố ý / đã biết)

- `query_processes` chỉ dùng `core_procs::table()`; không còn fallback CIM / `ps` của Node (bảng không đọc được giờ tạo → coi như không xác minh được, như khi CIM lỗi).
- `diagnostics.node` = chuỗi rỗng (không có Node); trace Rust che field này. `codex` / `deepseek` trong diagnostics chưa có (S5).
- Launch / focus / open folder / pick folder là hook `Deps` trả `None` = `unsupported` / không làm gì (S8 nối RPC `host.*`). `focusing` hàng đợi và `picking` busy chưa cần vì hook đồng bộ.
- Publish sau sync chỉ khi tracker đổi (`takeChanged` như `CoreTracker`); Node SessionTracker publish mọi lần — output bằng nhau (store bỏ upsert giống hệt).
- `scheduleDiff` đọc cwd lúc chạy git, Node lúc lên lịch (chỉ khác khi cwd đổi trong 2 s).
- Registry đọc theo thứ tự thư mục (Node: thứ tự `Promise.all` xong); chỉ ảnh hưởng thứ tự `terminal.created` khi nhiều phiên cùng lượt.
- `monitor-serve` giữ lock store trong lúc reconcile đọc transcript (Node cũng đơn luồng); git / `claude stop` chạy thread riêng.
- Sensitive: activity `sensitive` cần `SensitiveCheck` (S6 cắm `core-sources`); hiện `touch` bị bỏ. Snapshot dùng settings mặc định (fixture demo không có thao tác nhạy cảm).
- Twin: cả 2 phía tắt watcher (Node vẫn bật `fs.watch` trong `reconcileOnce` dù `watch: false` — test vá `setupWatchers` để không có sync lạc giữa bước).
- `spawnDepth` lẻ bị cắt thành số nguyên (kiểu `u64` của `core-store` từ S0).

## Việc cho WP sau

- S5: `Monitor` đã có `claude_targets()`; nối `CodexMonitor` / `DeepseekMonitor` vào `dismiss_ended`, `open_folder`, `focus_window`, diagnostics, giữ agent `provider: codex` khi publish (đã lọc như Node).
- S6: implement `SensitiveCheck` bằng `core_sources::sensitive` và gán `monitor.sensitive`; `SensitiveWatch` thay `default_sensitive()` trong `serve/`; sampler dùng `last_device_call_at()` / `port_conflicts()`.
- S7: toast cần `ask_of` / `reply_of` / store; `serve/after_tick` là chỗ gọi observe (Node gọi trong listener flush).
- S8: hiện thực `Deps::launch|focus_window|open_folder|pick_folder` qua RPC `host.*`.
- S10: `orrery-core serve` lấy `serve/` làm khung (đã có `listening`, stdin shutdown / EOF, exit 3); thêm `busy` / `resources` ra stdout, `CCM_CORE=rust|node`.
- `reread_finished` có thể bật luôn cho `CoreTracker` của Node khi bỏ bản Node (S12).

STATUS: DONE
