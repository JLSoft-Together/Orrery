## WP-v87-2-C: core-pty lõi (Rust)
- Spec: `.planning/PLAN-v87-step2-core-pty.md` — đọc toàn bộ. WP C ở §5 (bảng WP + "Test từng WP"). Protocol renderer §2.1–2.3, control main ↔ core §2.4 + §3, thiết kế Rust §4.2–4.3, rủi ro §7.
- Contract đã đóng băng (A1, đã commit 964d72d): `packages/shared/src/ptyControl.ts` (type + validator), fixture `apps/desktop/test/fixtures/pty-control.json`. Bản Node tham chiếu: `apps/desktop/src/pty/host.ts` (cùng contract, chế độ stdio `CCM_PTY_CONTROL=stdio`), test `apps/desktop/test/pty-host-stdio.test.ts`. Rust phải nói y hệt bản Node trên stdin / stdout và WS `/pty`.
- Khung có sẵn (WP-B, biên dịch được, 8 test pass): `apps/core/` workspace, `crates/core-pty/src/{control,session,ws,jobs/*,ring,flow,owners}.rs`, `crates/core-bin` (stub init → listening). Báo cáo B: `.planning/wp/WP-v87-2-B-report.md`.
- Mục tiêu: mọi message §2.1–2.2 trừ ring / replay / owners / census / reaper / MAX_LIVE (để WP-D) chạy qua WS thật: control NDJSON đủ (init, resolved, kill-all, shutdown, stdin EOF, pid, exited, error, removed theo contract), WS accept + auth (Host / Origin / subprotocol `ccm.pty.<token>`, sai → 403 trước upgrade, so token constant-time), spawn từ `resolved` (env_clear rồi set env main gửi, dedupe key hoa thường trên Windows, kiểm `is_dir`), write, resize, kill, exit (đọc nốt tới EOF max 200 ms, frame cuối rồi `exit`), pid, early-exit log, Job Object `KILL_ON_JOB_CLOSE` mỗi session + job cho chính core (Windows), `setsid` + `killpg(SIGHUP)` rồi SIGKILL sau 2 s (macOS / Linux). Flush leading-edge đơn giản (chunk đầu gửi ngay, gom chunk tích lại) được làm luôn nếu gọn; backpressure để D.
- Crate: `portable-pty` trước (spike). Kiểm sớm: pid, exit code (kể cả NTSTATUS, `exited.code` i64), resize, deadlock khi đóng, đầu ra cuối có mất không — test `cmd /c exit 3` lặp ≥ 200 lần. Vướng → ConPTY trực tiếp qua `windows-sys` + `libc::forkpty`; ghi số đo + lý do vào báo cáo (câu O4 cho chủ nhân). Giữ system conhost (không bundle conpty.dll). Thêm crate phải ghi lý do; `Cargo.lock` cập nhật, giữ `--locked` chạy được.
- Phạm vi file: `apps/core/**`, `.code_index/core.md`. Cấm đụng: `apps/desktop/**`, `packages/**`, `apps/web/**`, `.github/**`, `apps/desktop/test/pty-parity/**` (WP-E1 đang làm song song).
- Phụ thuộc: A1, B (xong). Song song với: E1.
- Model: opus (concurrency, ConPTY, kill cây, auth).
- Lệnh test: `cargo test --locked -j 4` trong `apps/core` (PowerShell; Git Bash có `link` của coreutils che `link.exe` — nếu dùng Git Bash thì gọi qua `powershell -Command`). `cargo clippy --locked -j 4 -- -D warnings` sạch. Integration test giả main bằng stdin / stdout của binary `orrery-core` + WS client thật, shell giả là `node -e` (deterministic, chạy cả Win + mac).
- Tiêu chí nghiệm thu:
  - [ ] init → listening; stdin EOF / shutdown → kill mọi pane rồi thoát 0; kill-all.
  - [ ] Auth: Host / Origin / token sai → 403 trước upgrade; đúng → chọn đúng subprotocol.
  - [ ] Spawn qua resolved (và resolved lỗi → `error` đúng kind), write byte-exact (kể cả OSC 7 / 633;E, `\x1b[6n`), resize, kill theo id, exit code 0 / khác 0 / bị kill, `pid` gửi đúng.
  - [ ] Kill cây: grandchild chết khi kill pane và khi core chết (Windows Job; unix killpg) — có test.
  - [ ] Spike exit nhanh ≥ 200 lần không treo, không mất frame cuối.
  - [ ] Không log nội dung terminal / env / token; `eprintln!` một dòng mỗi sự kiện.
  - [ ] Tests + clippy sạch; `.code_index/core.md` cập nhật module + quyết định.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-v87-2-C-report.md`

## Nhật ký
- 2026-11-09 spike portable-pty 0.9 Windows: 4/4 `cmd /c exit 3` treo 5 s (INHERIT_CURSOR phát \x1b[6n và chờ trả lời), node tail mất 1/1 → chuyển ConPTY trực tiếp (windows-sys) trên Windows, portable-pty chỉ unix
- 2026-11-09 C viết xong control/hub/ws/server/session/pty_win/pty_unix/jobs; integration 14 test + spike 200× cmd exit 3 (0 treo, 0 mất tail, p50 122 ms), 50× node write+exit (0 mất)
- 2026-11-09 audit/review xong, cargo test 32 pass, clippy sạch (win + aarch64-apple-darwin), code_index + báo cáo WP-v87-2-C-report.md
- 2026-11-09 vòng 2: sửa #1 #2 #4–#11, 33 pass, clippy sạch win + mac target
