# PLAN v91 — Theme "Orrery" (mặc định mới) + "Console"

Chủ nhân 2026-11-11: chọn style B và D trên canvas thiết kế
(https://claude.ai/artifact/8Bw9wH1fNr3ukNexFFsaj3, bảng "B · Đảo phẳng kiểu IDE", "D · Console tương phản cao"),
"B là theme mặc định của mình luôn từ giờ".

## Chốt
- B → theme **Orrery** (`orrery-dark` + `orrery-light`, cặp sáng/tối như Apple), **mặc định** cho máy chưa chọn theme
  (không có `ccm.theme`), theo chế độ sáng/tối của hệ thống như hiện tại.
- Chủ nhân đổi (2026-11-11): bản **4.0.2 chuyển MỌI máy sang Orrery một lần**, kể cả người đã chọn theme khác: chưa có
  `ccm.themeMigration = '4.0.2'` → `ccm.theme` = `orrery-dark` nếu theme đang lưu là dark (không lưu gì thì theo hệ thống),
  ngược lại `orrery-light`, rồi ghi `ccm.themeMigration`. Sau đó lựa chọn của chủ nhân giữ nguyên (không migrate lại).
  Chạy trong script trước paint `index.html` (`pick`, không nháy) + `initialTheme` (`lib/themes.ts` `startTheme`).
- D → theme **Console** (`console`, chỉ dark).
- Apple (`light`/`dark`) và các theme cũ giữ nguyên, vẫn chọn được.
- Theme không chỉ là palette nữa: thêm **token hình dáng** để mỗi theme đổi component mà không sửa component theo id theme.

## Token hình dáng (mặc định = Apple hiện tại, theme khác override)
| token | Apple | Orrery | Console |
|---|---|---|---|
| `--radius-card` / `--radius-control` / `--radius-inner` | 12 / 8 / 6 | 10 / 6 / 4 | 6 / 4 / 3 |
| `--ccm-panel-border` (viền panel/card) | none | none (nổi bằng độ sáng) | 1px solid line |
| `--ccm-tab-style` → tab đang chọn | nền raised | gạch dưới 2px accent, không nền | gạch trên 2px accent + nền surface |
| `--ccm-chip-fill` (nền chip trạng thái) | soft tint | transparent (chấm + chữ) | soft tint 12% |
| `--ccm-pane-active` | ring 1px focus + header tint | thanh trái 2px accent, không tint | ring 1px accent + header tint |
| `--ccm-seg-style` (segmented control) | nền fill + nút raised | không nền, nút raised nhẹ | không nền, gạch dưới accent |
| `--ccm-label-font` (nhãn section nhỏ IN HOA, footer) | sans | mono | mono, letter-spacing rộng |
| `--ccm-gap-shell` (khoảng giữa panel) | 8px | 6px | 6px |
| `--ccm-dim-inactive` (pane không chọn) | 1 | 0.78 | 0.8 |

Cách làm: biến CSS + vài class tiện ích trong `style.css` (`ccm-tab-on`, `ccm-chip`, `ccm-pane-active`, `ccm-seg`,
`ccm-label`, `ccm-panel`) mà component dùng; theme chỉ đặt biến. Không `if (theme === …)` trong component.

## Palette
Orrery dark (từ bảng B): canvas `#131315`, surface `#1b1b1f`, raised `#232329`, ink `#ececf1`, muted `#9b9ba7`,
faint `#8e8e9a`, accent `#7c6cf0` (accent-ink sáng hơn cho chữ đạt 4.5:1), working `#f5a524`, waiting `#f7d046`,
done/success `#4cc38a`, well (terminal) `#17171a`.
Orrery light: cùng tông indigo, canvas `#f3f3f6`, surface `#ffffff`, raised `#ececf1`, ink `#17171c`, accent `#5b4bd6`.
Console (từ bảng D): canvas `#0c0d10`, surface `#111318`, raised `#181b21`, line `#262a31`, ink `#e8ebf0`,
accent `#3dd6f5` (on-accent tối `#071015`), working `#ffb224`, waiting `#ffe14d`, done `#4be08a`, well `#08090b`.
Mọi giá trị qua `lib/themes.test.ts` (4.5:1 chữ, 3:1 glyph); chỉnh nhẹ nếu trượt.

## Việc
1. Token hình dáng + class tiện ích (`style.css`), áp vào: tab bar Workspace, header pane + pane đang chọn, chip trạng
   thái (pane / tab / rail / card Monitor), segmented control (header Giám sát/Workspace, sidebar phải), panel
   (rail, sidebar, pane, card), nhãn section, footer/status bar nếu có.
2. 3 block theme trong `themes.css` + `lib/themes.ts` (`THEMES`, swatch, cặp) + script trước paint `index.html`.
3. Mặc định mới: không có `ccm.theme` → `orrery-light` / `orrery-dark` theo hệ thống (index.html + `initialTheme`).
4. Settings → Giao diện: 2 theme mới đứng đầu danh sách, Orrery ghi "mặc định".
5. Test: `themes.test.ts` (đủ token, tương phản, index.html khớp `THEMES`), test mặc định mới; E2E chụp Workspace + Monitor +
   Settings ở 3 theme (Orrery dark/light, Console).
6. Cập nhật `CLAUDE.md` (Design system: mặc định Orrery, Apple vẫn có), `.planning/COLOR-SYSTEM.md` (token hình dáng),
   `.code_index/web.md`.
