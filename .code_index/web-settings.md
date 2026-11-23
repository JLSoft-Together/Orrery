# web-settings: Settings, Extensions, Update

Đường vào: `apps/web/src/features/settings/index.ts` (`SettingSection`, `SettingEmbed`, `entries`, `searchSettings`, `useSettingsEnv`, `flashSettingRow`, `tour*`, `capture*`, `ghAccounts`), `apps/web/src/features/extensions/index.ts`, `apps/web/src/features/update/index.ts`.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/settings/components/index.ts` | đăng ký trang Settings + chỉ mục tìm |
| `apps/web/src/features/settings/components/SettingsMenu.vue` | cửa sổ Settings (8 trang) |
| `apps/web/src/core/stores/settings.ts` | setting `ccm.*` (nằm ở core) |
| `apps/web/src/core/lib/uiTypes.ts` | `SETTINGS_PAGES`, `TourId` (feature export lại) |
| `apps/web/src/features/extensions/components/ExtensionsView.vue` | cài MCP server / plugin cho Claude Code, panel Hooks |
| `apps/web/src/features/extensions/stores/extensions.ts` | state Extensions |
| `apps/web/src/features/extensions/lib/claudeHooks.ts` | hooks Claude |
| `apps/web/src/features/update/stores/update.ts`, `lib/update.ts` | cập nhật app |
| `apps/web/src/features/update/lib/changelog.ts`, `lib/tips.ts` | What's new, tip phím tắt |

## Bất biến

- Cài / gỡ MCP / plugin chỉ sau native confirm, chỉ qua CLI `claude` (docs §11r).
- Trang Settings riêng của feature khác (`SettingsRemote`, `SettingsDisk`, `SettingsDeepSeek`...) nằm trong feature đó, đăng ký ở `components/index.ts`.
- Token GitHub chỉ nhập một lần, niêm phong bằng keychain OS, không gửi lại trang.

## Liên kết

- Phía desktop: `core-desk.md` (`extensions/`, `claude_hooks/`, `github/`, `update/`, `backup/`). Phát hành: `shell.md`. Plan: `.planning/PLAN-v28-*.md`, `PLAN-v58-github-accounts.md`. E2E: `extensions`, `github-accounts`, `whats-new`, `v27`.
- 2026-11-23 WP-112b: Quick Setup không còn modal. `lib/setupChecklist.ts` (store `setupChecklist`: trạng thái notify / toast / sound / statusLine / hooks / folder, `pendingSetup`, chip ẩn qua `ccm.setupChip.hidden`, `ccm.setupDone` cũ coi như đã ẩn), `components/SetupChecklist.vue` (đầu trang General, `data-setup-row`), chip nhắc ở `WorkspaceFooter.vue`. `OnboardingTour.vue` tour workspace chờ `workspace.tabs.length > 0`. E2E: `v112b-first-run`.
