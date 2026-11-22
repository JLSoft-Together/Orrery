# devices

Tab Devices (Android qua adb): kết nối USB / Wi-Fi / QR, emulator (AVD), capture, UI test, app data, files, perf, APK size, device menu của browser pane.

- Đường vào: `@features/devices` (`DeviceMenu`, `DeviceCaptureDialog`, `hasDevices`, `hasApkSize`, `hasDeviceCapture`, `DEVICES` / `frameFit`, `useDevicesStore`, `useEmulatorsStore`).
- `DevicesView` và `ApkSizeDialog` nạp lazy trực tiếp trong App.vue / WorkspaceView.vue.
- File chính: `lib/devices.ts` (bridge), `stores/devices.ts`, `components/DevicesView.vue`.
- `lib/devPerf.ts`, `lib/devFiles.ts` chỉ dùng bởi DevicePerfDialog / DeviceFilesDialog (dời từ resources).
- i18n: `i18n/{devices-v61,devices-v72,devices-v75,emulators-v74,apk-v73,appdata-v73}.ts`, import tĩnh trong `src/i18n/{en,vi}.ts`.
