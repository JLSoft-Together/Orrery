# WP-v88-W2 báo cáo

Phạm vi đã đổi giữa chừng (main, 2026-11-11): chỉ tab `Bảng` + panel "Lệnh đang chạy"; tab `Bầu trời` giữ nguyên Monitor cũ; mặc định `board`.

## Đã làm
- Thanh chế độ (`ModeBar`): tablist Bảng / Bầu trời, chip Cần chủ nhân / Đang chạy, dropdown project (`ProjectStrip`, chỉ ở Bảng). Lưu `ccm.monitorMode` (mặc định `board`).
- Tab Bảng: 4 cột (Cần chủ nhân / Đang chạy / Rảnh / Xong hôm nay) từ `groupSessions` (cột cuối chỉ session đóng trong hôm nay). Thẻ: icon agent (`PresetIcon`, tên chỉ ở aria-label / tooltip), project, tiêu đề, tool + thời gian, số subagent đang chạy, nút "Mở pane trả lời" cho thẻ cần chủ nhân (chỉ `workspace.show`, disabled khi chưa có pane). Right-click = `cardMenu` (bỏ rename / note vì là inline edit của SessionCard) + `nodeMenu` scope card.
- Tab Bầu trời: Monitor cũ không sửa (SessionList, AgentMap, toggle Office, bottom sheet < 768 px).
- Panel Lệnh đang chạy (cả 2 tab): thanh 32 px, bấm / phím backtick, `aria-expanded`, `ccm.cmdPanelOpen`. Bảng Session · Lệnh (`currentToolSummary ?? currentTool`, mono, ellipsis + tooltip) · Tool · Đã chạy (dùng `now` 1 s có sẵn, không timer mới) · Mở pane. Dòng chờ quyền tô `bg-warning-soft text-warning` + icon. Gồm subagent. Thiếu `currentToolStartedAt` -> hiện "chưa rõ" / "unknown".
- Hàng chờ có quỹ đạo mini: KHÔNG làm (theo thay đổi phạm vi), không có code chết.
- Office, `office/`, `ui.mapView`: không động.

## File
Mới: `apps/web/src/components/monitor/{ModeBar,SessionBoard,BoardCard,CommandPanel,OpenPaneButton}.vue`, `components/monitor/monitor.test.ts`, `lib/monitorRows.ts` (+test), `composables/{useMonitorSessions,useSessionMenu,useMonitorKeys}.ts`, `i18n/ui-v88-monitor.ts`.
Sửa: `MonitorView.vue`, `stores/ui.ts` (+test), `lib/shortcuts.ts`, `i18n/{en,vi}.ts`, `.code_index/{web,_meta}.md`, 7 file e2e.

## Khác biệt / lưu ý
- Phím mode: plain 1 / 2 đã là "mở session tab n" (`useShortcuts`), nên dùng **Alt+1 / Alt+2**. Backtick không xung đột. Cả hai có trong ShortcutsDialog.
- Nhãn vi giữ "Cần chủ nhân" (đã có `monitorV79.group`), không phải "Cần anh".
- `SessionList` vẫn dùng ở tab Bầu trời.
- Tour `lib/tour.ts`: không đổi (target `tabs` / `palette` còn nguyên; `data-tour="sessions"` đặt cả trên board).

## Selector E2E đổi
Mặc định mới là `board` nên các script cần map / aside Sessions phải ép sky: thêm `${UI}.monitorMode = 'sky';` trước `setView('monitor')` / `view = 'monitor'` trong `agent-map`, `readme-shots`, `v67-ide`, `v76`, `v78`, `v79-shots`, `v80-shots`. Selector mới: `[data-monitor-mode]`, `[data-board-card]`, `[data-board-column]`, `[data-cmd-toggle]`, `[data-cmd-row]`. Không chạy E2E.

## Test
- `npm test -w @ccm/web -- --maxWorkers=4`: 220 file, 1877 test pass (gồm themes contrast).
- `npm run typecheck`: 0 lỗi.

## Sửa sau review
1. `BoardCard`: Enter / Space trên chính thẻ (`e.target === e.currentTarget`) mở focus; Enter của nút Mở pane không bị nuốt.
2. `lib/monitorRows.ts` `frozenSort`: session mới xếp cuối bằng hằng hữu hạn + so id, hết NaN.
3. `useMonitorSessions`: reset freeze khi `monitorMode` đổi và khi unmount.
4. "Xong hôm nay" tính theo `dayKey` (computed từ `now`, chỉ đổi lúc qua nửa đêm), không re-sort mỗi giây.
5. Lọc project chỉ áp dụng ở board (`filterIds` null ở sky).
6. Bỏ `data-session-card` khỏi `BoardCard` (chỉ `SessionCard` còn dùng; e2e dùng nó chỉ ở chế độ sky). Giữ `data-board-card`.
7. Thời gian tool: chỉ format khi `duration` có giá trị, không thì chỉ hiện tên tool.
8. Tab có `id` + `aria-controls`; vùng board và sky có `role="tabpanel"` + `aria-labelledby`.
9. Store không có cờ riêng cho menu / dialog ngoài danh sách đang kiểm (palette, settings, tour, recap, diagnostics, shortcuts, whatsNew, đã chặn sẵn); bỏ qua.
Test thêm: Enter / Space + nút, tool không có duration, `frozenSort`, reset freeze khi đổi mode / unmount, qua ngày, lọc project chỉ ở board.
Test: 220 file, 1884 test pass; typecheck 0 lỗi.

STATUS: DONE
