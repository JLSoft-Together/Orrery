# core-desk-mcp: MCP server `orrery`

Đường vào: `apps/core/crates/core-desk/src/mcp/tools.json` (nguồn danh sách tool, đọc ở `mcp/defs.rs::tool_defs`), server `mcp/server.rs::McpServer`, đăng ký pane `mcp/registry.rs::Registry`. Vỏ chạy server trong tiến trình: `apps/shell/src/mcp.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-desk/src/mcp/tools.json` | tên, mô tả, schema từng tool |
| `apps/core/crates/core-desk/src/mcp/tools.rs` | gọi tool theo tên |
| `apps/core/crates/core-desk/src/mcp/t_sessions.rs` | `list_sessions`, `send_to_session`, `close_session`, `wait_session` |
| `apps/core/crates/core-desk/src/mcp/t_open.rs` | `open_session` |
| `apps/core/crates/core-desk/src/mcp/t_files.rs` | `open_in_workspace`, `who_is_editing` (trang trả lời, đọc lại ở `parse_page.rs`) |
| `apps/core/crates/core-desk/src/mcp/t_run.rs` | `run_app`, `run_status`, `run_log`, `device_screenshot` (đọc lại ở `parse_run.rs`) |
| `apps/core/crates/core-desk/src/mcp/t_disk.rs` | `disk_search`, `disk_usage`, `disk_propose_delete` |
| `apps/core/crates/core-desk/src/mcp/t_telegram.rs` | `telegram_notify`, `telegram_send`, `telegram_receive` |
| `apps/core/crates/core-desk/src/mcp/t_desk.rs` | `preview_layout`, `list_servers`, `free_port` |
| `apps/core/crates/core-desk/src/mcp/pane_host.rs` | cầu nối tới pane: tên tool → method trang (`workspace.show`, `file.editors`, `device.screenshot`...) |
| `apps/core/crates/core-desk/src/mcp_desk/desk.rs` | tool chạm desk (screenshot thiết bị, layout) |
| `apps/shell/src/mcp.rs::start` | ghi `<data>/orrery-mcp.json`, gắn `Registry` + `McpServer` |

## Bất biến

- Gate `/mcp`: Host loopback, không Origin, POST, Bearer 64 hex (token riêng mỗi pane), 401 trước khi đọc body, body max 64 KB.
- `open_session` chỉ prefill prompt, không submit trừ khi agent truyền `submit: true` (ngoại lệ chủ nhân duyệt, docs §11w); link `agent-monitor://` không bao giờ submit.
- `disk_propose_delete` chỉ đề xuất; xoá luôn qua dialog native.

## Liên kết

- Thêm tool: sửa `tools.json`, thêm handler `mcp/t_*.rs`, test `mcp/tests_tools.rs`; tên tool phía web `packages/shared/src/mcpCall.ts`, `mcpDesk.ts`, `mcpRun.ts`.
- Docs: `docs/claude-code-integration.md` §11w. Plan: `.planning/PLAN-v49-*.md`. E2E: `v49-open`, `v80-mcp`.
