# run

Run pane (chạy app Android / Flutter, log, crash / ANR), XML Layout / Drawable / Flutter preview, bảng chuỗi đa ngữ, kiểm phát hành, kiểm lib, .env, i18n scan, Scripts, SDK / XML Preview settings.

- Đường vào: `@features/run` (`RunPane`, `RunBadge`, `RunProjectMark`, `useRunStore`, `runBadge`, `handoffPaneFor`, `inRoot`, `hasXmlInstall`, `hasReleaseCheck`, `RunSdkSettings`, `XmlPreviewSettings`).
- Dialog nặng (Env / I18n / Release / Strings / LibUpdates) nạp lazy trực tiếp trong `WorkspaceView.vue`; preview nạp trong `FilePane.vue`.
- File chính: `stores/run.ts`, `lib/runUi.ts`, `lib/runCrash.ts`, `lib/layoutPreview.ts`, `components/RunPane.vue`.
- `lib/desktop.ts` import type từ `lib/{layoutPreview,drawablePreview}` (client IPC).
- i18n: `i18n/*.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
