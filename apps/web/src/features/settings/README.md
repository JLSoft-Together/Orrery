# settings

Cửa sổ Settings (8 trang), Quick setup, tour, Backup / Capture / GitHub accounts / Agent data / Awake, error log + bug report.

- Đường vào: `@features/settings` (`SettingSection`, `SettingEmbed`, `entries`, `searchSettings`, `useSettingsEnv`, `flashSettingRow`, `tour*`, `capture*`, `ghAccounts`).
- File chính: `components/index.ts` (đăng ký trang + chỉ mục tìm), `components/SettingsMenu.vue`, `components/Settings*.vue`.
- Trang riêng của feature khác nằm trong feature đó (`SettingsRemote`, `SettingsDisk`, `SettingsDeepSeek`, ...), được đăng ký trong `components/index.ts` / SettingsMenu.
- `stores/settings.ts`, `stores/ui.ts` ở `core/stores/`; `uiGen` ở `features/next/stores/`. `SETTINGS_PAGES` / `TourId` ở `core/lib/uiTypes.ts`, file ở đây export lại.
- i18n: `i18n/*.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
