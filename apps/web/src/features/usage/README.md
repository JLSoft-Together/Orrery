# usage

Tab Usage: chi phí / token theo ngày, model, project, limit của plan.

- Đường vào: `UsageView` (App.vue), `PlanLimits` (HeaderStatusPopover), `useUsageStore`.
- File chính: `components/UsageView.vue`, `stores/usage.ts`.
- i18n: `i18n/usage-v53.ts`, `i18n/ui-v80-limits.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
- Không import feature khác; chỉ dùng core (`core/stores/settings`, `core/lib/format`...).
