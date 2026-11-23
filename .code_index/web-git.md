# web-git: Diff, Git Log, Changes, commit, worktree

Đường vào: `apps/web/src/features/git/index.ts` (`CommitBox`, `DiffView`, `PushDialog`, `useTurnChanges`, `useFileChanges`, `parseUnifiedDiff`, `pathKey`, `useCheckpointsStore`). App nạp thẳng `ChangesPane`, `SplitDiff`, `components/log/LogPane`, `Worktree*` (giữ chunk lazy).
Phía desktop: kênh `ccm:git-*`, `ccm:changes-*` -> `apps/core/crates/core-desk/src/preview/` (xem `core-desk-preview.md`).

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/git/components/ChangesPane.vue` | pane Changes: stage / discard / commit |
| `apps/web/src/features/git/components/DiffView.vue` | diff inline (`SplitDiff` là bản chia đôi) |
| `apps/web/src/features/git/components/log/LogPane.vue` | Git Log: đồ thị commit, refs, filter, Push dialog |
| `apps/web/src/features/git/composables/useGitLog.ts` | nạp log theo trang, cache theo root |
| `apps/web/src/features/git/lib/diff.ts` | `parseUnifiedDiff` |
| `apps/web/src/features/git/components/PrPanel.vue` | dải PR + CI trong Git Log: tạo PR (`PrCreateDialog`), checks, log job lỗi, Gửi lỗi cho agent, Merge (`PrMergeDialog`) |
| `apps/web/src/features/git/lib/prView.ts` | thuần: `ciLook`, `panelState`, `chipState`, `toastParts`, `readMethod` / `writeMethod`, `errorKey` |
| `apps/web/src/features/git/lib/prAgent.ts` | pane agent của project, body PR từ `exportChat(short)`, `errorPrompt` |
| `apps/web/src/features/git/stores/ci.ts` + `components/CiChip.vue` | trạng thái CI theo project cho chấm trên rail (pending / đỏ / xanh 10 phút) |
| `apps/web/src/features/git/composables/useCiEvents.ts` | nghe `onGhEvent`: toast + OS notification khi CI xong, nuôi store CI (một listener dùng chung) |
| `apps/web/src/features/git/lib/diffBlocks.ts` | tách hunk cho DiffView / SplitDiff |
| `apps/web/src/features/git/lib/changesView.ts` | gom file thay đổi |
| `apps/web/src/features/git/components/log/StashMenu.vue` | dropdown stash trong header Git Log: list, Apply, Pop, Drop (confirm native); đếm không poll, nạp khi mở |
| `apps/web/src/features/git/components/StashDialog.vue` | hộp tên stash + checkbox untracked, mở từ menu `...` của Changes (`ChangesPane.vue::askStash`) |
| `apps/web/src/features/git/lib/stash.ts` | `defaultStashName`, `stashErrorKey`, `undoErrorKey`, `canOfferUndo` (HEAD, 1 parent, chưa có trên remote); `LogPane.vue::undoLast` / `redoCommit` = menu commit gần nhất + thanh Commit lại |

## Bất biến

- Xoá nhánh chỉ sau confirm native (docs §11s). Push dùng tài khoản GitHub của repo (v58), token không bao giờ về trang.
- Commit / push bị chặn khi quét ra token (secret guard, `web-small.md`).
- `highlight.worker.ts` của files import thẳng `diffBlocks`: không đưa lib này phụ thuộc index git.
- Worktree setup / dọn (v111 G2): `lib/worktreeFlow.ts` (`joinSetup`, `cleanupWorktree`), `components/WorktreeCleanup.vue`, `lib/askCleanup.ts::askCleanupWorktree(root, branch)` mount hộp độc lập (WP-111f gọi sau merge PR); `WorktreeMenu.vue` có dòng config + nút gợi ý. Setup chạy bằng `workspace.runScript` sau native confirm `wtApprove`.

- Checklist trước merge (v111 G3): `components/WorktreeChecklist.vue` nằm trên nút Merge trong `WorktreeMenu.vue` (list việc, thêm / tick / xoá, chọn script từ `scriptsIn`, mặc định `qa`, nút "Chạy check"); `lib/checklist.ts`: `checkView` (kết quả gắn HEAD, commit mới = `stale`, `running` chỉ khi chạy sống trong app), `mergeWarning`, `runCheck` (mở shell pane bằng `workspace.runScript`, `waitShellResult` đợi `shellLast.code` từ OSC 633), `onCheckDone`. Merge không chặn: còn việc / check lỗi thì hiện hộp xác nhận "Vẫn merge". Sau squash, `askCleanupWorktree(root, branch, {squashed})` cho phép xoá nhánh bằng `-D` khi nội dung đã nằm trong main.

## Liên kết

- i18n: `apps/web/src/features/git/i18n/`. Plan: `.planning/PLAN-v30-git-log.md`, `PLAN-v36-log-branches-toolbar.md`, `PLAN-v58-github-accounts.md`. E2E: `log`, `split-diff`, `diff`, `v103-log-diff`, `github-accounts`.
- Phụ thuộc: `features/files`, `features/agent-tools` (turn changes).
