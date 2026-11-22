# core-desk-devices: adb, emulator, APK, capture, phone

Đường vào: `apps/core/crates/core-desk/src/devices/ipc.rs` (`register`, kênh `ccm:dev-*`), `devices/manager.rs::DeviceManager`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-desk/src/devices/adb.rs` | gọi adb (timeout, parse) |
| `apps/core/crates/core-desk/src/devices/manager.rs` | danh sách thiết bị, connect USB / Wi-Fi / QR / mDNS, nhớ thiết bị |
| `apps/core/crates/core-desk/src/devices/qr.rs` | QR pair Wi-Fi |
| `apps/core/crates/core-desk/src/devices/emulators.rs`, `avd.rs` | AVD |
| `apps/core/crates/core-desk/src/devices/recorder.rs` | quay màn hình |
| `apps/core/crates/core-desk/src/devices/devfs.rs`, `files.rs` | file trên máy (pull / push / xoá) |
| `apps/core/crates/core-desk/src/devices/appdata.rs`, `sqlguard.rs` | dữ liệu app (SQLite chỉ đọc an toàn) |
| `apps/core/crates/core-desk/src/devices/perf.rs`, `ui_tester.rs`, `uidump.rs` | perf, UI test, dump giao diện |
| `apps/core/crates/core-desk/src/apk/analyze.rs` | APK size, manifest (`axml.rs`) |
| `apps/core/crates/core-desk/src/capture/mod.rs` | thư mục lưu ảnh / video, `capture.json` |
| `apps/core/crates/core-desk/src/phone/send.rs` | gửi push tới điện thoại (episode) |

Phía vỏ: `apps/shell/src/host_devices.rs` (`devices.confirm`, `devices.owner`, `device_used`, `devfs.progress`, `capture.*`), `capture.rs`, `shot.rs`, `native.rs` (chụp WebView).

## Bất biến

- Thao tác phá huỷ (gỡ thiết bị, wipe, xoá / ghi đè file, ghi prefs, `perf-cold`) qua `devices.confirm` native; kind lạ trả `false`, không mở dialog.
- Serial chỉ `[A-Za-z0-9._:-]{1,64}`; `device.used` throttle 15 s mỗi serial.
- Thiết bị đang dùng bởi owner nào thì giữ theo `<label webview>:<key>`, nhả khi trang reload.

## Liên kết

- Web: `web-devices.md`. Plan: `.planning/PLAN-v61-adb-devices.md`. E2E: `devices`, `v97-device`, `v101-*` không liên quan.
