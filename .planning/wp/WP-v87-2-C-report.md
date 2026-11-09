## WP-v87-2-C — DONE (Windows chạy thật; macOS mới type-check, CHƯA CHẠY)

- Đã đổi (chỉ `apps/core/**`, `.code_index/core.md`):
  - `crates/core-pty/src/control.rs` — đủ contract `ptyControl.ts` (parse tay theo `parseMainToCore`, `LineSplitter` = `NdjsonReader`), test fixture cả valid + invalid.
  - `hub.rs` (mới) — state machine y `host.ts`: resolve + queue 4 MB, timeout 15 s, attach / `4000 replaced`, kill theo id, cwd / wt / branch relay, census, kill-all, shutdown. `server.rs` (mới) — stdin/stdout thread, init → listening, linger 300 ms rồi exit 0.
  - `ws.rs` — handshake Host / Origin / `ccm.pty.<token>` (`subtle`), sai → 403 trước upgrade; writer gom chunk binary liên tiếp (flush leading-edge, không timer).
  - `session.rs` (PtyProc, WriteQueue 4 MB, reader, clamp_dim, dedupe env), `pty_win.rs` (mới, ConPTY trực tiếp), `pty_unix.rs` (mới, portable-pty), `jobs/{windows,unix}.rs` (Job `KILL_ON_JOB_CLOSE`, job cho chính core, walk Toolhelp; killpg HUP → KILL 2 s).
  - `core-bin/src/main.rs` gọi `server::run`; `tests/common/mod.rs`, `tests/pty.rs` (15), `tests/spike.rs` (mới). Crate thêm: `tokio-tungstenite`/`futures-util` (WS), `subtle` (token), `windows-sys` (ConPTY, Job, Toolhelp), `libc` + `portable-pty` chỉ unix, serde_json `preserve_order` (giữ thứ tự env của main); dev `tungstenite` (WS client test). `Cargo.lock` cập nhật, `--locked` chạy được.
- Tiêu chí: 7/7 trên Windows. Dòng chưa chứng minh: kill cây unix (`killpg`) + toàn bộ integration trên macOS — code có test, CHƯA CHẠY (máy Windows; CI `test.yml` macos sẽ chạy).
- Spike portable-pty (O4) → **quyết định: Windows dùng ConPTY trực tiếp (`windows-sys`), portable-pty chỉ cho macOS/Linux.** Lý do: portable-pty bật `PSEUDOCONSOLE_INHERIT_CURSOR` (conhost phát `ESC[6n` và chặn child tới khi có trả lời) và tự nạp `conpty.dll` nếu có trên đường tìm DLL (trái "giữ system conhost").

| Đo (Windows 11, debug build) | n | Treo | Mất tail | Sai code | p50 | max |
|---|---|---|---|---|---|---|
| portable-pty 0.9 `cmd /c exit 3` | 4 | 4 (5 s timeout) | — | 4 | 5026 ms | 5031 ms |
| portable-pty node write+exit | 1 | 0 | 1 | 1 | 5035 ms | — |
| core `cmd /c echo tail& exit 3` | 200 | 0 | 0 | 0 | 122 ms | 143 ms |
| core node write+exit | 50 | 0 | 0 | 0 | 140 ms | 202 ms |
| core `process.exit(-1073741510)` → 3221225786 | 3 | 0 | — | 0 | 109 ms | 124 ms |
| core, bỏ chờ settle trước ClosePseudoConsole | 200 / 50 | 0 | 0 | 0 | 91 / 123 ms | 203 ms |

  Giữ settle 30 ms quiet / max 150 ms làm biên an toàn cho Windows 10 (máy này không mất tail kể cả khi bỏ). Flag ConPTY = `RESIZE_QUIRK | WIN32_INPUT_MODE` như node-pty (cùng `ESC[?9001h ESC[?1004h` đầu phiên).
- Audit: không comment / debug print; `ring.rs` / `flow.rs` / `owners.rs` vẫn rỗng cho D; `CoreToMain::Pid` giữ cho contract (core không cần gửi `pid`: pid có ngay lúc spawn). Log: 1 dòng mỗi sự kiện, không nội dung / env / token / path (có test).
- Review edge case: socket đóng giữa resolve → `removed` (test); socket đóng giữa write → writer dừng, PTY chạy tiếp; child thoát trước khi client đọc → `ready, exit` đúng thứ tự (test); stdin EOF lúc đang spawn → pending nhận `error spawn`, resolved muộn → `removed` (test shutdown); input sau exit / resize sau exit bị bỏ (test).
- Test: `cargo test --locked -j 4` → 32 pass / 0 fail (unit 12, stub 4, pty 15, spike 1). `cargo clippy --locked -j 4 --all-targets -- -D warnings` → sạch; thêm `--target aarch64-apple-darwin` → sạch.
- Ngoài phạm vi cần main xử lý:
  - E1 parity: Rust trả 403 khi auth sai (Node `ws` trả 401); Rust không bao giờ gửi `pid`; kill trên unix báo `exit code 0` (giống node-pty); `owners` tạm trả `{}` / `null` tới WP-D.
  - WP-D nối vào `hub.rs`: ring + replay trong `attach`, backpressure ở `ws.rs` writer, `MAX_LIVE` (`limit`) trong `on_resolved`, reaper 30 phút (`removed`), owners.
  - Máy: đã `rustup target add aarch64-apple-darwin` (để clippy mac). Có 1 `node.exe` pid 21956 (tạo 09:45, cha đã chết, script `hi from child`) không phải của WP này — con không đụng.
- Commit message đề xuất: `🦀 core-pty: NDJSON control + WS /pty auth, direct ConPTY (Windows) / portable-pty (unix) spawn, write, resize, kill tree via Job Objects / killpg, exit flush; integration + 200x fast-exit spike tests`

## Vòng 2 — sửa theo review (DONE)

- #1 `jobs/unix.rs`, `hub.rs`, `server.rs`: shutdown gửi SIGHUP mọi group một lượt (`PtyProc::hangup` → `Force`), hết linger 300 ms chung thì `Hub::force_kill` SIGKILL group còn sống; runtime không bị chặn. Bỏ `kill_now`. Windows: `hangup` = terminate job, `Force` no-op.
- #2 `pty_unix.rs`: reader riêng trên fd dup (`F_DUPFD_CLOEXEC`) + `poll` 100 ms + cờ dừng; leader thoát mà 200 ms chưa EOF → SIGHUP group, dừng reader → thread + fd đóng.
- #4 `pty_win.rs`: walk Toolhelp chỉ khi không có job hoặc `TerminateJobObject` lỗi.
- #5 `ws.rs`: handshake timeout 5 s. #6 `server.rs`: accept lỗi → sleep 50 ms.
- #7 `server.rs`: dòng trước `init` bị bỏ qua (log 1 lần), chờ init; test stub đổi thành `lines_before_init_are_ignored_then_init_still_listens`.
- #8 `hub.rs`: bỏ Data sau khi session đã gửi `exit`. #9 `pty_unix.rs`: lỗi sau `spawn_command` → `child.kill()` + `wait()`.
- #10, #11, #3: ghi vào `.code_index/core.md` (KILL_ON_JOB_CLOSE giết cả process mồ côi khác Node; giữ từ chối resolved trùng id; mục "Required for WP-D": channel `Out` / `Event` phải có giới hạn + backpressure trước khi đổi mặc định).
- Test mới: `silent_connection_is_dropped_after_handshake_timeout`, `shutdown_hangs_up_every_group_once_then_kills_survivors` (cfg unix, panes bỏ qua SIGHUP, shutdown < 900 ms — CHƯA CHẠY, chỉ type-check), stub #7. #8 chưa có test: trên Windows không tạo được data sau exit một cách xác định.
- Test: `cargo test --locked -j 4` → 33 pass / 0 fail (Windows). `cargo clippy --locked -j 4 --all-targets -- -D warnings` sạch trên Windows và `--target aarch64-apple-darwin`. Spike 200× vẫn 0 treo / 0 mất tail sau khi bỏ walk Toolhelp.
