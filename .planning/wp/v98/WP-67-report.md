# WP-67 report: remote (Orrery Remote) trên desktop theo UI next

## Quyết định vị trí trạng thái điện thoại

**Chip "Điện thoại" ở chân rail, cùng hàng với nút Telegram và pill cập nhật** (`[data-next-rail-channels]`); rail thu gọn 56 px thì thành nút icon có badge. Thêm 2 mục Ctrl+K: "Ghép điện thoại", "Thiết bị đã ghép".

Lý do:
- SPEC v102 S1 bỏ AppHeader, chuyển nút của header vào chân rail. Telegram (kênh điều khiển từ xa anh em) đã nằm ở đó, nên điện thoại đứng cạnh Telegram cho cùng một nhóm "kênh từ xa".
- Footer đáy (v103 #6) đã chốt chỉ hiện hạn mức, một dòng gọn. Nhét chip vào đó làm footer hết đơn mục đích.
- Không có header next để đặt (S1). Rail luôn hiện ở chế độ full / icon; overlay (< 1024) thì mở rail là thấy, Ctrl+K vẫn tới được.
- Chip ẩn khi điều khiển từ xa tắt (không chiếm chỗ của người không dùng).

Trạng thái chip (icon + chữ + màu, không dùng màu đơn độc):
| Tone | Khi nào | Hiển thị |
|---|---|---|
| idle | bật nhưng chưa máy nào nối | icon điện thoại + "Điện thoại" + chấm xám |
| live | có máy đang nối | "N điện thoại" + chấm xanh; rail 56: badge số máy |
| alert | có IP bị **tự** chặn (lan / relay, không tính chặn tay), chặn còn hiệu lực, xảy ra trong 24 giờ | icon khiên + "Chặn N IP", nền warning-soft; rail 56: badge số IP |
| (thêm) | mọi máy có push đều tắt push | thêm icon chuông gạch |

Tooltip / aria-label: "2/3 điện thoại đã ghép đang nối, Push bật cho 2/3 máy, Đã chặn 1 IP trong 24 giờ qua, Bấm để mở Cài đặt, Điều khiển từ xa". Bấm → Settings trang `remote`, flash mục "Điện thoại đã ghép nối" (alert thì mục "IP bị chặn"). Đổi tone sang live / alert: viền sáng 1 lần (`--ccm-dur-signal` 450 ms, motion off = 0).

## Đã làm theo việc của brief

1. **Settings → Điều khiển từ xa trong next**: component giữ nguyên, nhận glass của `.mist-settings` (dialog `--ccm-glass-pop`, group `--ccm-glass`), đã soi mist sáng, nebula tối, galaxy tối: chữ đọc rõ, không cuộn ngang. Sửa thêm:
   - `lib/settingsFlash.ts` `centerRow`: flash một mục chỉ cuộn list gần nhất. Trước đây `scrollIntoView` kéo cả dialog lên ~15 px (cắt tiêu đề "Cài đặt"), bị lộ khi bấm chip / Ctrl+K.
   - Danh sách "IP bị chặn" đọc `remote.blocked` (một nguồn với chip); `useRemoteGuard.run` ghi luôn vào store.
   - `RemotePairPanel` ô QR: `bg-page` khi có mã, `bg-fill` khi lỗi (trước đây 2 class đè nhau).
2. **Trạng thái trong khung next**: như trên. Ctrl+K "Ghép điện thoại" mở Settings và mở luôn panel QR (`remote.askPair()` → `pairWanted`, `SettingsRemote` nhận khi đã bật); điều khiển từ xa đang tắt thì dòng phụ ghi "Bật điều khiển từ xa trước" và flash công tắc bật. "Thiết bị đã ghép" ghi "N/M đang nối".
3. **Pane từ điện thoại khớp next**:
   - `ws.open` / `ws.move` vốn đi qua store (`joinProject` → `dockNear`, `movePane` → `attach` / `dockGroup`) nên đã vào đúng nhóm project (`GroupNode`). Đã kiểm lại.
   - `ws.move` nay đi qua `workspaceBridge.track('phone', …)` → `useNextLayout.track` → toast "Điện thoại đã chuyển pane" + Hoàn tác 5 s, Ctrl+K "Hoàn tác bố cục" dùng được. Classic: `track` chạy thẳng, không toast.
   - `ws.close` / `ws.reopen`: đã vào `closedStack` nên toast Hoàn tác của desktop (`useCloseFlow`) hiện như khi đóng tay. Không thêm view transition cho open / close / reopen từ điện thoại vì `startViewTransition` chạy thay đổi bất đồng bộ (handler cần kết quả ngay); giống `open_session` của MCP.
   - **Hợp đồng mới** (tuỳ chọn, không phá app cũ): `RemoteTab.groups?: RemotePaneGroup[]` = `{project, name?, tone?, folded?, panes[]}`, chỉ có khi tab chia nhóm; `tree` vẫn phẳng như cũ. Ghi ở `docs/remote-integration.md` §4.3. Rust shell chỉ chuyển JSON nguyên, patch RFC 6902 tự xử lý field mới. Web app / Android chưa vẽ nhóm (WP-68 giữ `apps/pwa`, Android để gói sau).
4. **Click menu v100**: pane mở từ điện thoại là `TermPane` thường nên có thanh trả lời như pane khác (điều kiện bar không phụ thuộc nguồn mở). Phím từ điện thoại (`ws.keys` → `pane.typeKeys`) giờ **chờ** khi thanh click đang gửi phím nhiều bước (`termMenu.busy`, ví dụ chờ Submit rồi Enter), không chen giữa; ngược lại khi còn phím điện thoại chưa ghi thì click / pick trên thanh không gửi (`useTermMenu` option `blocked`). `ws.keys` vẫn có `since` chặn câu trả lời cũ.
5. **Classic**: không đổi giao diện (chip chỉ ở `NextShell`); 2 mục Ctrl+K có ở cả hai, typecheck + test pass.

## File + symbol

- Mới: `apps/web/src/components/next/rail/RailPhoneChip.vue`; `apps/web/src/lib/next/phoneChip.ts` (`phoneChip`, `recentBlocks`, `pushMuted`, `PHONE_ALERT_SEC`) + test; `apps/web/src/i18n/mist-remote.ts` (`mistRemote.*` en + vi) + test; `apps/web/src/lib/settingsFlash.test.ts`; `apps/desktop/e2e/v98-remote-next.cjs`.
- Sửa: `components/next/NextShell.vue` (chip, `phoneShown`, `data-next-rail-channels`); `stores/remote.ts` (`blocked`, `loadGuard`, watch `guardRev` + `enabled`, `pairWanted`, `askPair`); `composables/useRemoteGuard.ts`; `components/settings/SettingsRemote.vue`; `components/settings/remote/RemotePairPanel.vue`; `components/CommandPalette.vue` (`c:phone-pair`, `c:phone-devices`, `openPhone`); `lib/settingsFlash.ts` (`centerRow`); `lib/remoteSummary.ts` (`groupsOfTab`) + test; `packages/shared/src/remote.ts` (`RemotePaneGroup`, `RemoteTab.groups`); `lib/next/workspaceBridge.ts` (`track?`); `components/next/canvas/useNextLayout.ts` (`LayoutChange` thêm `phone`); `components/workspace/WorkspaceView.vue` (đăng ký `track`); `composables/useRemotePhone.ts` (`move`) + test; `composables/useTermMenu.ts` (`blocked`) + test; `components/workspace/TermPane.vue` (watch `typeKeys` + `termMenu.busy`, `blocked`); `i18n/mist-v102.ts` (`mist.drag.toast.phone`); `i18n/en.ts`, `vi.ts`; `docs/remote-integration.md`; `.code_index/_meta.md`, `web.md`.

## Test (lệnh chạy thật)

- `npm run qa -- --no-e2e --no-cargo`: typecheck `@ccm/web`, `@ccm/desktop`, `@ccm/collector`, `@ccm/shared` PASS; vitest related shared PASS; vitest related web: 1 fail `officeView.test.ts` (đếm lần gọi, chạy riêng `npx vitest run src/components/office/officeView.test.ts` 66/66 pass, chập chờn khi máy tải).
- `npx vitest run` (apps/web, toàn bộ): 314 file, **2699 pass, 1 fail** `DevicesView.test.ts` "qr popover" (chạy riêng 25/25 pass, chập chờn). `useRemotePhone.test.ts` "rename and move" (WP-70b báo fail) lần này pass trong cả bộ; chạy riêng 16/16.
- File liên quan: `npx vitest run src/composables/useRemotePhone.test.ts src/composables/useTermMenu.test.ts src/lib/next src/lib/remoteSummary.test.ts src/i18n src/components/settings/remote src/components/next` 28 file, 200 pass; `src/lib/settingsFlash.test.ts src/components/settings` 7 file, 47 pass.
- E2E desktop (Electron dev, `vite build` web trước, cổng riêng, dữ liệu riêng):
  - `v98-remote-next.cjs` (mới, `CCM_E2E_PORT=4481 CCM_E2E_DBG=9481`): **23 PASS, 0 FAIL** (chip ẩn khi tắt, idle / live / push tắt / alert, nằm ở chân rail không ở footer, bấm mở đúng trang + flash mục chặn, trang Phone glass + 3 máy + không cuộn ngang, Ctrl+K tìm "ghép" ra 2 mục và mở panel ghép, 3 palette, rail 56 có badge, motion off).
  - `v103-rail.cjs` **39 PASS 0 FAIL**, `v103-polish.cjs` **16 PASS 0 FAIL**, `v103-explorer.cjs` **38 PASS 0 FAIL**.
  - Vì đổi `TermPane` / `useTermMenu`: `term-ime.cjs` 9/0, `v100-menu-bar.cjs` 15/0, `v100-menu-click.cjs` 25/0.
- Không đụng Rust nên không chạy `cargo test`.
- E2E chạy lại làm ghi đè ảnh cũ ở `docs/qa/v100`, `docs/qa/v103`: đã `git checkout` trả lại.

## Ảnh `docs/qa/v98/WP-67/`

`01-chip-live-mist-light`, `02-chip-alert-mist-light`, `03-settings-phone-mist-light`, `04-settings-devices-mist-light`, `05-palette-phone-mist-light`, `06-settings-pair-mist-light`, `07-chip-mist-dark`, `08-chip-nebula-light`, `09-chip-galaxy-dark`, `10-settings-devices-galaxy-dark`, `11-settings-guard-galaxy-dark`, `12-settings-phone-nebula-dark`, `13-chip-collapsed-nebula-dark`. Electron không có remote thật nên trạng thái được bơm vào store (`remote.status`, `remote.blocked`); ảnh panel ghép ở trạng thái "không tạo được mã" (Electron không có `remotePairStart`).

## QA tay Windows (bản Tauri thật)

1. Settings → Điều khiển từ xa → bật. Chân rail hiện "Điện thoại" chấm xám. Tắt → chip biến mất (mờ dần).
2. Ghép web app / Android. Chip thành "1 điện thoại" chấm xanh, viền sáng 1 lần. Hover: số máy, push. Bấm: mở đúng trang, mục "Điện thoại đã ghép nối" nháy, dialog không bị xê dịch.
3. Tắt push cho mọi máy (chuông push trong Settings) → chip có chuông gạch.
4. Dùng sai token từ một máy khác (relay) tới khi bị chặn → chip vàng "Chặn 1 IP", bấm → mục "IP bị chặn". Gỡ chặn → chip về xanh.
5. Ctrl+K gõ "ghép" → "Ghép điện thoại" mở thẳng QR; "Thiết bị đã ghép" ghi N/M đang nối.
6. Thu gọn rail (Ctrl+Shift+B): chip thành icon có badge số máy / số IP.
7. Từ điện thoại: mở pane ở project đang có nhóm → pane vào đúng nhóm; chuyển pane sang tab khác → desktop hiện toast "Điện thoại đã chuyển pane", bấm Hoàn tác về chỗ cũ; đóng pane → toast Hoàn tác như đóng tay; đổi tên tab hiện trên tab pill.
8. Pane Claude đang hỏi (menu chọn): thanh click hiện như pane thường; bấm "Cho phép" trên điện thoại ngay lúc đang bấm một mục multi-select trên desktop → phím không lẫn nhau (một bên đợi bên kia xong).
9. Đổi palette mist / nebula / galaxy, sáng / tối, motion Giảm / Tắt: chip và trang Settings đọc rõ.

## Test macOS (chép vào `.planning/QA-mac-checklist.md`)

- Chip ở chân rail khi có title bar overlay: không bị vùng kéo cửa sổ nuốt click, tooltip nhiều dòng hiện đúng.
- Cmd+K "Ghép điện thoại" mở Settings + QR; quét được bằng iPhone (QR trên nền sáng của ô, cả khi theme tối).
- Settings → Điều khiển từ xa: flash mục khi bấm chip không làm dialog nhảy (WKWebView cuộn khác Chromium).
- Cmd+Shift+B thu gọn rail: chip icon + badge, không tràn 56 px.
- Phím từ điện thoại trong lúc thanh click đang gửi (multi-select + Submit): không chen phím.
- Galaxy tối có sao: chip vàng "Chặn N IP" vẫn tương phản.

## Rủi ro / việc dở

- Web app (`apps/pwa`, WP-68) và Android chưa đọc `groups`; chỉ là hợp đồng sẵn, hiển thị nhóm để gói sau.
- Chip alert dựa vào `remote.blocked` (gọi `remoteGuard get` khi `guardRev` đổi). Shell tự chặn thì `guard.strike` lưu file (tăng `rev`) rồi `Ctx::changed()` phát status mới (`guard_hooks.rs` `strike_lan`), nên chip tự cập nhật; chưa thử bằng bản Tauri thật.
- Open / close / reopen từ điện thoại không có view transition (lý do ở mục 3); động tác thấy được là toast + chip pane `RemotePhoneChip` + rail row.
- Ô QR khi lỗi tạo mã vẫn để trống icon trong lúc chuyển (chỉ thấy trên Electron, Tauri tạo mã thật).
- Không có tiến trình nền nào của gói còn chạy.
