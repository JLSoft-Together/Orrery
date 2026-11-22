# web-rail: rail classic, sidebar Workspace

Đường vào: `apps/web/src/features/rail/index.ts` (`useRail`, `useRailDrag`, `useClusterDrag`, `railModel`, `railMenu`). `WorkspaceSidebar.vue` do `WorkspaceView` nạp trực tiếp.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/rail/components/WorkspaceSidebar.vue` | sidebar danh sách session / project |
| `apps/web/src/features/rail/components/RailClusterView.vue` | cluster, row, group head |
| `apps/web/src/features/rail/lib/railModel.ts` | model dòng rail (nhóm theo project / status) |
| `apps/web/src/features/rail/lib/railMenu.ts` | menu chuột phải |
| `apps/web/src/features/rail/composables/useRail.ts` | chọn nhiều, kéo thả, sắp xếp |

## Bất biến

- Thứ tự kéo thả lưu theo người dùng; không đổi thứ tự khi đang trỏ chuột vào (đóng băng).
- Phụ thuộc vòng: workspace <-> rail (store, `usePaneInfo`, `splitLayout`); next -> rail (LiveRail).

## Liên kết

- i18n: `apps/web/src/features/rail/i18n/`. UI next: `web-next.md`. E2E: `rail`, `rail-reorder`, `v103-rail`.
