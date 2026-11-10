# WP-v87-5-S10 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S10.md`. Không commit / push, không sửa `apps/collector/src`, `packages/shared`. Mọi lần chạy `serve` thử: `CCM_PORT` riêng (0 / cổng trống / 4399 của E2E), data / Claude / Codex / Codewhale home tạm (trừ E2E đọc `~/.claude` thật như bộ E2E vốn làm, data dir tạm); không chạm 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`; không toast / Telegram thật (E2E `CCM_TOAST=0`); chỉ kill pid tự spawn.

## Đã làm

- **Crate mới `apps/core/crates/core-serve`** = `orrery-core serve [--dev]` (`core-bin` thêm nhánh `serve`). Khung `core-monitor/src/serve/` + bin `monitor-serve` của S4 chuyển sang đây (đã xoá ở core-monitor). Nối:
  - `config.rs`: đọc env như `collectorEnv` / `config.ts` (`CCM_APP_KEY`, `CCM_PORT`, `CCM_DATA_DIR`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `CODEWHALE_HOME`, `CCM_CODEX`, `CCM_DEEPSEEK`, `CCM_TOAST`, `CCM_TOAST_BACKEND`, `CCM_TOAST_LINK`, `CCM_STALE_TTL_MIN`, `CCM_PEER_TTL_MIN`, `CCM_WEB_DIST`, `CCM_VERIFY_PROCESSES`, `CCM_STATUSLINE_*`, `CCM_DEV_ORIGIN_PORT`, `CCM_RES_FIXTURE`, `CCM_HOST_RPC`, mới `CCM_SERVE_STATS`).
  - `alerts.rs`: `core-toast` `Toaster` + `SnoozeStore`; observe sau tick khi store có event chờ flush + timer 15 s `check_loops`; `notify.update|test`, `client.presence`, `terminal.snooze`, `/focus`, `/snooze`, `disk.alert`, `agent.note`, `toast.health` (main gửi), sensitive activity (listener store); `drain()` → stdout (`full` trên Windows, ngắn trên macOS electron).
  - `sampler.rs`: `core-sources` `ResourceSampler` + `Runner` (bật khi có cửa sổ hoặc `idle_watch`), report → `set_resources` + `observe_idle` + `{type:'resources'}`; `device.used` từ stdin. `busy` chỉ gửi khi đổi.
  - `sensitive.rs`: `SensitiveWatch` (file `sensitive.json`) thay `default_sensitive`, cắm vào `Monitor.sensitive` qua `SensitiveCheck`; `sensitive.update|ignore`.
  - `host_ops.rs`: pickFolder / focus / open / launch / prompt search chạy thread riêng.
  - `procs.rs` + `stats.rs`: cache bảng process đọc ngoài store lock; số đo `CCM_SERVE_STATS=1`.
  - `dispatch.rs` / `runtime.rs` / `lib.rs`: client message, vòng monitor, kênh stdin/stdout, hook core-http.
- **core-monitor** (nhỏ): `procs_wanted` / `scan_wanted`, `prepare` (đọc registry + dựng / catch-up tracker mới ngoài lock), `focus_pid` / `open_dir` / `favorite_target` / `launch_done`, `query_processes_in`, `list_named_in`, `list_{codex,deepseek}_processes_in`.
- **core-http** (để monitor giữ store không làm đứng tokio): `seq` atomic (`/health` không chờ lock), `try_with`, flush loop không chặn (thử lại 10 ms), snapshot WS connect / `resync` trong `spawn_blocking`.
- **Desktop**: `core-serve-host.ts` (`CoreServeHost`: spawn `<CCM_CORE_BIN> serve` + `CCM_HOST_RPC=1`, NDJSON stdin/stdout, `host.req` → `HostRpc`, log `collector.log`, restart max 3, boot 30 s, exit 3 = port in use, shutdown line + stdin end, kill sau 4 s; stdin đóng khi main chết → serve thoát; `SwitchingHost` / `createMonitorHost`: `CCM_CORE=rust|node`, **mặc định `node`**, rust không lên → Node + thông báo 1 lần, taken port không lui). `serve-toast.ts` (Windows vẽ toast `full`, gửi `toast.health` sau boot / sau toast test). `main.ts` (+21 dòng): chọn host, dòng `core serve: rust|node (…fell back: …)` trong log report (Diagnostics copy), IPC `ccm:core-serve`; `preload.ts` `coreServe`; `strings.ts` `serveFallbackTitle/Body` en + vi.
- **Office** (chủ nhân chốt làm sau): `rust` → `office.enable` chỉ log 1 lần; web `OfficeNeedsNode.vue` (icon + chữ + màu warning, i18n en + vi `settings.officeNeedsNode`) hiện trong Settings dưới công tắc Office và thay Office view trên map khi `coreServe === 'rust'`.
- `scripts/trace-capture.mjs`: `--server=rust|both` chạy `orrery-core serve --dev` (mặc định `target/{release,debug}/orrery-core`, hoặc `CCM_CORE_BIN` / `--rust-bin`) — sửa ngoài phạm vi brief vì `monitor-serve` đã bỏ.
- Docs: plan §3 dòng S10 + §3.9, `.code_index/{core,desktop,collector,_meta}.md`, `CLAUDE.md` (env `CCM_CORE`, `CCM_SERVE_STATS`).

## File (khớp `git status`)

Mới: `apps/core/crates/core-serve/` (Cargo.toml + `src/{lib,config,runtime,dispatch,alerts,sampler,sensitive,procs,host_ops,stats}.rs`), `apps/core/crates/core-bin/tests/serve.rs`, `apps/core/crates/core-http/tests/busy_store.rs`, `apps/core/crates/core-monitor/tests/prepare.rs`, `apps/desktop/src/core-serve-host.ts`, `apps/desktop/src/serve-toast.ts`, `apps/desktop/test/core-serve-host.test.ts`, `apps/web/src/components/office/OfficeNeedsNode.vue`, báo cáo này.
Sửa: `apps/core/{Cargo.toml,Cargo.lock}` (chỉ thêm package local, không crate ngoài mới), `core-bin/{Cargo.toml,src/main.rs}`, `core-http/src/{server,ws}.rs`, `core-monitor/{Cargo.toml,src/lib.rs,src/cliprocs.rs,src/process.rs,src/codex/{monitor,process}.rs,src/deepseek/{monitor,process}.rs,src/monitor/{mod,actions,reconcile}.rs}`, `apps/desktop/src/{main,preload,strings}.ts`, `apps/web/src/{components/AgentMap.vue,components/settings/SettingsGeneral.vue,i18n/en.ts,i18n/vi.ts,lib/desktop.ts}`, `scripts/trace-capture.mjs`, `CLAUDE.md`, `.planning/PLAN-v87-step5-core-store.md`, `.code_index/{_meta,collector,core,desktop}.md`.
Xoá: `core-monitor/src/serve/{mod,dispatch}.rs`, `core-monitor/src/bin/monitor-serve.rs`.
Không phải của S10: `.planning/PROGRESS-2026-11-09-v87.md` (đã M từ trước), `apps/desktop/test/__snapshots__/fwp-host.test.ts.snap` (chỉ đổi EOL do test desktop; `git checkout` bị auto-mode chặn — main chạy `git checkout -- apps/desktop/test/__snapshots__/fwp-host.test.ts.snap`), các file `??` khác có sẵn.

## Test (số thật)

- `cargo clippy --locked -j 4 --workspace --all-targets -- -D warnings`: 0 cảnh báo.
- `cargo test --locked -j 4 --workspace` (trước khi thêm `prepare` / core-http): 452 pass, 0 fail, 6 ignored. Sau đó chạy lại các crate đổi `-p core-monitor -p core-http -p core-serve -p core-bin`: 196 pass, 0 fail + `core-http` đầy đủ pass (thêm `busy_store` 1). Mới: core-serve unit 12, `core-bin/tests/serve.rs` 5, `core-monitor/tests/prepare.rs` 1, `core-http/tests/busy_store.rs` 1.
- `npm test -w @ccm/collector -- --maxWorkers=4`: 51 file pass / 1 skip, 535 test pass / 3 skip (gồm twin monitor / agents / toast / sources / persist).
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 132 file, 1542 pass / 18 skip (mới `core-serve-host.test.ts` 9).
- `npm test -w @ccm/web -- --maxWorkers=4`: 218 file, 1865 pass.
- Typecheck web / desktop / collector: sạch.

## Trace + E2E

- `trace-capture --server=both` demo 30×6 (Node collector vs `orrery-core serve` release, cùng root sống): lượt a (bản trước `prepare`) **0 khác biệt** (822 / 627 frame); lượt b (bản cuối) **30+ khác biệt** chỉ ở bước cuối `replay-10`: 15 subagent Node `completed` / Rust `error` + `agent.failed`, `completedAt` lệch +2 ms; lượt c (bản cuối, chạy lại) **0 khác biệt** (860 / 651). b không tái hiện → nghi lệch thời điểm đọc file (§3.1), chưa chứng minh; S11 chạy thêm nhiều lượt để chốt.
- E2E `CCM_CORE=rust` (instance cô lập port 4399, `collector.log` xác nhận serve Rust `listening 4399`, không lui về Node): p2 14/14, p3 15/15, tabs-cwd 17/17, v49-open 33/33, v70-tools 43/43, agent-map 34/34, shell-agent 5/5 — 0 FAIL. Sau E2E không còn `orrery-core` mồ côi.

## 4 mục bắt buộc

- (a) `pick_folder` không chặn monitor: chạy thread riêng (`host_ops.rs`), cùng focus / open / launch / prompt search; test `an_open_folder_dialog_does_not_block_other_requests` (dialog chưa trả lời vẫn nhận `history.data` trong 3 s rồi `folder.picked` đúng).
- (b) Không `--dev` mà thiếu `CCM_APP_KEY` → exit 1 (test unit + process).
- (c) `CCM_PORT` sai (`nope`, `70000`, `-1`) → exit 2, không in `listening`; `0` = cổng trống ≠ 4317 (test). Khác Node (Node rơi về 4317) — có chủ đích.
- (d) Bảng process: đọc ngoài store lock khi `procs_wanted` dự đoán cần (cache chỉ sống 1 tick); đo 60 s demo 30×6: **0 lần đọc bảng dưới lock trong vòng lặp** (1–3 lần lúc boot). `/health` không còn chờ lock: p99 5,5 ms / max 28 ms (Node 13 / 16). **Chưa đạt "tick không trễ hơn Node"**: thời gian giữ store lock trong `Monitor::tick` p50 0,3 ms, p99 ~130 ms, max 0,65–0,95 s (event loop Node p99 ~19 ms, max ~50 ms) — phần chậm là đọc transcript / subagent (sync tracker + reconcile) trong `tick`; event tới WS trễ theo các tick dài này (stress 30×6, tick 400 ms). Đã thử đọc trước tracker ngoài lock (giữ draft) — không giảm đuôi nên bỏ; giữ `prepare` cho registry + tracker mới (boot catch-up 2,7–3,6 s trên demo giờ ngoài lock).

## Việc cho S11

- Tách đọc file của `MonTracker::sync` (và `discover` subagent) khỏi store lock, hoặc giảm IO mỗi lần sync, trước khi đổi mặc định `CCM_CORE=rust`; đo lại bằng `CCM_SERVE_STATS=1`.
- Chạy `trace-capture --server=both` nhiều lượt (≥ 3) để chốt lệch `completed` ↔ `error` của lượt b.
- `start_agents` (Codex / DeepSeek discover lúc boot) vẫn dưới lock; hook `/snooze` vẫn khoá store trên thread tokio (ngắn).
- E2E đầy đủ + thủ công Win/mac với `CCM_CORE=rust`: toast Windows vẽ ở main (chưa thử toast thật), focus / mở folder / launch / chọn folder thật, Telegram / phone episode, fallback khi xoá binary.
- Khác biệt biên đã biết: `--dev` Rust không có toast; link toast khi `CCM_PORT=0` dùng cổng 0; log `[collector]` ra stderr; `promptSearch.cancel` tới trước khi thread search đăng ký thì không huỷ được lượt đó; binary release 1,5 → 5,2 MB; Office cần `CCM_CORE=node` (S9).
- `build.mjs` / `electron-builder.yml` không cần đổi (cùng binary `orrery-core` đã đóng gói).

STATUS: PARTIAL
