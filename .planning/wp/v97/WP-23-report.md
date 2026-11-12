## WP-23 — DONE (gồm phần mở rộng đổi tên session)

### Đã đổi
- Shell: `apps/shell/src/remote/devices.rs` (`Device.alias` tách khỏi `name`, `clean_alias` max 40 ký tự bỏ ký tự điều khiển / ẩn, `Devices::rename`, `display_name`; `touch` trả tên hiển thị cho notification), `status.rs` (`name` = tên hiển thị, thêm `originalName`, `alias`), `mod.rs` (`rename_device`, kênh `ccm:remote-device-rename {deviceId, name}`, `CHANNELS` 17), `ccm.rs` (`LOCAL` 33), `unit_tests.rs` (+2 test), `shim.js` regen.
- Desktop/web: `preload.ts` `remoteDeviceRename`, `packages/shared/src/remote.ts` (`RemoteDevice.originalName/alias`, `REMOTE_DEVICE_ALIAS_MAX`), `lib/desktop.ts`, `lib/remote.ts`, `stores/remote.ts renameDevice`, `SettingsRemote.vue`, `RemoteDeviceRow.vue` (bút chì → input tại chỗ, Enter lưu, Esc / blur huỷ, rỗng = tên gốc, dòng "Ghép nối với tên …"), `i18n/remote-v97.ts` (vi + en), test mới `RemoteDeviceRow.test.ts` (5).
- Mobile: `lib/remote/machines.dart` (`Machine.alias`, `displayName`, `cleanAlias`, `MachineRepository.rename`, `save` luôn giữ alias hiện có, forget xoá), `providers.dart rename`, `ui/widgets/name_dialog.dart` (dialog dùng chung), `ui/workspace/session_rename.dart`, `pane_actions.dart` (mục "Đổi tên session"; "Đổi tên tab" = `ws.rename` ĐÃ có UI sẵn), `monitor_page.dart` + `monitor_model.dart` (menu ⋮ + giữ lâu), `home_page.dart` (menu ⋮ + giữ lâu, dòng "Máy tính đặt tên …"), `workspace_page.dart` (tiêu đề dùng `displayName`), `ui_providers.dart` + `workspace_model.dart` + `pane_facts.dart` (tên pane lấy alias từ snapshot), ARB vi / en qua `tool/gen_arb.mjs`, test `rename_test.dart` (7) + `rename_shots_test.dart`, `support/ui_fakes.dart` (FakeMachines.rename).
- Docs/index: `.code_index/{_meta,shell,web}.md`, `apps/mobile/.code_index/mobile.md`, `docs/orrery-remote.md` (1 đoạn).
- Phần session: KHÔNG cần lệnh mới. `terminal.alias {terminalId, alias|null}` đã có xuyên suốt (phone → `dashboard.rs DASHBOARD` → StoreTap `TAP_ALLOWED` → collector / core `setAlias`, max 80) nên điện thoại dùng luôn, desktop thấy ngay. Không sửa `apps/core` và `parseRemoteCommand` (lệnh này không đi qua parser `ws.*`).

### Tiêu chí 4/4
- [x] `cargo test --locked -j 3 remote` -> 135 pass (rename lưu + nạp lại, rỗng trả tên gốc, 40 ký tự đếm theo emoji, bỏ ký tự điều khiển / U+200B / U+202E, id lạ -> false, thiếu name / sai kiểu -> false, thu hồi rồi ghép lại -> alias rỗng).
- [x] `npm run typecheck -w @ccm/web -w @ccm/shared -w @ccm/desktop` sạch; Vitest RowRename 5 pass (sửa / Enter / Esc / rỗng / không đổi / emoji 40), i18n + stores + `remoteParse` + settings 26 pass. `cargo test -p core-http tap` 3 pass.
- [x] `flutter analyze` sạch; `flutter test --concurrency=4` 195 pass (17 skip: integration + shots) gồm alias lưu / xoá / giữ sau save, hiện ở home / pane / Monitor, `terminal.alias` đúng payload (giá trị rỗng -> null).
- [x] vi + en, không comment, index cập nhật. File: `shell/remote/mod.rs` 425 dòng (đã 409 từ trước, +16 của con), còn lại < 400.

### Audit
Diff mine chỉ trong phạm vi brief + phần mở rộng. Không đụng `pair_page.dart`, `apps/relay`, `apps/core`. Trong working tree còn thay đổi KHÔNG phải của con: `fcm_files.rs`, `push_setup*.rs`, `docs/remote-integration.md`, `apps/mobile/shots/{01-machines,01b-machines-dark,11-pane-actions}.png`, snapshot tracker / fwp-host, WP-24.md (WP khác).

### Review (edge case)
- Tên toàn khoảng trắng / ký tự điều khiển: web, shell, mobile đều trim + lọc, kết quả rỗng = xoá alias về tên gốc.
- Emoji 40+: cắt theo ký tự (rune), không vỡ cặp surrogate (Rust `chars`, Dart `runes`, web `Array.from`). Ảnh `rename-4-long` cho thấy cắt đúng 40.
- Đổi tên khi thiết bị đang kết nối: alias ghi vào `devices.json` + `ctx.changed()` phát lại status, không đụng phiên đang chạy; chỉ thiết bị nằm trong list mới đổi được (id lạ -> false).
- Thu hồi rồi ghép lại: desktop xoá cả device nên alias mất (có test). Điện thoại: forget xoá alias; ghép lại máy chưa forget thì giữ alias (`save` không bao giờ ghi đè alias, kể cả copy cũ của connection đang giữ).
- Đổi tên máy trên desktop (hello `name`) cập nhật `name` gốc ở điện thoại, alias giữ nguyên.
- Đổi tên cục bộ, không đổi giao thức, push title / Telegram deep link không đổi.

### Test
Như trên. Lỗi lặp 1 lần: shots flutter thất bại vì tooltip vi ("Thao tác tab") — đã sửa script.

### Nhìn
- Desktop: `docs/qa/v97/WP-23/rename-{1-list,2-editing,3-saved,4-long,5-esc,6-restored}-{light,dark}.png` (12 ảnh) + `shots.cjs`; dev Tauri port 4432, debug 9352, remote 4330, data dir `%TEMP%\ccm-e2e-wp23`, seed `devices.json` 3 máy. Con mở xem editing-light, long-dark, restored-light: input có focus ring, bút chì cạnh tên, dòng "Ghép nối với tên …", cắt 40 ký tự, chip trạng thái không bị đẩy, không cuộn ngang. Process orrery-shell của con đã dọn; Orrery.exe thật không đụng.
- Mobile: `docs/qa/v97/WP-23/mobile/` 14 ảnh (home alias / menu / dialog, pane header / menu / dialog, Monitor menu, light en + dark vi). Con mở xem home-menu-light, pane-menu-dark, monitor-menu-light: menu "Đổi tên máy tính", "Đổi tên session" và "Đổi tên tab" cùng có mặt, tiêu đề đã là alias ("Ads work", "Office PC"). Viền đen quanh popup / nút ghép là focus ring của test headless, cũng có ở ảnh cũ.
- Không dùng A21s.

### Hot file cần thêm / chưa làm
- "Log vào Lệnh từ điện thoại" cho đổi tên session: CHƯA làm. `terminal.alias` đi thẳng phone → shell dashboard → StoreTap, không qua trang web nên log desktop (`stores/remote.ts record`) không thấy. Muốn có thì cần shell phát 1 event khi nhận `terminal.alias` (`apps/shell/src/remote/phone_ws.rs` / `dashboard.rs`) và `useRemotePhone.ts` ghi log; xin duyệt nếu cần.
- Kiểm tra chưa làm: chạy thật điện thoại (cấm A21s); tên pane trên desktop sau khi đổi từ điện thoại dựa vào collector `setAlias` đã có (không kiểm lại trong WP này).

### Commit message đề xuất
feat(remote): rename paired phones on the desktop and computers and sessions on the phone, with a local alias that keeps the original name and a 40 character limit
