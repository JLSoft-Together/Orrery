# PLAN v87 bước 5 — `core-store` + `core-sources`, bỏ Node collector

Ngày 2026-11-10. Con của `PLAN-v87-rust-core.md` §6 bước 5. Nối sau `PLAN-v87-step4-core-transcript.md` (usage, tracker, codex rollout, side files đã Rust, mặc định `rust`).
Bước 6 (vỏ Tauri không Node sidecar) chỉ quyết sau số đo của bước này.

## 1. Hiện trạng — `apps/collector/src` = 14 592 dòng, test 9 820 dòng / 51 file

Chạy trong utilityProcess `collector.cjs` (host: `apps/desktop/src/collector-host.ts`, 142 dòng; `main.ts` ~1440 `new CollectorHost`). Cổng cố định `127.0.0.1:4317` (origin của localStorage renderer: **không đổi**), header `x-ccm-app`, `/health` không cần key, exit 3 = port in use, tắt qua `parentPort 'shutdown'`.

| Nhóm | File (dòng) | Vai trò / phụ thuộc ngoài | Rust? |
|---|---|---|---|
| Entry / config | `index.ts` 403, `config.ts` 75, `datadir` 8, `json-file` 56 | boot, dispatch client msg, `parentPort` (busy, resources, phone, telegram, core-fallback; nhận shutdown, deviceUsed, diskAlert, agent.note) | chưa |
| Store + server | `store.ts` 475, `server.ts` 446 | diff / coalesce / batch 50 ms / seq; HTTP `/health` `/snooze?` `/focus` `/api/timeline/*` + static web dist; WS `/ws` (heartbeat 30 s, 128 KB) | chưa |
| Monitor | `monitor.ts` 942, `registry` 112, `process` 186, `repo` 69, `peers` 178, `jobs` 128, `job-stop` 79, `model-catalog` 90, `claude-settings` 14, `gitstat` 92 (spawn git), `daystart` 26 | fs.watch registry + projects, reconcile 5 s, verifyLive 60 s, extras poll, spawn `claude stop`, spawn git | chưa |
| Stores nhỏ | `aliases` 70, `favorites` 142, `history` 60, `snooze` 68, `response` 103, `timeline` 124, `forecast` 145 | file JSON trong `CCM_DATA_DIR` (aliases, notes, snooze, history, timeline, limit-history…) | chưa |
| Seam đã có Rust | `tracker` 685 + `trackerSource` 85 + `coreTracker` 341 + `trackerView` 248 + `spanMirror` 70; `transcript` 401, `tailer` 134, `tool-spans` 84; `usage` 675 + `usageSource` 186 + `coreTranscript` 215; `hooks` 85, `statusline` 102, `sideFileSource` 49, `nodeSideFiles` 26, `coreSideFiles` 134; `coreProcs` 297, `procSource` 65, `coreFallback` 24; `codex/{coreRollout,rolloutSource,nodeRollout,rollout,reader}` | chỉ còn là **seam + bản Node dự phòng**: xoá ở S11, thay bằng gọi trực tiếp crate | Rust xong |
| Codex | `codex/monitor` 687, `processes` 310, `link` 157 | watch `sessions/`, link Claude agent, `LimitHistory` | rollout Rust; monitor chưa |
| DeepSeek | `deepseek/monitor` 456, `outbox` 241, `processes` 120 | tail outbox, match process (đang tắt: `CCM_DEEPSEEK_FEATURE`) | outbox Rust; monitor chưa |
| Resources | `resources/{sampler 290, procs 209, ports 230, tree 234, idle 41, fixture 69}`, `ps-host` 125 | sampler 15 s, report → main; bảng procs/ports từ core | bảng Rust; sampler chưa |
| Sensitive / search | `sensitive/rules` 302 + `watch` 132, `prompt-search` 350 | luật lệnh nguy hiểm (tracker đang gọi bản Node), đọc prompt cũ (chủ nhân đã duyệt) | chưa |
| OS actions | `focus` 131, `focus-mac` 93, `osascript` 26, `opener` 88, `launcher` 90, `folder-pick` 128, `win-exe` 41 | PowerShell / osascript / wt.exe, dialog chọn folder | chưa |
| Toast | `toast` 869, `toast-electron` 17 | waiting / done / limit / stuck / context / disk / note, quiet hours, snooze, phone + Telegram episode (qua parentPort) | chưa |
| Office (v60) | `office/{hub 110, model 315, runtime 67, locale 6}` + `@ccm/habblaud` timeline recorder / handler | chỉ chạy khi client gửi `office.enable`; ghi `timeline/<day>.jsonl`, `office-names.json` | chưa |

## 2. Ranh giới đích

- Subcommand mới `orrery-core serve` (crate `core-store` = state + diff + batch + persistence; `core-sources` = monitor, registry, codex, deepseek, resources, sensitive, prompt-search, toast logic; `core-http` = HTTP + WS + static). Gọi **trực tiếp** `core-procs` / `core-transcript` như thư viện, bỏ NDJSON pipe + `FinalizationRegistry` + seam.
- **Giữ nguyên**: protocol WS (`packages/shared` `MonitorEvent` / `ClientMessage`), mọi HTTP route, `x-ccm-app`, `/health`, cổng 4317, exit 3, env `CCM_*`, format file trong `CCM_DATA_DIR` (Node đọc được file Rust và ngược lại). Web + renderer không sửa.
- Kênh điều khiển main ↔ `serve` qua stdin / stdout NDJSON (thay `parentPort`): vào = `shutdown`, `deviceUsed`, `diskAlert`, `agentNote`; ra = `busy`, `resources`, `phone`, `telegram`, `core-fallback`, và RPC `host.*`.
- **Chuyển sang desktop main (TS) thay vì Rust**: `focus` / `opener` / `launcher` / `folder-pick` (qua RPC `host.focus|open|launch|pickFolder`; Electron có sẵn `dialog` / `shell`, tránh viết lại PowerShell / osascript bằng Rust). Hiển thị toast Windows (`ps-host`) + `Notification` + phone + Telegram vẫn ở main; Rust chỉ quyết định episode và gửi ra.
- PTY: giữ process `orrery-core` (PTY) riêng, không gộp vào `serve` (panic hay treo ở store không giết pane; core PTY có job `KILL_ON_JOB_CLOSE`).

## 3. Work package (thứ tự phụ thuộc)

| WP | Việc | File chính | Model | Cổng parity |
|---|---|---|---|---|
| S0 | Hợp đồng + harness: thu golden trace WS (snapshot + events + direct reply) từ collector Node với `demo-fixture` 30×6, fixture thật (27 MB + 41 sub), kịch bản `monitor.test`; script so 2 trace (bỏ `seq` / thời gian); sinh type Rust từ `packages/shared` bằng `ts-rs`, test khớp | `apps/core/crates/core-store`, `scripts/trace-compare.mjs`, `test/` | opus (hợp đồng) + sonnet | so trace tự chạy được trên Node vs Node = 0 khác biệt |
| S1 | `core-store`: maps terminal / agent / link, activity + toolActivity ring, diff → patch, coalesce, batch 50 ms, seq, mọi setter (usage, jobs, limits, favorites, notify, snooze, peerLinks, loading, contextPct, projectRoots) | `store.ts` | opus | test `store` / `monitor.test` phần store chuyển thành Rust; coalesce + `null` xoá field khớp |
| S2 | `core-http`: bind 127.0.0.1, `hasAppKey` constant-time, origin check, WS heartbeat / `maxPayload`, validate client msg, `/health`, `/snooze`, `/focus`, static web dist + `cacheControlFor` + CSP, stdin shutdown, `port in use` exit 3 | `server.ts`, `hardening.test`, `csp.test` | sonnet | `hardening` + `csp` chuyển thành test Rust; snapshot đầu khớp golden — xong 2026-11-10, xem §3.3 |
| S3 | Stores nhỏ + `JsonFile` (tmp + rename, debounce): aliases / notes, favorites (rule path), history, snooze, StatusTimeline, LimitHistory + forecast, ResponseTracker, daystart, `notify.json` | 7 file ở mục 1 | sonnet | Node ghi → Rust đọc và ngược lại, deep-equal; `favorites` / `daystart` / `datadir` test |
| S4 | Registry + Monitor: `notify` watch registry + projects, reconcile 5 s, verifyLive, TTL, dismiss, extras poll (jobs, limits, hooks, cost, day roll), RepoRoots, gitstat (coalesce, NOT_A_REPO 5 min), peers, `claude stop`, model catalog; nối thẳng core-procs / core-transcript | `monitor.ts` + 8 file | opus | `monitor.test`, `tracker-parity`, `peers`, `job-actions`, `extras`, `context` chuyển sang test Twin Rust vs Node trên cùng fixture |
| S5 | Codex monitor + link + processes; DeepSeek monitor + processes + outbox watcher | `codex/`, `deepseek/` | sonnet | `codex-parity`, `codex.test`, `deepseek.test`; link Claude↔Codex đúng |
| S6 | Resources sampler + tree + idle + `busy` / `resources` ra main; Sensitive rules + watch (bỏ bản Node đang gọi từ `coreTracker`); prompt-search | `resources/`, `sensitive/`, `prompt-search.ts` | sonnet | `resources.test`, `hints.sensitive`, `prompt-search` fixture; CPU sampler không tăng |
| S7 | Toast logic (episode, quiet hours, snooze, forecast / limit / stuck / context / disk / note, Telegram `reply_text`, phone episode) trong Rust; render vẫn ở main | `toast.ts` | opus | `toast.test` (513 dòng) chuyển 1:1, cùng chuỗi episode trên cùng chuỗi event |
| S8 | RPC `host.*` giữa `serve` và main + đổi `focus` / `opener` / `launcher` / `folder-pick` thành handler main | `main.ts`, `host-rpc.ts` (mới) | sonnet | `focus-mac` test; thủ công Win + mac: focus, mở folder, launch, chọn folder |
| S9 | Office: model + hub + runtime (refcount `office.enable`) + TimelineRecorder + `/api/timeline/*`, `office-names.json`, retention 7 ngày | `office/`, habblaud server | opus | `office-model` + `office-hub` chuyển Rust; `timeline/*.jsonl` Node↔Rust đọc chéo; không ghi gì khi chưa ai bật |
| S10 | Supervisor + build: `CoreServeHost` (spawn binary, stdin / stdout, log, restart max 3, boot timeout 30 s, exit 3) thay `CollectorHost`; cờ `CCM_CORE=rust|node`; boot fail → fallback Node + thông báo 1 lần; `collectorEnv`; `build.mjs` / `electron-builder.yml` / CI | `collector-host.ts`, `main.ts`, `build.mjs` | sonnet | e2e khởi động / tắt, port clash, kill main không để mồ côi |
| S11 | Nghiệm thu (Codex chạy đo / test): parity cả bộ, E2E Win + mac, RAM / CPU; đổi mặc định `CCM_CORE=rust`; ghi `.code_index` + `docs/` | `scripts/`, `docs/perf/` | Codex (opus duyệt) | mục 4 |
| S12 | (Bước 6, Tauri) xoá `collector.cjs`, seam + bản Node, test Node thừa | nhiều file | sonnet | bỏ khi 1 release không ai dùng `CCM_CORE=node` |

Song song được: S3 // S5 // S6 sau S1; S7 sau S4; S9 sau S2 + S4. Không chạy 2 tool nặng cùng lúc, `cargo -j <= 6` (`JIN_MAX_THREADS`).

### 3.1 Harness so trace (S0, dùng cho S1–S11)

**Thu trace WS thật** — `node scripts/trace-capture.mjs --out=<file>.trace.json` (mặc định `--sessions=30 --subagents=6 --seed=8704 --steps=10`):

- Tạo `demo-fixture` seed vào thư mục tạm, rồi phát lại theo 10 bước vào một Claude root tạm. Mỗi session: subagent jsonl → chờ 250 ms (`--split`, để subagent được đọc trước task-notification của main) → main jsonl → meta mới + jsonl rỗng (như Claude tạo lúc spawn), cách session sau 40 ms (`--gap`). Hết lượt ghi, chờ 500 ms rồi `utimes` lại đúng các file vừa ghi theo cùng thứ tự (`--nudge`): watch Windows có lúc rơi event, lượt chạm này cho collector đọc nốt — nếu event đầu đã tới thì lượt chạm không sinh event nào. Session chẵn có registry từ đầu, session lẻ vào ở bước 5. Pid trong registry = process giữ chỗ (`PING.EXE -n 86400` / `sleep`) do script tự spawn và tự kill.
- Chạy collector `node --import tsx apps/collector/src/index.ts --dev` với `CCM_PORT` trống (không bao giờ 4317), `CCM_DATA_DIR` / `CODEX_HOME` / `CODEWHALE_HOME` tạm, `CCM_CODEX=0`, `CCM_DEEPSEEK=0`, `CCM_TOAST=0`, `CCM_VERIFY_PROCESSES=0`, `CCM_RES_FIXTURE` bảng rỗng, mọi `CCM_CORE_*=node` (`--core=inherit` giữ env hiện tại). Bản Rust (S10) sẽ thêm `--server=rust` chạy `orrery-core serve` cùng env.
- Bước: `boot` → `snapshot-boot` (gửi `resync`) → `replay-2..10` → `client` (alias, note, snooze, `favorite.toggle`, `favorite.add`, alias id sai) → `end` (xoá registry + kill 3 session) → `queries` (`history.get`, `timeline.get`, `toolSpans.get` ×2, `diagnostics.get`, `toolstats.get`, `heatmap.get`) → `dismiss` → `final` (`resync`). Mỗi bước chờ ít nhất 5,7 s (≥ 1 lượt reconcile 5 s, `--stepMin`) rồi tới khi 800 ms không có message, không `loading`, không `scanning` (`--quiet`, max 60 s).
- File `ccm-trace/1`: `wall` (start / end), `pids` (pid giữ chỗ → `p<i>`), `replace` (đường tạm → `<root>` / `<data>` / `<tmp>`), `ignore` (`resources`), `steps[] { name, sent[], frames[] }` (frame = `ServerMessage` nguyên văn, kể cả `direct`).

**So 2 trace** — `node scripts/trace-compare.mjs a.trace.json b.trace.json [--strict] [--limit=N] [--ignore=type,...]`, exit 0 khi 0 khác biệt. Quy tắc chuẩn hoá:

1. Bỏ `seq`, ranh giới batch và thứ tự key (so bằng JSON canonical, key sort).
2. Thời gian thật (thêm: key trong `durations` của trace — `workingMs`, `waitingMs`, `totalMs`, `medianMs`, đo bằng đồng hồ thật — → `<ms>`): chuỗi ISO và số epoch ms nằm trong `[wall.start − 10 phút, wall.end + 10 phút]` → `<now>`; ngày `YYYY-MM-DD` của khoảng đó → `<today>`. Thời gian logic của fixture (2027) và thời gian giả trong test giữ nguyên.
3. Activity bỏ `id` (`Date.now` base36 + bộ đếm).
4. Pid giữ chỗ → alias (`processId`, `parentProcessId`, `pid`, và tiền tố `<pid>:` trong mọi chuỗi: terminal id, agent id `…:main`, link id). Peer link xoay lại `a < b` sau alias (đổi `ab` / `ba` / `lastDir`) vì Node chọn chiều theo thứ tự chuỗi id có pid.
5. Đường tạm theo `replace` (cả `\` lẫn `/`).
6. Khi báo khác biệt, mảng có `id` so theo id, mảng activity so multiset. Mảng object có `id` / `key` (terminals, agents, links, buckets, history, spans, lanes, jobs) và activity → sắp xếp; mảng khác (favorites, deniedTools, segments) giữ thứ tự.
7. Mỗi bước so: `state` (fold như client: terminals / agents / links theo created-updated(null = xoá)-removed, usage buckets + roots + scan, favorites, jobs, limits, codexLimits, response, modelNames, notify, snoozes, loading, sensitive), `fired` (multiset: activity, tool.span, history.added, favorite.rejected, …), `direct` (multiset), `snapshots` (mọi snapshot trừ snapshot boot đầu tiên, vì lúc nối WS collector còn đang load). `--strict` so thêm danh sách event phẳng theo thứ tự (chỉ dùng cho kịch bản trong process, thời gian giả).
8. `ignore` (trace + `--ignore`): bỏ event đó và key cùng tên trong snapshot; mặc định `resources` (sampler 30 s, thời điểm không cố định — S6 so riêng).

**Kịch bản trong process** — `npm test -w @ccm/collector -- test/trace-golden.test.ts`: kịch bản kiểu `monitor.test` (fixture 1 session + subagent, tool result, `toolSpans.get`, alias / note, subagent xong, subagent lồng + fail, rename / waiting / shell / `/clear` / end / TTL) chạy 2 lần → `--strict` và fold = 0 khác biệt; `CCM_TRACE_OUT=<dir>` ghi `monitor-a|b.trace.json`.

**Mẫu cho Rust** — `node scripts/trace-samples.mjs <trace...> --out=apps/core/crates/core-store/tests/golden/<name>.json [--per-type=4]`: mỗi loại event lấy message nhỏ nhất có thêm đường field mới (pid → 10000+i, đường tạm thay như trên); `cargo test --locked -j 4 -p core-store` round-trip từng message.

**Kết quả S0 (2026-11-10, Windows, CPU 7–36 % lúc chạy)**: 4 lượt thu (mỗi lượt ~175–193 s, 816–865 frame, ~23 100 event) → so a↔b = 0, c↔d = 0, a↔d = 0 khác biệt. Trước khi có `--split` / `--nudge` cùng một Node collector cho 2 trace lệch (span `interrupted` ↔ `ok`, subagent thiếu dòng cuối, usage thiếu 4 message) — đó là hành vi thật của Node, không phải lỗi harness; Rust (S4) nên đọc lại file khi subagent xong để không phụ thuộc thứ tự này, và khi đó trace Rust có thể "đúng hơn" Node trên kịch bản không có `--split`.

**Giới hạn đã biết**: Node collector tự nó phụ thuộc thời điểm đọc file (watch Windows rơi event, subagent xong trước khi jsonl được đọc → span `interrupted` thay vì `ok`, agent thiếu token). Harness giảm bằng thứ tự ghi + giãn cách ở trên; máy bận (CPU > 80 %) làm lệch lại — chỉ chạy khi máy rảnh. Fixture thật (27 MB + 41 sub) chưa có trong S0: dùng `--keep` + chép `projects/` vào nguồn khi cần (việc cho S4).

### 3.2 S1 — `Store` Rust + replay (2026-11-10)

- `core-store` `Store` = `store.ts` 1:1 (`src/store/{mod,settings,batch}.rs`). Diff bắt chước `===`: field scalar so theo giá trị, field object (`deniedTools`, `diff`, `limitHit` của terminal) là `Arc` so bằng `Arc::ptr_eq` — monitor Rust (S4) muốn không sinh patch thì phải giữ lại đúng `Arc` cũ như Node giữ tham chiếu. Batch không có timer bên trong: `emit` đặt `flush_at = clock + batch_ms`, chủ (S2, tokio) gọi `next_flush_at()` / `poll(now)`.
- **Ghi chuỗi setter từ Node** (không sửa `store.ts`): `scripts/trace-store-record.mjs` bọc method của `MonitorStore` (instance trong test, prototype trong collector qua `--import scripts/trace-store-hook.mjs` + `CCM_TRACE_OPS`). Op `ccm-store-ops/1`: `init` (startedAt, activityLimit, batchMs), `step` (tên + `sent`), `set` (`statusLineCommand` / `dayStartedAt`), `call` (tên + args JSON, `refs` = id đối tượng của field object, `now` / `at` của activity), `flush` (ghi trước khi listener chạy để lệnh listener gọi lại store đứng sau; `dropped` khi không frame nào ra WS — lúc chưa có client), `snapshot`, `direct` (bắt từ `ws.send`). Step gán theo giờ (`trace-capture` ghi giờ `begin`).
- **Phát lại**: `cargo build --locked -j 4 -p core-store --bin store-replay`, rồi `node scripts/trace-capture.mjs --out=a.trace.json --ops=a.ops.json` và `node scripts/trace-replay.mjs a.ops.json a.trace.json [--out=a-rust.trace.json]` (so `--strict` + fold). Kịch bản trong process: `npm test -w @ccm/collector -- test/trace-replay.test.ts` (bỏ qua khi chưa có binary); bản cố định cho cargo: `tests/replay/monitor.{ops,node.trace}.json` (đường tạm → `<root>`).
- **Kết quả**: demo 30×6 2 lượt — mỗi lượt Node WS trace vs Rust replay = 0 khác biệt cả `--strict` lẫn fold (828 và 840 frame, 54 932 và 58 199 op); so thô từng frame lượt a (kể cả `seq`, id activity) cũng 0. `monitor` trong process: frame Rust bằng hệt Node. Node a ↔ Node b lượt này lệch (step `end`, lệch thời điểm đọc file của Node, §3.1) — không ảnh hưởng vì mỗi replay so với Node cùng lượt.
- **Giới hạn**: replay so Store, không so monitor — trace Rust ở đây dùng đúng thứ tự lệnh Node đã gọi. Demo 30×6 và kịch bản `monitor` không có terminal nào mang `diff` / `deniedTools` / `limitHit` (refs dùng lại = 0), nhánh `Arc` chỉ có test đơn vị (`tests/store.rs`, `tests/replay.rs`). Đối tượng bị sửa tại chỗ (cùng id, nội dung khác) được đếm `mutated_refs` (2 lượt demo: 0).
- **Ý tưởng sau (chưa làm)**: so sâu field object (bỏ patch khi nội dung bằng nhau) giảm event `terminal.updated` thừa mỗi lượt git stat; chỉ đổi khi cả Node cùng đổi hoặc khi bỏ `--strict` khỏi cổng parity.

### 3.3 S2 — `core-http` (2026-11-10)

- Crate `apps/core/crates/core-http` = `server.ts` 1:1 without a web framework (`httparse` head + manual 101 + `tokio-tungstenite` frames); `StoreHandle` (`Mutex<Store>` + `Notify`) runs the 50 ms batch on tokio; `Hooks` replace the `startServer` callbacks. Details in `.code_index/core.md` (core-http). Bin `http-serve` (env `CCM_PORT` / `CCM_WEB_DIST` / `CCM_APP_KEY`, `--dev`) is a test / dev harness; the real `orrery-core serve` is S10.
- Gate: `hardening.test` + `csp.test` ported (`tests/hardening.rs`, `tests/static_files.rs`, `src/csp.rs`); `tests/golden.rs` = first WS frame equals the Node snapshot frame (replay of `monitor.ops.json` up to each snapshot op), seq included; `tests/process.rs` = exit 3 on a taken port, stdin `shutdown` / EOF exit 0.
- Differences kept on purpose: origin ports use the bound port (Node: requested port, so port 0 matched nothing); a non-loopback `host` is refused; `toolSpans.get.since` accepts ISO-8601 only; `/focus` with `Transfer-Encoding: chunked` counts as an empty body; keep-alive is off (`Connection: close`); `/api/timeline/*` is S9.

### 3.4 S3 — stores nhỏ + `JsonFile` (2026-11-10)

- `core-store` `src/persist/`: `json_file` (`JsonFile` tmp + rename, debounce do chủ chạy `next_due` / `poll(now)` như `Store`), `aliases` (alias + notes), `favorites` (`path.resolve` / `basename` / `isAbsolute` tay trong `paths`), `history`, `snooze`, `response`, `timeline`, `forecast` (`LimitHistory`), `daystart`, `datadir`, `notify_file` (đọc / ghi `notify.json`, sanitizer của `packages/shared/src/notify.ts`; toast logic ở S7). `localtime` (`Zone::Local` / `Fixed`, `parse_iso` = `Date.parse` ISO, bản sao của `core-transcript/src/time.rs` để store không phụ thuộc transcript), `collate` (bản sao `locale_cmp`).
- Cổng parity: `apps/collector/test/persist-cross.test.ts` + bin `persist-cli` (`cargo build --locked -j 4 -p core-store --bin persist-cli`, hoặc `CCM_PERSIST_CLI`): mỗi store 2 chiều (Node ghi → Rust đọc, Rust ghi → Node đọc) so deep-equal; 14 test.
- Khác biệt biên: lỗi ghi file chỉ log `eprintln`; ghi alias / favorites chạy đồng bộ ngay trong lệnh gọi (Node async); `Date.parse` chỉ hiểu ISO; token trong `history.json` là số nguyên (`SessionRecord` dùng `u64`, phần lẻ bị cắt); id dạng số nguyên thuần không bị JS sắp lại thứ tự key; thứ tự key trong object ghi ra có thể khác Node (nội dung bằng nhau).

## 4. Cổng cuối

- Parity: trace WS Node vs Rust (S0) = 0 khác biệt sau chuẩn hoá trên demo 30×6, fixture thật, `monitor` + `tracker` + `codex` + `deepseek` + `office` kịch bản; toàn bộ test collector có bản Rust / parity tương đương (liệt kê trong báo cáo S11).
- E2E: `npm run e2e -w @ccm/desktop` pass trên Windows (CDP); macOS: workflow `desktop` + checklist tay (không có CDP): focus, open, toast, Telegram, Workspace, Office, Devices.
- Số đo (so với bước 0, `docs/perf/`): RAM private tổng **530 → 280–330 MB**; CPU lúc rảnh gần 0; CPU nhiều agent giảm 2–3 lần; giữ cold usage scan 4 GB <= 5 CPU-s; boot tới `/health` không chậm hơn Node. Không đạt → không đổi mặc định.
- Dự phòng: 1 release vẫn bundle `collector.cjs`; `CCM_CORE=node` bật lại; `serve` không lên được → app tự chạy Node và báo 1 lần (như `noteCoreFallback`). Giữa phiên không chuyển qua lại (state khởi động lại).

## 5. Rủi ro + Không được phá

| Rủi ro | Cách giảm |
|---|---|
| Lệch diff / coalesce → web nhấp nháy hoặc mất update | S0 trace compare từ đầu, S1 chạy trước mọi thứ |
| Hai bản ghi cùng file `CCM_DATA_DIR` (Node cũ + Rust) | một process ghi tại một lúc (cờ chọn bản); format giữ nguyên; test đọc chéo |
| Cổng 4317 / origin đổi → mất localStorage | Rust serve web dist trên đúng `127.0.0.1:4317` |
| Toast / Telegram sai thời điểm hoặc gửi trùng | S7 so chuỗi episode; quiet hours + snooze test 1:1 |
| Rust panic kéo sập cả monitor | `panic = abort` + supervisor restart max 3, giữ `loading` đúng; PTY ở process khác |
| Cargo + Node + Gradle build lâu | `CCM_SKIP_CORE_BUILD`, cache cargo CI |

**Không được phá**: (1) không gửi prompt / không gọi Claude API / không tốn token, chỉ các ngoại lệ đã ghi trong `CLAUDE.md` (Compact, `hand_back`, Telegram receive, `telegram_send` button, `open_session submit`) — các luồng này chạy ở main, giữ nguyên; (2) redaction: chỉ counter / id / whitelist rời Rust, không nội dung session, env, secret, tool args, đường dẫn socket (`sockKey` = sha256); (3) identity terminal = `pid:procStart`, không từ title; (4) Telegram reply (`lastReply` <= 3000, chỉ memory, không ghi đĩa); (5) Office timeline: chỉ ghi khi có client bật, giữ 7 ngày, tên trong `office-names.json`; (6) bind 127.0.0.1 + `x-ccm-app` mọi route trừ `/health`; (7) client msg validate theo shape, drop khi sai; (8) app đang chạy của chủ nhân: dev dùng `CCM_PORT` + `CCM_DATA_DIR` riêng, không chiếm 4317, không ghi vào data thật; không build / test nặng khi chủ nhân đang dùng máy (<= 80% CPU / RAM, `JIN_MAX_THREADS`); (9) không comment trong code, file <= ~400 dòng, không commit tự động.

## 6. Quyết định chủ nhân (2026-11-10)

1. **Hai process Rust**: `orrery-core serve` riêng, PTY riêng.
2. **Toast / alert logic ở Rust** (S7), desktop main chỉ hiển.
3. Focus / mở folder / launch / chọn folder ở desktop main qua RPC `host.*` (S8) — theo đề xuất.
4. **Office làm sau**: S9 tách khỏi cổng đổi mặc định. Chế độ `CCM_CORE=rust` chưa có Office thì Office báo rõ "cần `CCM_CORE=node`" (không im lặng), S9 làm sau S11.
5. Shadow trong dev trên bản copy dữ liệu thật (cổng + `CCM_DATA_DIR` riêng) — theo đề xuất.
6. **Giữ bản Node tới bước 6 (Tauri)**: `CCM_CORE=node` + tự lui khi `serve` không lên vẫn bundle tới khi làm Tauri; S12 chuyển sang bước 6.
