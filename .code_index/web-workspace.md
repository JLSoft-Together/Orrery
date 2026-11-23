# web-workspace: terminal / browser pane, tab, split, launcher

Đường vào: `apps/web/src/features/workspace/index.ts` (store, `usePaneInfo`, `usePaneActions`, `leaves`). Index không export component để giữ `WorkspaceView` / `TermPane` lazy trong `App.vue`.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/workspace/stores/workspace.ts::useWorkspaceStore` | state pane / tab / split / project root (83 KB, nên tách) |
| `apps/web/src/features/workspace/components/WorkspaceView.vue` | khung Workspace (104 KB, nên tách) |
| `apps/web/src/features/workspace/components/TermPane.vue` | pane terminal xterm (76 KB); `refit` bỏ qua khi `viewportFits` (`lib/splitLayout.ts`) sai: minimize trên Windows thu webview còn 144x19, refit lúc đó resize PTY còn 1 cột làm mất màn |
| `apps/web/src/features/workspace/lib/devServerUrl.ts` | v111 W1: `devScanSlice` (4 KB đầu chunk), `DevOpenGuard` (tab tự mở 1 lần / URL); bắt URL ở `features/resources/lib/devServer.ts::DevServerDetector`; `workspace.autoOpenDevPreview`; chip `[data-dev-chip]` trên header `TermPane`; công tắc `settings.devPreviewAutoOpen` |
| `apps/web/src/features/workspace/components/BrowserPane.vue` | pane trình duyệt (child webview); v111 W2 `pickElement`: picker trả rect + source (Vue `__file` / React `_debugSource`) + css, chụp `capturePage` rồi `cropToAttachment` vào `useAttachmentsStore().add` của pane agent (chỉ khi resolve ra đúng 1 pane); phím `pickElementKey` (`lib/workspaceKeys.ts`, Ctrl+Shift+C / Cmd+Shift+C, chỉ khi pane đang active), guest forward qua `apps/shell/src/guest.js` |
| `apps/web/src/features/agent-tools/lib/elementCrop.ts` | v111 W2: `cropBox` (rect CSS px -> box ảnh + lề 8, kẹp mép), `thumbSize`, `cropToAttachment` (canvas) |
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
- Footer (WP-112b): `lib/footerChips.ts::planFooterChips` chia chip: luôn hiện running / waiting / setup, context vượt ngưỡng, branch >= 30 thay đổi, focus đang bật; còn lại vào popover `...` (`data-ws-footer-more`). CPU / RAM (`footerSys`) và Share luôn hiện.
- Header pane agent (WP-112a): `lib/paneHeadActions.ts` chọn nút thường trực (Đính kèm, Chat, Changes lượt này `data-pane-turn`, MCP) theo bề rộng pane; Split và worktree của pane agent vào menu `...` (pane shell giữ nguyên). Tab chỉ 1 tín hiệu (`lib/tabSignals.ts`: chờ bạn > lỗi > đang chạy), phần phụ vào tooltip + aria-label (`data-tab-signal`). `lib/browserHandoff.ts`: ảnh cắt từ Browser đi theo `deliver` kể cả khi chọn agent.
- Vòng phụ thuộc workspace <-> rail / next / files / git / run / agent-tools / monitor: các feature import sâu `stores/workspace`.

## Liên kết

- i18n: `apps/web/src/features/workspace/i18n/`. Plan: `.planning/PLAN-v50-workspace-groups.md`, `PLAN-v52-workspace-polish.md`. E2E: `split`, `v50-groups`, `v52-polish`, `tabs-cwd`, `term-ime`.
- Lõi PTY: `core-pty.md`; vỏ: `shell.md`.
