## WP-v87-4-T3 — PARTIAL (code + parity + test xong; cổng CPU collector ≤ 7.6 % chỉ đạt 1/4 lượt, tổng collector + core không giảm)

- Đã đổi:
  - Rust `apps/core/crates/core-transcript/src/tracker/` mới: `mod.rs` (241, `Tracker` + `syncOnce` chạy tiếp được theo trang: Main → discover → Subs → resolve + prune), `read.rs` (214, xử lý từng record như `applyMain` / `readSubagent`, lite filter đọc tập agent tool-use qua `RefCell`), `subs.rs` (223, `cwd_slug`, `find_transcript`, meta, `isLiteCatchUp`, `resolve`), `view.rs` (104, view delta), `serve.rs` (111, op `tracker.open|sync|closeSpans|close`), `tests.rs` (151). `serve.rs` (crate) thêm `tracker_op` trước `signal_op`; `lib.rs` khai báo; `spans.rs` thêm `ToolSpanLog::with_drops` / `take_dropped` (tắt cho `signals.*`).
  - Collector (file mới): `trackerSource.ts` (78, interface `Tracker`, `trackerMode`, `createCoreHub`, `createTracker`), `coreTracker.ts` (~270, `CoreTracker`), `trackerView.ts` (~220, view + builder copy 1:1 từ `tracker.ts` + `mergeView`), `spanMirror.ts` (66). `monitor.ts`: đúng 2 dòng (import + `new SessionTracker` → `createTracker`), không dài thêm. `tracker.ts` không đụng.
  - Test mới: `test/tracker-twin.ts`, `tracker-parity.test.ts`, `tracker-parity-corpus.ts`, `tracker-parity-demo.test.ts`.
  - Docs: plan §2.3 (protocol + quyết định + khác biệt biết trước), `.code_index/{core,collector,_meta}.md`, `docs/perf/v87-step4-T3-win32.json`.
- Tiêu chí 4/5:
  - [x] Parity TS ↔ Rust: `Twin` chạy SessionTracker + CoreTracker trên cùng file, sau MỖI sync so activity, terminal, agents, toolSpans, codexCalls, device / port, lastReply, pendingAsk (+ drainSpans / drainPeers / closeOpenSpans / setRecord). Kịch bản `tracker.test.ts` (message counts, live, coalesce, subagent, spans, session switch, real-shaped catch-up, span cap, skip-resolved, targeted sync) + sensitive / hints context / reply / AskUserQuestion / limit / port / adb / compact / mode. Demo 30×6 seed 8704 (temp dir) phát lại 20 bước, meta xuất hiện khi Agent call xuất hiện, nửa số session catch-up muộn với `now` +3 h: 30 session, 918 subagent, 203 lite catch-up, 7 059 activity live, 510 sync — khớp hết. Snapshot `tracker.test.ts` không đổi.
  - [x] Cờ: mặc định `node` → `SessionTracker` y như cũ (monitor tests pass); `rust` không có `CCM_CORE_BIN` → TS + warn 1 lần; reply null → mở lại key mới + catch-up im lặng; mở lại hỏng (core không chạy được / restart > 3 lần / phút) → `SessionTracker` TS nội bộ (test: lost reply, core missing).
  - [x] Redaction: activity chỉ kind / id / tên tool / `touch` (= `ToolTouch` TS, seam bóc ra) / agentType + description 160 / số compaction / mode; test Rust kiểm không có `SECRET` trong activity + view; module tracker không `eprintln`.
  - [~] Bảng đo: có (dưới). Cổng ≤ 7.6 % collector: 7.0 % (đạt) / 8.3 % (không) ở vòng code cuối; cộng process con `orrery-core transcript` thì ngang Node.
  - [x] Test + clippy + typecheck sạch; index + plan §2.3.
- Audit / Review:
  - Không comment trong code mới (rg `//`, `/*` = 0); không log nội dung dòng; file mới đều < 400 dòng; chỉ dùng thư mục tạm (`ccm-t3-*`), không chạm port 4317 / `%APPDATA%\Agent Monitor` / `%LOCALAPPDATA%\ccm`; không còn `orrery-core` sót.
  - Lệch ranh giới (b) (ghi plan §2.3): Rust dừng ở state theo agent + activity / spans / peers; `TerminalSession` / `Agent[]` vẫn dựng ở TS (`trackerView.ts`) vì `monitor.ts` gọi `buildTerminal` / `buildAgents` / `toolSpans` / `closeOpenSpans` đồng bộ giữa các sync, với record / hints (status line, alias) Rust không có.
  - Không chặn `ping` / `usage.query`: phân trang `maxBytes` 4 MiB trên main thread (pages ghép lại = 1 `syncOnce`, test Rust `pages_add_up_to_one_sync` + op test).
  - Khác biệt biết trước: activity của đúng lần sync bị mất reply (trước khi mở lại) mất; TS mirror prune lúc `toolSpans()` còn Rust prune lúc sync (span cùng id thấy lại sau khi chỉ TS prune: TS thêm lại, Rust bỏ qua); meta JSON sâu > 128 / số vượt f64 → `{}` ở Rust. Rust state của tracker đã bị monitor bỏ được đóng qua `FinalizationRegistry` (không đảm bảo thời điểm GC).
  - Flaky không thuộc T3: `core-procs.test.ts` "does not carry a half line…" (timeout 400 ms) fail 1 lần khi máy tải nặng; đã giảm tải test demo (duration 2 → 1, 65 s → 35 s), 2 lượt full suite sau đó pass.
- Test (số thật):
  - `cargo test --locked -j 4` (apps/core, PowerShell): toàn workspace pass — core-transcript 35 pass + 2 ignored (tracker 5 mới), core-bin transcript 4, flow 8 (+2), procs 3 (+1), pty 16, spike 1, stub 4, core-procs 19, core-pty 35.
  - `cargo clippy --locked -j 4 --all-targets -- -D warnings` Windows + `--target aarch64-apple-darwin` → sạch.
  - `npm test -w @ccm/collector -- --maxWorkers=4` → 39 file pass / 1 skip; 458 pass / 3 skip / 0 fail (2 lượt liên tiếp). Parity mới: 15 + 1 test.
  - `npm run typecheck -w @ccm/collector` → sạch.
- Đo (Windows, `perf-bench --stage --scenario demo-30x6 --seed 8704`, 120 s sau 20 s settle, core release, % một core; usage vẫn Node):

| Vòng | Mode | Collector avg / p95 | Core transcript (con) | Collector + core | Collector RSS |
|---|---|---|---|---|---|
| A (view đầy đủ mỗi sync) | node 1 / 2 | 10.1 / 16.0 · 12.3 / 19.6 | — | 10.1 · 12.3 | 110 · 109 MB |
| A | rust 1 / 2 | 7.7 / 12.7 · 7.9 / 12.8 | 3.7 · 3.4 (7 MB) | 11.4 · 11.3 | 102 · 108 MB |
| B (view delta, code cuối) | node 1 / 2 | 10.5 / 18.3 · 10.2 / 16.1 | — | 10.5 · 10.2 | 118 · 110 MB |
| B | rust 1 / 2 | 7.0 / 11.6 · 8.3 / 16.9 | 2.9 · 2.9 (6 MB) | 9.9 · 11.2 | 105 · 97 MB |

  - Collector −25 % (10.4 → 7.7 trung bình), RSS −8 MB; nhưng đọc + parse chuyển sang process con nên tổng gần như không đổi. CPU tổng app dao động mạnh theo renderer / gpu (máy có tải khác) nên không dùng để so.
  - Phần còn lại trong collector (profile rust mode, `--collector-prof` 30 s, ms/s): đọc + `JSON.parse` reply core 1.8 + 0.4, `store.diff` 1.5, `buildTerminalOf` 1.2, `spawn` 0.9, `publish` 0.7, `routeProjectChange` 0.7, usage `ingestLine` 0.6 (usage vẫn Node), `upsertAgent` 0.5. Trong core con (~29 ms/s ≈ 25 sync/s): mở / stat file mỗi lần đọc, `serde_json::Value` cả dòng (T2 đã thấy), serialize reply.
  - Hướng để qua cổng: bật `CCM_CORE_USAGE=rust` cùng lúc (bỏ `ingestLine` + `notify` khỏi collector), parse typed bỏ body như `record.rs` T1, gửi activity / view gọn hơn (ít JSON hơn mỗi sync), và giảm `publish` / `store.diff` (WP-H) — phần này ngoài T3.
- Ngoài phạm vi / cần main xử lý:
  - Không đụng `apps/desktop` / `scripts`: perf-bench truyền `process.env` (đã có `CCM_CORE_TRANSCRIPT`), main tự đặt `CCM_CORE_BIN`.
  - Working tree có thay đổi không phải của T3: `apps/desktop/test/__snapshots__/fwp-host.test.ts.snap`, `.planning/RELEASE-v2.3.0.md`, `docs/qa/...png` — không đụng.
  - `apps/core/target/release/orrery-core.exe` đã build lại (có op `tracker.*`); stage `apps/desktop/stage` build lại bằng `node apps/desktop/build.mjs`.
- Commit message đề xuất: `✨ v87 step 4 T3: SessionTracker in Rust (core-transcript tracker, orrery-core transcript tracker.open/sync/closeSpans/close with paged sync and delta view) behind CCM_CORE_TRANSCRIPT=rust (default node, Node fallback), collector seam trackerSource/coreTracker/trackerView/spanMirror, Node/Rust tracker parity incl. seeded demo 30x6, perf v87-step4-T3`

### Vòng 2 (review opus: 3 med + 2 low, sửa hết)
- (1 med) Span sau phục hồi: `CoreTracker` chụp `mirror.all()` trước khi mở lại; sau catch-up (Rust) hoặc `toNode()` không fresh (so với `fallback.toolSpans()`), `queueRecovered` đẩy vào `pendingSpans` span mới hoặc đổi `end` / `outcome` → store không kẹt span mở. `SpanMirror.all()` mới. Test: `tracker-parity.test.ts` "reopens…" so `drainSpans` với TS; `tracker-recovery.test.ts` "queues the spans a Node fallback finds…".
- (2 med) Mọi request `tracker.*` dùng `{timeoutMs: 30 s, restartOnTimeout: false}`: timeout / null chỉ làm tracker đó mở lại, mở lại hỏng thì chỉ tracker đó về Node; core chung không bị giết. Test "never restarts the shared core…" (tracker A hỏng → Node, B vẫn Rust, mọi request tracker có `restartOnTimeout: false`).
- (3 med) `Tracker.dispose?()`: `CoreTracker.dispose` gửi `tracker.close` + unregister, sync sau đó trả `[]`; `monitor.ts` `forget()` thêm 1 dòng `tracker.dispose?.()` (monitor tổng +1 dòng so với gốc). `FinalizationRegistry` chỉ dự phòng. Key `t<n>` giờ duy nhất theo process (trước đây theo hub — test nhiều hub trên 1 core bị trùng key). Test "closes its Rust tracker on dispose".
- (4 low) `run` trả `{out, ok}`: page lỗi giữa chừng giữ drafts của các page trước, rồi mới mở lại + catch-up. Test "keeps the activity of pages read before a failed page" (page 1500 byte, page 3 lỗi, activity Rust = phần đầu của TS, view / spans khớp sau phục hồi).
- (5 low) Rust: `Run.sig` = chữ ký args (trừ `now`); `tracker.sync` có args khác run đang dở → bỏ run cũ, chạy mới (args hỏng khi đang chạy → tiếp run cũ). Test Rust `new_args_drop_a_pending_run`. Plan §2.3: tiếp trang phải gửi lại đúng args.
- Không đổi đường nóng (chỉ đường lỗi + so chữ ký mỗi sync) → không đo lại perf.
- Test vòng 2: `cargo test --locked -j 4` toàn workspace pass (core-transcript 36 + 2 ignored, tracker 6); clippy `-D warnings` Windows + aarch64-apple-darwin sạch; `npm test -w @ccm/collector -- --maxWorkers=4` 3 lượt: lượt 1 có 1 fail ở `core-procs.test.ts` "does not carry a half line…" (timeout 400 ms, flaky có từ trước, không thuộc T3; chạy riêng 3/3 pass), lượt 2 + 3: 40 file pass / 1 skip, 462 pass / 3 skip / 0 fail; typecheck sạch. Không comment mới, không `orrery-core` / thư mục tạm sót.
