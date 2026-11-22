# web-disk: Disk view

Đường vào: `apps/web/src/features/disk/index.ts` (`DiskBadge`, `SettingsDisk`, `SettingsDiskMap`, `hasDiskMap`, `formatSize`, `hasBuildClean`, `useDisk*Store`, `useDiskSizes`). `DiskView` và `BuildCleanDialog` nạp lazy trong `App.vue`; `core/stores/ui.ts` tự kiểm bridge `diskMapScan`.

Nội dung: scan / index, search, treemap + sunburst, Clean up, duplicates, Organize Downloads, Apps, RAM, Move folder, snapshots, DiskBadge, Settings Disk.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/disk/lib/diskMap.ts` | client IPC `ccm:disk-*` (bridge) |
| `apps/web/src/features/disk/stores/diskMap.ts` | state scan / index / kết quả |
| `apps/web/src/features/disk/components/DiskView.vue` | màn Disk (tab: Map, Clean, Duplicates, Organize, Apps, RAM, Snapshots) |

## Bất biến

- Mọi xoá đi qua dialog native, Recycle Bin / Trash mặc định (docs §11at); file cloud-only không bao giờ mở.
- `core/stores/ui.ts` không import `lib/diskMap` (tự kiểm bridge) để core không phụ thuộc feature.

## Liên kết

- Phía desktop: `core-disk.md`, `shell-disk.md`. Kiểu IPC: `packages/shared/src/diskMap.ts`, `diskClean.ts`, `diskApps.ts`, `diskMove.ts`, `organize.ts`. i18n: `apps/web/src/features/disk/i18n/`. Plan: `.planning/PLAN-v101-disk-manager.md`. E2E: `v101-*.cjs`.
