# PLAN v87 bước 3 — `core-procs` thay koffi / CIM / ps

Nguồn: khảo sát 2026-11-09 (sonnet, chỉ đọc). Plan gốc `.planning/PLAN-v87-rust-core.md` §6 bước 3. Cổng: kết quả trùng bản koffi trên cùng máy; 1 lần quét ≤ 20 ms.

## 1. Hiện trạng
| Nguồn | File | Dữ liệu | Ai gọi |
|---|---|---|---|
| koffi Toolhelp + GetProcessTimes | `apps/collector/src/win-procs.ts` (`nativeProcessTable`, `nativeUsage`, `nativeCommandLine`) | pid, ppid, name, start FILETIME; workingSet, privateBytes, cpuMs; cmdline (NtQueryInformationProcess 60) | collector `process.ts` (verify 60 s), `resources/procs.ts` (sampler 30 s), `codex/processes.ts`, `deepseek/processes.ts`, `focus.ts`; desktop `pty/owners.ts`, `agentData/procs.ts`, `resources/agentPorts.ts`, `resources/kill.ts`, `run/proc.ts` |
| CIM PowerShell (fallback, `CCM_NATIVE_PROCS=0`) | `process.ts` `queryProcessesCim`, `resources/procs.ts` `cimReader`, `owners.ts` | như trên, chậm | như trên |
| `ps` (macOS / Linux) | `process.ts`, `resources/procs.ts` `psReader`, `owners.ts`, `codex/processes.ts` | pid, ppid, lstart UTC text, rss, time, command | như trên |
| Ports koffi `GetExtendedTcpTable` v4/v6, fallback `Get-NetTCPConnection`, unix `lsof -F pnT` | `apps/collector/src/resources/ports.ts` | pid, state (2 LISTEN / 5 ESTABLISHED), address, port | sampler |
| Rust `owners.rs` | `apps/core/crates/core-pty/src/owners.rs` | pid → ppid | core-pty owners |

## 2. Thiết kế
- **Crate mới `apps/core/crates/core-procs`**: `table()`, `usage(pids)`, `cmdline(pids)`, `ports()`. Windows: Toolhelp32 + `GetProcessTimes` / `K32GetProcessMemoryInfo` / `NtQueryInformationProcess(60)` / `GetExtendedTcpTable` (v4 + v6), qua `windows-sys`. macOS: `libproc` qua `libc` (`proc_listallpids`, `proc_pidinfo` PROC_PIDTBSDINFO / PROC_PIDTASKINFO, `KERN_PROCARGS2` cho cmdline); ports macOS giữ `lsof -F pnT` (deadline 15 s) giai đoạn đầu. Linux: `/proc`. `core-pty/owners.rs` chuyển sang gọi `core-procs::table()` (một nguồn).
- **Chế độ server**: `orrery-core procs` (argv) — không bind port, không token: NDJSON request / response trên stdin / stdout của chính process con, thoát khi stdin EOF. Ai cần thì tự spawn 1 con (collector 1, desktop main 1), mỗi con ~2–3 MB. Không phụ thuộc lifecycle lazy của core-pty.
- **Protocol** (mỗi dòng 1 JSON, `id` do client đặt, trả lời theo thứ tự bất kỳ):
  - `{"id":1,"op":"ping"}` → `{"id":1,"ok":true,"version":"<crate ver>"}`
  - `{"id":2,"op":"table"}` → `{"id":2,"ok":true,"rows":[{"pid":n,"ppid":n,"name":"x.exe","startMs":n|null,"startRaw":"<FILETIME decimal>"|"<lstart UTC text>"|null}]}` — `startRaw` giữ đúng định dạng bản TS đang so (`procStartMatches`: FILETIME ± 1 µs; unix `ps lstart` với `LC_ALL=C TZ=UTC`).
  - `{"id":3,"op":"usage","pids":[..]}` → rows `{pid, workingSet, privateBytes, cpuMs}` (bytes, ms; pid không mở được thì bỏ).
  - `{"id":4,"op":"cmdline","pids":[..]}` → rows `{pid, cmd}`.
  - `{"id":5,"op":"ports"}` → rows `{pid, state, address, port}` (state số như koffi: 2 LISTEN, 5 ESTABLISHED; address chuỗi như `parseTcpTable4/6`).
  - Lỗi: `{"id":n,"ok":false,"error":"<kind>"}`. Dòng hỏng → bỏ qua. Dòng ≤ 1 MB; `pids` ≤ 4096.
- **Phía TS**: module `apps/collector/src/coreProcs.ts` (client: spawn `CCM_CORE_BIN`, NDJSON, timeout 5 s / request, restart khi con chết, max 3 lần / phút rồi coi như không có) + adapter cùng shape `WinProcApi` / reader hiện có, để `nativeProcessTable` / `nativeUsage` / `nativeCommandLine` / `winPortTable` / `psReader` / `owners.readPidTable` chọn backend tại một chỗ. Cờ `CCM_CORE_PROCS=rust|node` (mặc định `node`); `rust` mà core không có / lỗi → rơi về koffi / CIM / ps như cũ (giai đoạn này giữ koffi). Main đặt `CCM_CORE_BIN` (đường dẫn `coreBinaryPath`) vào `collectorEnv`.
- **Bỏ koffi** (sau cổng, WP sau): xoá dependency, `CCM_KOFFI`, `electron-builder.yml` extraResources koffi + `@koromix`, external trong `build.mjs`.

## 3. WP
| WP | Việc | File scope | Model | Phụ thuộc |
|---|---|---|---|---|
| P1 | crate `core-procs` + `orrery-core procs` + owners.rs dùng chung + test + bench | `apps/core/**`, `.code_index/core.md` | opus (FFI Windows / libproc) | — |
| P2 | `coreProcs.ts` client + adapter + cờ + nối main env, test với child giả | `apps/collector/src/{coreProcs.ts mới, win-procs.ts, process.ts, resources/procs.ts, resources/ports.ts, codex/processes.ts, focus.ts}`, `apps/desktop/src/{pty/owners.ts, agentData/procs.ts, resources/agentPorts.ts, resources/kill.ts, run/proc.ts, main.ts (chỉ collectorEnv)}`, test liên quan, `.code_index/{collector,desktop}.md` | sonnet | protocol §2 (song song P1) |
| P3 | Test parity koffi ↔ Rust trên máy thật (table / usage / cmdline / ports), đo thời gian quét, E2E Resources dialog + verify session với `CCM_CORE_PROCS=rust` | `apps/collector/test/core-procs-parity.test.ts`, `apps/desktop/e2e/core-procs.cjs`, `docs/perf/` | **codex** | P1 + P2 |
| P4 | Đổi mặc định `rust`, bỏ koffi + CIM khỏi bộ cài | như §2 "Bỏ koffi" | sonnet | P3 đạt + chủ nhân duyệt |

## 4. Không được phá
Verify session (stale khi pid chết / start lệch), terminal id `pid:start`, Resources dialog + port conflict, Codex / DeepSeek orphan detection, focus window, kill chống reuse pid, owners của pane. App Orrery đang chạy của chủ nhân không bị đụng khi test.
