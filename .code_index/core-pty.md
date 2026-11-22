# core-pty: terminal (ConPTY / pty), hub, viewers

Đường vào: `apps/core/crates/core-pty/src/server.rs` (`run`), vỏ spawn `orrery-core` chế độ pty từ `apps/shell/src/pty.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-pty/src/hub/mod.rs::Hub` | chủ duy nhất của state session / conn |
| `apps/core/crates/core-pty/src/hub/spawn.rs` | resolve, attach, settle, `request_spawn` |
| `apps/core/crates/core-pty/src/hub/viewers.rs` | viewer `Own` / `View`, `input_owner`, claim, size |
| `apps/core/crates/core-pty/src/hub/frames.rs` | khung WS, owners |
| `apps/core/crates/core-pty/src/control.rs::MainToCore` | hợp đồng NDJSON main <-> core (khớp `packages/shared/src/ptyControl.ts`) |
| `apps/core/crates/core-pty/src/ws.rs` | `/pty`: Host, Origin, subprotocol `ccm.pty.<token>` (so sánh hằng thời gian) |
| `apps/core/crates/core-pty/src/pty_win.rs` | ConPTY trực tiếp bằng `windows-sys` (không `portable-pty`) |
| `apps/core/crates/core-pty/src/pty_unix.rs` | `portable-pty` cho macOS / Linux |
| `apps/core/crates/core-pty/src/jobs/windows.rs` | Job object `KILL_ON_JOB_CLOSE`, `adopt_self`, `descendants` |
| `apps/core/crates/core-pty/src/ring.rs::Ring` | ring 256 KiB cho replay |
| `apps/core/crates/core-pty/src/flow.rs::Gate` | backpressure (credit) cho PTY reader |
| `apps/core/crates/core-pty/src/marks.rs::OscScanner` | OSC 633 / 133 (shell integration, lệnh xong / lỗi) |
| `apps/core/crates/core-pty/src/owners.rs` | pid -> owner qua `core_procs` |

## Bất biến

- Tối đa 16 session live (`MAX_LIVE`); `resolved.id` trùng thì `error spawn`, không ghi đè pane cũ.
- Session nằm trong Job: pane / core chết thì cả cây process chết.
- Mọi producer có credit (Gate 256 KiB, socket pause 512 / 128 KiB, WS 2 MiB): child xả 8 MiB không làm core phình.
- Input chỉ đổi owner qua `{t:"claim"}` (không claim ngầm khi có byte nhị phân), vì xterm tự trả lời DA / CPR.
- Log không bao giờ chứa byte terminal, env, token, đường dẫn.

## Liên kết

- Vỏ: `apps/shell/src/pty.rs`, `presets.rs`, `pty_branch.rs`, `shell_init.rs` (`shell-hosts.md`). Web: `web-workspace.md` (`TermPane`).
- Remote điện thoại gắn viewer: `shell-remote.md`. Plan: `.planning/PLAN-v87-*.md`, `PLAN-v97-mobile.md`.
