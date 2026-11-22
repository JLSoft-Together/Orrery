# history

Tab History: heatmap, timeline, tool stats, tìm prompt; Away summary và Recap.

- Đường vào: `@features/history` (`HistoryView`, `AwaySummary`, `useAwayStore`, `useHistoryStore`, `usePromptSearchStore`, `formatMs`).
- `RecapDialog` nạp lazy trực tiếp trong App.vue (`lazyOverlay`).
- File chính: `components/HistoryView.vue`, `stores/history.ts`, `lib/toolTimeline.ts`, `lib/recap.ts`.
- i18n: `i18n/toolstats-v73.ts`, `i18n/promptsearch-v75.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
