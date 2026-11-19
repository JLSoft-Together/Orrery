WP-6: ĐẠT
File: mới `lib/next/palette.ts`, `lib/next/paletteHooks.ts`, `lib/next/palette.test.ts`; sửa `components/CommandPalette.vue` (nhánh next trong cùng file, không tạo NextCommandPalette vì DOM chỉ khác row/scope/footer), `composables/usePaneInfo.ts` (thêm `modelOf`), `i18n/mist-v102.ts` (khối `mist.cmdk`, không đặt tên `palette` vì đã là khối theme của WP1), `next/tokens.css` (khối "WP6 overlay" cuối file).
SPEC ID đã làm: K1 (nhóm Cần bạn / Đang chạy / Xong chưa xem / Gần đây rồi Lệnh rồi Project; row 52 có icon trạng thái, tên, lý do + tool, project, nhánh, model, thời gian), K2 (model + nhánh + repo vào chuỗi khớp), K3 (tab Tất cả, Session @, File #, Lệnh >, Project :; Enter mở giữ split, Shift+Enter mở pane mới / tab mới / split file), lệnh "Về giao diện Orrery cũ" (kiểm, có từ WP1), lệnh "Hoàn tác bố cục" (chỉ hiện khi có hàm undo đăng ký), lớp nổi kính, toast.
Ghi chú hành vi:
- Ở next, prefix `:` = Project (đổi từ Cài đặt); mục Cài đặt vẫn tìm được ở Tất cả / Lệnh khi có chữ gõ. Classic giữ nguyên 4 prefix cũ.
- Enter ở session next gọi `paneInfo.goToPeer` (pane trong Workspace thì kích hoạt, không thì mở Monitor); Shift+Enter giữ hành vi cũ (focus cửa sổ / resume).
- "Xong chưa xem" = `pane.attention === 'done'`; session không có pane thì vào Gần đây (sort theo `statusSince` mới nhất).
- Hook undo bố cục: WP4b gọi `registerLayoutUndo(run, canRun?)` từ `lib/next/paletteHooks.ts`; chưa đăng ký thì lệnh ẩn, an toàn.
- Kính overlay: `.ccm-sheet` / `.ccm-menu` dưới `:root[data-ui='next']` trong `@layer components` (utility trong component vẫn thắng, ví dụ Settings `bg-canvas`): nền `--ccm-glass-pop`, blur, viền glass-edge, radius card 18 (menu radius control 12), bóng. 55 dialog + 41 menu + QuickOpen + ProjectSwitcher + toast + popover tự đổi, không sửa file nào. `data-glass="reduced"` đã tắt blur qua token WP1. Toast `undo` có nút accent-soft.
- Toast: `Toast.actions: {label, run, primary?, keys?, icon?}` đã có sẵn từ `stores/toasts.ts`, không cần thêm API cho hoàn tác 5s (R5 / D9).
Typecheck: pass (`npm run typecheck -w @ccm/web`). Test: 48 file / 464 pass / 0 fail (`npx vitest run --root apps/web src/lib/next src/lib/paletteSearch.test.ts src/components src/i18n src/composables`). Không chạy E2E.
Rủi ro / việc cho gói sau:
- Shell không cần mount thêm gì: `CommandPalette` vẫn mount ở App.vue, tự theo `isNext`.
- E2E cũ đếm tab scope / prefix của palette chỉ ảnh hưởng ở next (classic không đổi); WP11 kiểm.
- Cần ảnh chụp (rỗng, có kết quả, không kết quả, dialog) chưa chụp, main chạy app để nghiệm thu thị giác. Scope tab ở next đổi sang pill bằng CSS (`[data-palette-scope]`), DOM không đổi.
- WP4b nối `registerLayoutUndo`; WP-D9 dùng `toasts.push` với `kind: 'undo'` + `actions`.

## Sửa sau review

- TelegramToggle có prop `iconOnly`, NextShell render ở rail icon (icon + chấm trạng thái, aria-label + title); classic không đổi.
- `railMode.ts`: hysteresis 24px đối xứng (xuống mode khi < breakpoint - 24), test dao động quanh 1200 và 1024.
- Radiogroup rail icon: roving tabindex + phím mũi tên; nút mở rail overlay có `aria-expanded` + `aria-controls`.
- CommandPalette next: placeholder + nhãn scope dùng `mist.cmdk.*` (en + vi); classic giữ `paletteV80.*`.
- `compareSessions`: bucket needs xếp chờ lâu nhất trước, bucket khác mới nhất trước (test thêm).
- Tab chỉ chặn ở classic; ở next Tab đi tuần tự, mũi tên đổi scope khi focus ở tablist.
- Xoá comment `WP6 overlay` và `monitor (WP9)` trong `next/tokens.css`.
- Placeholder Ctrl+K next escape `{'@'}` trong `i18n/mist-v102.ts`, `messages.test.ts` pass.
