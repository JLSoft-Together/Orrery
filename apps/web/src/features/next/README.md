# next

UI next gen "Sương kính" (v102): NextShell, LiveRail, canvas (layout menu, drop overlay, split hint), Explorer drawer, glass primitives, quota bar, bảng màu (mau / random / custom palette) và `tokens.css`.

- Đường vào: `App.vue` nạp `NextShell`, `ExplorerDrawer`, `NextInvite` trực tiếp; `@features/next` chỉ export lib bảng màu; store `uiGen` ở `stores/uiGen.ts` của feature này.
- File chính: `components/NextShell.vue`, `components/rail/LiveRail.vue`, `components/canvas/useNextLayout.ts`, `lib/railLive.ts`, `lib/customTheme.ts`, `tokens.css` (import trong `main.ts`).
- Phụ thuộc vòng: next -> workspace (layout, store, pane) và next -> rail (railModel, useRail, railMenu); workspace/rail import ngược `lib/next/*` (`workspaceBridge`, `railLive`, `quotaBar`...).
- Test đọc file ngoài feature bằng đường tương đối: `palettes.test.ts` (style.css, themes.css, index.html, packages/mist-tokens), `lib/mau.test.ts` (.planning/site, index.html).
- i18n: `i18n/{mist-v102,mist-remote,rail-select-v98,v105-u1,v105-u2,v105-u3,v106-rail}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
