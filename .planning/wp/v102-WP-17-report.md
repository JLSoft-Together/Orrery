WP-17: ĐẠT
File:
- thêm `apps/web/src/lib/next/customTheme.ts`, `apps/web/src/lib/next/customTheme.test.ts`, `apps/web/src/components/next/ui/ColorPicker.vue`, `apps/web/src/components/settings/CustomThemeBuilder.vue`
- sửa `apps/web/src/next/palettes.ts` (id `custom`, `NEXT_CUSTOM_PALETTE`, `NEXT_THEME_IDS`, `nextPaletteCards()` 10 thẻ), `apps/web/src/lib/uiGeneration.ts` (`NEXT_KEYS.custom`), `apps/web/src/stores/uiGen.ts` (custom state + áp biến inline + lưu debounce), `apps/web/src/components/settings/SettingsMistAppearance.vue` (thẻ thứ 10 + builder dưới lưới), `apps/web/index.html` (`NP.custom`, pre-paint biến custom), `apps/web/src/i18n/mist-v102.ts` (khối `mist.custom` en + vi), `apps/web/src/next/tokens.css` (khối mới cuối file: `--ccm-cp-*`, `.mist-cp-*`, `.mist-ctb-*`), `apps/web/src/next/palettes.test.ts` (9 -> 10 thẻ, NP so với `NEXT_THEME_IDS`), `apps/web/src/stores/uiGen.test.ts` (test custom), `.code_index/web.md`
SPEC ID đã làm: C1, C2, C3, C4, C5 / chưa làm: không
Typecheck: pass  Test: 40 pass / 0 fail (`npx vitest run src/lib/next/customTheme.test.ts src/next/palettes.test.ts src/stores/uiGen.test.ts src/i18n src/components/next/ui/primitives.test.ts`)

## Ghi chú thiết kế
- `custom` không nằm trong `NEXT_PALETTES` để 8 bảng cũ (CSS block + test tương phản) không đổi; có trong `byId` nên `isNextPaletteId('custom')`, `nextSchemeFor`, `startNext`, pre-paint đều nhận.
- Biến đặt inline trên `documentElement.style` (thắng mọi khối CSS), xoá khi đổi bảng / về classic. `data-glass=reduced` thì không đặt `--ccm-glass` để khối reduced vẫn thắng.
- "Đã chỉnh độ sáng để chữ đủ rõ" chỉ hiện khi accent bị nhích L để đạt 3:1 (ink / accent-ink luôn được fit 4.5:1 âm thầm).
- Sắc nền: lấy hue, chroma max 0.04. Vệt sáng chọn tay: giữ hue, L / C theo scheme.
- Độ trong kính 0..100, 50 = alpha mặc định (sáng 0.55, tối 0.05).
- Mẫu accent trong picker = 9 thẻ (`paletteAccents()`), "dùng gần đây" max 8, thêm khi đóng picker có thay đổi.

## Selector cho main chụp kiểm
- Thẻ custom: `[data-palette-card="custom"]`
- Builder: `[data-custom-builder]`, hàng `[data-custom-row="accent"]` (glowA, glowB, tint, clarity), chip `[data-custom-adjusted]`, `[data-custom-autoglow]`, `[data-custom-seed]`, `[data-custom-reset]`
- Nút mở picker: `[data-custom-row="accent"] [data-color-picker-open]`
- Popover: `[data-color-picker]`, ô vuông `[data-color-area]`, thanh hue `[data-color-hue]`, ô hex `[data-color-hex]` (sai: `aria-invalid="true"` + class `mist-cp-hex-bad`), mẫu `[data-color-swatch]`, gần đây `[data-color-recent]`

## Việc cho main / gói sau
- E2E `apps/desktop/e2e/v102-next.cjs` check "9 thẻ palette" phải sửa thành 10 (thẻ cuối `data-palette-card="custom"`). Không sửa file e2e theo brief.
- `TermPane.vue` watch theme xterm chỉ theo `uiGen.nextTheme` / `uiGen.dark`: đổi màu custom (well / accent) chưa đổi terminal ngay, cần thêm `uiGen.customScheme` vào nguồn watch (1 dòng, file ngoài gói).
- Chưa kiểm hình thật (không chạy app theo brief).
