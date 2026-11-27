## WP-113e2 — PARTIAL

Lib, wizard, hero, Advanced, settingsFlash xong và xanh vitest + typecheck. Chưa nhìn app, chưa chạy E2E (lệnh cấm lượt này), API e1 còn stub.

### Đã đổi
- `apps/web/src/features/remote/lib/connectFlow.ts`: reducer thuần 22 state, `findNewDevice` (deviceId mới HOẶC `pairedAt` mới), hằng thời gian.
- `lib/connectUsb.ts`: `checkDevice` / `usbTargets` (ẩn x86), `runUsb` với deps injection: install, reverse, pairStart(via usb, extraLan 127.0.0.1, adbSerialHash), openPair, chờ thiết bị mới, luôn gỡ reverse + huỷ mã khi lỗi.
- `lib/connectCheck.ts`: parse + tóm tắt kết quả `remote-check`, retry 3 lần, `unknownCheck` khi API chưa có.
- `lib/connectApi.ts`: adapter duy nhất tới desktop. Gọi wrapper có sẵn (`remote.ts`, `remoteInstall.ts`, `devices.ts`); API e1 gọi qua bridge ép kiểu, thiếu thì trả null / false (không throw).
- `components/connect/{PhoneConnectWizard,ConnectStepper,ConnectConsent,ConnectInstallStep,ConnectUsbStep,ConnectQrStep,ConnectPairStep,ConnectTestStep,ConnectLink}.vue`, `useConnectWizard.ts` (điều phối: enable, USB, QR, pair, test, toast).
- `components/RemoteHeroCard.vue` (thẻ trạng thái + 3 tag Wi-Fi/Relay/Push + nút + công tắc `data-remote-enable`), `components/RemoteAdvanced.vue` (gập `[data-remote-more]`: cổng + firewall, APK tuỳ chọn, relay, push, giữ máy thức, guard 3 mục, nhật ký), `components/SettingsRemote.vue` viết lại (hero/wizard, danh sách máy, Advanced).
- `i18n/v113-connect.ts` (en + vi, namespace `v113Connect`).
- `features/settings/lib/settingsFlash.ts`: `REMOTE_ADVANCED_ROWS`, mở fold rồi flash ở tick sau (hàng dùng `v-show` nên phải chờ hiện). `features/settings/components/index.ts`: thêm hàng `remoteConnect` (tạm dùng key cũ `remoteV97.add.*`, xem Hot file).
- Test: `connectFlow.test.ts`, `connectUsb.test.ts`, `connectCheck.test.ts`, `components/connect/PhoneConnectWizard.test.ts` (16 ca, chạy cả composable + từng bước với api giả), `settingsFlash.test.ts` (+4).
- E2E mới `apps/shell/e2e/v113-connect.cjs`, `scripts/qa-fast.mjs` (map `features/remote` thêm `v113-connect`).
- Sửa theo main: `connectApi.hasUsb` thành getter lười (DevicesView.test mock `../lib/devices` không còn lỗi); `connectCheck.test.ts` dùng `?.` cho `lines[i]` (TS2532).

### Tiêu chí
- [x] Vitest connectFlow (mọi chuyển + ghép lại máy cũ, relay tắt không LAN, signature, remote tắt giữa chừng), connectUsb (mọi lỗi + luôn gỡ reverse), connectCheck, component từng bước, settingsFlash mở fold rồi flash.
- [ ] E2E cũ remote + v113-connect: CHƯA KIỂM: E2E chạy ở lượt sau (không chạy E2E / build shell lượt này).
- [x] `npm run typecheck`, vitest devices + remote + settings + importRules.
- [x] Không comment, file ≤ 400 dòng (lớn nhất `useConnectWizard.ts` 362), i18n en + vi đủ trong file riêng.
- [ ] Chụp từng state vào `docs/qa/v113/WP-113e2/`: CHƯA NHÌN.

### Audit
- Selector cũ còn: `data-settings-remote`, `data-remote-enable` (trong hero), `data-remote-add` (nút Kết nối), `data-remote-pair` / `data-remote-qr` / `data-remote-countdown` (bước pairWait), `data-remote-devices`, `data-remote-log` (trong fold, `v-show` nên DOM vẫn có), `data-setting-row` remote, remoteAdd, remoteConnect (lồng 3 lớp trong 1 hộp), remoteDevices, remoteInstall, remoteRelay, remotePush, remoteAwake, remoteBlocked, remoteTrusted, remoteAccess, remoteLog. Đã mất: `data-remote-listener` / `-relay` / `-firewall` / `-address` vẫn còn nhưng nằm trong fold; `data-remote-add` không còn bị disabled khi remote tắt (đi qua bước consent).
- `RemotePairPanel.vue` không còn được dùng bởi SettingsRemote (file còn nguyên, test cũ vẫn xanh). Ctrl+K "Ghép điện thoại" (`remote.pairWanted`) mở wizard thẳng ở bước pairWait.
- Không đụng Rust, `desktop.ts`, `remote*.ts`, i18n en/vi.

### Review
- Mỗi lần vào `pairWait` / `qrInstall` / `usbInstalling` lấy baseline mới; so deviceId HOẶC pairedAt.
- `pairConfirming` chỉ phát khi đã thấy `status.pairing` khác null rồi về null (giống pha confirming cũ của RemotePairPanel).
- Rời state QR gọi `installArm(false)` + `installStop`; rời pairWait/USB gọi `pairCancel`; unmount dọn hết (abort USB, check, timer).
- Reduced-motion: bước chuyển chỉ còn fade 160ms, đường nối hành tinh tắt animation.
- Giới hạn đã biết: `enableFail` phân biệt "port" (enabled nhưng không listening) và "timeout" (không enabled) bằng suy luận; cần e1 / main xác nhận lỗi cổng bận thật.
- Chưa có: nút "Gỡ và cài lại" cho `signature` (chưa có API gỡ app; hiện chỉ hướng dẫn gỡ tay), tiến độ % cài (desktop chưa phát), tile "Tải file APK / Lưu vào Downloads" (chờ op `save` của e1), nút "Mở trên điện thoại" ở pushHelp (cần API adb mở cài đặt).

### Test
- `npx vitest run src/features/devices src/features/remote src/features/settings src/importRules --hookTimeout=90000 --testTimeout=90000` (trong apps/web) → 43 file, 348 test xanh.
- Cùng lệnh nhưng timeout mặc định 10s: `useRemoteSummary.test.ts` và `useRemotePhone.test.ts` (không đổi) timeout hook 7 đến 10s do import lạnh nặng khi máy CPU ~97%; chạy lại với timeout 90s xanh hết. Nên chạy lại khi máy rảnh.
- `npm run typecheck` (root, sau sửa cuối) → không có dòng lỗi TS nào (lọc error/TSxxxx rỗng).
- E2E: CHƯA KIỂM: E2E chạy ở lượt sau. `v113-connect.cjs` chỉ qua `node --check`.

### Nhìn
CHƯA NHÌN: không chạy app lần này. Cần chụp idle, install, usbInstalling, qrInstall, pairConfirm, testDone, done (sáng + tối) sau lần sửa code cuối.

### Hot file cần thêm
`apps/web/src/i18n/en.ts` và `apps/web/src/i18n/vi.ts` (cạnh dòng `v106Install`):
```
import v113Connect from '../features/remote/i18n/v113-connect'
...v113Connect.en,   // vi.ts: ...v113Connect.vi,
```
Sau khi gộp: trong `apps/web/src/features/settings/components/index.ts` đổi hàng `remoteConnect` sang `labelKey: 'v113Connect.hero.connect', hintKey: 'v113Connect.hero.none.hint'` (đang dùng tạm key cũ để `index.test.ts` không đỏ), chạy lại `settings/components/index.test.ts`, snapshot i18n nếu có.
`apps/web/src/features/devices/index.ts`: nên export `devInstall, devOpenPair, devReverse, onDevEvent, hasDeviceInstall` để `connectApi.ts` bỏ import sâu `../../devices/lib/devices`.

### API e1 cần nối
- `desktop.remotePairStart(opts)` nhận `{via:'usb'|'qr'|'lan', extraLan?, adbSerialHash?}` và trả `{qr, exp, relay?, lan?}` (adapter đã truyền opts qua bridge; bản Rust cũ bỏ qua opts).
- `desktop.remoteCheck(deviceId)` trả `{lan:{ok,ms?}, relay:{state,ms?}, push:{ok,error?}}` (error `rate` / `none` có xử lý riêng). Thiếu thì wizard hiện "không chạy được kiểm tra" và vẫn cho xong.
- `desktop.remoteInstall({op:'arm'|'disarm'})` (adapter `installArm`).
- Sự kiện `desktop.onRemoteInstallFetched(handler({ip}))` cho `install.fetched` (adapter `onInstallFetched`; tên bridge là giả định, e1 đặt tên khác thì sửa 1 dòng trong `connectApi.ts`).
- Ticket: `parseTicket` trong `remoteInstall.ts` còn bắt đuôi `/orrery.apk`; wizard cắt phần tên file để QR trỏ trang `/install/<token>/`. Nếu e1 đổi format path thì cập nhật regex.
- `remoteRevoke(id, grace)`, `remoteRevokeUndo(id)`: lượt này chưa dùng (nằm ở F4 / danh sách máy).
- Cần thêm (chưa có trong brief e1): op `uninstall` có xác nhận cho `usbFail{signature}`, op `save` APK, mở cài đặt thông báo / pin qua adb.

### Ngoài phạm vi
- `RemoteDeviceRow` giữ nguyên (F4 thay).
- Bản APK đi kèm: wizard lấy `installState().apk` hoặc `suggested`; chưa có APK đóng gói sẵn thì QR báo lỗi `apk`, USB báo `other`.

### Commit message đề xuất
```
feat(remote): three step phone connect wizard with usb install, qr page, connection test and collapsible advanced settings
```
