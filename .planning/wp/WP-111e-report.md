## WP-111e — DONE
- Đã đổi:
  - Mới `apps/core/crates/core-desk/src/preview/checklist.rs` (+ `checklist_tests.rs`): store `<data>/checklists/<sha256 16 hex của đường dẫn worktree chuẩn hoá>.json` `{items:[{id,text,done,by,at}], check:{script,state,at,head}, seq}`, ghi nguyên tử, file hỏng / quá 256 KB đọc như rỗng, max 50 việc, text 1 dòng ≤ 200 (dùng `clean_note_text`); `remove` từ chối `by: owner` khi người gọi là agent; `worktree_of` chỉ nhận worktree nằm trong `git worktree list` của repo đã duyệt; `reply` cho IPC trả thêm `head` hiện tại. `preview/mod.rs`: `pub mod checklist` + `set_data_dir`.
  - `preview/ipc_wt.rs`: kênh `ccm:checklist` (op list / add / toggle / remove / setScript / setCheck; `dir` = pane đã duyệt, `wt` = worktree).
  - MCP: `mcp/t_todo.rs` (`todo_add`, `todo_done`, `todo_list`), `mcp/tools.json` (+3 tool), `mcp/tools.rs`, `mcp/mod.rs`, `mcp/host.rs` (`ToolHost::checklist_dir`, mặc định data dir toàn cục), worktree = `git rev-parse --show-toplevel` của cwd pane gọi, rate 20 / phút, việc agent thêm `by: agent`. Không có tool xoá nên agent không xoá được việc owner. `tests_tools.rs` (Fake có `data`), `tests_rpc.rs` (21 -> 24 tool), `tests_todo.rs` mới.
  - `apps/shell/src/worktree.rs`: `wt.remove` nhận `squashed`; `git branch -d` từ chối thì chỉ khi `git diff --quiet <main> <branch>` rỗng mới `-D`, còn khác giữ nhánh. Test `worktree/tests.rs` (squash rỗng -> deleted; nhánh khác -> kept; không `squashed` -> kept).
  - Web: `core/lib/preview.ts` (type + `checklist(dir, wt, req)`, `hasChecklist`), `shim/preload.ts` (+ `shim.js` gen lại), `packages/shared/src/pty.ts` (`squashed`), `features/git/lib/checklist.ts` (+ test), `components/WorktreeChecklist.vue`, `WorktreeMenu.vue` (checklist trên nút Merge, hộp xác nhận "Vẫn merge" khi còn việc / check lỗi, truyền `squashed`), `WorktreeCleanup.vue` + `lib/askCleanup.ts` (`{squashed}`), i18n `workspace.wt.checklist.*` en + vi (27 key, snapshot thêm tay).
  - E2E mới `apps/shell/e2e/v111-checklist.cjs` (13 kiểm); `.code_index/` (core-desk-preview, core-desk-mcp, web-git, lookup, _meta).
- Tiêu chí: 4/4 đạt.
  1. Rust test store (crud, quyền xoá owner / agent, giới hạn, file hỏng, check, ipc reply, toplevel, worktree_of) + MCP (`tests_todo.rs`: add / list / done, theo worktree, input sai, rate, không có tool xoá).
  2. vitest `checkView` (cũ khi HEAD đổi, running chỉ khi sống), `mergeWarning`, `runCheck`, `waitShellResult` (26 test file git).
  3. E2E (file riêng `v111-checklist.cjs`, không sửa `v111-wt.cjs`): thêm việc, chạy check thật `npm run qa` trong pane shell ra pass (OSC 633 exit 0), commit mới -> cũ, fail.flag -> fail, merge squash báo còn việc + check lỗi, Quay lại không merge, "Vẫn merge" squash vào main, hộp dọn xoá cả worktree và nhánh (`-D`).
  4. `cargo test --locked -j 4` core (core-desk 403) và shell (620) pass; `npm run typecheck` sạch.
- Audit: WorktreeMenu / worktreeFlow / askCleanup của 111d, `workspace.runScript`, `useShellDone` (shellLast), `scriptsIn`, MCP registry / PaneHost (cwd là root của pane) đọc đủ.
- Review:
  - `ccm:checklist` nhận `dir` là pane đã duyệt + `wt` phải có trong `git worktree list` của repo đó (worktree anh em không nằm trong root pane nên `check_dir` trực tiếp bị `denied`; lỗi này E2E bắt được lần đầu).
  - Menu đóng khi pane check mở ra; `runCheck` chạy ngoài component, mở lại menu thấy kết quả (`onCheckDone` làm mới nếu menu mở lại giữa chừng). Trạng thái `running` mà app không còn run sống (tắt app giữa chừng) hiện là "chưa chạy".
  - Kết quả cần shell integration bật; tắt thì hiện câu nhắc, check ở `running` đến khi pane đóng (về `idle`).
  - Squash `-D` theo đúng chữ: chỉ khi `git diff <main> <branch>` rỗng; nếu main đã có commit khác sau squash thì nhánh vẫn được giữ (an toàn, không mất commit).
  - Chưa phủ: agent đánh dấu xong việc `owner` được phép (chỉ xoá bị chặn); MCP chưa có E2E qua HTTP thật (chỉ test Rust với Fake host); script check nhiều dòng.
- Test:
  - `cargo test --locked -j 4` (apps/core) -> pass, core-desk 403 passed; (apps/shell) -> 620 passed, 4 ignored.
  - `npm run typecheck` -> sạch; `npm test -w @ccm/web -- src/features/git src/i18n src/importRules.test.ts` -> 26 file, 182 passed.
  - `node apps/shell/e2e/v111-checklist.cjs` (cổng 4392 / 9392, `CCM_E2E_OUT` + `CCM_WEB_DIST` tạm) -> 13/13 PASS.
- Nhìn: `docs/qa/v111/WP-111e/v111e-checklist.png` (đạt), `v111e-checklist-fail.png`, `v111e-merge-confirm.png`, `v111e-squash-removed.png`. Chụp sau lần sửa code cuối (chỉ có sửa .code_index sau đó). E2E không ghi đè ảnh docs/qa đã track.
- Hot file cần thêm: không (đã sửa `tools.json`, `preview.ts` với tư cách chủ; `preload.ts` chỉ thêm 1 dòng `checklist`).
- Ngoài phạm vi cần main xử lý: `packages/shared/src/pty.ts` thêm `squashed`, `i18n/en.ts` / `vi.ts` + snapshot (chỉ dòng của WP); docs integration chưa ghi kênh `ccm:checklist` và 3 tool MCP `todo_*` (docs/claude-code-integration.md §11w).
- Commit message đề xuất: feat(worktree): pre-merge checklist with check script and MCP todo tools, drop squashed branches after merge
- Dòng changelog (en / vi, ≤ 100 ký tự): Pre-merge checklist per worktree, agents can add todos / Checklist trước merge theo worktree, agent thêm được việc
