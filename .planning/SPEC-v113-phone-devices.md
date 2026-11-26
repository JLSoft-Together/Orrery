# SPEC v113: điện thoại, thiết bị, 20 tính năng

Đi kèm `.planning/PLAN-v113-phone-devices.md`. Mỗi phase: mục tiêu, file, hành vi, hợp đồng dữ liệu, state, test, xong khi, rủi ro. Path tính từ gốc repo; `remote/` = `apps/shell/src/remote/`, `desk/` = `apps/core/crates/core-desk/src/`, `web/` = `apps/web/src/`, `mobile/` = `apps/mobile/lib/`.

---

## F0. Nền: thông tin thiết bị, keystore cố định

**Mục tiêu.** Desktop biết phiên bản app, model, Android version của từng điện thoại; APK luôn ký bằng 1 key để cài đè được.

**Hiện tại.** `hello` chỉ gửi `proto`, `app`, `platform`, `caps` (`mobile/remote/connect_route.dart:36-41`); pair gửi `deviceName`, `deviceId`. `Device` (`remote/devices.rs:19-48`) không có version / model. Bản release đang ký bằng debug key (`apps/mobile/android/app/build.gradle.kts:37`): mỗi máy build một chữ ký khác, cài đè sẽ lỗi.

**Làm.**
1. Phone: thêm `package_info_plus`; CHỈ `hello` gửi thêm `appVersion` (versionName), `appBuild`, `model` (`device_info_plus`: manufacturer + model, ≤ 40 ký tự), `os` (vd `Android 14`). Không đổi pair message (tránh đụng `core-remote-proto/src/pair/mod.rs:96-107`, `core-remote-wasm`, `crypto_rust.dart`, PWA).
2. Desktop: `Device` thêm `app_version`, `app_build`, `model`, `os` (`Option<String>`, serde default để file cũ đọc được); cập nhật lúc xử lý hello ở `remote/dashboard.rs:285-297` (kế `set_platform`). `status.rs::device_json` xuất ra. Kiểm độ dài, ký tự in được (như `valid_platform`, `core-remote-proto/src/pair/mod.rs:57`).
3. `packages/shared/src/types.ts`: `RemoteDevice` thêm `appVersion?`, `model?`, `os?`.
4. Capability 2 chiều: `welcome` (`remote/ctx.rs:317-337`, hiện có `version`, chưa có `caps`) thêm `caps: [...]` liệt kê op desktop hỗ trợ (`clip`, `runlog`, `checklist`, `apkoffer`, `danger`, `menuPush`…). App điện thoại ẩn tính năng desktop chưa có; desktop ẩn / tắt tính năng điện thoại chưa có (theo `caps` trong hello, đã có `['chat']`). Test cả 2 chiều lệch bản.
5. Keystore: tạo `orrery-remote.jks` cố định ở `D:\keys\orrery\` (ngoài repo; `key.properties`, `*.jks` đã nằm trong `apps/mobile/android/.gitignore`); `build.gradle.kts` dùng signingConfig `release` khi có `key.properties`, không có thì build release dừng với thông báo rõ. Debug build vẫn dùng debug key như cũ. Ghi `apps/mobile/android/cert.sha256` (SHA-256 cert, không bí mật) vào repo. Cập nhật `docs/orrery-remote.md:118` (lệnh build) và cách sao lưu keystore; Mac nhận keystore + `key.properties` qua sao lưu riêng của chủ nhân (không qua git).
6. Migration 1 lần: máy đang cài bản ký debug key (A33) cài đè bản mới sẽ lỗi chữ ký. Bước chuyển: gỡ app → cài bản mới → ghép đôi lại; con làm qua adb trên A33 ngay khi xong F0 (chủ nhân chỉ bấm "Cho phép"); ghi vào QA và What's new. Wizard F3 bắt lỗi `signature` và giải thích đúng việc này.
7. `versionName` app điện thoại lấy từ version desktop (vd `4.1.0`) để so lệch bản.

**Test.** `remote/unit_tests.rs` (đọc Device cũ thiếu field), `session_tests.rs` (hello có / không field mới), Dart `connection_test` (hello gửi đủ field), Vitest parse `RemoteDevice`.

**Xong khi.** Settings hiện "Galaxy A33, Android 14, Remote 4.1.0" cho máy đã kết nối lại; build release không có `key.properties` thì dừng.

**Rủi ro.** Mất keystore = mọi máy phải gỡ app cài lại: sao lưu 2 nơi (chủ nhân giữ).

---

## F1. APK có sẵn trong bản cài (R3)

**Mục tiêu.** Không còn "Chọn APK…": desktop luôn có APK đúng phiên bản.

**Hiện tại.** `remote/install.rs::suggestion` (`:92`) chỉ thấy APK trong `apps/mobile/build/...` (máy dev); `Config.apk_path` / `apk_url` (`config.rs:74-77`); `install_start` (`install_ops.rs:46-53`) chỉ dùng `cfg.apk_path`; `resolve` (`install.rs:139`) chỉ nhận `/install/<t>/orrery.apk`; `Tickets` 1 slot (`install.rs:41`); bundle cấu hình ở `apps/shell/tauri.release.json:13-16`; `scripts/dist-tauri.mjs:13` cảnh báo khi Setup > 35 MB; dist build 2 dmg (arm64, x64) trên Mac.

**Làm.**
1. `scripts/dist-tauri.mjs`: bước `stageApk()` chạy 1 lần trước mọi `tauri build`: ưu tiên env `ORRERY_APK` (APK build sẵn); không có thì build `flutter build apk --release --target-platform android-arm64` trong `apps/mobile` (giới hạn luồng theo CLAUDE.md). Kiểm chữ ký bằng `apksigner` (đường dẫn từ `ANDROID_HOME/build-tools/*`) khớp `apps/mobile/android/cert.sha256`. Chép vào `apps/shell/resources/apk/orrery-remote.apk` + `version.txt`. Thiếu APK hoặc lệch chữ ký: dừng dist. `SETUP_MAX_MB` lên 70. Mac: ghi bước chuẩn bị (Flutter, Android SDK, keystore hoặc `ORRERY_APK`) vào `QA-mac-checklist`.
2. `tauri.release.json`: resource `"resources/apk/orrery-remote.apk": "apk/orrery-remote.apk"` và `version.txt`.
3. `Host` thêm `bundled_apk() -> Option<PathBuf>` và `save_apk(src) -> Result` (hộp thoại lưu) (`ctx.rs:68`, `host.rs`, fake trong `test_util.rs:55`) dùng `path().resolve(..., BaseDirectory::Resource)`.
4. 1 hàm `apk_source()` dùng chung cho op `get` và `start`: `apk_path` chủ nhân chọn (ở Nâng cao) > `bundled` > `suggestion` (máy dev). Reply thêm `source: "custom"|"bundled"|"dev"`, `version`.
5. Op mới `save`: hộp thoại lưu (mặc định `Downloads/orrery-remote-<ver>.apk`). Bỏ "Copy link" của wireframe (link LAN mang ticket là bí mật, xem F3).
6. `Tickets` thành map ≤ 4 slot, mỗi slot có loại (`apk-page`, `apk-offer`) và TTL; `resolve` / `Reply` viết lại để có route trang HTML (F3) và file. Làm ở F1 vì F3 và F7 cùng dùng.
7. ABI: APK arm64. Emulator x86_64 không cài được (`NO_MATCHING_ABIS`): F2 / F4 ẩn "Cài Remote" với emulator x86 và ghi lý do.

**Hợp đồng.** `ccm:remote-install` reply `{ok, apk:{path,name,size}|null, source, version, missing, suggested, url, running}`.

**Test.** `install_tests.rs`: thứ tự ưu tiên 3 nguồn cho cả `get` và `start`, `save` lỗi đích trả `{ok:false,error:"save"}`, ticket nhiều slot hết hạn riêng. Vitest `remoteInstall.test.ts` parse field mới.

**Xong khi.** Bản cài thật (Windows) mở Settings thấy "Orrery Remote 4.1.0 (có sẵn)", không cần chọn file.

**Rủi ro.** Bản cài to thêm ~25-30 MB. Build APK trên Mac và Windows phải cùng keystore (F0).

---

## F2. Cài qua cáp USB + deep link ghép đôi (R2)

**Mục tiêu.** 1 nút "Cài lên <máy>": cài APK, mở app, app tự ghép đôi; chủ nhân chỉ bấm "Cho phép".

**Hiện tại.** Device manager chưa có op install; cài chỉ có trong Run (`desk/run/android/runner.rs:245`, `adb install -r -t`, `install_failure`, thử gỡ khi lệch chữ ký). `ccm:dev-launch` (`desk/devices/ipc.rs:119`) mở app theo package. App điện thoại chỉ hiểu link `orrery-remote://m/…` (`mobile/app/pane_link.dart:34-48`), manifest chỉ có host `m` (`AndroidManifest.xml:38-43`). `pair_start` trả `{qr, exp, relay, lan}` (`remote/mod.rs:342`).

**Làm (desktop).** Code mới vào file mới `desk/devices/install.rs` (`manager.rs` đã 537 dòng, chỉ thêm 1-2 dòng gọi sang).
1. Dùng lại `install_failure` ở `desk/run/parse.rs:229-255` (đã có `VersionDowngrade`), chuyển sang module chung nếu cần; không đổi hành vi Run.
2. `install(id, apk) -> {ok, error?:"restricted"|"storage"|"signature"|"downgrade"|"abi"|"offline"|"unauthorized"|"timeout"|"other", detail}`; timeout 120s; KHÔNG tự gỡ app khi lệch chữ ký (khác Run): trả `signature`, UI hỏi chủ nhân có gỡ (mất ghép đôi trên máy đó) không.
3. `open_pair(id, qrText)`: `am start -n dev.ccm.orrery.remote/.MainActivity --es pairQr <qrText>` qua `sh_quote`. Mã đi bằng intent extra, không nằm trong URI (không lộ trong logcat dòng `START … dat=`, trang web không giả được).
4. `reverse(id, port)`: `adb reverse tcp:<port> tcp:<port>` trước khi ghép đôi qua cáp, để điện thoại tới desktop qua `127.0.0.1` dù khác Wi-Fi / firewall chặn; gỡ `adb reverse --remove` khi xong hoặc lỗi.
5. `package_version(id, package)`: `dumpsys package <pkg>` lấy `versionName` (cho F4).
6. Kênh: `ccm:dev-install`, `ccm:dev-open-pair`, `ccm:dev-reverse`, `ccm:dev-pkg-version`; type ở `web/core/lib/desktop.ts`, shim `scripts/gen-desk-shim.mjs`.
7. `Remote::pair_start` nhận `via: "usb"|"qr"|"lan"`, `extraLan?: ["127.0.0.1"]` (đường USB), `adbSerialHash?` (SHA-256 16 ký tự). Pair xong qua đường USB: lưu `adb_serial_hash` vào `Device` (F4 gộp dòng).

**Làm (phone).**
1. `MainActivity.kt`: đọc extra `pairQr` khi `onCreate` / `onNewIntent`, đưa qua MethodChannel vào `DeepLinkHost`. Activity là launcher nên exported: app khác trên máy cũng gửi được extra này, vì vậy chỉ ghi nguồn "qua cáp USB" khi `referrer` là `com.android.shell`, còn lại xử lý như link web; mọi nguồn đều hỏi xác nhận trên điện thoại.
2. Manifest: thêm host `pair` cho link từ trang tải LAN (F3). `pane_link.dart` thêm `parsePairLink(Uri) -> String? qrText` (chỉ host `pair`, `q` ≤ 640 byte như `MAX_QR_LEN`).
3. `deep_link_host.dart`: `_held` (`:49`) hiện chỉ giữ `PaneLink?`; thêm chỗ giữ riêng cho pair (đang khoá app thì chờ mở khoá). Nhánh pair luôn mở sheet xác nhận "Ghép đôi với <tên máy>?" [Huỷ] [Ghép đôi]. Link pair từ trình duyệt: chỉ nhận khi địa chỉ LAN trong QR cùng subnet Wi-Fi hiện tại, sheet ghi rõ "Mở từ trang web".

**Luồng (web, thuần TS `web/features/remote/lib/connectUsb.ts`, test bằng deps giả).**
`checkDevice` (online, không `unauthorized`, ABI arm64) → `install` → `reverse` → `pairStart({via:"usb", extraLan, adbSerialHash})` → `openPair(qr)` → chờ thiết bị mới → `paired` → gỡ `reverse`. Chờ thiết bị mới: so cả `deviceId` mới LẪN `paired_at` mới (máy đã ghép đôi rồi ghép lại dùng lại `deviceId`, watcher `RemotePairPanel.vue:77-86` không bắt được).

**Bảo mật.** Mã là PSK 1 lần, hết hạn 120s, max 5 lần thử / nguồn; vẫn bắt "Cho phép" trên máy tính (`pair.rs::run`), hộp "Cho phép" ghi kèm nguồn ("qua cáp USB: Galaxy A33").

**Test.** cargo: parse lỗi adb (`INSTALL_FAILED_USER_RESTRICTED`, `INSUFFICIENT_STORAGE`, `UPDATE_INCOMPATIBLE`, `VERSION_DOWNGRADE`, `NO_MATCHING_ABIS`), quote extra đúng, reverse gỡ khi lỗi. Dart: `parsePairLink` (hợp lệ, quá dài, host lạ, khác subnet), extra `pairQr` hỏi trước khi pair, giữ link khi khoá. Vitest `connectUsb.test.ts` mọi nhánh lỗi + ghép lại máy cũ. Thử thật A33: cùng Wi-Fi và tắt Wi-Fi (chỉ cáp).

**Xong khi.** Trên A33 chưa cài app, cáp USB 2.0 trở lên, đã bật Gỡ lỗi USB: bấm 1 nút → app mở, desktop hiện hộp "Cho phép" ≤ 30s (cả khi điện thoại tắt Wi-Fi, nhờ `adb reverse`).

---

## F3. Wizard 3 bước, Nâng cao, trang tải LAN, kiểm kết nối (R1, R4, R6)

**Mục tiêu.** Settings › Điện thoại: 1 thẻ trạng thái + nút "Kết nối điện thoại" + danh sách máy + "Nâng cao" gập.

**Bố cục.**
- `SettingsRemote.vue` còn: `RemoteHeroCard` (trạng thái / nút chính), `PhoneConnectWizard` (khi đang kết nối), danh sách máy (tạm giữ `RemoteDeviceRow`; F4 thay bằng trang Thiết bị), `RemoteAdvanced` (gập: relay, push, giữ máy thức, bảo mật 3 mục, nhật ký, APK tuỳ chọn, cổng + firewall).
- File mới (mỗi file ≤ 400 dòng): `components/connect/{PhoneConnectWizard,ConnectInstallStep,ConnectPairStep,ConnectTestStep,ConnectConsent}.vue`, `components/RemoteHeroCard.vue`, `components/RemoteAdvanced.vue`, `lib/connectFlow.ts` (reducer thuần), `lib/connectUsb.ts` (F2), `lib/connectCheck.ts`.
- Giữ id hàng tìm kiếm cũ (`remote`, `remoteAdd`, `remoteInstall`, `remoteDevices`, `remoteRelay`…) để Ctrl+K vẫn nhảy được; hàng nằm trong "Nâng cao" thêm vào tập mới `REMOTE_ADVANCED_ROWS` trong `web/features/settings/lib/settingsFlash.ts` (tự mở fold `[data-remote-more]` trước khi flash). Thêm id `remoteConnect`.
- Giữ selector E2E: `data-settings-remote`, `data-remote-enable`, `data-remote-add`, `data-remote-pair`, `data-remote-devices`, `data-remote-log`, `data-setting-row=*`. Selector nào đổi vị trí phải sửa E2E trong cùng commit (`v97-remote.cjs`, `v98-remote-next.cjs`).

**State (reducer `connectFlow.ts`).**

| State | Vào khi | Ra khi |
|---|---|---|
| `idle` | mở trang, remote tắt hoặc chưa kết nối | bấm Kết nối → `consent` (remote tắt) / `install` (remote bật) |
| `consent` | remote đang tắt | Bật → `enabling` ; Huỷ → `idle` |
| `enabling` | đã gọi `remoteEnable(true)` | `status.listening` true → `install`; quá 10s hoặc cổng bận → `enableFail` (báo cổng, nút thử lại) |
| `install` | bước 1 | chọn USB → `usbInstalling`; QR → `qrInstall`; "Đã cài" → `pairWait` |
| `usbInstalling` | chạy `connectUsb` | xong → `pairConfirm`; lỗi → `usbFail{error}` |
| `usbFail` | `restricted` / `storage` / `signature` / `offline` / `timeout` | Thử lại / Dùng QR / (signature) Gỡ và cài lại sau khi hỏi |
| `qrInstall` | ticket LAN đang mở | điện thoại tải → `qrInstalled` (sự kiện từ route install) ; hết hạn → `qrExpired` |
| `qrInstalled` | đã có GET APK | điện thoại bấm "Mở và ghép đôi" → `pairConfirm` |
| `pairWait` | QR ghép đôi đang hiện (120s) | điện thoại quét → `pairConfirm`; hết hạn → `pairExpired` |
| `pairConfirm` | hộp "Cho phép" đang mở (hộp native, `host.rs:28`) | Cho phép → `paired`; Từ chối → `denied` |
| `paired` | thiết bị mới xuất hiện | sau 1.5s → `testing` |
| `testing` | chạy `connectCheck` | xong → `testDone` / `testWarn` |
| `testDone` / `testWarn` / `pushHelp` | kết quả kiểm | Có, xong → `done`; Không thấy → `pushHelp` |
| `done` | xong | thẻ trạng thái + toast |

iPhone: `install` → `installIos` (có relay) / `iosNoRelay` → `pairWait`.

Nhánh phụ bắt buộc có test: ghép lại máy đã ghép (nhận theo `paired_at` mới), relay tắt + không cùng LAN (điện thoại quét được nhưng không tới được: `pairWait` hết hạn kèm gợi ý "cùng Wi-Fi hoặc cài relay"), `usbFail{signature}` (giải thích migration F0), đang ở wizard mà remote bị tắt từ chỗ khác (về `idle`).

**Trang tải LAN (R4) trong `remote/install.rs`.**
- Route mới `GET /install/<token>/` trả HTML tĩnh tối giản (không JS, CSS inline, vi / en theo `Accept-Language`, header `Cache-Control: no-store`, `Referrer-Policy: no-referrer`): tên app, phiên bản, kích thước, nút "Tải APK", nút "Mở và ghép đôi" trỏ `/install/<token>/pair`.
- `GET /install/<token>/pair` chỉ chạy khi đủ cả 3: (a) wizard desktop đang ở `qrInstall` / `qrInstalled`; (b) cùng IP đã tải APK qua ticket này; (c) chưa dùng lần nào (1 lần / ticket). Khi đó mới tạo mã ghép đôi, gắn với IP đó (handshake từ IP khác bị từ chối), trả 302 tới `orrery-remote://pair?q=…`. Sai điều kiện: 403, không tạo mã, không thay slot ghép đôi đang chạy (tránh bị gọi liên tục để chặn chủ nhân ghép đôi, vì `Pairing::start` `pair.rs:56` thay slot cũ).
- Hộp "Cho phép" trên desktop ghi kèm IP và nguồn "qua trang cài" (tên máy do điện thoại tự khai, không đủ tin).
- Mã đi qua HTTP thường trong LAN: chấp nhận được vì 1 lần, gắn IP, 120s, vẫn bắt duyệt; ghi rõ trong `docs/remote-integration.md`.
- Desktop nhận sự kiện `install.fetched` (GET APK lần đầu, kèm IP) để đổi state `qrInstalled`.
- QR trên desktop trỏ trang `/install/<token>/` thay vì thẳng file APK.

**Kiểm kết nối (R6).**
- `ccm:remote-check` (shell) trả `{lan:{ok, ms?}, relay:{state, ms?}, push:{ok, error?}}` cho 1 thiết bị.
- Đo ms bằng frame `Kind::Ping` 2 chiều có sẵn (`remote/dashboard.rs:220-233`, `Liveness` sent / heard): thêm ghi thời điểm gửi / nhận, không cần op mới, chạy được với app cũ.
- Push: gọi `push_test` có sẵn (`remote/push_setup.rs:89`), max 1 lần / 30s / thiết bị.
- `pushHelp`: thiết bị có adb thì nút "Mở trên điện thoại" chạy `am start -a android.settings.APP_NOTIFICATION_SETTINGS --es android.provider.extra.APP_PACKAGE dev.ccm.orrery.remote` và trang tối ưu pin.

**Motion.** Theo wireframe: bước trượt 14px 220ms, vạch stepper scaleX, hộp đồng ý trồi 8px, hành tinh chạy quỹ đạo 1 lần 600ms khi `paired` (chỉ `transform`, tắt khi reduced-motion), danh sách `ccm-list-inflow`.

**i18n.** File mới `web/features/remote/i18n/v113-connect.ts` (en, vi), cập nhật snapshot `keys.en.txt`, `keys.vi.txt`.

**Test.** Vitest: `connectFlow.test.ts` (mọi chuyển state, kể cả lỗi), `connectCheck.test.ts`, component test wizard (render từng state), `settingsFlash.test.ts` (mở fold rồi flash). cargo `install_tests.rs`: trang HTML, `/pair` tạo mã lúc bấm, 403 với IP ngoài LAN, sự kiện fetched. E2E mới `apps/shell/e2e/v113-connect.cjs` (adb giả + phone giả từ fake gateway) đi hết luồng USB và QR; cập nhật `E2E_MAP` trong `scripts/qa-fast.mjs`.

**Test bảo mật trang tải.** `/pair` từ IP khác IP đã tải → 403; gọi lần 2 → 403; wizard không ở bước QR → 403; không thay slot ghép đôi đang chạy; handshake từ IP khác IP gắn mã → từ chối.

**Xong khi.** Đi hết 3 kịch bản trên wireframe trong app thật; E2E cũ + mới xanh; test bảo mật trên xanh.

---

## F4. Settings › Thiết bị hợp nhất (R5)

**Mục tiêu.** Một danh sách: điện thoại Remote, máy adb, emulator; trùng máy thì 1 dòng.

**Làm.**
1. Trang mới `devices` trong `SETTINGS_PAGES` (`web/core/lib/uiTypes.ts:13`). `PAGE_NEEDS` (`web/features/settings/components/index.ts:86`) hiện chỉ nhận 1 key và `SettingsEnv` chưa có cờ devices: thêm cờ `devices` vào `SettingsEnv`, đổi `PAGE_NEEDS` nhận mảng key (hiện trang khi có 1 trong các cờ), test `index.test.ts`. Mục tìm kiếm `devicesList`, `devicesAdb`.
2. `web/features/devices/components/SettingsDevices.vue` + `lib/mergeDevices.ts` (thuần). Lấy remote qua barrel `@features/remote` (không import sâu, theo `importRules.test.ts`).
3. Luật gộp: (a) `Device.adb_serial_hash` = băm serial adb → gộp chắc chắn; (b) cùng model + tên gần giống → hiện gợi ý "Cùng một máy?" [Gộp] [Không]; lựa chọn lưu ở `ccm.deviceMerge` (localStorage). Không bao giờ tự gộp theo tên.
4. Mỗi dòng: tag Remote <ver> / Web app / USB adb / Wi-Fi adb / Emulator; trạng thái Online (LAN / relay, ms nếu có) / Offline <lúc>; nút chính theo tình huống: Cập nhật app (adb + `package_version` < bản desktop), Cài Remote (adb, chưa cài), Khởi động (emulator tắt).
5. Chi tiết (bấm dòng): thông báo bật / tắt, Gửi thử, Mở app, Chụp màn, Đổi tên, Thu hồi.
6. Thu hồi có Hoàn tác, làm ở core (không dựa vào frontend lúc thoát): kênh `ccm:remote-revoke {id, grace:5}` tạm treo thiết bị NGAY (đóng link đang mở, chặn handshake), giữ lệnh thu hồi chờ trong core; hết 5s hoặc lúc app tắt thì thu hồi thật; `ccm:remote-revoke-undo {id}` trong 5s thì mở treo. Test cargo: treo chặn handshake, tắt app trong lúc chờ vẫn thu hồi, undo khôi phục.
7. Màn Devices cũ (`ui.view === 'devices'`) giữ nguyên ở bản này; ghi `.code_index/issues.md` việc gộp 2 nơi sau khi chủ nhân dùng thử.

**Test.** `mergeDevices.test.ts` (gộp theo hash, gợi ý theo model, không gộp theo tên), component test (lọc, chi tiết, Hoàn tác, trống), E2E `devices.cjs` + `v113-connect.cjs` thêm bước mở trang Thiết bị.

---

## F5. Push nâng cao (P1, P3, P6)

**Điều kiện trước khi code.** Ngoại lệ 8 (`docs/core-principle.md`) chỉ cho push chở agent, session, project, câu hỏi (mask, ≤ 120 ký tự); ngoại lệ 6 nói về bấm menu trên desktop / chat điện thoại. P1 đưa nhãn các lựa chọn vào push và cho bấm ngay trên thông báo: chủ nhân đã duyệt 26/11: bước đầu F5 sửa câu chữ ngoại lệ 6 và 8 (+ tóm tắt trong `CLAUDE.md`), xong mới code P1. P3, P6 không chở nội dung mới, nằm trong ngoại lệ 8 hiện có.

**Quota.** `PER_HOUR=30` / thiết bị (`remote/push.rs:19`): push `running` / `done` / `open` / `clear` dùng quota riêng (`PER_HOUR_INFO=20`), push hỏi chủ nhân luôn được ưu tiên, không bao giờ bị push thông tin chiếm chỗ.

**P1 trả lời trên thông báo.**
- Desktop: push của session đang hỏi menu kèm `menu {sig, items:[{index,label}]}` (max 3 mục, nhãn ≤ 24 ký tự, đã mask). Nguồn menu: cùng nguồn đang cấp `RemoteMenu` cho chat (`remote/chat.rs` / `chat_src.rs`, op `ws.menu`).
- Phone: push data-only (đã có đường giải mã `push_seal.dart`) → local notification có action (flutter_local_notifications), mỗi action `showsUserInterface: true`: bấm là mở app, qua khoá app nếu bật, rồi gửi `ws.menu {paneKey, sig, index}` (có sẵn, kèm undo 3s đã có). Không gửi từ nền (không có kết nối nền đáng tin).
- PWA: `push-sw.js` thêm `actions` (Chrome / Android); iOS web push không có action thì bỏ qua.
- Menu đổi (sig khác) trước khi bấm: desktop trả lỗi `stale`, app báo "Câu hỏi đã đổi, mở pane để xem".

**P3 thông báo đang chạy.**
- Desktop: khi session chạy liên tục > 2 phút và có điện thoại bật push: gửi 1 push `kind:"running"` (tên pane, mốc bắt đầu); khi xong gửi `kind:"done"` cùng tag để thay thế. Max 2 push / lượt, tính vào `PER_HOUR` hiện có.
- Phone: notification `ongoing`, `usesChronometer` từ mốc bắt đầu, channel mới `running` (mức thấp, không rung); `done` thay bằng thông báo thường có nút Mở.

**P6 mở pane trên điện thoại.**
- `web/features/workspace/lib/paneHeadActions.ts`: key mới `phone` (nằm trong menu, chỉ hiện khi có điện thoại bật push).
- Kênh `ccm:remote-send-pane {paneKey}` → push `kind:"open"` tới mọi điện thoại bật push; phone mở `orrery-remote://m/<m>/p/<p>` (đường `pushLink` sẵn có).

**Test.** cargo `push_tests.rs`, `push_content_tests.rs` (menu trong payload, mask, cắt nhãn, max push), Dart notifier test (action → ws.menu sau mở khoá), Vitest `paneHeadActions.test.ts`. Thử thật A33 khoá màn hình.

---

## F6. Widget + Quick Settings (P2, E4)

- Push thêm field `waiting` (số pane đang chờ chủ nhân) cho mọi push; app ghi vào SharedPreferences ở handler nền, kèm mốc thời gian.
- Số chờ không được sai lâu (hard rule không bịa số): khi số chờ về 0 mà điện thoại không đang mở app, desktop gửi push `kind:"clear"` (quota thông tin); widget / tile luôn ghi "lúc 9:41" và tự đổi thành "Mở app để cập nhật" khi số đã cũ quá 30 phút.
- Kotlin: `OrreryWidgetProvider` (2x2: số chờ, 2 pane đầu, chạm mở app) và `OrreryTileService` (nhãn "Orrery", phụ đề "N chờ bạn"; chạm mở app; giữ (long-press mở `QS_TILE_PREFERENCES`) → hỏi tạm dừng 1 giờ). Tạm dừng = cập nhật `push_mute` sẵn có, hết giờ tự bật lại.
- Không poll: widget, tile chỉ cập nhật khi có push hoặc khi app mở.
- Test: Robolectric không có sẵn → test Dart phần ghi prefs; thử tay A33 (ghi QA).

---

## F7. Gửi APK build xong (E1), Run log + crash (E2)

**E1.**
- `desk/run/android/runner.rs:327`: patch session thêm `apk {path, size, applicationId}` khi build xong.
- Run pane: nút "Gửi lên điện thoại" (và công tắc "Tự gửi khi build xong" theo project).
- Shell: dùng `Tickets` nhiều slot (làm ở F1); op `ccm:remote-offer-apk {sessionId}`: KHÔNG nhận path từ web; desktop tự lấy `apk.path` đã patch vào run session, canonicalize, kiểm nằm trong thư mục build (`is_within`, `desk/apk/mod.rs:106`) và `is_apk`, tính SHA-256, cấp ticket loại `apk-offer`, push `kind:"apk"` `{url LAN, size, sha256, app, variant}`.
- Phone: thêm quyền `REQUEST_INSTALL_PACKAGES`, FileProvider, MethodChannel `dev.ccm.orrery.remote/install` (Kotlin `PackageInstaller` session hoặc intent `ACTION_VIEW`); tải về cache, kiểm size + SHA-256, rồi mở trình cài của Android (Android luôn hỏi xác nhận).
- Ngoài Wi-Fi nhà (không với tới LAN): thông báo "Cần cùng Wi-Fi để tải"; truyền qua relay để phase sau.

**E2.**
- Hiện chưa có kênh đọc log Run cho phone: chỉ có broadcast `ccm:run-event` (`desk/run/sessions.rs:335`) và `run-history-read`. Làm: trong shell giữ ring buffer 2 000 dòng / session từ `run-event`, viewer op `run.log {session, since?}` đọc buffer, `run.watch / run.unwatch` thêm biến thể `Watch` mới; tăng kích thước `SPECS` (`remote/viewer_map.rs`, hiện `[Spec; 31]`). Text qua `mask_log`.
- Phone: trang Run log (từ `run.sessions` có sẵn), dòng crash tô đỏ (mẫu `FATAL EXCEPTION`, `Unhandled Exception`, `E/flutter`), nút "Gửi crash cho agent" → chọn pane → điền sẵn vào ô nhập của pane đó trên điện thoại (chỉ phía điện thoại, không gửi lên desktop), chủ nhân bấm Gửi mới đi qua `ws.send`. Không cần op prefill mới.
- Pane khác term trên điện thoại: thêm kind `run` vào `workspace_model.dart` (chỉ đọc).

**Test.** cargo: offer-apk từ chối path ngoài build, ticket nhiều slot, mask log. Dart: tải + kiểm SHA sai → báo lỗi, không mở trình cài. E2E desktop `run-pane.cjs` thêm nút gửi (phone giả).

---

## F8. Vân tay cho lệnh nguy hiểm (E5)

- Luật mẫu ở desktop (`remote` config `danger_rules`, mặc định: `git push --force`, `git push -f`, `reset --hard`, `rm -rf`, `git clean -fd`, `drop table`), sửa được trong Settings › Nâng cao; gửi xuống phone khi `hello`.
- Phone: chat composer và InputBar chế độ gửi dòng: khớp mẫu → `AppLock.confirm()` / `DeviceAuth.authenticate` trước khi gửi.
- Desktop kiểm lại khi nhận `ws.send`: khớp mẫu mà thiếu `authed:true` → trả `needsAuth`, không gõ vào pane. `authed` do điện thoại tự khai: lớp này chỉ chống bấm nhầm và app cũ chưa có luật, không phải lớp bảo mật.
- Mô hình đe doạ: người khác cầm điện thoại đang mở khoá, hoặc chủ nhân gõ nhầm. Không chống được điện thoại bị chiếm quyền (ngoài phạm vi).
- App cũ không có luật: nhận `needsAuth` thì hiện "Lệnh này cần cập nhật app để gửi" (desktop gửi kèm text), không im lặng.
- Phạm vi: chỉ Orrery Remote. Telegram (`host_telegram.rs`) không áp ở bản này, ghi rõ trong Settings.
- Giới hạn ghi rõ: phím gõ thô từng ký tự vào terminal (PTY) và `ws.keys` không kiểm được.
- Test: cargo luật khớp / không khớp, `ws.send` thiếu authed bị chặn; Dart composer gọi auth.

---

## F9. Nói, chụp ảnh gửi vào pane (P4)

- Thêm `speech_to_text` (ưu tiên `onDevice`, máy không có gói ngôn ngữ offline thì hỏi 1 lần có dùng nhận dạng online không), `image_picker`.
- Giữ mic: nhận chữ vào ô nhập, không tự gửi.
- Ảnh: `fs.write` chỉ nhận text (`remote/viewer_map.rs:144-151`) và message ≤ 1 MiB, nên thêm op mới `fs.put {path, b64, seq, last}`: chunk ≤ 512 KB base64, ghép ở desktop, file ≤ 8 MB; CHỈ ghi vào `<project>/.orrery/inbox/` (tự tạo thư mục, tên do desktop đặt `<yyyyMMdd-HHmmss>.jpg`, không nhận path tuỳ ý, không cần xác nhận ghi đè vì luôn file mới). Nén JPEG cạnh dài 2048 px trước khi gửi.
- Chèn đường dẫn vào ô nhập. Lần đầu gợi ý thêm `.orrery/inbox/` vào `.gitignore`: chỉ sửa khi chủ nhân bấm "Thêm".
- Test: cargo `fs.put` (ghép chunk, sai thứ tự, quá cỡ, path thoát khỏi inbox bị từ chối); Dart nén + chia chunk.
- Công sức: M-L.

---

## F10. Clipboard chung (D3)

- Op phone → desktop `clip.push {text}` (≤ 64 KB), desktop ghi clipboard OS (plugin clipboard của Tauri). Desktop → phone: nút "Gửi clipboard sang điện thoại" (palette + tray) gửi event `clip` CHỈ qua phiên Noise đang mở; không có phiên thì báo "Mở app trên điện thoại rồi gửi lại". Không bao giờ đi qua push (ngoài phạm vi ngoại lệ 8).
- Chặn khi `mask_secrets` đổi nội dung (có secret): báo "Có vẻ là secret, đã chặn". Chỉ gửi khi bấm, không đồng bộ tự động. Ảnh để sau.
- Test: cargo chặn secret, giới hạn kích thước; Vitest nút gửi.

---

## F11. Checklist trước merge trên điện thoại (E6)

- Viewer op `checklist.get|toggle` ánh xạ tới `desk/preview/checklist.rs::reply` (`:201-222`, có `list/add/toggle/remove/setScript/setCheck`).
- `checklist.run`: Rust KHÔNG có runner; bước chạy nằm ở web (`web/features/git/lib/checklist.ts:60 runCheck`, mở shell pane + chờ kết quả). Chọn: shell chuyển yêu cầu sang web qua `host.ask` (đầu WP phải xác minh đường shell → web này có sẵn; không có thì dùng event `ccm:*` + reply theo id như các kênh desk), web gọi `runCheck` rồi ghi kết quả bằng `setCheck` như hiện tại. Cần xác nhận (`needsConfirm` + `confirm.answer`) vì tốn CPU máy nhà; 1 lần chạy mỗi lúc (`live` sẵn có); app desktop đang thu nhỏ vẫn chạy được (webview vẫn sống).
- Phone: trang checklist theo worktree, tick todo, nút chạy lại kiểm, tag "Sẵn sàng merge".
- Test: cargo map op + confirm + chuyển sang web, Vitest web nhận yêu cầu chạy, Dart trang.
- Công sức: M-L.

---

## F12. Chạy trên nhiều thiết bị + lưới ảnh (E3)

- Run store đã cho mỗi (config, device) 1 session (`web/features/run/stores/run.ts:424`): thêm chọn nhiều máy, chạy tuần tự build 1 lần rồi cài song song max 3.
- Biến thể: tối (`cmd uimode night yes`), chữ 1.3x (`settings put system font_scale 1.3`). Đây là thiết lập toàn máy: đọc giá trị cũ, ghi ra đĩa (`<data>/run/variants/<serial>.json`) TRƯỚC khi đổi, trả lại trong `finally`; desktop crash giữa chừng thì lần mở app sau tự khôi phục từ file đó khi máy cắm lại.
- Chụp `screencap` (`desk/devices/adb.rs:191`) cùng lúc sau khi app mở 2s; dialog `RunShotsDialog.vue` lưới ảnh, nút "Gửi ảnh cho agent" (đính file vào pane như `elementCrop` đã làm, không tự gửi).
- Test: Vitest store nhiều máy, cargo khôi phục biến thể khi lỗi.

---

## F13. Mirror màn thiết bị (D2), bắt đầu bằng spike

- Spike (1 ngày): scrcpy-server (Apache 2.0) đẩy qua adb, H.264 qua `exec-out`/socket, giải mã WebCodecs trong webview Tauri (Windows WebView2, macOS WKWebView). Đo CPU, độ trễ. WebCodecs `VideoDecoder` chỉ có từ Safari 16.4 mà bản Mac hỗ trợ từ macOS 11 (`tauri.release.json` `minimumSystemVersion`): spike đo cả macOS cũ, có phương án dự phòng (giải mã ở Rust, đẩy ảnh) hoặc ẩn tính năng dưới macOS 13.4. Kết quả ghi `.planning/SPIKE-v113-mirror.md`, chủ nhân duyệt mới làm tiếp.
- Nếu làm: pane kind `mirror`, chỉ stream khi pane hiện, max 30 fps, dừng khi thu nhỏ / đổi tab; chạm, Back, Home, Xoay; chụp gửi agent.

---

## F14. Xem dev server trên điện thoại (P5), ngoại lệ 9

- Trước khi code: ghi "Ninth exception (owner 26/11/2026, v113): …" vào `docs/core-principle.md` + 1 dòng trong `CLAUDE.md`.
- Spike: proxy HTTP qua kênh Noise (op `http.req` / `http.res` theo chunk ≤ 1 MiB), WebView trên phone chặn request về host ảo. Ghi `.planning/SPIKE-v113-preview.md`.
- Ràng buộc: tắt mặc định; chỉ cổng chủ nhân bật trong desktop; chỉ `127.0.0.1`; tự đóng sau 30 phút; không chuyển header `Cookie` / `Authorization` của trình duyệt desktop.

---

## Bảng test + QA chung

| Phase | Unit | E2E desktop | Máy thật |
|---|---|---|---|
| F0 | cargo devices / session, Dart connection | - | A33 kết nối lại |
| F1 | cargo install, Vitest remoteInstall | v113-connect (nguồn bundled) | bản cài Windows thật |
| F2 | cargo devices/install, Dart pane_link, Vitest connectUsb | v113-connect (adb giả) | A33 chưa cài app |
| F3 | Vitest connectFlow, settingsFlash, cargo install HTML | v113-connect, v97-remote, v98-remote-next | A33 USB + QR |
| F4 | Vitest mergeDevices | devices, v113-connect | A33 + emulator |
| F5 | cargo push, Dart notifier | v98-remote-next | A33 khoá màn |
| F6 | Dart prefs | - | A33 widget, tile |
| F7 | cargo offer-apk, mask, Dart sha | run-pane | A33 cài APK web-app |
| F8 | cargo rules, Dart composer | - | A33 vân tay |
| F9-F12 | theo mục | theo mục | A33 |
| F13-F14 | spike | - | A33 |

Mac: mỗi phase thêm bước vào `.planning/QA-mac-checklist.md` (chủ nhân thử 1 lần sau khi xong Windows).
