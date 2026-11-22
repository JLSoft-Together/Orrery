# monitor

Màn Monitor: session card, danh sách favorites / jobs, compact view, waiting strip, done tray, subagent bar, context gauge / ring, agent status + snooze, hành động session, Sensitive (deny / settings).

- Đường vào: `@features/monitor` (`ContextGauge`, `ContextTokens`, `useAgentStatusStore`, `useSnoozeStore`, `useSensitiveStore`, `useSessionActions`, `sortSessions`...).
- App nạp thẳng (lazy): `components/MonitorView.vue`, `components/CompactView.vue`.
- File chính: `components/MonitorView.vue`, `components/SessionCard.vue`, `lib/monitorRows.ts`, `composables/useSessionActions.ts`.
- map và monitor import sâu lẫn nhau (`../../map/...`, `../../monitor/...`): chưa qua index vì index sẽ tạo vòng.
- i18n: `i18n/{session-v53,monitor-v53,ui-v79-monitor,ui-v88-monitor,agent-status,sensitive-v76}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
