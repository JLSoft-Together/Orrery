# web-run: Run pane, preview XML / Drawable / Flutter, công cụ dự án

Đường vào: `apps/web/src/features/run/index.ts` (`RunPane`, `RunBadge`, `useRunStore`, `runBadge`, `handoffPaneFor`, `inRoot`, `RunSdkSettings`, `XmlPreviewSettings`). Dialog nặng (Env, I18n, Release, Strings, LibUpdates) nạp lazy trong `WorkspaceView.vue`; preview nạp trong `FilePane.vue`.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/run/components/RunPane.vue` | pane chạy app, log, crash / ANR |
| `apps/web/src/features/run/stores/run.ts` | state các lần chạy, config, thiết bị |
| `apps/web/src/features/run/lib/runUi.ts` | định dạng / lọc log |
| `apps/web/src/features/run/lib/runCrash.ts` | đọc crash / ANR từ log |
| `apps/web/src/features/run/lib/layoutPreview.ts` | client preview XML Layout |
| `apps/web/src/features/run/lib/drawablePreview.ts` | preview Drawable |

## Bất biến

- Daemon layout chỉ warmup khi có tín hiệu XML Android, không bật sẵn.
- Config lạ (`trust-config`) và cài lại (`reinstall`) cần xác nhận native; từ điện thoại xác nhận trên điện thoại.
- `apps/web/src/core/lib/desktop.ts` import type từ `lib/layoutPreview`, `lib/drawablePreview` (client IPC).
- Lịch sử chạy (v112 X6): `components/RunHistoryDialog.vue` (list, ghim mốc, so sánh), `RunHistoryLog.vue` (log ảo hoá, đọc theo trang 1000 dòng), `lib/runHistory.ts` (`normalizeLine`, `compareRuns`, max 500 dòng khác), nút `data-run-history-open` ở `RunPane.vue`, i18n `i18n/run-v112.ts`.

## Liên kết

- Phía desktop: `core-desk-run.md`, `shell-hosts.md` (`host_run.rs`). i18n: `apps/web/src/features/run/i18n/`. Plan: `.planning/PLAN-v31-*.md`, `PLAN-v32-*.md`. E2E: `run-pane`, `layout-speed`, `flutter-preview`, `drawable-preview`.
