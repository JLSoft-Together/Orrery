# protocol: packages/shared

WebSocket dashboard (core-http <-> web) và các kiểu dùng chung cho IPC / PTY. Đường dẫn tính từ `packages/shared/src/`. Union chính: `types.ts::MonitorEvent` (server -> client), `types.ts::ClientMessage` (client -> server), `types.ts::ServerMessage`.

## Bất biến

- Kết nối mới nhận 1 `snapshot`, rồi `ServerMessage {seq, events[]}` theo batch 50 ms. Snapshot `seq` = seq cuối đã flush, message kế là seq+1; lệch thì client gửi `resync`.
- Reply trực tiếp: `ServerMessage.direct = true`, `seq: -1`, chỉ gửi socket hỏi; web áp không kiểm seq.
- Batch gộp `terminal|agent|link.updated` theo id (field cuối thắng, gộp vào `created` đang chờ, không gộp qua `removed`). `Patch<T>`: `null` = xoá field.
- Event / field lạ bị bỏ qua; status lạ map về `unknown` (`status.ts`).
- ID terminal / session lấy từ process / session, không từ title. `TerminalSession.alias` = tên đặt ở dashboard, `title` = tên registry.
- `ActivityEvent.data` chỉ chứa nhãn, không có tham số tool thô. Ngoại lệ: `tool.span` mang `path` + `access` của Edit / Write / MultiEdit / NotebookEdit / Read (docs §11ar).
- Token: input = input + cache_creation; cacheRead riêng; total = input + output; dedupe theo `message.id`.

## Event server -> client (nhóm)

| Nhóm | Event | Người phát (Rust) | Người nhận (web) |
|---|---|---|---|
| Vòng đời | `snapshot`, `sessions.loading` | `core-http` + `Store` | `apps/web/src/app/stores/connection.ts` |
| Terminal / agent | `terminal.*`, `agent.*`, `link.*` | `core-monitor` qua `Store` | `apps/web/src/core/stores/terminals.ts`, `agents.ts`, `peers.ts` |
| Hoạt động | `activity`, `tool.span` | `core-monitor` | `activity.ts`, `toolSpans.ts` |
| Usage / limit | `usage.reset|updated|scan|roots`, `limits.updated`, `models.updated` | `core-monitor/src/usage.rs` | `apps/web/src/features/usage/stores/usage.ts` |
| Favorites / lịch sử | `favorites.updated`, `history.*`, `heatmap.*`, `timeline.*`, `toolstats.*` | `core-store/src/persist/` | feature `history` |
| Jobs / sensitive | `jobs.updated`, `sensitive.*`, `resources`, `diagnostics.*` | `core-sources`, `core-serve` | feature `monitor`, `resources` |
| Thông báo | `notify.*`, `snooze.updated`, `response.updated` | `core-toast` | `apps/web/src/app/composables/useNotifications.ts` |

## Client -> server (nhóm)

`resync`, `terminal.open|alias|note|snooze|dismiss`, `favorite.*`, `folder.pick`, `job.stop|dismiss`, `notify.*`, `sensitive.*`, `shell.done`, `client.presence`, `*.get` (history / heatmap / timeline / toolstats / diagnostics). Parse + validate: `apps/core/crates/core-http/src/client/mod.rs`; xử lý: `apps/core/crates/core-serve/src/dispatch.rs`.

## File theo miền

| File | Nội dung |
|---|---|
| `packages/shared/src/types.ts` | `TerminalSession`, `Agent`, `MonitorEvent`, `ClientMessage` |
| `packages/shared/src/ptyControl.ts`, `pty.ts` | hợp đồng main <-> core-pty, khung WS `/pty` |
| `packages/shared/src/orreryApi.ts` | API trang (`window.ccmDesktop`) dùng chung |
| `packages/shared/src/remote.ts`, `phone.ts` | giao thức Remote / điện thoại |
| `packages/shared/src/telegram.ts` | kiểu Telegram |
| `packages/shared/src/mcpCall.ts`, `mcpDesk.ts`, `mcpRun.ts` | tool MCP `orrery` |
| `packages/shared/src/secretMask.ts`, `sensitive.ts`, `status.ts` | mask, quy tắc nhạy cảm, chuẩn hoá status |

## Liên kết

- Test khoá Rust với TS: `apps/core/crates/core-store/tests/shared_ts.rs`. Thêm kiểu mới: sửa TS trước, rồi `core-store/src/model.rs`. Bản chi tiết cũ: `.code_index/archive/protocol-full.md`.
