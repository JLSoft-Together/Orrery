## WP-114b — DONE

### Đã đổi
- `apps/web/src/features/next/density.css` (mới, 6 token `--ccm-density-{row-h,row-py,gap,head-h,pal-h,set-py}`; Vừa = 48/6/8/34/52/8 đúng pixel cũ; Gọn / Thoáng ghi đè dưới `:root[data-ui='next'][data-density=…]`; không transition). Import 1 dòng `@import './density.css'` ở đầu `tokens.css`.
- `core/lib/density.ts` (mới: kiểu, guard, `ccm.density`), `core/stores/settings.ts` (chỉ thêm `density`, đọc / ghi `ccm.density`, set `data-density` sync + immediate).
- Class đổi sang token: `RailRowItem.vue` (min-h, py, gap-x), `ui/PaneHeader.vue` (h) - dùng chung cho mọi pane head next, `CommandPalette.vue` (min-h hàng next; thêm 3 lệnh `c:density:*` chỉ khi UI next), `SettingRow.vue`, `core/components/ToggleSwitch.vue` (py).
- `SettingsMistAppearance.vue`: segmented Mật độ (trong mục Surface); `settings/components/index.ts`: 1 entry search `density`.
- i18n: `settings/i18n/theme-v96.ts` thêm `themeV96.density.*` (en + vi).
- Test mới: `features/next/density.test.ts`, `core/stores/settings.density.test.ts`.

### Tiêu chí
- Vừa đúng pixel cũ: ĐẠT (test khoá giá trị token = số hard-code cũ; class cũ `min-h-[48px] py-1.5 gap-x-2`, `h-[34px]`, `min-h-[52px]`, `py-2` quy về cùng px).
- Không transition layout, không rẽ nhánh theme id: ĐẠT.
- Lưu `ccm.density`, search Settings, palette 3 lệnh: ĐẠT.
- E2E `density.cjs` + đếm resize PTY: CHƯA KIỂM (không chạy E2E theo luật; chưa viết file e2e).
- Terminal / editor / list ảo JS không đụng: ĐẠT.

### Audit
Diff chỉ gồm file trong phạm vi + 2 file test mới. Không comment. File mới < 400 dòng. Không đụng mobile / Rust / `i18n/{en,vi}.ts`.

### Review
- Classic: token mặc định ở `:root` = Vừa, override chỉ dưới `data-ui='next'` nên SettingRow / ToggleSwitch dùng chung vẫn đúng pixel cũ ở classic.
- Theme khác nhau: token chỉ là kích thước, không phụ thuộc theme.
- Reload giữ mức: store đọc `ccm.density` lúc khởi tạo. Còn chớp 1 frame mặc định Vừa nếu mức khác Vừa vì chưa set attribute trong `index.html` trước paint (ngoài phạm vi, đề xuất thêm).
- Đổi mức tức thì: chỉ đổi CSS var, thân pane thay đổi 1 lần (header đổi cao) nên PTY resize tối đa 1 lần / pane (chưa đo thật).
- Rail row không có list ảo; inbox desktop không tồn tại, "danh sách session" = rail row.

### Test
- `npm test -w @ccm/web -- src/features/settings src/features/next src/features/palette src/core --hookTimeout=60000` → 58 file, 418 pass.
- `npm test -w @ccm/web -- src/i18n src/importRules.test.ts` → importRules pass; `keys.snapshot` en + vi FAIL do key mới `themeV96.density.*` (main cập nhật snapshot, không `-u`).
- `npm run typecheck` → sạch.

### Nhìn
CHƯA NHÌN (không chạy app / E2E).

### Hot file cần thêm
- `apps/web/src/i18n/__snapshots__/keys.{en,vi}.txt`: thêm `themeV96.density.{label,hint,compact,regular,comfy,cmd}`.
- `apps/web/index.html`: set `data-density` từ `ccm.density` trước paint (tránh chớp).
- `apps/shell/e2e/density.cjs`: chưa viết.

### Ngoài phạm vi
List ảo JS (`ExplorerSearch`, `ExplorerTodo`, `LogTable`), terminal, editor, UI classic.

### Commit message đề xuất
feat(next): density setting Compact / Regular / Roomy for rail rows, pane headers, palette and settings rows
