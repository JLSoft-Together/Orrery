WP-3b: ĐẠT
File mới: lib/next/workspaceBridge.ts (+ test), components/next/ui/PaneLayoutMore.vue.
File sửa: WorkspaceView.vue (đăng ký / huỷ cầu nối ở mount / unmount), LiveRail.vue, composables/useRailDrag.ts + useClusterDrag.ts (thêm `cancel`), FilePane, ChangesPane, LogPane, RunPane, BrowserPane (prop `paneKey`, D11), lib/workspaceKeys.ts (+ layoutPaneMenu.test.ts).
1. Cầu nối `workspaceBridge` (shallowRef) mang `sideEvents` cũ (run, scripts, layouts, split, explorer, mission) + `projectDrag` + `paneDrag`. Menu row next đủ "Chia phải / dưới", "Hiện trong Explorer"; menu project đủ Run, Scripts, Layouts, Plan, Mission, Explorer (async như sidebar cũ). Không có cầu nối thì các mục đó ẩn.
   Kéo: row (kind pane) và head project arm pointerdown; con trỏ ra khỏi rect rail thì `railDrag.cancel()` + `clusterDrag.cancel()` rồi gọi `paneDrag` / `projectDrag` (chỉ bắt đầu khi vào canvas). Trong rail vẫn `useRailDrag` / `useClusterDrag`. Nhóm Trạng thái: chỉ kéo row ra canvas / tab, không đổi thứ tự project. Row kind tab không kéo ra canvas.
2. D11: FilePane, ChangesPane nối vào `headMore`; LogPane, RunPane, BrowserPane chưa có "..." nên thêm `PaneLayoutMore` (chỉ hiện ở next). WorkspaceView truyền `pane-key`.
3. `workspaceAction(..., next = false)`: Ctrl+Alt+T / Alt+Cmd+T chỉ ở next; WorkspaceView truyền `isNext`. Classic y 4.0.5.
Typecheck: pass. Test: 631 pass / 0 fail (vitest lib/next, rail, components/next, workspace, stores, layoutPaneMenu, workspaceKeys).
Rủi ro: chưa chạy E2E / xem bằng mắt kéo row ra canvas (cần kiểm tay: ghost của railDrag bị huỷ đúng lúc ra khỏi rail).
