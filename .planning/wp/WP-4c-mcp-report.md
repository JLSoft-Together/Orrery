## WP-4c-mcp: DONE

**Đã đổi**
- Rust (`apps/core/crates/core-desk/src`): `mcp/t_disk.rs` (mới: 3 tool, validate, rate `disk-delete` 3/60s, `disk-read` 30/60s), `mcp/tools.json` (+3 tool, instructions), `mcp/{mod,tools,pane_host,parse_desk}.rs`, `mcp/tests_disk.rs` (mới), `mcp/tests_rpc.rs` (18 -> 21 tool), `mcp_desk/{desk,deps,tests}.rs` (Deps::disk -> host `mcpDesk.disk`).
- registry.rs: CHỈ 1 hunk, hàm `desk_reply_ms` (dòng 21): `"free_port" | "telegram_receive" | "disk_propose_delete" => 600_000`. Không đụng `reset()` và `mod tests` của orrery-4e.
- Node: `diskMap/agent.ts` (mới, `createDiskAgent`), `diskMap/ipc.ts` (`removeWith`, `isDeleting`, `remove` gọi lại `removeWith`), `diskMap/delete.ts` (`firstCount`), `diskMap/dialog.ts` (`agentDeleteBoxOptions`), `strings.ts` (en + vi `agentTitle/agentDetail`), `mcpDesk.ts` + `mcpDeskHosts.ts` (`disk` dep, host `mcpDesk.disk`), `pty/mcpTools/{diskCommon,diskSearch,diskUsage,diskProposeDelete,index}.ts`, `pty/mcp.ts` (instructions), `pty/broker.ts` (deskReplyMs 600s), `main.ts` (import + `deskDeps.disk` + khối `diskAgent` cạnh `deskDeps`, dialog native, restore cửa sổ nếu ẩn/minimize).
- Shared: `packages/shared/src/mcpDesk.ts` (call/result/lỗi mới `not-indexed|not-in-index|scanning|invalid`, sanitize), `orreryApi.ts` (`disk.search|usage|proposeDelete`, `MCP_TOOL_ACTION`).
- Test/E2E/docs: `test/mcp-disk.test.ts` (mới), `test/mcp.test.ts` + `test/mcp-sessions.test.ts` (danh sách tool), `e2e/v101-mcp-disk.cjs` (mới), `docs/claude-code-integration.md` (mục Disk tools trong §11w).

**Audit**: đường xoá duy nhất = `DiskMapService.removeWith` -> `runDelete` (guard canonical, `known` phải trong index, confirm trước mọi thao tác, Thùng rác mặc định, permanent hỏi lần 2, default/Esc = Huỷ). Schema đóng, không có tham số bỏ qua confirm, tham số thừa bị bỏ. Chỉ bản chưa đóng gói + `CCM_E2E_DISKMAP_CONFIRM=1` mới auto-confirm (y như Disk view). Một lượt xoá tại một thời điểm (Disk view + mọi pane) -> `busy`.

**Tiêu chí 4/4**: (1) test schema/giới hạn/chưa index/không đường bỏ confirm/rate 3 mỗi phút/guard: đạt. (2) cargo + vitest + typecheck: đạt (xem dưới). (3) E2E bản dev tách biệt 26/26. (4) không comment trong code.

**Test (lệnh -> kết quả thật)**
- `cargo test -p core-desk -j 8 --lib mcp` -> 57 passed (có `tests_disk` 8 test); `--lib mcp_desk` -> 10 passed. Cả lib: 324 passed, 1 fail `telegram::tests::episodes_go_to_the_topic_with_buttons_only_when_receiving` (flaky, chạy riêng pass, không liên quan).
- `cargo clippy -p core-desk -j 8 --all-targets` -> không cảnh báo nào ở file của gói (còn warning dead-code cũ ở file khác).
- `npm run typecheck` -> sạch (shared, collector, web, desktop).
- `npm test -w @ccm/desktop -- test/mcp-disk.test.ts` -> 17 passed. `test/mcp test/orrery-api test/pty-broker test/desk-mcp test/diskMap` với `CCM_CORE_BIN` = binary mới -> 239 passed (twin Rust vs TS khớp instructions + danh sách tool).
- E2E: `CCM_CORE_BIN=<scratch>/t4cmcp/debug/orrery-core.exe node apps/desktop/e2e/v101-mcp-disk.cjs` (cổng 4473 / debug 9473, data riêng, không đụng Orrery thật) -> 26/26: not-indexed không tự quét, tìm Unicode, minSize/age, usage, not-in-index, 200 path bị từ chối không dialog, 201 path invalid, propose auto-confirm vào Thùng rác + history + index forget, lần 4 trong phút = rate.

**CHƯA KIỂM**
- Dialog native thật (nút Huỷ mặc định, hiện lại cửa sổ minimize/ẩn): chỉ test unit option hộp thoại + logic gọi; E2E dùng auto-confirm nên không bấm Huỷ thật.
- Tauri shell: `apps/shell/src/hosts.rs` chưa có handler `mcpDesk.disk` (cấm đụng) -> 3 tool trả `unavailable` trên shell, cần WP riêng.
- `desk-mcp.twin.test.ts` fail nếu `apps/core/target/release/orrery-core.exe` cũ (chưa build lại): cần rebuild core (hoặc `CCM_CORE_BIN`). `desk-agent-data.twin` fail EPERM symlink (quyền Windows, có sẵn).
- `.code_index/desktop.md` chưa cập nhật (file đang bị session khác sửa), main nên thêm 1 dòng: diskMap/agent.ts + mcpTools/disk*.ts.
- `stage/` đã build lại bằng `CCM_SKIP_CORE_BUILD=1 node build.mjs` (không build core vào target chính).

**Commit message đề xuất**: feat(mcp): add disk_search, disk_usage and disk_propose_delete tools to the orrery MCP server, read-only on the disk index and delete only after a native confirm through the guarded runDelete
