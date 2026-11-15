## WP-4c-mcp: 3 tool MCP `orrery` cho Ổ đĩa

- Mục tiêu (PLAN v101 Q25 "Đọc + đề xuất xoá"): agent trong pane Workspace gọi được:
  1. `disk_search` `{ query?, kind?, minSize?, maxSize?, olderThanDays?, under?, limit ≤ 200 }` → kết quả từ index `core-disk` (`diskMapSearch` / op `disk.search`): path, size, mtime, kind; tổng khớp; chưa có index → lỗi rõ "chưa quét, mở Settings > Data > Ổ đĩa để quét". Không tự quét.
  2. `disk_usage` `{ path, limit ≤ 100 }` → size thư mục + con lớn nhất (op `disk.children` / `disk.sizes`); path ngoài index → lỗi rõ.
  3. `disk_propose_delete` `{ paths: string[] ≤ 200, reason: string ≤ 500, permanent?: false }` → main hiện **dialog native** (tiêu đề nêu tên pane / agent đề xuất, lý do, số mục, tổng size, 5 path đầu) với nút mặc định = Huỷ; đồng ý thì xoá qua đường xoá có guard canonical sẵn có (`runDelete` của `apps/desktop/src/diskMap/delete.ts`, Thùng rác mặc định; `permanent: true` vẫn hỏi lần 2), trả `{ done, failed, freed, cancelled }`. Agent **không bao giờ** xoá mà không qua dialog; không có tham số bỏ qua confirm; giới hạn 3 lần đề xuất / phút mỗi pane.
- Nơi khai báo (theo session orrery-4e): tool MCP khai báo ở `tools.json` (+ instructions) , `defs.rs`, `t_*.rs` và map `tool_action` trong `pane_host.rs` (tool chạy phía page) / `desk_call` (tool chạy phía desk/main) của crate Rust PTY host (`apps/core/crates/core-pty` hoặc `core-desk/src/mcp/`: `rg -n "open_session|who_is_editing" apps/core/crates` để tìm đúng chỗ, làm theo mẫu tool `who_is_editing` / `free_port`). Bên Node khớp: `apps/desktop/src/pty/mcpTools/index.ts` + `mcpDesk.ts` (cho `CCM_CORE_PTY=node`). Tool Ổ đĩa nên chạy phía **desk/main** (không cần page) để chạy được cả khi cửa sổ chưa sẵn; nếu buộc phía page thì lưu ý Tauri chỉ cửa sổ main trả `ccm:orrery`.
- `apps/core/crates/core-desk/src/mcp/registry.rs`: session khác có 2 khối chưa commit (`pub fn reset(&self)` trước `pub(super) fn wait_spawn`, và `#[cfg(test)] mod tests` cuối file). Được sửa chỗ khác trong file nếu thật cần; **không đụng 2 khối đó**. Ghi rõ trong báo cáo các hunk của bạn trong file này để main lọc khi commit.
- Docs: thêm 3 tool vào mục MCP `orrery` trong `docs/claude-code-integration.md` (§11w hoặc mục tool list) + instructions của server (mô tả ngắn, nói rõ propose_delete luôn qua dialog của người dùng).
- Phạm vi file: crate Rust PTY / MCP (tools.json, defs.rs, t_disk.rs mới, map tool_action / desk_call), `apps/desktop/src/pty/mcpTools/**`, `apps/desktop/src/pty/mcpDesk.ts`, `apps/desktop/src/diskMap/**` (thêm hàm cho MCP, đọc lại trước Edit), khối riêng `main.ts`, `docs/claude-code-integration.md`, test (cargo + vitest), E2E `apps/desktop/e2e/v101-mcp-disk.cjs` nếu harness MCP có sẵn mẫu (tìm e2e của `open_session` / `who_is_editing`).
- Cấm đụng: `apps/desktop/src/diskApps/**`, `diskRam/**`, `DiskView.vue` (WP-4b); `apps/desktop/src/diskMap/snapshots.ts`, `schedule.ts`, `apps/desktop/src/disk/**`, Settings disk (WP-4c-snap); `apps/shell/**`; 2 khối registry.rs nói trên; `style.css`, `themes.css`.
- Model: sonnet (opus nếu vòng 1 fail).

## Tiêu chí nghiệm thu

- [ ] Test Rust + Node: schema tool, giới hạn tham số, chưa index → lỗi rõ, propose_delete không có đường bỏ confirm, rate limit 3/phút/pane, path qua guard canonical.
- [ ] `cargo test` các crate đụng tới `-j 8` + clippy sạch; `npm test -w @ccm/desktop` các file liên quan pass; typecheck sạch.
- [ ] E2E / smoke trên **bản dev tách biệt** (cổng riêng, không kill Orrery/Electron/orrery-core đang chạy, không dùng 4317 / 5173): gọi 3 tool qua MCP của pane dev (theo mẫu E2E MCP có sẵn), propose_delete với `CCM_E2E_DISKMAP_CONFIRM` (chỉ bản dev) trên thư mục tạm. Không gọi tool lên Orrery thật.
- [ ] Không comment trong code.
- Báo cáo: `.planning/wp/WP-4c-mcp-report.md` (ghi rõ hunk trong registry.rs nếu có).

## Nhật ký
- 2026-11-15 Rust xong (t_disk.rs, tools.json, parse_desk, deps/desk, tests_disk); Node agent.ts + mcpTools xong; còn test Node, docs, E2E
- 2026-11-15 DONE: test Node 17 + twin 239 + E2E 26/26, báo cáo WP-4c-mcp-report.md
