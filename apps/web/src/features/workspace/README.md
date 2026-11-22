# workspace

Workspace: terminal / browser pane, tab, split layout, launcher, layouts, pane info, project roots, restore, dust.

- Đường vào: `@features/workspace` (store, `usePaneInfo`, `usePaneActions`, `leaves`...). Index không export component để giữ `WorkspaceView` / `TermPane` lazy trong `App.vue`.
- File chính: `stores/workspace.ts`, `components/WorkspaceView.vue`, `components/TermPane.vue`, `lib/splitLayout.ts`, `lib/layout*.ts`, `composables/usePaneInfo.ts`.
- Các feature khác import sâu `stores/workspace` (vòng workspace <-> rail / next / files / git / run / agent-tools / monitor): chưa đổi sang index, chờ lô 12.
- `stores/attention.ts`, `stores/awake.ts` ở đây theo AUDIT; type `NodeSize` / `Point` nằm ở `core/lib/uiTypes.ts`, `lib/layout` export lại.
- i18n: `i18n/{workspace-*,tabs-v93,launcher-v93,pane-v9*,split-v97,shellmarks-v73,projects-v78}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
