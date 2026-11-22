# palette

Ctrl+K: tìm session, dự án, file, cài đặt, lệnh; tiền tố > @ # :.

- Đường vào: `@features/palette` (`CommandPalette`, `isPaletteKey`, `matchText`, `searchable`).
- File chính: `components/CommandPalette.vue`, `lib/paletteSearch.ts`, `lib/paletteKey.ts`.
- `lib/next/palette*.ts` là bảng màu UI next, không thuộc feature này.
- i18n: `i18n/ui-v80-palette.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
