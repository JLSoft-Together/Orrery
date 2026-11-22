# git

Diff (inline + split), Changes pane, commit box, conflicts, worktree, checkpoints, local history, image / text compare, Git Log (đồ thị commit, nhánh, Push dialog).

- Đường vào: `@features/git` (`CommitBox`, `DiffView`, `PushDialog`, `useTurnChanges`, `useFileChanges`, `parseUnifiedDiff`, `pathKey`, `useCheckpointsStore`...). Chỉ export phần feature khác cần.
- App nạp thẳng `ChangesPane`, `SplitDiff`, `components/log/LogPane`, `Worktree*` (không qua index để giữ chunk lazy).
- File chính: `components/ChangesPane.vue`, `components/DiffView.vue`, `components/log/LogPane.vue`, `composables/useGitLog.ts`, `lib/diff.ts`, `lib/changesView.ts`.
- files và git phụ thuộc vòng (22 / 13); module lib thuần import sâu nhau, chỉ file .vue đi qua index.
- i18n: `i18n/workspace-{diff,log,commit}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
