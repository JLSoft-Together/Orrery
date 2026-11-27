## WP-113f — PARTIAL

Code, test, typecheck xong. PARTIAL vì: (1) chưa NHÌN trang, (2) E2E chưa chạy, (3) cần main thêm import i18n + cập nhật snapshot keys.

### Đã đổi
- Mới: `features/devices/lib/mergeDevices.ts` (gộp theo hash, gợi ý theo model + tên gần giống, lưu `ccm.deviceMerge`, lọc/đếm), `lib/deviceActions.ts` (so version, nút chính update/install/boot/blocked), `composables/useSettingsDevices.ts` (state + hành động), `components/SettingsDevices.vue`, `components/SettingsDeviceRow.vue`, `i18n/devices-v113.ts` (en + vi, top-level `v113Devices`).
- Test mới: `lib/mergeDevices.test.ts` (25), `components/SettingsDevices.test.ts` (13).
- Sửa: `core/lib/uiTypes.ts` (page `devices` sau `remote`), `features/settings/components/index.ts` (cờ `devices`, `PAGE_NEEDS` nhận mảng, hàng `devicesList` / `devicesAdb`), `SettingsMenu.vue` (icon, render trang), `composables/useSettingsEnv.ts`, `settings/i18n/ui-v79-settings.ts` (tên trang `devices`), `devices/index.ts` (export `SettingsDevices`), `features/remote/index.ts` (barrel thêm export: `activeDevices, expMs, isConnected, remoteCheck, remoteRevoke, remoteRevokeUndo, remoteStatus, installGet, installUseSuggested, hashSerial`; không sửa `remote/lib/*`).
- Test cũ chỉnh cờ `devices` trong env: `settings/components/index.test.ts` (thêm test 2 cờ + hàng adb), `rows.test.ts`, `palette/lib/paletteSearch.test.ts`.
- E2E: `apps/shell/e2e/devices.cjs` thêm bước mở Settings > Devices (danh sách, lọc adb, mở chi tiết, chụp `devices-05-settings-page`).

### Tiêu chí
- [x] mergeDevices.test: gộp theo hash (kể cả khác hoa thường), gợi ý theo model + tên, không gộp chỉ theo tên, không gộp chỉ theo model, lựa chọn Gộp / Không được nhớ trong `ccm.deviceMerge`.
- [x] settings index.test: page devices hiện khi có 1 trong 2 cờ (`devices` hoặc `remote`).
- [x] Component test: lọc, mở chi tiết, Hoàn tác thu hồi, trạng thái trống, Cập nhật / Cài Remote, emulator x86_64, gợi ý gộp, desktop cũ.
- [x] Import qua barrel (importRules xanh); typecheck sạch; i18n en + vi (đã nằm trong file riêng).
- [x] Không comment, file ≤ 257 dòng.

### Audit
- Diff trong phạm vi, cộng: `uiTypes.ts`, `remote/index.ts` (barrel), `settings/i18n/ui-v79-settings.ts`, 3 test env. Không đụng Rust, `remote/lib/*`, `i18n/{en,vi}.ts`, màn Devices cũ, file của WP-113e3 / 113g1.
- Dùng sẵn: `ToggleSwitch`, `EmptyState`, `SettingEmbed`, `DeviceCaptureDialog`, `ccm-list` / `ccm-fade` (không CSS mới), token `bg-success-soft` / `bg-warning-soft` / `ccm-seg`.

### Review
- Không thiết bị: trạng thái trống 2 nút (Kết nối điện thoại -> `ui.settingsPage='remote'`; Ghép adb qua Wi-Fi -> đóng Settings, `ui.view='devices'`, ẩn nếu không có bridge adb).
- Chỉ có adb: dòng adb, nút Cài Remote / Cập nhật theo `devPkgVersion`; chưa biết version thì không hiện nút.
- Emulator x86_64 (`avd.abi` chứa x86): ẩn nút, ghi lý do; emulator tắt: nút Khởi động (store emulators).
- Phone đang revoking: dòng mờ, chip "Đang thu hồi", nút Hoàn tác ngay trên dòng + toast 5s có Hoàn tác (`remoteRevoke(id,5)`, `remoteRevokeUndo`); hết 5s core thu hồi thật, dòng rời list bằng `ccm-list`.
- Desktop cũ thiếu `appVersion` / `pairedVia` / `adbSerialHash` / `revoking`: tag "Remote" không số, không gộp theo hash, không nút; có test.
- Version desktop = `version` APK bundled (`installGet`); trống thì không hiện Cập nhật.
- ms hiển thị chỉ sau khi bấm Gửi thử (`remoteCheck` gửi push thật nên không gọi tự động).
- Reduced-motion: spinner `motion-reduce:animate-none`; chuyển động dùng `ccm-list` / `ccm-fade` (đã tôn trọng reduced-motion trong style.css).

### Test
- `npm test -w @ccm/web -- src/features/settings src/features/palette src/i18n src/importRules src/features/devices` -> 27 file pass, 1 file fail: `i18n/keys.snapshot.test.ts` (en + vi "keys are unchanged"), do key mới `settingsV79.pages.devices`; còn phải cộng `v113Devices.*` khi main thêm import. Cần cập nhật snapshot sau khi gộp các WP.
- `npm test -w @ccm/web -- src/features/devices` -> 13 file, 112 test pass (1 lần DevicesView "qr popover" fail khi máy 100% CPU, chạy lại pass, không liên quan).
- `npm run typecheck` -> sạch.
- CHƯA KIỂM: E2E lượt sau (`node apps/shell/e2e/devices.cjs`, theo brief không chạy); kênh `remoteRevoke` grace thật trong app (cần shell); `DeviceCaptureDialog` mở chồng lên cửa sổ Settings (z-index).

### Nhìn
CHƯA NHÌN. Cần chụp vào `docs/qa/v113/WP-113f/`: trang Thiết bị có dữ liệu (1 dòng gộp Remote + USB adb có nút Cập nhật, 1 Remote qua relay, 1 web app offline, 1 emulator tắt), chi tiết mở, hỏi thu hồi + toast Hoàn tác, gợi ý "Cùng một máy?", trạng thái trống; dark + light; kiểm không cuộn ngang ở 420 px.

### Hot file cần thêm
- `apps/web/src/i18n/en.ts`: `import devicesV113 from '../features/devices/i18n/devices-v113'` và `...devicesV113.en,` (cạnh `devicesV75`); `vi.ts` tương tự với `...devicesV113.vi`.
- Cập nhật snapshot `apps/web/src/i18n/__snapshots__/keys.{en,vi}.txt` (`vitest -u` sau khi gộp mọi WP).
- `apps/web/src/features/devices/README.md` và `.code_index`: thêm `SettingsDevices`, i18n `devices-v113.ts`, `mergeDevices.ts` (chưa làm).
- Ghi `.code_index/issues.md`: gộp màn Devices cũ và trang này sau khi dùng thử (spec F4 mục 7).

### Ngoài phạm vi
- `remote.revoke()` trong store vẫn không grace (trang Remote cũ giữ nguyên); trang này gọi `remoteRevoke(id, 5)` trực tiếp.
- Trang Remote vẫn còn danh sách máy riêng (`RemoteDeviceRow`), spec nói tạm giữ.

### Commit message đề xuất
feat(devices): unified Settings Devices page merging Remote phones, adb devices and emulators with revoke undo
