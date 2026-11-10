# WP-v87-6-D5 — báo cáo (2026-11-10)

Port `apk/` (3 kênh), `releaseCheck/` (4 kênh) và phần lõi `devices/` (23 kênh) sang `core-desk`, có twin test từng nhóm. Nhóm còn lại của `devices/` (recorder, appdata sql.js, devfs, perf) để TS. Cờ `CCM_DESK_APK|RELEASE_CHECK|DEVICES`, mặc định node (trong `DESK_NODE_DEFAULT`), twin bật rust.

## Đã làm

1. **`apk`** (`core-desk/src/apk/`): `zip.rs` (EOCD, central directory, giới hạn entries / cd bytes, inflate bằng `flate2` backend `rust_backend`, partial head 4 KB cho dex), `axml.rs` (AXML + aapt2 proto manifest, `manifest_summary`), `analyze.rs` (groups / top / `compare_snapshots`; `locale_cmp` xấp xỉ `localeCompare`), `history.rs` (`apk-history/<sha1(rootKey)>.json`, cùng schema nên đổi mode không mất history), `mod.rs` (cache 4 snapshot, picked set 50). Kênh `ccm:apk-pick|analyze|compare`.
2. **`releaseCheck`** (`core-desk/src/release_check/`): `rules.rs` (mask comment, parse gradle, pubspec, manifest / admob / cert / extras findings; regex qua `regress` để khớp JS), `scan.rs` (Flutter + `detect_android` của run, walk sort theo byte như libuv, cap 5000 file / 48 MB / 40 admob), `sign.rs` (`find_sign_tools`: JDK ≥ 17 từ run-settings / JAVA_HOME / Studio JBR / PATH, `apksigner.jar` build-tools mới nhất; chạy `java -jar apksigner verify --print-certs` hoặc `keytool -printcert -jarfile`), `store.rs` (`release-check/<sha1>.json`, giữ nguyên số `at`). Kênh `ccm:release-check|mark|verify|remember`; map `verified` giữ trong tiến trình như TS.
3. **`devices`** (`core-desk/src/devices/`): `parse.rs`, `history.rs` (`adb-devices.json`, indent 2, cap 200), `adb.rs` (connect / pair / disconnect / tcpip / wlan ip / mdns / list / screencap / ui_dump, mọi `-s` gọi host `devices.use`), `manager.rs` (hold + `AdbWatch` của D4b, merge devices, connect many giới hạn 5 luồng, to-wifi, remove có confirm, discover, auto-connect một lần, QR pairing thread + abort), `qr.rs`, `emulators.rs` (AVD list / start / stop), `manifest.rs`, `uidump.rs`, `toggles.rs` + `ui_tester.rs` (display tests, originals `adb-ui-originals.json`). 20 kênh `ccm:dev-*` (snapshot, hold, connect, pair, disconnect, connect-many, to-wifi, remove, alias, auto-connect, discover, qr-start, qr-cancel, screencap, uidump, ui-read, ui-set, ui-restore, manifest, launch) + 3 `ccm:avd-*` + `desk:dev-release-owner`, `desk:dev-auto-connect-once`. Event `ccm:dev-event` (thêm vào `DESK_EVENTS`).
4. **Mã pair / secret**: không log; mã pair chỉ đi vào argv `adb pair`; QR secret chỉ trả `payload` như TS.
5. **TS**: `apk|releaseCheck|devices/ipc.ts` dùng `deskHandle` cho các kênh đã port (kênh chưa port vẫn `ipcMain.handle`); `apk/deskHosts.ts` (`gate.dir` / `gate.file`), `devices/deskHosts.ts` (`devices.use` / `devices.confirm` standing), `DeviceManager.confirmRemoveIds|confirmWipeAvd|noteUse`; `main.ts` nối `desk` cho 3 module, standing hosts, và `desk:dev-auto-connect-once` thay cho `autoConnectOnce` TS khi mode rust; owner / release theo cửa sổ (`devices.owner` per request, `desk:dev-release-owner`).

## File

- Mới: `apps/core/crates/core-desk/src/{apk,release_check,devices}/**`; `apps/desktop/src/apk/deskHosts.ts`, `apps/desktop/src/devices/deskHosts.ts`; `apps/desktop/test/{apk-fixtures.ts,desk-apk.twin.test.ts,desk-release-check.twin.test.ts,desk-devices.twin.test.ts}`.
- Sửa: `core-desk/Cargo.toml` (+`flate2`), `Cargo.lock`, `core-desk/src/lib.rs`, `apps/desktop/src/{apk,releaseCheck,devices}/ipc.ts`, `devices/manager.ts`, `desk/forward.ts`, `main.ts`, `.code_index/{core,desktop,_meta}.md`, plan §4.

## Số test thật

- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 726 passed, 0 failed (core-desk lib 249, mới: 3 unit `devices::parse`, `release_check::rules`, `apk::analyze`). `cargo clippy -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 159 file passed, 1654 passed, 29 skipped, 0 failed (1 lượt đầu `run-gaps.test.ts` "a held root re-detects on pubspec changes" fail do trễ khi máy tải, file của D4b, chạy lại pass). Twin mới: `desk-apk` 5, `desk-release-check` 4, `desk-devices` 10; mỗi twin kết thúc `twin.logs` / `twin.fellBack` rỗng.
- `npm run typecheck`: 0 lỗi.
- Twin dùng adb / emulator / java / keytool giả (script sh trong thư mục tạm), DB / history tạm; không chạm `~/.android`, adb server thật, thiết bị thật. Thư mục tạm xoá cuối test. Không build release, không E2E.

## E2E cần chạy (chưa chạy theo yêu cầu gói)

Build trước: `node apps/desktop/build.mjs` + `cargo build --release -j 4 -p core-bin`; instance cô lập `CCM_E2E_PORT=4521 CCM_E2E_DBG=9521`, bật rust bằng `CCM_DESK_APK=rust CCM_DESK_RELEASE_CHECK=rust CCM_DESK_DEVICES=rust`:

| Bộ | Kênh | Lệnh |
|---|---|---|
| `devices.cjs` | `ccm:dev-*`, `ccm:avd-*` (Devices tab) | `CCM_E2E_PORT=4521 CCM_E2E_DBG=9521 CCM_DESK_DEVICES=rust node apps/desktop/e2e/devices.cjs` |
| `v73.cjs` | `ccm:apk-*`, `ccm:release-*` (`CCM_E2E_RELEASE_ARTIFACT` cho picker) | `CCM_E2E_PORT=4521 CCM_E2E_DBG=9521 CCM_DESK_APK=rust CCM_DESK_RELEASE_CHECK=rust node apps/desktop/e2e/v73.cjs` |

Chạy thêm 1 lượt với `CCM_DESK_*=node` (đường TS).

## Callback host mới (vỏ Tauri làm theo)

- `gate.dir` `{dir}` → real path | null (= `previewGate.resolveDir`); `gate.file` `{path}` → real path | null (= `resolveFile`).
- `apk.pick` `{defaultPath}` → đường dẫn `.apk|.aab` | null (dialog mở file); `release.pick` `{near}` → đường dẫn | null.
- `devices.owner` `{key}` → chuỗi `"<senderId>:<key>"` (per request; vỏ gọi `desk:dev-release-owner` với prefix khi cửa sổ reload / đóng).
- `devices.use` `{serial}` (standing; = `deviceUse.used`), `devices.confirm` `{kind:'remove', ids}` | `{kind:'wipe', name}` → bool (confirm i18n ở vỏ).
- Gọi vào Rust: `desk:dev-release-owner [prefix]`, `desk:dev-auto-connect-once`.

## Khác biệt còn lại / PARTIAL

- **Chưa port (vẫn TS, dùng `DeviceManager` TS qua `withReady`)**: `ccm:dev-rec-*` (recorder 307 dòng, ffmpeg-less screenrecord + pull), `ccm:appdata-*` (sql.js → `rusqlite` `bundled`, `sqlGuard`, `sqliteWal`, prefs XML), `ccm:devfs-*` (file browser pull / push), `ccm:perf-*`. Hai manager (TS + Rust) cùng chạy được vì readyCheck của TS tự `adb devices -l`; history `adb-devices.json` chỉ do một bên ghi (Rust khi mode rust). Cần `rusqlite` `bundled` + schema giữ nguyên file DB khi làm tiếp.
- `locale_cmp` (tie-break diff APK + sort AVD / device name) xấp xỉ ICU `localeCompare` (chữ thường trước hoa, ký tự đặc biệt < số < chữ); tên có dấu / Unicode có thể khác thứ tự.
- `utf16` lone surrogate trong AXML pool → ký tự thay thế (TS giữ nguyên).
- `restore` display test: khi lệnh đầu thất bại do adb mất (null) rồi lệnh sau lỗi có output, TS lấy detail của lệnh sau; Rust lấy lệnh đầu.
- Rust `release-verify` đọc JDK / SDK từ `run-settings.json` + env tiến trình + PATH; TS đọc `runManager.getSettings()` (cùng file). `java -version` dùng timeout 15 s như TS.
- Desk chưa chạy trên Windows thật cho nhóm này (`cfg`: `.exe` / `win32` path qua `pathx`; twin skip trên win32); macOS đã chạy thật.

STATUS: PARTIAL
