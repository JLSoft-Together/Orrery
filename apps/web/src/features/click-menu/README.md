# click-menu

Trả lời menu chọn của Claude / Codex bằng chuột (v100) và Inbox "Chờ trả lời".

- Đường vào: `@features/click-menu` (`useInboxStore`, `useMenuQueueStore`, `menuSig`, `parseTermMenu`, `useTermMenu*`, `useInboxGlobal`).
- `TermMenuBar`, `InboxChip`, `InboxPanel` nạp trực tiếp từ `components/` (TermPane, WorkspaceView, AppHeader, NextShell).
- File chính: `lib/termMenu.ts` (đọc menu từ màn terminal), `composables/useTermMenu.ts`, `stores/menuQueue.ts`, `stores/inbox.ts`.
- Fixture: `lib/__fixtures__/term-menu` (E2E `v100-menu-*.cjs` đọc bằng đường dẫn này).
- Bất biến: app không tự trả lời menu; mọi phím gõ vào pane đi sau một thao tác của chủ nhân.
