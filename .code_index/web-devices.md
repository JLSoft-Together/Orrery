# web-devices: tab Devices (Android qua adb)

Đường vào: `apps/web/src/features/devices/index.ts` (`DeviceMenu`, `DeviceCaptureDialog`, `hasDevices`, `hasApkSize`, `useDevicesStore`, `useEmulatorsStore`). `DevicesView` và `ApkSizeDialog` nạp lazy trong `App.vue` / `WorkspaceView.vue`.

Nội dung: kết nối USB / Wi-Fi / QR, emulator (AVD), capture, UI test, app data, files, perf, APK size, menu thiết bị của browser pane.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/devices/lib/devices.ts` | bridge `ccm:dev-*` |
| `apps/web/src/features/devices/stores/devices.ts` | danh sách thiết bị, thiết bị nhớ, auto-connect |
| `apps/web/src/features/devices/components/DevicesView.vue` | màn Devices |
| `apps/web/src/features/devices/lib/devPerf.ts`, `devFiles.ts` | chỉ dùng bởi DevicePerfDialog / DeviceFilesDialog |

## Bất biến

- Thao tác phá huỷ qua confirm native; thiết bị đang dùng giữ theo owner (xem `core-desk-devices.md`).
- adb lấy từ SDK của người dùng, không đóng gói.

## Liên kết

- Phía desktop: `core-desk-devices.md`. Kiểu: `packages/shared/src/devices.ts`, `apk.ts`, `emulators.ts`. i18n: `apps/web/src/features/devices/i18n/`. Plan: `.planning/PLAN-v61-adb-devices.md`. E2E: `devices`, `v97-device`.
