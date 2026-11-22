# extensions

Tab Extensions: cài MCP server / plugin cho Claude Code (global hoặc theo project) qua CLI `claude`, và panel Hooks.

- Đường vào: `ExtensionsView` (async, App.vue), `useExtensionsStore`, `BRAND_ICONS` (NextQuotaBar).
- File chính: `components/ExtensionsView.vue`, `stores/extensions.ts`, `lib/extensions.ts`, `lib/claudeHooks.ts`.
- i18n: `i18n/extensions-v28.ts`, `i18n/hooks-v74.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
- Bất biến: cài / gỡ chỉ sau native confirm (docs §11r).
