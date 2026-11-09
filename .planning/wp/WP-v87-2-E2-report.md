## WP-v87-2-E2 — DONE
- Đã đổi: `apps/desktop/src/pty/supervisor.ts` (`PtyHost`, `HostLauncher`, `HostChild`, `NodeHost` = cách cũ, `RustHost` spawn `orrery-core` stdio pipe + `NdjsonReader`, stderr → `pty.log`, token chỉ qua dòng `init`, `coreEnv` allowlist, `coreBinaryPath`; thiếu binary / thoát sớm / 10 s không `listening` → log + reject, không rơi về Node); `apps/desktop/src/main.ts` (`PTY_HOST` từ `CCM_CORE_PTY`, `core:` cho supervisor, dòng `terminal host:` trong `logReport`); `apps/web/src/lib/ptyClient.ts` (close 4001 với session id đã biết → attach lại theo id, tối đa 5 lần / 30 s, gửi `ESC c` cho xterm trước replay; còn lại `onClose` như cũ) + `ptyClient.test.ts` (5 test); `apps/desktop/test/pty-supervisor-rust.test.ts` (5 test); `apps/desktop/e2e/core-pty.cjs`, `core-pty-bench.cjs` (mới); `docs/perf/v87-step2-core-pty-win32.json`; `.code_index/{desktop,core,_meta}.md`.
- Tiêu chí 5/5 (Windows; macOS CHƯA KIỂM).
- Bảng đo (release core, Windows, 2 vòng xen kẽ, lấy median; host = `electron.exe` utility process vs `orrery-core.exe`; file `docs/perf/v87-step2-core-pty-win32.json`):

| Chỉ số | Node host | Rust core |
|---|---|---|
| Echo phím host p50 / p90 / p99 (300 mẫu, byte đơn qua WS) | 15.9 / 16.7 / 22.0 ms | 0.77 / 1.36 / 15.9 ms |
| RSS (working set) 1 pane / 8 pane | 118 / 173 MB | 6.1 / 8.0 MB |
| Private 1 pane / 8 pane | 43 / 140 MB | 1.4 / 3.9 MB |
| CPU host khi 1 pane xả 128 MB (đã gồm 8 pane mở) | 3.1 s = 14% 1 core | 1.4 s = 6.4% 1 core |
| Peak working set khi xả | 231 MB | 9.8 MB |
| Thông lượng khi xả | 6.4 MB/s | 6.2 MB/s (nghẽn ở ConPTY / conhost, cả hai như nhau) |

Cổng §6: p50 ≤ 5 ms đạt (0.77), RSS ≤ 20 MB / 8 pane đạt (8.0). p99 Rust một vòng 15.9 ms (vòng kia 5.5 ms): nhiễu lịch máy khi chủ nhân đang dùng.
- E2E (tên file → pass/fail; log `%TEMP%\ccm-e2e\<mode>-<tên>.log`, ảnh `%TEMP%\ccm-e2e\<mode>\<tên>\*.png`):
  - rust: `core-pty` 27/0, `p2` 14/0, `p4` 11/0, `tabs-cwd` 17/0, `v49-open` 33/0, `v80-mcp` 23/0, `shell-agent` 5/0 = 130 pass / 0 fail.
  - node (mặc định): `p2` 14/0, `p4` 11/0 (lần đầu 0 vì cổng 9333 chưa nhả sau p2, chạy lại pass), `tabs-cwd` 17/0, `v49-open` 33/0, `v80-mcp` 23/0, `shell-agent` 5/0, `core-pty` 19 pass / 1 fail (xem dưới).
  - `core-pty.cjs` phủ: log host đúng loại, core là con của app, command line không token / không argv, gõ + echo, resize (107 → 99 → 107 cột), split, `cd` + branch (đổi branch), cây con đóng pane chết hết (shell + grandchild), Claude pane link monitor + cây tiến trình chết khi đóng, `--resume <id>` thấy trong command line, quit confirm native đọc bằng UI Automation = "Quit and close 2 terminals?" + 2 dòng khớp census rồi Cancel (WM_CLOSE) app còn sống, kill core theo pid → mọi pane chết + UI báo lỗi, pane mới dựng core mới, kill main theo pid → không process mồ côi (core + pane).
  - Core không khởi động (`CCM_CORE_BIN` trỏ file không có), chạy Electron thật: pane `error`, app vẫn phản hồi, `pty.log` có `pty host did not start: Error: orrery-core not found at …`. Câu chữ lỗi trên pane là câu chung "Terminals run only in the Orrery desktop app" (đường chết host Node cũng ra câu này).
- Audit: diff chỉ nằm trong phạm vi brief; không comment mới trong code; không đụng `apps/core/**`, `packages/**`, `broker/host/mcp.ts`, `pty-parity/**`; mọi Electron dùng `--user-data-dir` + `CCM_DATA_DIR` tạm, cổng 4399 / 9333 / 4391 / 9391, không chạm app của chủ nhân (PID 10328 còn nguyên); dọn process chỉ theo pid tự spawn, sau mỗi lượt kiểm không còn process `ccm-e2e*`.
- Review: tiêu chí 1 đạt (test desktop đủ bộ + E2E node như trước; 1 fail ở dưới là hành vi host Node cũ, không phải do đổi này); 2 đạt; 3 đạt (test đơn vị + E2E thật); 4 đạt; 5 đạt. Phát hiện: host Node, đóng pane Claude để lại 2–4 process con còn sống sau 12 s (lặp lại 2 lần, tự hết sau đó); Rust job object chết ngay. CHƯA KIỂM: macOS (`core-pty.cjs` phần quit confirm / UI Automation chỉ Windows; `ps` nhánh posix chưa chạy); `--resume` chỉ kiểm command line với session id giả (Claude không có transcript để resume thật); nhánh 4001 kiểm bằng vitest với WebSocket giả, chưa tái hiện 8 MiB overflow thật trong app.
- Test: `npm test -w @ccm/desktop -- --maxWorkers=6` = 127 file / 1510 pass / 18 skipped; `npm run typecheck -w @ccm/desktop` và `-w @ccm/web` sạch; `npm test -w @ccm/web -- src/lib/ptyClient.test.ts` 5/5; `cargo build --release --locked -j 4 -p core-bin` 32 s (binary 927 KB); `pty-supervisor*.test.ts` 10/10 (chạy 3 lần ổn định).
- Ngoài phạm vi: (1) `apps/desktop/e2e/run.cjs` chưa có `core-pty` trong danh sách; (2) `TermPane.vue` không biết attach lại nội bộ nên `ptyClient` tự gửi `ESC c` để khỏi lặp nội dung replay (scrollback cũ mất khi 4001); nếu muốn giữ scrollback cần handler mới ở `TermPane.vue`; (3) `perf-bench.mjs` không đổi: đo host viết riêng ở `core-pty-bench.cjs` (đo qua WS trực tiếp, không qua renderer).
- Commit message đề xuất: `⚡ RustHost + CCM_CORE_PTY flag: supervisor launches orrery-core (or the Node host), renderer re-attaches on close 4001, core-pty E2E + Node vs Rust host bench`

### Nhật ký
Xem `.planning/wp/WP-v87-2-E2.md` (mục Nhật ký).
