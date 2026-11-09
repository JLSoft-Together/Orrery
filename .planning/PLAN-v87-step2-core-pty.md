# PLAN v87 bước 2 — `core-pty` thay PTY host

Ngày 2026-11-09. Con của `PLAN-v87-rust-core.md` (mục "Bước 2", §4 nguyên tắc, §5 build, §8 đã chốt). Nghiên cứu từ code hiện tại, chưa đổi dòng code nào.
Ref viết `host:376` = `apps/desktop/src/pty/host.ts:376`; `sup:` = `supervisor.ts`; `pty.ts:` = `packages/shared/src/pty.ts`; `cli:` = `apps/web/src/lib/ptyClient.ts`.

## 1. Kết luận thiết kế (đọc trước)

- Host hiện làm nhiều hơn PTY: resolve preset, ticket `open_session`, theo dõi branch, `wt.*`, validate cwd, MCP HTTP, ghi shim / shellInit / mcp config. Port hết sang Rust = viết lại 3000+ dòng TS, rủi ro cao.
- Chốt: **Rust chỉ làm PTY + WebSocket `/pty`**. Mọi logic cần TS (preset, env, ticket, git, MCP) chạy trong **Electron main** qua một **control channel** (NDJSON qua stdin / stdout của process Rust). Rust không đọc preset, không chạy git.
- Hệ quả: `presets.ts`, `shellInit.ts`, `shim.ts`, `git.ts`, `wtCopy.ts`, `deepseekMcp.ts`, `mcp.ts`, `mcpTools/*` **giữ nguyên TS**, chỉ đổi nơi chạy (host → main). Rust spawn đúng `file / args / env` main đã dựng sẵn.
- Node host được **làm mỏng cùng contract** (WP-A1) để bản Node và bản Rust nói chung một control protocol: cờ `CCM_CORE_PTY` đổi được qua lại, parity test chạy cùng một script cho cả hai.
- Vị trí crate: `apps/core/` theo plan gốc §3 (không phải `core/` ở root).

## 2. Protocol inventory

### 2.1 Renderer → host (WS `ws://127.0.0.1:<port>/pty`, subprotocol `ccm.pty.<token>`)

| Message | Fields | Ngữ nghĩa | Ref |
|---|---|---|---|
| text `spawn` | preset, cols, rows, cwd?, resume?, open?, session?, integrate?, shim? | Chỉ khi socket chưa gắn session. Lỗi trả `error` rồi **không** gắn: `preset` (sai preset / DeepSeek tắt / `resume` cho shell hoặc không phải UUID), `cwd`, `limit` (live ≥ 16), `spawn`. Thành công: tạo session, `attachTo(replay=false)` | host:397-415, pty.ts:20 |
| text `attach` | id | Gắn socket vào session có sẵn, `replay=true`; không có → `gone` | host:416-421 |
| text `resize` | cols, rows | `clampDim` 1..1000, mặc định 80 / 24; bỏ qua khi đã exit | host:423-428, presets.ts:220 |
| text `cwd` | cwd | Shell báo thư mục sau `cd` (OSC 7 / 9;9). Validate real path, thư mục tồn tại, không UNC, không root ổ đĩa; trả `cwd` (null = từ chối); đổi thì cập nhật `liveCwd` + theo dõi branch | host:367-374, 432 |
| text `kill` | id? | Không id = session đang gắn; có id = session bất kỳ (pane đóng khi socket đã rớt, `cli:27 killById`) | host:392-396, 429 |
| text `owners` | pids[] | Pid nào nằm trong session sống (walk ppid ≤ 32, ≤ 64 pid). Quét process table dùng chung 1.5 s. Lỗi quét → `owners: null`; không session / không pid → `{}` | host:297-305, owners.ts:18 |
| text `wt.list/add/remove` | cwd, branch, copy?, path | Worktree git. Throwaway socket (`cli:53 request`, 20 s). Chạy 1 mutation một lúc (`busy`) | host:317-361 |
| binary | bytes | Keystroke / mouse report → ghi thẳng vào PTY, không biến đổi; client cắt lát 64 KB, host max payload 1 MiB | host:615-624, cli:11 |
| JSON hỏng / type lạ | | Bỏ qua im lặng | host:376-383 |

### 2.2 Host → renderer

| Message | Fields | Ngữ nghĩa | Ref |
|---|---|---|---|
| `ready` | id, preset, pid, cwd, replay, si? | Gửi khi spawn / attach; `si` = nonce OSC 633;E (chỉ shell tích hợp) | host:282 |
| `pid` | pid | Chỉ khi pid ConPTY chưa có lúc spawn (poll 25 ms, tối đa 5 s) | host:204-211 |
| `exit` | code | Flush output cuối **trước** `exit`; attach lại session đã exit vẫn nhận `exit` | host:194-202, 286 |
| `gone` / `error` | message | Xem 2.1 | host:418, 398 |
| `branch` | branch \| null | Đẩy khi đổi + gửi lại khi attach | host:227-235, 285 |
| `cwd` | cwd \| null | Trả lời `cwd` + gửi lại `liveCwd` khi attach | host:283, 370 |
| `owners`, `wt.list/added/removed/error` | xem `pty.ts:38-54` | Trả lời trên chính socket hỏi | host:297, 307 |
| binary | bytes | Output PTY. Replay ring ngay sau `ready` khi `replay=true` | host:161, 284 |

### 2.3 Hành vi lõi (phải giống)

| Hạng mục | Hiện tại | Ref |
|---|---|---|
| Scrollback ring | 256 K ký tự, cắt khi > 1.5×; replay có thể bắt đầu giữa escape sequence | host:43, 186-188 |
| Flush | Gom `pending` bằng timer 8 ms trước khi gửi (trễ cố định tới 8 ms mỗi lần echo) | host:44, 191 |
| Backpressure | `bufferedAmount` > 512 KB → `proc.pause()`; poll 50 ms, < 128 KB → resume. Không có socket: vẫn đọc vào ring, không pause | host:48-49, 153-172 |
| Attach | Socket mới thay socket cũ (đóng cũ `4000 replaced`), xoá `pending` | host:278-287 |
| Socket đóng | Chỉ gỡ `ws` khỏi session, PTY chạy tiếp | host:628 |
| Giới hạn | `MAX_LIVE` 16; session đã exit nằm lại 30 phút nếu không có socket (reaper 60 s) | host:45-46, 653-661 |
| Auth | Host khớp `^127\.0\.0\.1:\d+$`, Origin ∈ `CCM_PTY_ORIGINS`, subprotocol token so `timingSafeEqual`, server chọn đúng protocol đó | host:116-121, 598-609 |
| Kill | Windows: `taskkill /T /F /PID` + `proc.kill()`; macOS: chỉ `proc.kill()` (SIGHUP vào shell) | host:261-276 |
| Shutdown | `shutdown` từ main: `taskkill` đồng bộ mọi cây rồi `exit` sau 300 ms; watchdog ppid 5 s | host:637-671 |
| Early exit | Chết < 15 s → ghi `pty early exit: preset=… code=… after=…ms` vào `pty.log` | host:196, presets.ts:211 |

### 2.4 Main ↔ host (hiện là `parentPort`)

| Message | Hướng | Dùng cho | Ref |
|---|---|---|---|
| `listening {port}` | host→main | Main dựng `PtyEndpoint {url, protocol}` | host:676, sup:75 |
| `census` / `{t:'census', sessions}` | main↔host | `PtySessionInfo[]` cho quit confirm + `paneRoots()`; 1.5 s timeout | host:633, 680, sup:118-129; main.ts:255, 687 |
| `shutdown` | main→host | Tắt sạch, 3 s rồi `kill` | sup:131-142 |
| `open` / `open-reply` | host↔main | `open_session`: ticket 60 s → hỏi page → `paneKey`; timeout 8 s | host:449-495, sup:97-104 |
| `mcp` / `mcp-desk` / `mcp-reply` | host↔main | Tool MCP cần page / desk; timeout 8 s, `free_port` / `telegram_receive` 600 s, `telegram_send` 200 s | host:499-526, sup:106-115 |
| env `CCM_PTY_TOKEN / ORIGINS / NODE_PTY / APP_VERSION / INIT_DIR / MCP_CONFIG` | main→host | Cấu hình lúc fork | sup:50-62 |

Env của pane: `childEnv` (xoá `CCM_*`, `ORRERY_*`, `ELECTRON_*`, biến session agent) + `spec.env` + `ORRERY_API`, `ORRERY_TOKEN` (token riêng mỗi pane), `ORRERY_MCP_CONFIG` (host:180).

## 3. Chia trách nhiệm

| Việc | Ở đâu sau bước 2 | Ghi chú |
|---|---|---|
| ConPTY / forkpty, đọc, ghi, resize | **Rust** `core-pty` | |
| WS server `/pty`, auth, binary / text frames | **Rust** | Không cần axum, `tokio-tungstenite` đủ |
| Ring 256 KB, replay, backpressure, flush leading-edge | **Rust** | |
| Kill cả cây, exit code, early-exit log | **Rust** | Job object (Win), process group (mac) |
| `owners` (ppid walk), census, reaper 30 phút, MAX_LIVE | **Rust** | Process table bằng `sysinfo`; bước 3 thay bằng `core-procs` |
| Resolve spawn (preset, cwd, resume, ticket, minted id, env, shim, shellInit) | **Main** `PtyBroker` (mới) | `presets.ts` / `shim.ts` / `shellInit.ts` không đổi |
| Mint `id` session + token MCP per pane | **Main** | Main đưa `id` cho Rust; mirror bảng session (cwd, preset, pid, exited, agentSession) |
| `cwd` validate + `trackBranch` + `wt.*` | **Main** | Rust chuyển tiếp qua control, relay lại trả lời cho đúng socket |
| MCP `orrery` HTTP + `mcpTools/*` + rate / `NoteQueue` / `askPage` / `askDesk` | **Main** (WP-A2, §8.3 plan gốc) | `relayOpen` / `relayMcp` thành gọi hàm trực tiếp |
| Ghi `orrery-mcp.json`, shim dir, shell-init dir, deepseek mcp file | **Main** | Rust chỉ nhận đường dẫn trong `env` |

### Control channel (stdin → Rust, stdout ← Rust, stderr = log vào `pty.log`)

NDJSON, mỗi dòng ≤ 1 MB, kiểu khai báo ở file mới `packages/shared/src/ptyControl.ts` (Rust mirror bằng serde; test JSON fixture hai phía).

| Message | Hướng | Fields |
|---|---|---|
| `init` | main→core | token, origins[], appVersion (dòng đầu tiên; token **không** đi qua argv / env) |
| `listening` | core→main | port |
| `resolve` | core→main | rid, msg (đúng `spawn` của renderer) |
| `resolved` | main→core | rid, ok, id, file, args, env (đầy đủ), cwd, agentSession?, si?, hoặc `message` (`preset`/`cwd`/`limit`/`spawn`) |
| `spawned` / `exited` / `removed` | core→main | id, pid / code / — (main cập nhật mirror, `spawnWaits.spawned`) |
| `cwd` / `cwd-ok` | core→main→core | rid, id, cwd / cwd (đã validate hoặc null) |
| `wt` / `wt-reply` | core→main→core | rid, msg / reply (Rust chỉ forward JSON) |
| `branch` | main→core | id, branch (core cache + gửi socket đang gắn + gửi lại khi attach) |
| `census` / `census-reply` | main↔core | sessions[] |
| `kill-all` / `shutdown` | main→core | — (stdin EOF = shutdown, thay watchdog ppid) |

## 4. Thiết kế Rust

### 4.1 Layout

| Path | Nội dung |
|---|---|
| `apps/core/Cargo.toml`, `rust-toolchain.toml`, `Cargo.lock` | workspace, pin stable, `--locked` |
| `apps/core/crates/core-pty/` | lib: `session.rs` (PTY + child + writer thread), `ring.rs`, `flow.rs` (flush + backpressure), `ws.rs` (accept + auth + frames), `owners.rs`, `jobs/{windows,unix}.rs` (kill tree), `control.rs` (NDJSON) |
| `apps/core/crates/core-bin/` | bin `orrery-core`: đọc `init`, chạy tokio, nối `core-pty` |
| `apps/core/tests/` | integration: giả main bằng stdin / stdout, WS client |

Chưa có `core-procs` / `core-store` (bước 3+); `owners.rs` viết sao cho thay được bằng `core-procs`.

### 4.2 Crate

| Crate | Dùng cho | Ghi chú |
|---|---|---|
| `portable-pty` | ConPTY + openpty | WP-C spike 1 ngày; nếu vướng (pid, exit, close deadlock) chuyển sang gọi ConPTY bằng `windows-sys` và `libc::forkpty` trực tiếp |
| `tokio` (rt, net, sync, time, io-util) | WS + control | `current_thread` hoặc 2 worker để RSS nhỏ; PTY đọc / ghi bằng `std::thread` riêng mỗi session (blocking IO) |
| `tokio-tungstenite` + `futures-util` | WS | `accept_hdr_async` để kiểm Host / Origin / subprotocol và chọn protocol trong response; `max_message_size` 1 MiB |
| `serde`, `serde_json` | control + control frames | `ts-rs` chỉ khi WP-E muốn sinh type; chưa bắt buộc |
| `subtle` | so token constant-time | thay `timingSafeEqual` |
| `sysinfo` (feature tối thiểu) | process table cho `owners` | cache 1.5 s như hiện tại |
| `windows-sys` (Job, Threading) / `libc` | kill cây | |

### 4.3 Hành vi

- **Flush leading-edge:** thread đọc PTY đẩy chunk (≤ 64 KB) vào kênh; task gửi lấy chunk đầu **ngay**, rồi `try_recv` gom mọi chunk tích lại trong lúc gửi trước đó thành một frame. Không timer → bỏ trễ 8 ms (Windows 15 ms thực tế). Frame boundary đổi so với Node, nội dung byte giống.
- **Ring:** vòng byte cố định 256 KB (Node: chuỗi tới 1.5×, UTF-16). Replay bắt đầu từ byte đầu của một ký tự UTF-8 (bỏ continuation byte); giữa escape sequence vẫn cho phép như hiện nay (xterm chịu được). Rust gửi byte thô, không decode: khác Node chỉ ở UTF-8 hỏng (Node thay U+FFFD) — parity test chuẩn hoá chỗ này.
- **Backpressure:** đếm byte chưa gửi theo socket; > 512 KB → thread đọc dừng đọc (child tự block); < 128 KB → đọc tiếp. Chưa có socket: vẫn đọc vào ring, không dừng. Số 512 / 128 KB giữ nguyên.
- **Ghi:** thread ghi riêng mỗi session, hàng đợi ≤ 4 MB; task WS không bao giờ block vì child không đọc stdin.
- **Exit:** child kết thúc → đọc nốt tới EOF (tối đa 200 ms) → gửi frame cuối → `exit`. Windows: đóng pseudoconsole **trong khi** vẫn đọc (tránh deadlock đã biết của ClosePseudoConsole).
- **Kill cây:** Windows: mỗi session một Job Object `KILL_ON_JOB_CLOSE`, gán child ngay sau spawn; `kill` = `TerminateJobObject`; dự phòng walk Toolhelp32 theo ppid. Cả `orrery-core` nằm trong một job `KILL_ON_JOB_CLOSE` riêng: core chết thì mọi pane chết (thay watchdog). macOS / Linux: `setsid` (forkpty có sẵn) → `killpg(SIGHUP)`, 2 s sau `SIGKILL` nếu còn. Tốt hơn Node (macOS hiện chỉ HUP shell).
- **Env / cwd:** Rust nhận `env` đầy đủ từ main và `env_clear()` rồi set (Windows key không phân biệt hoa thường: dedupe). `cwd` đã validate ở main; Rust chỉ kiểm `is_dir` lần cuối. Không bao giờ đọc env của chính nó để đưa cho pane.
- **Auth:** token cấp qua dòng `init`, bind `127.0.0.1:0`, in `listening`. Handshake giống `host:598-609`; sai Host / Origin / token → 403 trước upgrade.
- **Log:** `eprintln!` một dòng mỗi sự kiện (spawn, early exit, kill, lỗi), không in nội dung terminal, không in env / token.

### 4.4 Main khởi động Rust, build, đóng gói

| Việc | Cách làm |
|---|---|
| Cờ | `CCM_CORE_PTY=rust\|node`, mặc định `node` cho tới khi qua cổng §6. Đọc ở `main.ts` (chỗ dựng `PtySupervisor`, dòng 1440). Ghi bản đang chạy vào `pty.log` + `logReport` |
| Supervisor | Tách `PtyHost` interface (`endpoint`, `census`, `stop`); `NodeHost` = `utilityProcess.fork` như cũ, `RustHost` = `child_process.spawn(orrery-core, {stdio:['pipe','pipe','pipe'], windowsHide:true})`, gửi `init`, chờ `listening` 10 s. Lỗi khởi động / chết bất thường: ghi log, **không** tự đổi sang Node giữa chừng (câu hỏi O1) |
| Vị trí binary | Packaged: `resources/core/orrery-core(.exe)`; dev: `apps/core/target/release/`; override `CCM_CORE_BIN` (dùng cho test) |
| `build.mjs` | Thêm `buildCore()` cạnh `buildDaemon`: kiểm `cargo` + target, `cargo build --release --locked -p core-bin -j $JIN_MAX_THREADS`; `CCM_SKIP_CORE_BUILD=1` bỏ qua; dev chỉ build khi thiếu binary; `--dist` bắt buộc build. Giữ `pty-host.cjs` + node-pty trong gói cho tới khi bỏ bản Node |
| `electron-builder.yml` | `extraResources`: binary → `core/`. Windows: `from: ../core/target/release/orrery-core.exe`. macOS 2 dmg: `${arch}` trong `from` trỏ `target/<triple>/release/orrery-core` (kiểm electron-builder có hỗ trợ macro trong extraResources; không thì override cấu hình theo arch trong `scripts/dist.mjs`) |
| macOS | Build 2 triple (`aarch64-apple-darwin`, `x86_64-apple-darwin`), không `lipo` vì dmg tách theo arch. Ký ad-hoc cùng app (hiện `identity: '-'`, hardenedRuntime off); khi có Developer ID thì thêm vào bước ký + notarize ngay |
| `.gitignore` | `apps/core/target/` |
| CI | `desktop.yml` job `build`: `dtolnay/rust-toolchain@stable` (+ targets mac), `Swatinem/rust-cache`, `-j` giới hạn. `test.yml` (win + mac): thêm `cargo test --locked` và `npm test` parity (WP-E) |

## 5. Work packages

Giới hạn máy: tối đa `JIN_MAX_SUBAGENTS` song song; cargo `-j $JIN_MAX_THREADS`; không chạy 2 tool nặng cùng lúc. Mọi WP: không comment trong code, không `git commit`, test chạy trên `CCM_DATA_DIR` tạm.

| WP | Việc | File scope (độc quyền) | Phụ thuộc |
|---|---|---|---|
| **A1** TS | Control contract + `PtyBroker` trong main: resolve spawn, ticket + `spawnWaits`, mirror session, `cwd` validate, `trackBranch`, `wt.*`; `host.ts` mỏng lại (PTY + ws + owners + census), nói contract mới qua `parentPort` **và** chế độ stdio (`CCM_PTY_CONTROL=stdio`) | `packages/shared/src/ptyControl.ts` (mới), `apps/desktop/src/pty/{broker.ts mới, host.ts, supervisor.ts, spawnWait.ts}`, `main.ts` (chỉ khối `PtySupervisor`), test `pty*.test.ts` | — |
| **A2** TS | MCP `orrery` sang main: HTTP server loopback trong main, token → session qua mirror, `ToolHost` ở main (`openFrom`, rate, `NoteQueue`, `askPage`, `askDesk` gọi thẳng `orrery`), `ORRERY_API` trỏ port main; bỏ `relayOpen` / `relayMcp` | `apps/desktop/src/pty/{mcp.ts, mcpTools/*, broker.ts}`, `supervisor.ts`, `main.ts` (khối MCP), `mcp*.test.ts` | A1 |
| **B** Rust + build | Scaffold workspace, `rust-toolchain.toml`, crate rỗng chạy được (`init` → `listening` → `shutdown`), `buildCore()`, `electron-builder.yml`, `.gitignore`, workflow, `.code_index/core.md` khung | `apps/core/**`, `apps/desktop/{build.mjs, electron-builder.yml, daemon-kit.mjs}`, `scripts/dist.mjs`, `.github/workflows/*`, `.gitignore`, `package.json` (script) | song song với A1 / A2 |
| **C** Rust | `core-pty` lõi: control NDJSON, WS accept + auth, spawn từ `resolved`, write, resize, kill, exit, pid, early-exit log, Job Object / process group, `kill-all` + stdin EOF | `apps/core/crates/**` (session, ws, control, jobs) | A1 (contract đóng băng), B |
| **D** Rust | Ring + replay, backpressure, flush leading-edge, census, `owners`, reaper 30 phút, `MAX_LIVE`, attach / replaced | `apps/core/crates/core-pty/src/{ring,flow,owners}.rs`, nối vào `session.rs` / `ws.rs` | C |
| **E1** TS test | Harness parity: script JSON các bước chạy trên cả host Node (stdio) và Rust, giả main tối thiểu, shell giả là `node -e` (deterministic, cùng chạy Win + mac) | `apps/desktop/test/pty-parity/**` | A1 (chạy với Node trước), dùng được với Rust từ C |
| **E2** TS | `RustHost` + cờ `CCM_CORE_PTY` trong supervisor, E2E CDP mở shell + Claude, đo độ trễ phím, RSS, `docs/perf/` | `supervisor.ts`, `main.ts` (khối dựng host), `apps/desktop/e2e/core-pty.cjs`, `scripts/perf-bench.mjs` (case mới), `.code_index/{desktop,core}.md` | A2, D, E1 |
| **F** (sau) | Soak 1 bản release, đổi mặc định `rust`, giữ Node thêm 1 bản rồi xoá `pty-host.cjs` + node-pty | | cổng §6 |

Thứ tự: A1 → A2; B song song A1; C sau (A1 + B); D sau C; E1 song song C (khung) rồi chạy với Rust khi C xong; E2 cuối. Hai WP chạy song song không chung file (A và B tách thư mục; E1 chỉ ghi `test/pty-parity/`).

### Test từng WP

| WP | Lệnh | Tiêu chí xong |
|---|---|---|
| A1 | `npm test -w @ccm/desktop -- test/pty.test.ts test/spawnWait.test.ts`; `npm run typecheck`; E2E `p2`–`p6` trên dev Electron với `--user-data-dir` tạm | Hành vi renderer không đổi; host Node chỉ còn PTY + ws; `grep` `host.ts` không còn `runGit`, `trackBranch`, `presetCommand` |
| A2 | `npm test -w @ccm/desktop -- test/mcp*.test.ts test/deepseek-mcp.test.ts`; E2E `v49-open.cjs`, `v80-mcp.cjs`, `shell-agent.cjs` | Codex / Claude pane gọi được `open_session`, `notify_owner`, `list_sessions`; token sai → 401 trước khi đọc body; `ORRERY_API` port ≠ port `/pty` |
| B | `cargo test --locked -j 6 --manifest-path apps/core/Cargo.toml`; `CCM_SKIP_CORE_BUILD=1 npm run desktop` vẫn chạy | Binary stub trả `listening` rồi thoát khi stdin đóng; `npm run dist:win` kèm binary trong `resources/core/` (chạy khi chủ nhân yêu cầu build) |
| C | `cargo test` (integration: spawn `node -e`, write, resize, kill, exit code, kill cây: grandchild chết) | Mọi message §2.1-2.2 trừ ring / owners / census chạy qua WS thật |
| D | `cargo test` (ring wrap, replay UTF-8 boundary, backpressure với child xả 8 MB, MAX_LIVE 16, owners sâu 3 cấp, reaper bằng clock giả) | Cổng RSS ≤ 20 MB với 8 pane, p50 echo ≤ 5 ms (đo trong E2) |
| E1 | `npm test -w @ccm/desktop -- test/pty-parity` với `CCM_CORE_BIN` | Cùng script → cùng chuỗi message sau chuẩn hoá (mask id / pid, gộp frame, bỏ khác biệt UTF-8 hỏng) cho Node và Rust |
| E2 | `npm run e2e -w @ccm/desktop` (dev build, `CCM_CORE_PTY=rust`) | Mở pane shell + Claude, gõ, resize, đóng, `--resume`, quit confirm đếm đúng census |

Script parity phải gồm: spawn mọi preset hợp lệ + preset sai / cwd sai / `resume` sai; echo bytes + OSC 7 + OSC 633;E đi nguyên vẹn; resize; attach sau khi thoát (replay + `exit`); attach thay socket (`4000`); kill theo id từ socket khác; owners; census; limit 16; ring > 256 KB (replay đúng đuôi); child xả output khi không có socket; exit code 0 / khác 0 / bị kill.

## 6. Cổng đổi mặc định sang `rust` (WP-F)

Parity xanh Win + mac; E2E CDP pass; p50 độ trễ phím phía host ≤ 5 ms; RSS `orrery-core` ≤ 20 MB / 8 pane; kill cây sạch (không process mồ côi sau quit, sau crash core); chủ nhân dùng thật 1 bản release với cờ `rust` không lỗi.

## 7. Rủi ro + không được phá

| Rủi ro | Cách giảm |
|---|---|
| ConPTY qua `portable-pty` khác node-pty: pid, exit code, resize, deadlock lúc đóng, đầu ra cuối bị mất | Spike ở WP-C, test exit nhanh (`cmd /c exit 3`) hàng trăm lần; dự phòng ConPTY trực tiếp. Giữ system conhost như hiện (`useConptyDll: false`) |
| Exit code Windows (NTSTATUS như `0xC0000142`) hiển thị khác Node | Parity test ghi giá trị Node trả; Rust phát cùng kiểu (i32) |
| Truy vấn terminal (DA, DSR `\x1b[6n`, bracketed paste, mouse) | Core không lọc / biến đổi byte hai chiều; xterm ở renderer trả lời qua binary frame. Test echo byte-exact |
| `--resume`, `--session-id`, `-n`, `--mcp-config` mất khi chuyển resolve sang main | Giữ nguyên `presetCommand`; test `pty.test.ts` chạy trước / sau A1 phải y hệt |
| OSC 7 / 9;9 cwd, nonce OSC 633;E sai chỗ | Shell init vẫn do TS sinh; `ready.si` + `cwd` + `branch` gửi lại khi attach (kiểm trong parity) |
| Kill cây Windows bỏ sót process (job gán muộn) | Job gán ngay sau spawn + walk Toolhelp dự phòng; test grandchild |
| Main chậm / treo làm spawn đứng (resolve qua control) | Timeout `resolve` 5 s → `error spawn`; resolve chỉ ở spawn, không nằm trên đường phím |
| MCP trong main tranh event loop | Body ≤ 64 KB, batch ≤ 16 giữ nguyên; `run_log` / `wait_session` không block |
| Quản lý token: lộ qua argv / env | Token chỉ qua dòng `init` trên stdin; token pane `ORRERY_TOKEN` chỉ ở env của chính pane |
| Ký / notarize binary macOS, `${arch}` trong `extraResources` | Kiểm sớm ở WP-B bằng workflow thủ công trên runner mac |

**Tuyệt đối không phá:** pane Claude / Codex / shell mở, gõ, resize, đóng; `--resume` mở lại; OSC 7 theo `cd`; truy vấn terminal; kill cây khi đóng pane và khi thoát app; `x-ccm-app` / token / bind `127.0.0.1`; mọi test hiện có của `@ccm/desktop`.

**Quy tắc an toàn máy chủ nhân:** test và E2E chỉ dùng `CCM_DATA_DIR` tạm, `--user-data-dir` tạm, port 0; **không** chạm app Orrery đang chạy của chủ nhân (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`). Dọn process test chỉ theo pid đã ghi từ chính lần chạy, **không** `taskkill /IM orrery-core.exe` / `electron.exe` / `node.exe` (trùng tên với app đang dùng).

## 8. Câu hỏi cho chủ nhân

| # | Câu hỏi | Đề xuất |
|---|---|---|
| O1 | Binary Rust không khởi động được: tự rơi về Node hay báo lỗi? | Chỉ giai đoạn `node` mặc định thì không cần; sau khi đổi mặc định, **báo lỗi rõ + nút dùng Node**, không im lặng đổi |
| O2 | `wt.*`, `cwd` validate, theo dõi branch ở lại TS trong main qua control channel (không port sang Rust) | Đồng ý? (khớp "Rust chỉ lo PTY", tránh viết lại git) |
| O3 | Crate đặt ở `apps/core/` (theo plan gốc) hay `core/` ở root | `apps/core/` |
| O4 | Tự rơi sang `portable-pty` hay ConPTY trực tiếp nếu spike lỗi | Quyết sau spike WP-C, báo chủ nhân số đo |
