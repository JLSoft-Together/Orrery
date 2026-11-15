WP-0 + WP-1: ĐẠT

File:
- Mới: `apps/web/src/lib/uiGeneration.ts`, `apps/web/src/stores/uiGen.ts` (+ `uiGen.test.ts`), `apps/web/src/next/tokens.css`, `next/palettes.css`, `next/palettes.ts`, `next/palettes.test.ts`, `apps/web/src/components/next/NextShell.vue`, `components/next/ui/{GlassDialog,GlassMenu,GlassTooltip,SegmentedControl,Chip,PaneHeader,ContextGaugeMini}.vue`, `components/next/ui/{focus,chipTone}.ts`, `components/next/ui/primitives.test.ts`, `apps/web/src/i18n/mist-v102.ts`, `apps/desktop/e2e/ui-gen.cjs`.
- Sửa nhỏ: `apps/web/index.html` (khối `pickNext`), `main.ts` (import 2 CSS), `App.vue` (NextShell thay AppHeader khi next, footer ẩn ở next, observer header qua `watch(header)`), `components/CommandPalette.vue` (lệnh `c:ui-generation`), `components/settings/SettingsAppearance.vue` + `settings/index.ts` (mục `uiGeneration`), `i18n/en.ts` + `vi.ts` (spread `mistV102`), `lib/fonts.ts` + `scripts/gen-fonts.mjs` + `fonts.css` + `public/fonts-LICENSES.txt` (Manrope), `apps/web/package.json` + `package-lock.json` (`@fontsource-variable/manrope` 5.3.0), `.code_index/web.md` (mục "v102 next UI"), `.code_index/_meta.md`.

SPEC ID đã làm: S4 (2 nút chuyển: Settings → Appearance, Ctrl+K, nút tạm trong NextShell; đổi ngay không restart), G4 (7 khoá lưu), M1 (data-motion 3 mức + mặc định theo prefers-reduced-motion, phần store + CSS), M2 (token motion), M5 (`--ccm-meteor-pane: none` + tắt animation meteor panel ở next), mục 9 (token, 8 palette / 9 thẻ, working xanh dương tách vàng, text-2xs 11px, primitive), Q1 theo chốt mới: kính, `data-glass="reduced"` chỉ là trợ năng (panel đặc, tắt blur, giữ bo góc / bóng).
Chưa làm + lý do: S1-S3, M6 (shell thật, responsive, dừng animation khi ẩn) thuộc WP2; G1 (trang Giao diện chọn palette) thuộc WP10, store đã có `setNextTheme` / `nextPaletteCards()`; artifact Design System không làm (chủ nhân 15/11: không tạo artifact).

Typecheck: pass (`npm run typecheck -w @ccm/web`).
Test: 59 + 141 pass / 0 fail (`npx vitest run src/next src/stores/uiGen.test.ts src/components/next src/lib/themes.test.ts src/lib/fonts.test.ts src/components/settings/index.test.ts src/i18n/messages.test.ts src/stores/settings` và `src/stores/workspace.test.ts src/lib/fileLinks.test.ts src/lib/libVersions.test.ts src/lib/viewer.test.ts src/components/ui`). Không chạy E2E desktop.

Quyết định kỹ thuật:
- Ở next gỡ `data-theme` khỏi root (pre-paint + store), nên không block theme cũ nào áp vào (kể cả `galaxy:not(...)` specificity 0,4,0). Về classic gọi lại `applyTheme(settings.theme)`.
- Sáng / tối dùng setting hiện có (mode của `settings.theme` + `themeFollow`). Palette chỉ tối (Ngân hà, Console) ép `.dark`.
- Contrast test đo cả 3 tâm glow (điểm nóng nhất), trên surface / raised / glass / material. Để đạt 4.5: glow tối giảm alpha (mist tối 0.38/0.34/0.24 → 0.24/0.22/0.16, các palette khác tương tự), ink-faint / idle / error / done / peer tối sáng hơn proposal một chút, light done / idle / error đậm hơn một chút, Bình minh sáng accent `#B4532F` (on-accent trắng đạt 4.5), `--ccm-working-soft` 10%.

Rủi ro / việc cho gói sau:
- i18n: mọi key `mist.*` nằm trong 1 file `i18n/mist-v102.ts` (spread object `mist` sẽ đè nhau nếu gói khác tạo file thứ 2 cũng có key `mist`). Gói sau thêm key vào file này (khối con riêng: `mist.shell`, `mist.rail`...), không tạo file mới cùng gốc `mist`.
- `modeOf(settings.theme)` ở component lá (ExplorerTree, PaneIcon, CommandPalette) sẽ sai khi palette next chỉ tối mà theme cũ đang sáng; dùng `useUiGenStore().dark` thay.
- Terminal xterm đọc `settings.codeFont` trực tiếp nên ở next vẫn là font hệ thống khi chưa chọn; WP4 nên dùng `NEXT_CODE_FONT` khi `isNext` và `codeFont === 'system'`. `clearTerm` mới chỉ lưu khoá, WP4 nối `--ccm-well-clear`.
- Blur chỉ có qua class `.glass` / `.glass-pop` / `.glass-side`; `.ccm-panel` dùng `--ccm-panel-bg` = surface trong suốt, không blur (đúng rule pane không blur).
- Classic khác 4.0.5 đúng 2 chỗ (theo brief S4): mục "UI next gen" đầu Settings → Appearance và lệnh Ctrl+K; root có thêm `data-ui="classic"` + `data-motion` (không CSS nào dùng ở classic). E2E đếm hàng Settings / lệnh palette (nếu có) cần kiểm ở WP11.
