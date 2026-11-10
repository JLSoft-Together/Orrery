# WP-v87-6-T2b: vỏ Tauri lấp chỗ thiếu (2026-11-11)

Không build release, không E2E, không commit.

## Đã làm
1. `telegram.notice`: `apps/shell/src/notice.rs`, host trong `hosts.rs`. Notification hệ điều hành, chữ en/vi như `RELOGIN` của `deskHosts.ts`, locale lấy từ `appearance::Store`. Trả `true` khi `show()` thành công, `false` khi `kind` khác `relogin`.
2. `resources.report`: core-serve chạy cùng process nên `core-serve/src/sampler.rs` giữ báo cáo mới nhất (`keep_latest`, `latest_resources()`), host trả đúng dữ liệu Electron nhận qua message `resources`. `mcp_desk/deps.rs` đã gọi host này.
3. `agentData.stopCollector` / `startCollector`: `collector.rs`. Stop = `Serve::stop` (watcher + usage thả hết file lock), start = `Serve::start(shell.serve_cfg)`. Bỏ qua khi app đang thoát. Web tự reconnect (như Electron `host.stop/start`). `Shell` thêm `serve_cfg`.
4. Worktree `wt.*`: `worktree.rs` + `wtcopy.rs` port `answerWorktree`, `parseWorktrees`, `worktreeDest`, `copySetupFiles`. Chạy trên thread riêng (git đến 120 s không chặn `resolve`). Chặn remove khi pane chưa exit nằm trong worktree (`Panes.dirs`: cwd, liveCwd, exited).
5. Shim: `shim.rs` port `shim.ts` 1:1 (posix + cmd), `apply_shim_env` (PATH đầu + `ORRERY_SHIM_DIR`), PowerShell `PS_SHIM_FIRST`, zsh wrapper `shell-init-tauri` (re-prepend sau `.zshrc`, OSC 7). Chỉ khi shell pane gửi `shim:true` và có MCP config.

## Test (macOS arm64)
- `apps/shell`: `cargo test -j 4` 102 passed (trước 86), clippy `-D warnings` sạch.
  - worktree 5 test (git thật: init, add có copy `.env`, exists, branch, main, unknown, busy, dirty, remove, ngoài git), wtcopy 3, shim 6 (chạy shim posix thật với claude / codex giả: `--session-id`, `--mcp-config`, `--resume`, subcommand, `--`, codex `-c`, 127), presets 1, pty cập nhật 2.
- `apps/core`: `cargo test -j 4 --no-fail-fast` 877 passed, 6 ignored; clippy sạch (thêm 1 test `latest_resources`).

## Còn thiếu / rủi ro
- Shell integration OSC 633 (nonce, command marks) chưa có ở Tauri; bash `--init-file` re-prepend shim chưa có: profile bash đặt lại PATH thì mất shim (shim luôn tìm claude thật bỏ qua thư mục của nó nên không hỏng lệnh).
- Windows: shim `.cmd` + `PS_SHIM_FIRST` chỉ kiểm text, chưa chạy trên Windows. `Serve::start` lần 2 dùng `take_stdin` lại (Windows `SetStdHandle`): chưa thử, cần E2E agentData move trên Windows.
- Stop collector chờ tối đa 4 s; hết giờ mà thread chưa xong thì không start lại được (cổng còn bị giữ), log `[shell] collector restart failed`.
- Zsh wrapper chỉ bật khi `shim:true`; shell zsh không bật shim vẫn không có OSC 7 (như trước).

STATUS: DONE (chưa E2E)
