# web-monitor: Monitor, Agent Map, Quỹ đạo, Board, History, Usage

Đường vào: `apps/web/src/features/monitor/index.ts` (`ContextGauge`, `useAgentStatusStore`, `useSnoozeStore`, `useSensitiveStore`, `useSessionActions`, `sortSessions`), `apps/web/src/features/map/index.ts` (`mapTree`, `buildNav`). App nạp lazy `MonitorView`, `CompactView`.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/monitor/components/MonitorView.vue` | màn Monitor (card, danh sách, waiting strip, done tray) |
| `apps/web/src/features/monitor/components/SessionCard.vue` | thẻ session |
| `apps/web/src/features/monitor/lib/monitorRows.ts` | sắp xếp / nhóm dòng session |
| `apps/web/src/features/monitor/composables/useSessionActions.ts` | focus, mở, snooze, dọn |
| `apps/web/src/features/map/components/AgentMap.vue` | Agent Map (Vue Flow): main -> sub-agent, peer edge |
| `apps/web/src/features/map/components/OrbitStarMap.vue` | Quỹ đạo (Orbit / star map) |
| `apps/web/src/features/map/lib/orbitRings.ts`, `starMap.ts`, `mapText.ts` | bố cục + chữ |
| `apps/web/src/features/history/components/HistoryView.vue` | heatmap, timeline, tool stats, tìm prompt |
| `apps/web/src/features/history/stores/history.ts` | dữ liệu History, Away summary |
| `apps/web/src/features/usage/components/UsageView.vue` | chi phí / token theo ngày, model, project, limit plan |
| `apps/web/src/features/usage/stores/usage.ts` | store usage (từ event `usage.*`) |
| `apps/web/src/features/quota/forecast.ts` | dự báo quota |

## Bất biến

- Agent Map cập nhật node tăng dần, không dựng lại cả đồ thị mỗi event; activity list có giới hạn (~200-500).
- Không hiện token count khi không có dữ liệu tin cậy.
- map và monitor import sâu lẫn nhau (chưa qua index vì sẽ tạo vòng).
- Test Office đọc `AgentMap.vue?raw` (`apps/web/src/features/office/components/officeView.test.ts`).

## Liên kết

- Phía core: `core-monitor.md`, `protocol.md`. i18n: `apps/web/src/features/monitor/i18n/`, `features/map/i18n/`. Office: `office.md`. E2E: `agent-map`, `v76`, `v79-shots`.
