# disk

Disk view: scan / index, search, treemap + sunburst, Clean up, duplicates, Organize Downloads, Apps, RAM, Move folder, snapshots; DiskBadge, BuildCleanDialog, Settings → Disk.

- Đường vào: `@features/disk` (`DiskBadge`, `SettingsDisk`, `SettingsDiskMap`, `hasDiskMap`, `formatSize`, `hasBuildClean`, `useDisk*Store`, `useDiskSizes`).
- `DiskView` và `BuildCleanDialog` nạp lazy trực tiếp trong App.vue; `core/stores/ui.ts` tự kiểm bridge `diskMapScan` (không import `lib/diskMap`).
- File chính: `lib/diskMap.ts`, `stores/diskMap.ts`, `components/DiskView.vue`.
- Mọi xoá đi qua dialog native, Recycle Bin mặc định (xem `docs` §11at).
- i18n: `i18n/disk-*.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
