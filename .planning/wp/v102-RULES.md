# Luật chung cho mọi gói việc v102 (UI next gen Sương kính)

Worktree: `F:\work\web\Orrery-mist`, nhánh `feat_mist_ui`. Chỉ sửa file trong worktree này, KHÔNG đụng `F:\work\web\Orrery`.

## Đọc trước
1. `.planning/SPEC-v102-mist-ui.md` (spec có ID, nguồn sự thật chức năng)
2. `.planning/PLAN-v102-mist-redesign.md` (kiến trúc: cờ, tách code, token)
3. `.planning/ui-inventory/00-SUMMARY.md`, rồi file 01-07 phần liên quan gói của mình (có số đo + file:line)
4. Checklist nghiệm thu gói của mình: `.planning/wp/v102-ACCEPTANCE.md` (mục đúng tên gói), làm đủ các dòng `- [ ]`.
5. Mockup nguồn: `.planning/mockups/v102/src/*.dc.html` (HTML + CSS thật, lấy số đo / màu / cấu trúc từ đây). Bản chạy: https://orrery-ui.jinvu.vn
6. `.code_index/web.md` để tìm file; `CLAUDE.md` repo (rule UI: i18n, token, status = icon + chữ + màu)

## Bất biến (vi phạm = gói bị trả lại)
- `data-ui="classic"` render y như 4.0.5. Mọi CSS mới nằm dưới `:root[data-ui='next']` hoặc trong component chỉ mount ở next. E2E cũ không sửa.
- Logic dùng chung, không copy: stores, `lib/rail/*`, `splitLayout`, `usePaneInfo`, `useRailDrag`, `view/geometry.ts`, `lib/workspaceKeys.ts`, `lib/shortcuts.ts`. Cần dữ liệu mới thì thêm hàm vào chỗ dùng chung.
- Component mới của shell / bố cục: `apps/web/src/components/next/**`. Component lá dùng chung: sửa giao diện bằng token, chỉ tách khi DOM phải khác.
- Chuỗi UI qua vue-i18n namespace `mist.*`, đủ `en` + `vi` (file i18n hiện có, thêm khối `mist`).
- Màu qua token `--ccm-*`, không hex trong component. Status = icon + chữ + màu.
- Animation chỉ `transform` + `opacity`, tôn trọng `data-motion` (full / reduced / off) và `prefers-reduced-motion`.
- Blur (`backdrop-filter`) chỉ ở chrome + lớp nổi; pane terminal không blur.

## Code
- **KHÔNG viết comment trong code** (mọi loại: `//`, `/* */`, `<!-- -->`, JSDoc). Lý do / invariant ghi vào `.code_index/web.md` hoặc report.
- Tìm kiếm bằng tool Grep / `rg`, không `grep -r`.
- Không dùng em dash (—) và dấu chấm giữa (·) làm ngăn cách trong chuỗi UI / tài liệu; dùng phẩy hoặc gạch ngang.
- Từ cấm: "trần" → "max", "watchdog" → "deadline chờ ...".
- KHÔNG `git commit` / `git push` / `git stash` / `git checkout --`. Để nguyên working tree.
- Nhiều gói có thể chạy song song trong cùng worktree: chỉ sửa file thuộc gói mình (liệt kê trong brief). Cần sửa file chung (`App.vue`, `stores/settings.ts`, i18n, `style.css`) thì sửa nhỏ, đúng khối, không format lại cả file.

## Kiểm tra trước khi báo xong
- `cd F:/work/web/Orrery-mist && npm run typecheck -w @ccm/web` pass.
- `npm test -w @ccm/web -- <file test liên quan>` pass; thêm test Vitest cho logic mới (hàm thuần trong `lib/`).
- Không tự chạy E2E desktop trừ khi brief bảo.

## Report (ghi `.planning/wp/v102-WP-<n>-report.md`, KHÔNG ghi đè file WP-*-report.md cũ của plan khác, trả lời main ≤ 25 dòng)
```
WP-<n>: ĐẠT | CHƯA ĐẠT
File: <danh sách file sửa / thêm>
SPEC ID đã làm: ...  /  chưa làm + lý do: ...
Typecheck: pass/fail  Test: N pass / M fail (lệnh)
Rủi ro / việc cho gói sau: ...
```
