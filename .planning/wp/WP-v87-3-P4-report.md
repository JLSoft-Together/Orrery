## WP-v87-3-P4 — DONE
- Đã đổi: `procSource.ts` mặc định `rust` (`CCM_CORE_PROCS=node` quay về CIM/ps; thiếu `CCM_CORE_BIN` → warn 1 dòng); `tableWin/commandLinesWin/usageWin` chỉ core, null/rỗng thì caller tự rơi về CIM/ps; `win-procs.ts` chỉ còn type `WinProc`/`ProcUsage`; xoá `nativeReader`, `bindTcpTable`, `winPortTable`; `main.ts` bỏ `CCM_KOFFI`; `build.mjs` bỏ external koffi; `electron-builder.yml` bỏ extraResources koffi/@koromix; `npm uninstall koffi -w @ccm/collector` (package.json + lock); `coreProcsEnv.ts` không đổi code (đã dựa `coreProcsMode`), main vẫn gán process.env + `collectorEnv` nên pty host Node cũng có `CCM_CORE_BIN`.
- Tiêu chí 4/4: default Rust + fallback log 1 dòng (đạt); không còn koffi trong code/build/package (Grep apps/packages/scripts: 0 file; đạt); quit kill cây pane (đạt, `resources-kill` + `pty-supervisor*` pass); test + typecheck + index + CLAUDE.md (đạt).
- Kill sync lúc quit: xem Vòng 2 (bản vòng 1 kill luôn daemon đã bị bác).
- Audit: diff chỉ trong phạm vi; `parseTcpTable4/6` giữ lại (ports.test dùng) dù src không gọi nữa.
- Review: không comment mới; file không dài thêm (main.ts ngắn đi 3 dòng).
- Test: `npm run typecheck` sạch; collector 435 pass / 1 skip (35 file); desktop 1513 pass / 18 skip (128 file); parity `core-procs-parity` + seam chạy riêng với `CCM_CORE_BIN=...orrery-core.exe` (PowerShell) pass 12/12. Parity đổi sang Rust ↔ CIM (`cimReader`) + `Get-NetTCPConnection`. Test koffi xoá khỏi `win-procs.test.ts`, `resources.test.ts`.
- Ngoài phạm vi: lock file bị `npm uninstall` sửa (-353 dòng, chỉ koffi/@koromix); `.planning/PROGRESS`/PLAN không đụng; chưa đo size dist (main làm). Lưu ý: sync kill không còn spare Gradle daemon khi quit, chủ nhân duyệt nếu muốn khác.
- Commit message đề xuất: ✨ core-procs default Rust: drop koffi (dep, CCM_KOFFI, extraResources, win-procs binding, TCP table binding), CIM / ps fallback, sync quit kill via taskkill /T, parity test Rust vs CIM

## Vòng 2
- Chọn snapshot (không dùng before-quit async: kéo dài quit và đụng main.ts). `run/proc.ts`: `defaultSpawner` (win32, không detached, core bật) gọi `watchTree` làm mới bảng core (pid, ppid, cmd subtree) mỗi 3 s khi child còn sống, ngừng + xoá sau khi child exit 15 s. `killTreeSparing({sync:true})`: snapshot <= 15 s thì `treePids` (trừ cây GradleDaemon) rồi `taskkill /F /PID ...` từng pid, không /T; không có / quá hạn thì `killTree` (/T) + `onFallback` (log 1 dòng).
- Test mới `apps/desktop/test/kill-sparing-sync.test.ts` (2 test: daemon + worker không nằm trong danh sách kill, không /T; không snapshot / stale → `/T /F /PID 100` + onFallback). Typecheck sạch, desktop 1515 pass / 18 skip (129 file).
- Giới hạn: snapshot tối đa 3 s cũ nên pid sinh trong 3 s cuối trước quit có thể sót (wrapper bị kill thì thường chết theo).

## Vòng 3
- 1: `nativeProcRows` giữ `startMs` (FILETIME → ms) nên `treePids` giữ guard pid reuse; `SNAPSHOT_MAX_AGE_MS` = 6 s. Không kiểm được start live lúc kill sync (không có đọc đồng bộ không koffi), ghi vào index.
- 2: xoá snapshot chỉ khi `snapshots.get(pid) === snap` của chính child. 3: `sparingPlan` = tổ tiên của daemon kill plain (`/F /PID`), các gốc không chứa daemon kill `/F /T /PID`: tối đa 2 lần execFileSync (1 lần không đủ vì /T áp cho cả lệnh), timeout 2.5 s mỗi lần.
- 4: bỏ check core lúc spawn, ticker mỗi 3 s tự thử lại khi core lên; `CCM_CORE_PROCS=node` → không snapshot → quit kill cả daemon (ghi `.code_index/desktop.md`). 5: một ticker chung (`snapshotTick`): 1 `table()` + 1 `cmdline` của hợp các subtree mỗi tick, dừng khi hết child.
- 6: `coreReader.fill` không cache / không ghi đè khi `usage` / `cmdline` trả null. 7: xoá 4 comment mới (proc.ts JSDoc, ports.ts, procs.ts x2); `git diff` không còn dòng comment thêm.
- Test (`kill-sparing-sync.test.ts`, 6 test): plan sync, fallback không/stale, `startMs` + reuse guard, ticker chung (1 table + 1 cmdline union), không xoá snapshot child mới cùng pid. Typecheck sạch; desktop 1520 pass / 18 skip; collector 434 pass / 2 skip (parity cần `CCM_CORE_BIN`, không đổi từ vòng 1).
