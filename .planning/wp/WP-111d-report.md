## WP-111d — DONE
- Đã đổi:
  - Mới `apps/core/crates/core-desk/src/preview/wtconfig.rs` (+ `wtconfig_tests.rs`): parse `.orrery/worktree.json`, hash duyệt (`<data>/worktree-approved.json`), gợi ý theo lockfile, `cleanup_target` (chặn dirty / main / unknown), `run_cleanup` (timeout 120 s, kill tree, PowerShell trên Windows, `sh -c` nơi khác). `preview/mod.rs` thêm `pub mod wtconfig`.
  - `preview/exec.rs`: thêm `exec_file_at(dir, ...)` (`exec_file` gọi lại, hành vi cũ giữ nguyên).
  - `preview/ipc_wt.rs`: kênh mới `ccm:wt-config`, `ccm:wt-approve`, `ccm:wt-suggest`, `ccm:wt-cleanup` (phase `check` / `run`).
  - `apps/shell/src/worktree.rs`: `wt.add` dùng `copy` của file (không có thì dùng copy của app) và trả `setup: true`; `wt.remove` nhận `deleteBranch` chạy `git branch -d` (không `-D`), trả `branch: deleted | kept` + `branchDetail`. Test thêm ở `worktree/tests.rs` (thêm mutex SERIAL vì `BUSY` là static toàn cục).
  - `apps/shell/src/preview_text.rs`: 3 kind confirm `wtsetup` / `wtcleanup` / `wtconfig` (en + vi), ngoài danh sách phạm vi nhưng bắt buộc cho native confirm; giữ nguyên phần ghMerge của 111a.
  - `packages/shared/src/pty.ts`: `wt.remove.deleteBranch`, `wt.added.setup`, `wt.removed.branch / branchDetail`.
  - `apps/shell/shim/preload.ts` (+ `shim.js` đã gen lại): `wtConfig`, `wtApprove`, `wtSuggest`, `wtCleanup`.
  - `apps/web/src/core/lib/preview.ts`: type + wrapper `wtConfig / wtApprove / wtSuggest / wtCleanup`, `hasWtFlow` (chỉ thêm, giữ phần 111a).
  - Web mới: `features/git/lib/worktreeFlow.ts` (`joinSetup`, `panesInside`, `cleanupWorktree`), `lib/askCleanup.ts` (`askCleanupWorktree(root, branch)` mount hộp độc lập, WP-111f gọi được ở bất kỳ đâu), `components/WorktreeCleanup.vue`; `WorktreeMenu.vue`: setup sau create, hộp dọn sau merge (ff / squash / giữ conflict), dòng config, nút gợi ý, copy list lấy từ file.
  - i18n `workspace.wt.cleanup.*`, `workspace.wt.config.*`, `workspace.wt.copy.fromFile`, sửa `workspace.wt.merge.done` (bỏ câu hỏi xoá cũ) ở en.ts / vi.ts; snapshot `keys.en.txt` / `keys.vi.txt` thêm tay 37 key mỗi file.
  - Test mới: `worktreeFlow.test.ts`, `WorktreeCleanup.test.ts`; E2E `apps/shell/e2e/v111-wt.cjs`; `.code_index/` (core-desk-preview, web-git, lookup, _meta), index-check 0 lỗi.
- Tiêu chí: 6/6 đạt.
  1. Rust test parse config (thiếu field, sai kiểu, JSON hỏng, BOM, cap) + hash duyệt đổi khi nội dung đổi + gợi ý theo lockfile (8 ca).
  2. Remove bị chặn khi dirty (test shell cũ + `cleanup_target` dirty) / main / unknown; chưa merge: worktree vẫn xoá, nhánh GIỮ và báo (xem Review).
  3. `run_cleanup` timeout test (700 ms, kill tree, < 15 s) + dừng ở lệnh lỗi đầu tiên.
  4. vitest hộp dọn (4 ca) + flow (9 ca).
  5. E2E `v111-wt.cjs` 16/16.
  6. cargo test core (380 + các bộ khác) và shell (608) pass; `npm run typecheck` sạch.
- Audit: nền `worktree.rs` / `wtcopy.rs` / `worktrees.rs` / `WorktreeMenu.vue` đọc đủ; WorktreeMenu đã có xác nhận xoá cũ sau merge nên thay bằng hộp mới; menu xoá tay (thùng rác) giữ nguyên, không xoá nhánh.
- Review:
  - `git branch -d` của squash merge luôn "chưa merge" theo git, nên sau squash worktree bị xoá còn nhánh được giữ + thông báo (diễn giải "chưa merge thì báo, không xoá" theo hướng an toàn: commit không mất). ff thì xoá được cả nhánh (E2E kiểm).
  - Lệnh `setup` / `cleanup` từ file trong repo: native confirm lần đầu mỗi repo và mỗi khi nội dung đổi (hash riêng cho setup và cleanup). Windows nối bằng `a; if ($?) { b }` (PowerShell 5.1), nơi khác `&&`.
  - Pane thuộc worktree đóng bằng `workspace.requestClose(keys, () => false, { record: false })` rồi `wt.remove` thử lại tối đa 7 lần (400 ms) khi còn `busy` vì pty chưa thoát.
  - Cleanup chạy khi worktree sạch; nếu lệnh cleanup tự làm worktree dirty thì `git worktree remove` báo dirty, không xoá.
  - Chưa phủ: lệnh nhiều dòng trong `setup` (gõ nguyên một dòng vào pane), duyệt khi `.orrery/worktree.json` nằm trong worktree chứ không ở main (chỉ đọc ở main).
- Test:
  - `cargo test --locked -j 4 -p core-desk wtconfig` -> 11 passed.
  - `cargo test --locked -j 4` (apps/core) -> không có FAILED, core-desk 380 passed.
  - `cargo test --locked -j 4` (apps/shell) -> 608 passed, 4 ignored.
  - `npm run typecheck` -> sạch; `npm test -w @ccm/web -- src/i18n src/importRules.test.ts src/features/git` -> 21 file, 129 passed.
  - `node apps/shell/e2e/v111-wt.cjs` (cổng 4391 / 9391, `CCM_E2E_OUT` tạm, `CCM_WEB_DIST` tạm) -> 16/16 PASS.
- Nhìn: `docs/qa/v111/WP-111d/v111d-cleanup.png` (hộp dọn), `v111d-menu.png` (menu có dòng config), `v111d-menu-suggest.png` (nút gợi ý). Chụp sau lần sửa code cuối. E2E không ghi đè ảnh docs/qa đã track.
- Hot file cần thêm: không (dùng `workspace.runScript`, `requestClose`, `openFile`, `panes`, `launch` có sẵn).
- Ngoài phạm vi cần main xử lý:
  - `preview_text.rs`, `pty.ts`, `exec.rs` được sửa nhẹ như trên.
  - WP-111f gọi `askCleanupWorktree(root, branch)` từ `features/git/lib/askCleanup.ts` (trả `'removed' | 'kept' | 'failed'`); chưa export qua `features/git/index.ts` vì file đó có thể do WP khác sửa.
  - Docs integration (`docs/`) chưa ghi kênh `ccm:wt-*` mới.
- Commit message đề xuất: feat(worktree): .orrery/worktree.json setup in a new shell pane, cleanup box after merge, suggest button
- Dòng changelog (en / vi, ≤ 100 ký tự): Worktrees set up and clean up by themselves via .orrery/worktree.json / Worktree tự setup và tự dọn theo .orrery/worktree.json

## Nhật ký
- Đọc brief + RULES, audit worktree.rs / wtcopy.rs / worktrees.rs / WorktreeMenu.vue.
- Rust: wtconfig.rs + IPC + shell add/remove + confirm text; test pass.
- Web: preview.ts / preload / worktreeFlow / WorktreeCleanup / askCleanup / WorktreeMenu / i18n / snapshot; vitest + typecheck pass.
- E2E v111-wt 16/16; ảnh vào docs/qa/v111/WP-111d; code_index + index-check ok.
