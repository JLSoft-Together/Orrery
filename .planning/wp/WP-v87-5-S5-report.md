# WP-v87-5-S5 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S5.md`, dòng S5 plan `PLAN-v87-step5-core-store.md` (§3.8 mới). Không commit / push, không sửa `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`, không đụng file của S7 / S8. Không chạm app Orrery đang chạy: test dùng thư mục tạm + root giả; lần thử `monitor-serve` đầu dùng nhầm cổng 4317 (env `CCM_PORT=0` bị bỏ qua) và bind thất bại ngay (không kết nối, không ghi gì), lần sau dùng cổng 47931 + data / Claude / Codex / Codewhale home tạm. `~/.claude` / `~/.codex` / `~/.codewhale` thật không đọc. CPU lúc chạy 22–43 %.

## Đã làm

1. **Codex** (`core-monitor/src/codex/`), port 1:1 của `codex/monitor.ts` + `link.ts` + `processes.ts` + `rollout.ts` (state), đọc rollout bằng `core_transcript::codex` (tail + `read_session_meta`, đã có từ T5a):
   - `monitor.rs` `CodexMonitor` (timer = mốc giờ, `next_due()` / `tick`, `sync` / `scan_now` / `reconcile`, `dismiss_ended`, `pid_of` / `cwd_of` / `is_live`, `diagnostics`), `threads.rs` (discover lúc khởi động chỉ giữ thread còn chạy, thêm thread, đọc rollout, limits Codex qua `LimitHistory` file `codex-limit-history.json`), `scan.rs` (quét process, `link`, `find_resumed` cho `codex resume <id>` ngoài thư mục ngày), `publish.rs` (vị trí: link vào agent Claude đã gọi / dưới Codex parent / standalone `codex:<id>`; terminal + agent; activity `terminal.started|ended`, `agent.started|completed|cancelled|failed`, `tool.started`; `forget`), `link.rs` (`match_processes`, `claude_target_for`, `caller_of`, `normalize_dir`), `process.rs` (`mode_of_command_line`), `state.rs` (`CodexThreadState`, `mode_of`), `names.rs` (`session_index.jsonl`).
2. **DeepSeek** (`core-monitor/src/deepseek/`), port của `deepseek/{monitor,outbox,processes}.ts`: `monitor.rs` + `read.rs` (outbox qua `sidefiles.outbox` của `core_transcript`, có sẵn từ T5b; vòng `more` / `offset` / `skipping` như `CoreOutboxTail`) + `publish.rs` + `state.rs` (`DeepseekThreadState`, subagent max 50) + `config.rs` (`outbox_path_from` TOML, `codewhale_home`) + `process.rs` (`parse_codewhale_args`, `match_deepseek_processes`) + `tail.rs`. Xoay outbox khi `config.toml` đổi, resume `-r`, process chết.
3. **Dùng chung**: `cliprocs.rs` (`list_named`: bảng `core_procs` + `cmdline`, cây cha, `orphan`; luật Windows FILETIME và macOS lstart như `windowsProcessesNamed` / `unixProcessesNamed`), `fswatch.rs` (`FsWatch` trên `notify`, có `wake` để thread monitor dậy), `agentprocs.rs` (trait `AgentProcs`, `SystemProcs`).
4. **Nối vào `Monitor`** (`monitor/agents.rs`): `Monitor.codex` / `.deepseek`, `enable_agents(procs, persist, verify)`, `start_agents` (sau `start`, như `index.ts`), `tick_agents` trong `Monitor::tick`, `with_codex` (closure targets trên `claude_targets`), `next_due` / `stop` / `flush_persist` / `set_wake`; `dismiss_ended`, `open_folder`, `focus_window`, `diagnostics` giao cho `codex:` / `deepseek:` như Node. `MonitorConfig` thêm `codex_home`, `codewhale_home`, `home_dir`. `serve` bật theo `CCM_CODEX` / `CODEX_HOME` / `CCM_DEEPSEEK` / `CODEWHALE_HOME` (mặc định bật như `config.ts`).
5. **Harness**: `script.rs` thêm lệnh `codexProcs`, `deepseekProcs`, `startAgents`, `codexSync`, `codexScan`, `codexTick`, `codexNames`, `deepseekRead`, `deepseekResolve`, `deepseekScan`, `deepseekTick`, `agentInfo`, `diagnostics` + `FakeProcs`. `apps/collector/test/monitor-twin.ts` thêm `agents` (Node `CodexMonitor` / `DeepseekMonitor` dùng chung `MonitorStore`, vá `scheduleScan` / `scheduleRead` / `setupWatchers` thành no-op để so bước xác định) + `setCodexProcs` / `setDeepseekProcs`.
6. **Test mới**: `apps/collector/test/agents-twin.test.ts` (2 test), `core-monitor/tests/codex.rs` (5), `tests/deepseek.rs` (5), unit trong `cliprocs`, `codex/{process,link,state,names,publish}`, `deepseek/{config,state,process,tail}`.
7. **Docs**: plan §3.8 + đánh dấu dòng S5, `.code_index/core.md` (core-monitor agents), `collector.md` (twin agents), `_meta.md`. `desktop.md` không đổi (S5 không đụng desktop).

## File (khớp `git status`, phần của S5)

- Mới: `apps/core/crates/core-monitor/src/{agentprocs,cliprocs,fswatch}.rs`, `src/codex/{mod,monitor,threads,scan,publish,link,process,state,names}.rs`, `src/deepseek/{mod,monitor,read,publish,state,config,process,tail}.rs`, `src/monitor/agents.rs`, `tests/{codex,deepseek}.rs`, `apps/collector/test/agents-twin.test.ts`, báo cáo này. File dài nhất 378 dòng (`deepseek/monitor.rs`), không comment.
- Sửa: `core-monitor/src/{lib,script}.rs`, `src/monitor/{mod,actions}.rs`, `src/serve/mod.rs` (3 chỗ nhỏ: env codex / deepseek vào `MonitorConfig`, `enable_agents` sau `Monitor::new`, `start_agents` sau `set_loading(false)`; file này S8 cũng sửa, hai phần không chồng nhau), `apps/collector/test/monitor-twin.ts`, `.planning/PLAN-v87-step5-core-store.md`, `.code_index/{core,collector,_meta}.md`. Không thêm dependency (`Cargo.toml` / `Cargo.lock` không đổi bởi S5).
- Không phải của S5 nhưng có trong `git status`: `apps/core/Cargo.{toml,lock}` + `core-toast/` + `toast-twin*` + `WP-v87-5-S7*` (S7), `core-monitor/src/host/`, `tests/host_rpc.rs`, `apps/desktop/{src/host-rpc.ts,test/host-rpc.test.ts}`, `WP-v87-5-S8*` (S8), `.code_index/desktop.md`, `apps/collector/.vitest/` (cache vitest), `.planning/PROGRESS-*`, `RELEASE-v2.3.0.md`, `docs/qa/*`.

## Test (số thật)

- `cargo test --locked -j 4` (apps/core, cả workspace): **435 passed, 6 ignored, 0 failed** (49 suite, gồm S6 / S7 / S8 đã chạy song song). `core-monitor`: **77 passed** (unit + `peers` 8 + `job_actions` 5 + `reread` 2 + `host_rpc` + `codex` 5 + `deepseek` 5).
- `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/collector -- --maxWorkers=4`: **51 file passed + 1 skipped, 535 passed, 3 skipped** (gồm `agents-twin` 2, `monitor-twin` 2, `codex.test`, `codex-parity`, `deepseek.test` giữ nguyên và pass). `npm run typecheck -w @ccm/collector`: sạch.
- Thử đột biến: đảo thứ tự sắp xếp cặp thread↔process trong `link.rs` (`Reverse`) → `agents-twin` Codex fail; trả lại → pass.
- Smoke `monitor-serve --dev` (cổng 47931, root tạm): lên, in `listening`, thoát khi stdin đóng, ghi `codex-limit-history.json`.

## Kết quả parity

| Kịch bản | Bước | fold | `--strict` |
|---|---|---|---|
| twin Codex (exec dưới agent con `a1` của Claude, standalone, subagent dưới Codex, plugin theo cwd, thread mới sau scan, `--resume` thread cũ, process chết, reopen, dismiss, stale) | 12 | 0 | 0 |
| twin DeepSeek (outbox có dòng hỏng, turn failed, subagent lỗi, phiên mới, session_end + resume bằng `-r`, xoay outbox, process chết, dismiss, stale) | 9 | 0 | 0 |
| twin Claude (S4, chạy lại sau khi sửa `monitor/*`) | 21 × 2 lượt | 0 | 0 |

Snapshot Codex ở bước boot có đủ: terminal Claude + agent `codex:<id>` dưới `a1` (link theo ancestry / lời gọi), subagent dưới Codex parent, plugin dưới `a1` (theo cwd), standalone, `codexLimits` (`plus`, 5h 33 %, 7d 12 %). Không chạy demo 30×6 mới (harness `trace-capture` tắt Codex / DeepSeek); phần demo của S4 không đổi.

## Khác biệt biên (cố ý / đã biết)

- Không còn fallback CIM / `ps` của Node: process không mở được (elevated / người dùng khác) → `list_named` trả `None`, scan coi như thất bại và giữ view cũ.
- `readdir` được sort (Node theo thứ tự OS); `mtime` so bằng float ms; khoá nhận diện outbox xoay = creation time (+ inode trên macOS), Windows không có inode ổn định trong std (Node dùng `ino:birthtime`, trên NTFS cũng là file index).
- Timer `scheduleScan` / `SYNC_DEBOUNCE` / `SCAN_WAIT` / `READ_DEBOUNCE` chạy theo `next_due()` của `Monitor` (đồng hồ thật), `deps.now()` dùng cho logic (giả được); script harness gọi trực tiếp.
- `PlanLimits` Codex: `persist = false` (script) dùng thẳng giá trị thô như Node khi không có `LimitHistory`; `serve` dùng `codex-limit-history.json` như `index.ts`.
- Quét process chạy đồng bộ trong thread monitor (Node async); `table()` + `cmdline` ~ vài chục ms.

## Việc cho WP sau

- S10: `serve` đã tự bật Codex / DeepSeek theo env; supervisor chỉ cần truyền `CCM_CODEX` / `CCM_DEEPSEEK` / `CODEX_HOME` / `CODEWHALE_HOME` như `collectorEnv`.
- S11: nghiệm thu chạy `trace-capture` với Codex / DeepSeek bật trên máy thật để so với Node (đọc `~/.codex` thật, chỉ đọc).
- `codex-parity.test.ts` vẫn so reader rollout Node ↔ `core-transcript`; `CodexMonitor` Rust dùng cùng reader đó nên được bao phủ gián tiếp bằng twin.

STATUS: DONE
