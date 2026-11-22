# app

Tầng app: file dùng chung nhưng phải import nhiều feature. Được import `core` và `features`; core không được import vào đây.

- `stores/connection.ts`: WebSocket client, snapshot / event vào các store core; còn gọi thẳng history, resources, monitor (sensitive, snooze), usage. Feature mới nghe event qua `core/stores/connectionEvents.ts` thay vì sửa file này.
- `composables/useNotifications.ts`, `useAppToasts.ts`, `useMcpCalls.ts`: gom thông báo / toast / lệnh MCP của nhiều feature.
- `components/AppHeader.vue`, `HeaderStatusPopover.vue`, `ZoomHud.vue`.
- `main.ts` và `App.vue` vẫn ở `src/` (index.html nạp `/src/main.ts`).
- Feature còn import ngược vào đây (chủ yếu `stores/connection`): cho phép tạm, test `importRules` chỉ cảnh báo.
