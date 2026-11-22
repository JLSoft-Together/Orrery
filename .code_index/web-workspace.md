# web-workspace: terminal / browser pane, tab, split, launcher

Đường vào: `apps/web/src/features/workspace/index.ts` (store, `usePaneInfo`, `usePaneActions`, `leaves`). Index không export component để giữ `WorkspaceView` / `TermPane` lazy trong `App.vue`.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/workspace/stores/workspace.ts::useWorkspaceStore` | state pane / tab / split / project root (83 KB, nên tách) |
| `apps/web/src/features/workspace/components/WorkspaceView.vue` | khung Workspace (104 KB, nên tách) |
| `apps/web/src/features/workspace/components/TermPane.vue` | pane terminal xterm (76 KB) |
| `apps/web/src/features/workspace/components/BrowserPane.vue` | pane trình duyệt (child webview) |
| `apps/web/src/features/workspace/components/LaunchPicker.vue` | chọn preset / project khi mở pane |
| `apps/web/src/features/workspace/components/ProjectSwitcher.vue` | đổi project |
| `apps/web/src/features/workspace/lib/splitLayout.ts` | cây split, kéo thả |
| `apps/web/src/features/workspace/lib/layout.ts` | node layout, preset, snap, undo (`layoutPresets.ts`, `layoutUndo.ts`) |
| `apps/web/src/features/workspace/lib/ptyClient.ts` | WebSocket `/pty` tới core-pty |
| `apps/web/src/features/workspace/lib/dust.ts` | hiệu ứng xoá thành bụi + hoàn tác |
| `apps/web/src/features/workspace/lib/shellMarks.ts` | OSC 633: lệnh xong / lỗi |
| `apps/web/src/features/workspace/lib/workspaceKeys.ts` | phím tắt Workspace |
| `apps/web/src/features/workspace/composables/usePaneInfo.ts` | thông tin pane (title, agent, branch) |
| `apps/web/src/features/workspace/stores/attention.ts`, `awake.ts` | chú ý / giữ máy thức |

## Bất biến

- Terminal chạy trong core-pty (Rust); trang chỉ giữ xterm + WebSocket. `claim` input khi chủ nhân gõ thật (xem `core-pty.md`).
- Đóng pane có thể hoàn tác (agent quay lại bằng `--resume`).
- Vòng phụ thuộc workspace <-> rail / next / files / git / run / agent-tools / monitor: các feature import sâu `stores/workspace`.

## Liên kết

- i18n: `apps/web/src/features/workspace/i18n/`. Plan: `.planning/PLAN-v50-workspace-groups.md`, `PLAN-v52-workspace-polish.md`. E2E: `split`, `v50-groups`, `v52-polish`, `tabs-cwd`, `term-ime`.
- Lõi PTY: `core-pty.md`; vỏ: `shell.md`.
