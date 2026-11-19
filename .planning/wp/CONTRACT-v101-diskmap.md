# Contract v101 phase 1b: bridge Ổ đĩa (desktop ↔ web)

Tên dùng prefix `diskMap` / `ccm:diskmap-*` để không đụng tính năng cảnh báo ổ đầy v77 (`ccm:disk-*`, `diskState`).

Types ở `packages/shared/src/diskMap.ts` (export từ `packages/shared/src/index.ts`).

```ts
type DiskLevel = 'light' | 'balanced' | 'max'
interface DiskVolume { mount: string; label: string; fs: string; kind: string; network: boolean; total: number; free: number }
interface DiskEntry { path: string; name: string; size: number; files?: number; dir: boolean; mtime: number; cloud?: boolean; kind?: string }
interface DiskChildren { path: string; size: number; total: number; items: DiskEntry[] }
interface DiskProgress { files: number; bytes: number; current: string; elapsedMs: number }
interface DiskStatus { scanning: boolean; paused: boolean; indexed: boolean; roots: string[]; scannedAt?: number; entries?: number }
interface DiskSearchQuery { query?: string; minSize?: number; maxSize?: number; after?: number; before?: number; kind?: string; only?: 'file' | 'dir'; under?: string; sort?: 'size' | 'name' | 'mtime'; limit?: number }
interface DiskSearchResult { total: number; ms: number; items: DiskEntry[] }
interface DiskDeleteResult { done: { path: string; size: number }[]; failed: { path: string; reason: string }[]; freed: number; permanent: boolean; cancelled?: boolean }
interface DiskHistoryEntry { at: number; action: 'trash' | 'delete'; count: number; freed: number; paths: string[]; failed: number }
```

Bridge `window.ccmDesktop` (preload), tất cả optional ở phía web (`apps/web/src/lib/diskMap.ts` bọc như `lib/devices.ts`):

| Method | Trả về | Ghi chú |
|---|---|---|
| `diskMapVolumes()` | `DiskVolume[]` | |
| `diskMapStatus()` | `DiskStatus` | lần đầu gọi thì load index đã lưu nếu có |
| `diskMapScan({ roots, level, excludes })` | `{ started: boolean }` | xong tự save index |
| `diskMapPause()` / `diskMapResume()` / `diskMapCancel()` | `boolean` | |
| `diskMapChildren({ path, sort, limit })` | `DiskChildren \| null` | |
| `diskMapSearch(q)` | `DiskSearchResult` | |
| `diskMapDelete({ paths, permanent })` | `DiskDeleteResult` | main hiện dialog native (tổng size + số mục); `permanent` thì confirm lần 2; mặc định vào Thùng rác (`shell.trashItem`); sau khi xoá cập nhật index (op Rust `disk.forget`); ghi lịch sử |
| `diskMapReveal(path)` | `void` | `shell.showItemInFolder` |
| `diskMapHistory({ limit })` | `DiskHistoryEntry[]` | `CCM_DATA_DIR/disk/history.jsonl` |
| `onDiskMapProgress(cb)` / `onDiskMapDone(cb)` | unsubscribe fn | event `ccm:diskmap-progress` / `ccm:diskmap-done` |

Main chỉ trả lời frame của app (`isAppFrame`), validate mọi tham số (path phải tuyệt đối, nằm trong root đã index, không phải root ổ / thư mục hệ thống trong danh sách cấm).
