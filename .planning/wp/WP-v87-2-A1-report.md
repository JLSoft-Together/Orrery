# WP-v87-2-A1 — báo cáo

**Trạng thái: DONE** (E2E p2–p6 **CHƯA KIỂM**, giao diện **CHƯA NHÌN** — cần build stage, luật cấm build verify và WP-B đang sửa `build.mjs`).

## Đã đổi

| File | Việc |
|---|---|
| `packages/shared/src/ptyControl.ts` (mới, 156 dòng) | Kiểu contract `PtyMainToCore` / `PtyCoreToMain` (:16-38), relay chỉ cho host Node `PtyNodeHostRelay` + `mcp-reply` (:40-44), validator `parseCoreToMain` (:50) / `parseMainToCore` (:79), `encodeControl`, `NdjsonReader` (:119, dòng ≤ 1 MB, bỏ dòng hỏng / quá dài), `PTY_RESOLVE_TIMEOUT_MS` 5 s. Không export qua `index.ts` (web không cần) |
| `apps/desktop/src/pty/broker.ts` (mới, 453 dòng) | `PtyBroker` trong main: `resolve` (:203, cùng thứ tự lỗi preset → cwd → limit → resume, ticket, `mintedSessionId`, mint `id` + token MCP, `presetCommand`, env đầy đủ `childEnv` + `ORRERY_*`), mirror pane + giữ chỗ (reservation tính vào `MAX_LIVE`, quên sau `RESERVE_MS` 30 s nếu core không xác nhận), `handle` (:117) cho `spawned` / `pid` / `exited` / `removed` / `cwd` / `wt`, `trackBranch` (:288, HEAD watch → `branch`), `answerWorktree` (:333), `openFrom` (:411, ticket 60 s, rate 20/phút, `spawnWaits`, askPage max 8 s), `checkoutRoots` (:448), ghi `orrery-mcp.json` / shell-init / shim lúc khởi tạo. Git inject được (`BrokerGit`) để test |
| `apps/desktop/src/pty/host.ts` (690 → 594 dòng) | Chỉ còn PTY + ws + ring / flush / backpressure + owners + census + reaper + kill / shutdown + MCP HTTP (A2 chuyển). `spawn` → `resolve` qua control (:236), frame tới trong lúc chờ được xếp hàng rồi chạy lại đúng thứ tự (`Conn.queue`, `settle` :229); `onResolved` (:248) spawn đúng `file/args/env/cwd` main đưa; `cwd` / `wt.*` chuyển tiếp theo `rid` và trả về đúng socket hỏi; `branch` từ main → cache + gửi socket gắn + gửi lại khi attach. Transport: `parentPort` hoặc `CCM_PTY_CONTROL=stdio` (NDJSON stdin / stdout, stdin EOF = shutdown) (:587). Token / origins / appVersion chỉ qua `init` (không còn `CCM_PTY_TOKEN` trong env). `open_session` + `checkoutRoots` của MCP relay sang main (`mcp-open` / `mcp-roots`, deadline 30 s) |
| `apps/desktop/src/pty/supervisor.ts` | Fork host không kèm token; gửi `init`; mỗi lần fork một `PtyBroker` (dispose khi host chết); route `listening` / `census-reply` / message còn lại vào broker; `relayOpenFrom` / `relayRoots` / `relayMcp` trả `mcp-reply`; census / shutdown dùng object `{t}` thay chuỗi |
| `main.ts` | **Không đổi** (options `PtySupervisor` giữ nguyên) |
| Test mới | `test/pty-control.test.ts` (6), `test/pty-broker.test.ts` (22), `test/pty-host-stdio.test.ts` (5, bundle host.ts bằng esbuild vào thư mục tạm + node-pty thật), `test/pty-supervisor.test.ts` (5, mock `electron`, supervisor + broker + host thật, có MCP `open_session` end to end), fixture `test/fixtures/pty-control.json` (WP-C đọc cùng file này để test serde) |

## Tiêu chí A1: 6/6 (+ E2E chưa kiểm)

| # | Tiêu chí | Kết quả |
|---|---|---|
| 1 | Control contract ở `packages/shared/src/ptyControl.ts`, fixture JSON hai phía | ✓ |
| 2 | `PtyBroker` trong main: resolve spawn, ticket + `spawnWaits`, mirror, `cwd` validate, `trackBranch`, `wt.*` | ✓ |
| 3 | host nói contract qua `parentPort` **và** `CCM_PTY_CONTROL=stdio` | ✓ (stdio chạy thật trong test) |
| 4 | `grep` host.ts không còn `runGit`, `trackBranch`, `presetCommand` | ✓ (cả `resolveCwd`, `takeTicket`, `spawnWaits`, `writeShellInit`, `writeShims`) |
| 5 | Hành vi renderer không đổi (cùng message, thứ tự, lỗi) | ✓ theo test (xem "Khác biệt nhỏ") |
| 6 | MCP `orrery` vẫn ở host, relay chạy | ✓ (test `open_session` qua HTTP thật, token sai → 401) |
| — | E2E `p2`–`p6` trên dev Electron | **CHƯA KIỂM** |

## Audit / Review / Test / Nhìn

- **Audit diff:** `git diff --stat` → host.ts 540 dòng đổi, supervisor.ts 83; file mới như bảng trên. Không đụng file của WP-B. Không thêm comment (3 comment cũ đi theo code `answerWorktree` / `dropWatcher` sang broker).
- **Review edge case:**
  - Spawn lỗi từng loại: `preset` / `cwd` / `limit` / resume sai từ main; `limit` / `spawn` (node-pty throw, socket đã đóng) từ host → host gửi `removed` để main nhả chỗ giữ. Main không trả lời 5 s → `error spawn`, câu trả lời muộn bị bỏ + `removed`. Socket sau lỗi spawn lại được.
  - Attach session đã exit: `ready replay:true` + ring + `exit` (test).
  - 2 socket cùng session: socket cũ đóng `4000` (test).
  - Ticket hết hạn / sai preset: không dùng, không cấp `--session-id` (test).
  - Host chết giữa chừng: broker `dispose()` (watcher đóng, mirror xoá); `post` chỉ gửi khi đúng child; openFrom treo tự hết theo deadline 8 s + 10 s.
  - Shutdown khi còn resolve dở: host kill hết rồi exit 300 ms; reservation ở main bị bỏ cùng broker.
  - Pane bị xoá khi `git rev-parse` còn chạy: không gửi branch muộn (`branchGen` + test).
- **Test (lệnh → kết quả thật):**
  - `npm run typecheck -w @ccm/desktop` → pass; `npm run typecheck -w @ccm/shared` → pass.
  - `npm test -w @ccm/desktop -- test/pty.test.ts test/spawnWait.test.ts test/pty-broker.test.ts test/pty-control.test.ts` → 57/57 pass.
  - `vitest run test/pty-host-stdio.test.ts` → 5/5 (7 s); `test/pty-supervisor.test.ts` → 5/5.
  - `npm test -w @ccm/desktop -- --maxWorkers=6` → 124 file / 1471 test pass (lần 2). Lần 1: `layout-daemon.int.test.ts` EPERM lúc xoá thư mục tạm (flaky có sẵn, không liên quan PTY); trước đó `pty-supervisor` cũng EPERM khi xoá tmp dưới tải → đã thêm `maxRetries` + bỏ qua lỗi dọn.
  - Không chạy test web: `packages/shared` chỉ thêm file mới, không export.
- **Nhìn:** CHƯA NHÌN (không chạy app).

## Contract cuối cùng — khác plan §3

| Điểm | Plan | Thực tế |
|---|---|---|
| `pid` core→main | không có | `{t:'pid', id, pid}` khi ConPTY trả pid muộn (spawned có thể `pid: 0`) |
| `cwd` core→main | `cwd` bất kỳ | `cwd: string` không rỗng; giá trị khác core tự trả `{t:'cwd', cwd:null}` |
| `exited.code` | i32 | số nguyên, có thể > i32 (node-pty Windows trả u32 NTSTATUS, vd `3221225794`) → Rust dùng `i64` |
| `resolved` | rid, ok, id, file, args, env, cwd, agentSession?, si? | giữ nguyên; `cols` / `rows` / `preset` core lấy từ `spawn` đang chờ (`clampDim` 1..1000, mặc định 80 / 24) |
| `removed` | sau kill / reaper | thêm: core từ chối sau `resolved ok` (limit, spawn throw, socket đã đóng, trả lời muộn) |
| Socket đóng trong lúc resolve | — | không spawn (trước đây spawn ra pane mồ côi tới khi thoát app) |
| Timeout | resolve 5 s | thêm: main quên reservation chưa xác nhận sau 30 s |
| `init` | chỉ Rust | host Node cũng dùng `init` (token không còn trong env utilityProcess) |
| Relay chỉ host Node (bỏ ở A2) | `open` / `open-reply` | thay bằng `mcp-open {id, from, args}` / `mcp-roots {id, from}` → `mcp-reply {id, result}`; `mcp` / `mcp-desk` giữ nguyên |
| census / shutdown qua parentPort | chuỗi `'census'` / `'shutdown'` | object `{t:'census'}` / `{t:'shutdown'}` |

Khác biệt nhỏ renderer có thể thấy: trả lời `cwd` và `ready` sau spawn trễ thêm 1 vòng IPC main (vài ms); phím / `resize` gửi trước `ready` được xếp hàng rồi ghi vào PTY như cũ.

## Việc cho WP sau

- **A2:** bỏ `mcp-open` / `mcp-roots` / `mcp` / `mcp-desk` relay; MCP HTTP sang main dùng `PtyBroker` (cần thêm tra token → pane: token đã lưu ở `Pane.token`, `rates` / `NoteQueue` chuyển từ host `Session`); `ORRERY_API` trỏ port main (hiện `port()` = port host). Host đọc token MCP từ `resolved.env.ORRERY_TOKEN` — bỏ cùng MCP.
- **C:** đọc `apps/desktop/test/fixtures/pty-control.json` cho test serde; `exited.code` i64; gửi `removed` ở mọi nhánh từ chối sau `resolved ok`; xếp hàng frame trong lúc resolve.
- **E1:** dùng sẵn cách của `test/pty-host-stdio.test.ts` (esbuild bundle host.ts → tmp, `CCM_PTY_CONTROL=stdio`, `CCM_NODE_PTY` = `require.resolve('@lydell/node-pty')`).
- **E2 / main:** cập nhật `.code_index/desktop.md` mục "PTY host & MCP" (dòng 52-57: host mỏng, broker, contract, test mới) và `.code_index/protocol.md` (thêm `ptyControl.ts`) — ngoài scope A1 nên con chưa sửa. Chạy E2E `p2`–`p6` với `--user-data-dir` tạm khi được build.

## Commit message đề xuất

```
♻️ PTY control contract: PtyBroker in main resolves spawns, cwd, branch and worktrees; Node host keeps PTY + ws and speaks the contract over parentPort or stdio NDJSON
```

## Vòng 2 (8 điểm reviewer) — DONE

| # | Sửa | Chỗ |
|---|---|---|
| 1 | Đường `cwd` (OSC 7) + branch không còn fs sync trên main: `resolveCwd` async (`promisify(fs.realpath)` = bản JS như `realpathSync` cũ, **không** dùng `fs.promises.realpath` native vì native đổi ổ map / subst thành UNC → bị từ chối), `stat` / `readFile` HEAD (`branchOfHead`) async, `wt.add` kiểm dest bằng `stat` async. Mỗi pane một chuỗi `cwdChain` để nhiều `cd` liền nhau trả lời đúng thứ tự, `liveCwd` cuối = lần cuối; `cwd-ok` gửi **trước** khi bắt đầu theo dõi branch (giữ thứ tự cũ). `resolve` / `openFrom` async; kiểm limit + giữ chỗ chạy đồng bộ sau `await` nên resolve song song không vượt 16. Còn sync: `codewhalePath` (`isDir`) + `writeDeepseekMcpConfig` chỉ cho preset deepseek (đang tắt), ghi file cấu hình 1 lần lúc khởi tạo broker | `broker.ts` |
| 2 | `PTY_RESOLVE_TIMEOUT_MS` 15 s | `ptyControl.ts` |
| 3 | Cờ `closing` khi shutdown: `requestSpawn` trả `error spawn` ngay; `resolved` tới sau → `removed` + `error spawn`, không `pty.spawn` | `host.ts` |
| 4 | `PtyFrameQueue` (shared, Rust làm theo): max 4 MB tổng, binary vượt thì bỏ, text luôn giữ; log `pty input dropped while spawning: N frames` | `ptyControl.ts`, `host.ts` |
| 5 | `NdjsonReader` đếm max theo byte UTF-8 (`utf8Bytes`, khớp `Buffer.byteLength`) | `ptyControl.ts` |
| 6 | Env fork: bỏ mọi key `CCM_PTY_CONTROL` / `CCM_NODE_PTY` (không phân biệt hoa thường) rồi set `CCM_NODE_PTY` | `supervisor.ts` |
| 7 | `this.env = opts.env ?? process.env` (không copy) | `broker.ts` |
| 8 | `census-reply` có 1 dòng sai → cả reply `null`; `invalidCensusRows` đếm, supervisor ghi `pty census reply dropped: N invalid rows` vào `pty.log` (census lúc đó trả `null` sau 1.5 s như host không trả lời) | `ptyControl.ts`, `supervisor.ts` |

Test thêm: broker (resolve song song không vượt `MAX_LIVE`, dispose khi resolve còn chạy, chuỗi `cd` giữ thứ tự), control (census hỏng cả reply + đếm, max theo byte với chuỗi tiếng Việt, frame queue), host stdio (queue > 4 MB bỏ 1 frame + log, shutdown giữa lúc resolve: `removed` + `error spawn`, không `spawned`; timeout resolve 15 s; EOF chạy trên host thứ 2).

Lệnh → kết quả: `npm run typecheck -w @ccm/desktop` pass; `npm run typecheck -w @ccm/shared` pass; `vitest run test/pty-broker.test.ts` ×3 → 25/25 mỗi lần; `npm test -w @ccm/desktop -- --maxWorkers=6` → 124 file / 1478 test pass. E2E vẫn CHƯA KIỂM, CHƯA NHÌN.

Contract thêm cho WP-C: queue input lúc resolve max 4 MB (bỏ binary vượt); max dòng NDJSON tính byte; timeout resolve 15 s; `census-reply` phải hợp lệ toàn bộ.
