# WP-68b1 báo cáo: `packages/mist-tokens` + khung web app "Sương kính"

## Đã làm
1. **`packages/mist-tokens`** (`@ccm/mist-tokens`, workspace): `tokens.css` (chỉ biến + glass reduced + motion, selector `:root[data-ui='next']` giữ nguyên tên biến), `palettes.css`, `palettes.ts` (git mv từ `apps/web/src/next/`). `exports`: `.` -> `palettes.ts`, `./tokens.css`, `./palettes.css`.
   - `apps/web`: 6 file import `@ccm/mist-tokens`; `main.ts` import package `tokens.css`, rồi `next/tokens.css` (phần component, giờ có thêm khối `background-image` + `background-attachment: fixed` của desktop vì pwa không dùng `fixed`), rồi package `palettes.css`. Thứ tự cascade giữ như cũ. `palettes.test.ts` đọc CSS từ package + `next/tokens.css`.
2. **pwa dùng package**: `index.html` `data-ui="next" data-next-theme="mist"`; `main.ts` import package CSS, `styles/fonts.css`, `styles/tokens.css`, `styles/base.css`.
   - `styles/tokens.css` chỉ còn tên không có trong mist (font, `--ease-out`, `--ccm-term-*`, `--radius-chip`, `--ccm-scanner`) + `--ccm-bar` (78% `--ccm-material`, đặc khi Giảm trong suốt), blur điện thoại 14 px / 16 px, `--ccm-tabbar-*`, `--ccm-safe-*`. Alias `--ccm-info` -> working, `--ccm-agent-shell` -> `agent-shell-mark`.
   - `base.css`: nền sương + sao vẽ 1 lần trên `html::before` (fixed, không `background-attachment`), body trong suốt; `.glass-bar`, `.glass-sheet`; `html[data-fade]` chuyển màu 320 ms khi đổi bảng màu; `.card` kính; `.btn-primary` gradient accent.
   - Thanh trên `PageBar` sticky kính full bề ngang; dải kính dưới status bar (`App.vue` `.shell::before`); tab bar viên nổi (`TabBar.vue`, absolute trong `.master`, `.scroll.tabbed` chừa chỗ); sheet kính (`BottomSheet`, `ConfirmHost`, `RemoteConfirmHost`); FAB / UndoBar dời lên trên viên nổi; `Toggle` theo mockup. Blur chỉ ở thanh trên, dải status, tab bar, sheet; thẻ / danh sách / terminal / dock không blur.
3. **Font**: `styles/fonts.css` Manrope + JetBrains Mono (latin / latin-ext / vietnamese, `@fontsource-variable` cùng gói desktop, thêm vào `apps/pwa/package.json`), build ra `assets/*.woff2` (6 file, ~110 KB), CSP `font-src 'self'` không đổi, SW precache tự lấy. Terminal xterm giữ font hệ thống (không đổi đo cell).
4. **Cài đặt → Giao diện**: `PaletteCards.vue` (8 thẻ radiogroup, preview từ `NextPreview`, mặc định Sương kính, bảng chỉ tối thì ghi "Chỉ có nền tối"), select Sáng / Tối / Theo máy giữ, toggle **Giảm trong suốt** (mặc định tắt, tự bật khi `prefers-reduced-transparency`). Lưu `localPrefs.palette` + `reduceTransparency`; áp tức thì qua `lib/appearance.ts` (`resolveAppearance`, `applyAppearance`, `startAppearance`), có fade màu, cập nhật `theme-color`. `data-motion=reduced` theo `prefers-reduced-motion`.
5. **Màn hiện có**: không đổi cấu trúc; nhận token mới. Màu diff terminal đổi sang `--ccm-term-add|del|hunk*` (nền terminal luôn tối).
6. i18n `en` + `vi` (không em dash, không chấm giữa). `.code_index/web-pwa.md` + `_meta.md` cập nhật.

## Lưu ý / quyết định
- `--ease-out` trước đây **không được định nghĩa** nên animation `push` / `pop` / `rise` của pwa không chạy. Giờ định nghĩa = `--ccm-ease-enter` nên chúng chạy thật (300 ms). Hệ quả duy nhất trong test: Chromium với camera giả quét lại QR cũ rất nhanh ngay khi vào `/pair`, nên các chỗ bấm tab "Nhập mã" (`ctx.pairPaste`, `pane-steps`, `shots`, `scenario`, `lock-offer-step`) đổi sang `dispatchEvent('click')` (không đợi `stable`). Hành vi app không đổi.
- `STEPS_PER_BROWSER` 43 -> 44 (thêm 1 bước palette + Giảm trong suốt). **WP-68b2 / b3 nếu cũng thêm bước E2E thì phải cộng dồn con số này khi gộp.**
- `npm install --offline` đã chạy: `package-lock.json` có thêm workspace `packages/mist-tokens` + 2 gói fontsource cho pwa.
- Không đụng: shell remote, `remoteSummary`, `packages/shared`, store pwa của b2. Cả hai gói cùng sửa `.code_index/_meta.md` (chỉ nối dòng cuối).

## Test (lệnh chạy thật)
- `npm test -w @ccm/web -- src/next src/lib/uiGeneration`: 12 / 12 (`palettes.test.ts`).
- `npm test -w @ccm/web`: 2699 / 2700 (313 / 314 file); 1 fail là `RemotePairPanel.test.ts` "QR v2 counts down" (không đụng file này), chạy riêng `npm test -w @ccm/web -- src/components/settings/remote/RemotePairPanel.test.ts` 5 / 5 pass, tức flake dưới tải.
- `npm test -w @ccm/pwa`: 227 / 227 (24 file, gồm `lib/appearance.test.ts` mới: resolve, apply, startAppearance + fade, migration prefs).
- `npm run typecheck -w @ccm/web`, `npm run typecheck -w @ccm/pwa`: sạch.
- `npm run e2e -w @ccm/pwa -- --no-peer-build --relayPort=8793 --inspectorPort=9353`: **88 / 88** (WebKit 44 + Chromium 44), 0 vi phạm CSP, không tràn ngang; bước mới kiểm blur tab bar <= 16 px, đổi 8 bảng màu, Giảm trong suốt bỏ blur, lưu qua reload.
- Desktop: `npm run build -w @ccm/web` ok; E2E `apps/desktop/e2e/v103-rail.cjs` (cổng 4431 / 9431, dữ liệu riêng, sau `npm run build -w @ccm/desktop` vì `stage` cũ không lên): 0 FAIL, toàn bộ check PASS (giao diện desktop không đổi).
- Ảnh: `docs/qa/v98/WP-68b1/` (Home / Monitor A / Settings phone + iPad sáng / tối trên WebKit; Settings từng bảng màu phone WebKit + Chromium; Settings Giảm trong suốt). Ảnh cũ `WP-55b|56a|61a|61b|65` và `v103/WP-70a` bị e2e ghi đè đã `git checkout --` lại.
- Ghi chú: `vite build` trong e2e thỉnh thoảng chết lúc thoát (`Assertion failed ... async.c`, node 24 trên Windows) dù bundle đã xong, chạy lại là qua. Bước push Chromium từng timeout 1 lần ở lượt có tải cao rồi qua ở lượt sau.

## Việc dở (cho b3 / b4)
- Màn Home / Monitor / Workspace / Viewer chỉ nhận token, chưa restyle chi tiết (b3 / b4); iPad vẫn dùng master / detail + tab bar viên nổi trong cột 380 px, rail trái là b4.
- Dải kính dưới status bar chỉ thấy khi có safe-area (iPhone thật); trên emulator desktop `safe-area-inset-top` = 0.
- Blur thật (`backdrop-filter`) trên WebKit headless không đại diện cho iPhone: cần kiểm tay.

## Test macOS / iPhone
- [ ] Mở web app trên iPhone (Safari và bản Thêm vào Màn hình chính): nền sương hiện đủ khi cuộn dài, không giật; nền không trượt theo cuộn.
- [ ] Thanh trên + dải trên status bar liền một khối kính, nội dung cuộn lướt dưới thanh thấy mờ; tab bar viên nổi cách đáy đúng home indicator, bấm 3 tab đủ vùng chạm 44 px, tab đang chọn có nền accent nhạt.
- [ ] Cài đặt → Giao diện: bấm từng thẻ trong 8 bảng màu, đổi tức thì có fade; Ngân hà và Console luôn tối dù chọn Sáng; sáng / tối / theo máy đúng khi đổi Appearance của iOS.
- [ ] Bật iOS Cài đặt → Trợ năng → Hiển thị & cỡ chữ → Giảm độ trong suốt: web app tự chuyển Giảm trong suốt (thanh / sheet đặc); công tắc trong app tắt / bật tay được; thoát app mở lại vẫn nhớ.
- [ ] Sheet (menu pane, gửi prompt, xác nhận) dạng kính nổi cách mép 8 px, kéo / bấm nền để đóng; không bị home indicator che nút.
- [ ] Chữ Manrope hiện đúng tiếng Việt có dấu (không rơi về font hệ thống), terminal vẫn đúng độ rộng cột.
- [ ] Xoay ngang iPhone và iPad: tab bar không che nội dung cuối danh sách; cuộn vẫn 60 fps (ảnh hưởng của blur).
- [ ] Desktop macOS: giao diện Mist vẫn y như trước (nền, thẻ, rail, Settings → bảng màu), vì token đã tách sang package.
