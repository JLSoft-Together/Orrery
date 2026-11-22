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
| `apps/web/src/features/git/lib/diffBlocks.ts` | tách hunk cho DiffView / SplitDiff |
| `apps/web/src/features/git/lib/changesView.ts` | gom file thay đổi |

## Bất biến

- Xoá nhánh chỉ sau confirm native (docs §11s). Push dùng tài khoản GitHub của repo (v58), token không bao giờ về trang.
- Commit / push bị chặn khi quét ra token (secret guard, `web-small.md`).
- `highlight.worker.ts` của files import thẳng `diffBlocks`: không đưa lib này phụ thuộc index git.

## Liên kết

- i18n: `apps/web/src/features/git/i18n/`. Plan: `.planning/PLAN-v30-git-log.md`, `PLAN-v36-log-branches-toolbar.md`, `PLAN-v58-github-accounts.md`. E2E: `log`, `split-diff`, `diff`, `v103-log-diff`, `github-accounts`.
- Phụ thuộc: `features/files`, `features/agent-tools` (turn changes).
