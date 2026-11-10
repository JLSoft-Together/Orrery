# WP-v87-6-D8 — báo cáo (2026-11-11)

MCP server `orrery` (JSON-RPC + 19 tool), phần main của tool desk (`mcpDesk`, `agentPorts`, `mcpShot`) và `agentData` sang Rust. Vỏ Tauri chạy MCP không cần Node. Electron giữ đường TS: mặc định node, bật Rust bằng `CCM_DESK_MCP=rust` và `CCM_DESK_AGENT_DATA=rust`. Chưa build release, chưa chạy E2E.

## Đã làm

### Rust `core-desk/src/mcp/` (MCP framework + tool)
- `rpc.rs`: `handle_mcp` và `mcp_gate` port 1:1 từ `pty/mcp.ts`.
  - batch max 16, `initialize` / `ping` / `tools/list` / `tools/call`, mã lỗi -32600 / -32601 / -32602;
  - gate: path `/mcp`, Host `127.0.0.1:<port>`, không Origin, POST, `Bearer <64 hex>`.
- `defs.rs` + `tools.json`: định nghĩa tool sinh từ TS bằng `node scripts/gen-mcp-tools.mjs`. `--check` báo file cũ. DeepSeek tắt thì bỏ `hand_back` và dùng `open_session` không có deepseek, giống `mcpToolsFor(false)`.
- Tool:
  - `t_open.rs`: `open_session`;
  - `t_notes.rs` + `notes.rs`: `notify_owner`, `NoteQueue` (6 / phút, gộp "+N");
  - `t_files.rs`: `open_in_workspace`, `who_is_editing`, `hand_back`;
  - `t_sessions.rs`, `t_run.rs`, `t_desk.rs`, `t_telegram.rs`, `t_common.rs`: các tool còn lại.
- `paths.rs`: `resolve_inside`. Chặn UNC, tên device trên Windows, link trỏ ra ngoài thư mục project, link tới share.
- Kiểm kết quả trả về: `parse_page.rs` / `parse_run.rs` / `parse_desk.rs` (= `parseMcpPageResult` / `parseMcpRunResult` / `parseMcpDeskResult`).
- `text.rs`: `clean_note_text`, `clean_pane_prompt`, `clean_run_text`, đếm UTF-16 như JS. `mask_run` dùng `core_transcript::mask::mask_log` (mới, port `createLogMasker`).
- `server.rs` `McpServer`:
  - HTTP/1.1, chỉ bind `127.0.0.1:0`, có keep-alive và chunked;
  - trả 401 trước khi đọc body;
  - body max 64 KB (413), max 64 kết nối, idle 120 s.
- `registry.rs` + `open.rs` + `pane_host.rs` (cho Tauri, bản thay `PtyBroker`):
  - token 32 byte mỗi pane; token chỉ dùng được khi pane đã spawn và chưa exit;
  - mỗi pane có rate riêng; `NoteQueue`; ticket `open_session` 60 s;
  - `open_session`: 20 lần / phút mỗi pane; live + ticket < 16; chờ spawn 10 s; `resume` pane đang chạy thì trả pane đó;
  - `submit` chỉ khi agent truyền `submit: true` và có prompt. Shell pane không có prompt nên không bao giờ submit;
  - `checkout_roots` = `git worktree list --porcelain`;
  - deadline chờ trang 8 s, chờ desk theo `desk_reply_ms` (40 / 200 / 600 s);
  - `notify_owner` thành công thì gửi `agent.note` tới collector.
- `ipc.rs`: kênh `desk:mcp-handle` cho Electron. `ToolHost` của `PtyBroker` TS được trả qua host `mcp.take|open|notify|roots|page|desk`.

### Rust `core-desk/src/mcp_desk/` (subagent)
- Kênh `desk:mcp-desk` và `desk:mcp-screenshot`, args `[call, {deps: host|native}]`.
- `scope.rs`: `pane_scope`, `servers_for`, `plan_free_port`.
- `desk.rs`: `preview_layout`, `free_port`, `telegram_*`, `list_servers`, khoá `confirm-open`.
- `shot.rs`: screenshot.
- `deps: native`: đi thẳng các kênh Rust đã có (`desk:layout-agent-render`, `desk:tg-*`, `ccm:dev-*`, `ccm:capture-*`, `ccm:run-sessions`, `core_procs`).
- `lib.rs` thêm `Caller` để module gọi kênh khác trong desk. `resources.rs` thêm `res_kill_with`.

### Rust `core-desk/src/agent_data/` (subagent)
- 9 kênh `ccm:agentdata-*` và `desk:agentdata-recover`.
- `agent-data-move.json` giữ đúng format TS.
- Host `agentData.confirm` / `pickFolder` / `stopCollector` / `startCollector`.

### TS (Electron)
- `pty/mcpRust.ts`: `rustMcpHandle`, `mcpToolHosts`.
- `pty/mcp.ts`: thêm `McpHttpOptions.handle`. Kết quả null thì chạy TS `handleMcp`. `MCP_INSTRUCTIONS_NO_DEEPSEEK` được export.
- `pty/supervisor.ts`: option `mcpHandle`.
- `main.ts`:
  - `mcpHandle` chạy khi `deskRouter.mode('mcp') === 'rust'`;
  - `viaRustDesk` thay `createDesk` / `mcpScreenshot` bằng `desk:mcp-desk` / `desk:mcp-screenshot` + `mcpDeskHosts.ts`. Desk chết thì chạy lại đường TS.
- `desk/forward.ts`: module `mcp` và `agentData` (mặc định node), event `ccm:agentdata-event`.
- `agentData/ipc.ts`: route qua desk.

### Vỏ Tauri `apps/shell`
- `mcp.rs`:
  - `desk::wire` gọi `mcp::start`;
  - ghi `<data>/orrery-mcp.json` (0600, cùng chuỗi TS, chỉ chứa `${ORRERY_API}` / `${ORRERY_TOKEN}`, không chứa token);
  - `Registry` + `McpServer`;
  - `ShellBridge`: hỏi trang bằng `Shell::ask` `ccm:orrery`, gọi desk native qua `desk::call_inside`, gửi `agent.note` vào core-serve qua `Serve::post`.
- `pty.rs` `resolve`:
  - lấy ticket (session / tên do `open_session` đặt), mint token;
  - env `ORRERY_API`, `ORRERY_TOKEN`, `ORRERY_MCP_CONFIG`;
  - message `spawned` / `pid` / `exited` / `removed` / `cwd` cập nhật registry.
- `presets.rs` `preset_command_with(AgentOpts{mcp, name})`, cùng cờ như `presets.ts`:
  - claude: `--mcp-config` và `-n '<name>'`;
  - codex: `-c mcp_servers.orrery.url=…` và `bearer_token_env_var=ORRERY_TOKEN`.
- `hosts.rs` / `desk_text.rs` (subagent): `mcpDesk.confirmKill`, `mcpDesk.confirm`, `agentData.confirm` / `pickFolder`, câu en/vi lấy từ `strings.ts`.
- Sửa `main.rs` đúng 1 dòng: `mod mcp;`.

## Số test thật (macOS arm64)
- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 876 passed, 0 failed, 6 ignored.
  - Test mới: `mcp::` 37 (`tests_rpc` có server HTTP thật + registry + `open_session` chờ spawn + max 16 pane; `tests_tools` / `tests_desk` của tool), `mcp_desk` 9, `agent_data` 14, `mask_log`.
- `cargo clippy -j 4 --all-targets -- -D warnings` cả `apps/core`: sạch.
- `apps/shell`: `cargo test -j 4` 86 passed, clippy `-D warnings` sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 168 file passed, 1 skipped; 1708 test passed, 29 skipped, 0 failed.
  - Twin mới `desk-mcp.twin.test.ts`: 2 test, khoảng 60 message JSON-RPC (mọi tool, các nhánh lỗi, batch, deepseek bật / tắt, không có desk). So response và chuỗi host call TS ↔ Rust: giống nhau, không fallback.
  - `desk-mcp-desk.twin.test.ts`: 9 test. `desk-agent-data.twin.test.ts`: 5 test.
- `npm run typecheck`: 0 lỗi. `node scripts/gen-mcp-tools.mjs --check`: tools.json khớp.

## E2E cần chạy (chủ nhân, sau build)
1. Electron với `CCM_DESK_MCP=rust` (rồi chạy lại với `node`):
   ```
   CCM_E2E_PORT=4561 CCM_E2E_DBG=9561 node apps/desktop/e2e/v49-open.cjs
   CCM_E2E_PORT=4561 CCM_E2E_DBG=9561 node apps/desktop/e2e/agent-api.cjs
   ```
   Thêm các bộ có MCP: `rg -l "open_session|ORRERY_TOKEN|/mcp" apps/desktop/e2e`.
2. Agent data với `CCM_DESK_AGENT_DATA=rust`: scan, clean, strip, move / undo / delete original, quit giữa lúc move. Biến E2E: `CCM_E2E_AGENT_TARGET`, `CCM_E2E_AGENT_BACKUP`, `CCM_E2E_AGENT_PROCS`, `CCM_E2E_CONFIRM`.
3. Tauri (`npm run shell:dev`, port / data tạm), kiểm tay:
   - Mở pane Claude, gọi `/mcp`: thấy `orrery` connected.
   - `open_session` codex có `submit: true` thì prompt được gửi. Shell pane không bao giờ submit.
   - Gọi `open_session` 21 lần / phút thì bị `rate`. Có 16 pane thì bị `limit`.
   - `notify_owner` hiện toast. `list_sessions` / `send_to_session` / `wait_session` chạy được.
   - `who_is_editing`, `open_in_workspace`, `run_status` / `run_log` chạy được.
   - `telegram_send` có nút (dùng bot test riêng).
   - Deep link `agent-monitor://open`: max 10 / phút (giữ nguyên `links.rs`).
4. Qua E2E thì bỏ `mcp` / `agentData` khỏi `DESK_NODE_DEFAULT`.

## Còn thiếu / rủi ro
- **Tauri `resources.report`**: vỏ chưa có sampling Resources, nên `list_servers` trả `no-report` và `free_port` không chạy. Cần port `resReport` (Electron đang lấy từ collector) hoặc cho vỏ trả host `resources.report`.
- **Tauri agentData**: `stopCollector` / `startCollector` ở vỏ chưa làm gì. Trên Windows, move có thể ra `locked` khi watcher của core-serve giữ handle.
- Trong Tauri, page action đi thẳng từ Rust tới trang (`ccm:orrery`) và không qua `ORRERY_ACTIONS.parse` như `createOrrery` TS. Args do chính tool Rust dựng nên đã đúng shape; trang vẫn tự kiểm như cũ.
- Tauri chưa có: shim claude/codex cho shell pane, `wt.*` (worktree), theo dõi nhánh git, DeepSeek (`hand_back`), `CCM_DEEPSEEK_FEATURE`.
- `NoteQueue` Rust dùng thread sleep cho timer; registry giữ `Arc` vòng qua closure của `NoteQueue`, được dọn khi pane removed hoặc khi dispose.
- `who_is_editing` so `Date.parse` bằng `core_transcript::time::parse_iso`, chỉ nhận ISO (trang chỉ gửi ISO).
- Code Windows (`cfg(windows)`: worktree path `\`, device names, junction của agentData) mới đọc, chưa biên dịch trên Windows.
- `main.ts` bị subagent agentData sửa bằng script thay chuỗi trên cả file; con đã kiểm lại các đoạn MCP còn nguyên và typecheck / test đều pass.

STATUS: PARTIAL
