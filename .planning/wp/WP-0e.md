## WP-0e: toast OS có nút đáp án

- Mục tiêu (`.planning/PLAN-v100-click-menu.md` phase 0e): khi một pane Claude / Codex bắt đầu hiện menu (hàng chờ `stores/menuQueue.ts` có mục mới) và cửa sổ Orrery **không focus**, toast hệ thống hiện câu hỏi + max 3 nút đáp án (menu ≤ 3 đáp án có số); bấm nút = chọn đáp án trong đúng pane (cùng đường `useTermMenu.pick` đọc lại màn trước khi gửi phím); menu > 3 đáp án hoặc không chắc → toast chỉ có nút "Mở" (focus app + nhảy pane). Bấm thân toast = mở app + nhảy pane. Không lặp toast cho cùng một menu; menu biến mất → gỡ toast nếu nền tảng cho phép.
- Hiện trạng: toast của app đi qua đâu? Tìm trước (Windows: crate `core-toast` / `toast-script`, link `agent-monitor://` qua `CCM_TOAST_LINK`; macOS `CCM_TOAST_BACKEND=electron` dock badge; collector `CCM_TOAST=0` tắt). Dùng lại đường có sẵn và các setting tắt toast hiện có (tắt toast thì 0e cũng tắt). Windows: toast có `actions` cần toast XML (`Notification` với `toastXml`, activation qua protocol hoặc callback); macOS: Electron `Notification` `actions` (chỉ hiện khi app ký / bundle, ghi CHƯA KIỂM nếu không test được). Không gửi nội dung câu hỏi ra ngoài máy (Telegram có luồng riêng, không đụng).
- Bảo mật: nút toast chỉ mang id ngắn (pane id + menu signature + index), main tra lại trong state hiện tại, không đưa text tuỳ ý vào lệnh; link activation sai / cũ → bỏ qua.
- Công tắc: dùng chung công tắc `clickMenu` + setting toast hiện có; thêm 1 tuỳ chọn "Toast có nút" trong Settings > Workspace nếu cần tách (mặc định bật).
- Phạm vi file: chỗ toast hiện có ở `apps/desktop/src/**` (file toast / notification, không phải `diskMap/`, `diskClean/`), `apps/desktop/src/preload.ts` + `main.ts` (chỉ khối của WP này), `apps/web/src/stores/menuQueue.ts`, `apps/web/src/composables/useTermMenu.ts`, component Settings Workspace + `stores/settings.ts` nếu thêm công tắc, `apps/web/src/i18n/click-menu-v100.ts`, E2E `apps/desktop/e2e/v100-menu-toast.cjs`, Rust `core-toast` nếu đường Windows bắt buộc đi qua đó.
- Cấm đụng: `apps/desktop/src/diskMap/**`, `apps/desktop/src/diskClean/**`, `apps/web/src/components/disk/**`, `stores/diskMap.ts`, `stores/diskClean.ts`, `lib/diskMap.ts`, `lib/diskClean.ts`, `i18n/disk-*.ts`, `apps/core/crates/core-disk/**` (WP-1d, WP-2a đang làm), `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`. `preload.ts` / `main.ts` có WP khác thêm khối song song: Edit báo file đổi thì đọc lại rồi thêm, không sửa dòng của WP khác.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] Unit test: quyết định hiện toast (focus / không focus, đã toast menu này chưa, > 3 đáp án → chỉ nút Mở), map nút → pane + index, activation cũ / sai bị bỏ.
- [ ] Windows: toast thật hiện nút trên bản dev tách biệt (chụp màn hình toast, ghi cách chụp); bấm nút gửi đúng phím (E2E gọi thẳng handler activation nếu không bấm được toast tự động). macOS: CHƯA KIỂM nếu không có máy.
- [ ] `npm test` các workspace đụng tới pass; typecheck sạch.
- [ ] E2E trên **bản dev tách biệt** (`apps/desktop/e2e/cdp.cjs`, cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173). Toast của bản dev không được bấm nhầm vào Orrery thật: activation phải đi về đúng instance (kiểm cổng / app key), ghi rõ cách đảm bảo.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có (toast).
- Báo cáo: `.planning/wp/WP-0e-report.md`.

## Nhật ký
- 18:05 0e: viet apps/desktop/src/menuToast.ts (logic), mo rong collector toast.ts (answers)
- 18:05 0e: main.ts (setupMenuToast, handleMenuLink, openLink menu), preload menuToastSync/onMenuToast xong; tiep: web store + settings + test
- 18:07 0e: web lib/menuToast, composables/useMenuToast (gan o MenuQueueChip), settings menuToast, i18n; test desktop menuToast.test.ts viet
- 18:09 0e: web tests (lib/menuToast, composables/useMenuToast) pass; tiep: typecheck, full test, build, E2E
- 18:13 0e: them grace 2s cho menu nhap nhay (menuToast.ts), E2E v100-menu-toast.cjs dang chay thu
- 18:16 0e: E2E 14/14 PASS, chup toast docs/qa/v100/WP-0e/01-toast-crop.png; con: typecheck web, full test
