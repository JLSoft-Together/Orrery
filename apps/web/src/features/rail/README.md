# rail

Rail classic và Sidebar của Workspace: cluster / row / group head, model dòng rail, menu chuột phải, kéo thả, chọn nhiều, nhóm theo project.

- Đường vào: `@features/rail` (`useRail`, `useRailDrag`, `useClusterDrag`, `railModel`, `railMenu`...). `WorkspaceSidebar.vue` do `WorkspaceView` nạp trực tiếp.
- File chính: `components/WorkspaceSidebar.vue`, `components/RailClusterView.vue`, `lib/railModel.ts`, `lib/railMenu.ts`, `composables/useRail.ts`.
- Phụ thuộc vòng: workspace <-> rail (store, `usePaneInfo`, `splitLayout`), next -> rail (LiveRail, `lib/next/railLive`). Đang import sâu cả hai chiều.
- i18n: `i18n/{workspace-rail,workspace-v50-rail,workspace-v52-rail,rail-v92,groups-v99}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
