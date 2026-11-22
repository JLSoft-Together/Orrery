# resources

Resources (CPU / RAM / port theo session), DevTools (JWT, base64, url, time, count, uuid), Diagnostics, dev server detector.

- Đường vào: `@features/resources` (`ResourcesButton`, `PortConflictBar`, `useResourcesStore`, `formatBytes`, `DEV_TOOLS`, `devServer` lib).
- `ResourcesDialog`, `DevToolsDialog`, `DiagnosticsDialog` nạp lazy trực tiếp trong App.vue.
- File chính: `stores/resources.ts`, `lib/resources.ts`, `lib/devTools.ts`.
- i18n: `i18n/resources-v76.ts`, `i18n/devtools-v75.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
