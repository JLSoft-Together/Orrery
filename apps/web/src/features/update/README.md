# update

Cập nhật app, What's new, tip phím tắt.

- Đường vào: `@features/update` (`useUpdateStore`, `useTipsStore`, `hasUpdater`, `APP_VERSION`, `tipKeys`, `WhatsNewDialog`, `UpdateSettings`).
- `UpdateDialog` nạp lazy trực tiếp trong App.vue (`lazyOverlay`).
- File chính: `stores/update.ts`, `lib/update.ts`, `lib/changelog.ts`, `lib/tips.ts`.
- i18n: `i18n/update-v74.ts`, `i18n/whats-new-v71.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
