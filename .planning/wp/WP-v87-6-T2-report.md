# WP-v87-6-T2 — báo cáo (2026-11-11)

Vỏ Tauri `apps/shell` đóng gói được thành bản phát hành thay Electron: migrate localStorage + token, NSIS / dmg, updater, script + CI. Chưa chạy `tauri build` / dist / E2E (theo đề). Gói D8 sửa song song `apps/shell/src/{desk,hosts,main}.rs` + `core-desk` (đã ghép `mcp::start` vào `desk::wire`); trong lúc D8 dở, cổng Rust chạy trên bản sao `apps/core` HEAD, cuối cùng chạy lại trên cây thật.

## Đã làm

1. **localStorage** (`apps/shell/src/webstorage.rs`, `apps/desktop/src/webstorageExport.ts`)
   - Electron ≥ 3.5: lúc thoát (`main.ts` `exportPageStorage`, chạy song song `host.stop()` trong `before-quit`, cả đường update) ghi `CCM_DATA_DIR/webstorage-export.json` `{version, origin, savedAt, local}` (mọi key, chỉ giá trị string, ghi tmp + rename, max 1,5 s).
   - Electron 3.4 không có bước đó, nên Tauri còn đường dự phòng: đọc bản sao `<Agent Monitor>/Local Storage/leveldb` bằng `rusty-leveldb` (key `_http://127.0.0.1:4317\0` + 0x01 latin1 / 0x00 UTF-16LE, bỏ `LOCK`, xoá bản sao sau khi đọc). Max 5000 key / 32 MB.
   - `with_import` chèn prelude trước init script: chỉ trang cùng origin, bỏ qua nếu đã có key `ccm-shell.webstorage-imported`, không ghi đè key đã có. Trang `main` load xong → marker `<data>/webstorage-imported`, lần sau không đọc nữa. Origin giữ `127.0.0.1:4317` (cổng mặc định của bản release).
   - Debug build không đọc profile Electron thật (chỉ khi có `CCM_ELECTRON_USER_DATA`).
2. **safeStorage → keyring** (`vault.rs`, `legacy.rs`, `hosts.rs:181`)
   - `keychain.available / seal / unseal` (hợp đồng D2). Một master key 32 byte nằm trong keyring (`keyring` 3, Windows Credential Manager / macOS Keychain), service `Orrery` (release) / `Orrery Dev` (debug) / `CCM_KEYRING_SERVICE`; giá trị seal = `o1:` + base64(nonce ‖ AES-256-GCM). Chỉ 1 entry dù có bao nhiêu token.
   - Ciphertext Electron cũ (base64 của `v10…`): Windows = AES-256-GCM, key từ `Local State` `os_crypt.encrypted_key` (bỏ `DPAPI`, `CryptUnprotectData`); mac = AES-128-CBC IV 16 dấu cách, key PBKDF2-HMAC-SHA1(`saltysalt`, 1003, 16 byte) từ mật khẩu Keychain `Orrery Safe Storage` / account `Orrery Key`. Key cũ giải được thì lưu vào entry `electron-safe-storage-key` (lần sau không cần `Local State` / prompt Keychain). Mac chỉ đọc Keychain khi profile Electron có `Local State`.
   - Hỏng / không có key: host trả lỗi → core-desk coi như không có token (GitHub: access `unknown`, push đòi token; Telegram / phone: chưa cấu hình) + 1 notification mỗi phiên `tokens_lost` (vi/en): "thêm lại tài khoản GitHub / token Telegram trong Settings". Không panic.
   - Token chỉ được ghi lại dạng `o1:` khi user sửa tài khoản; file cũ vẫn chứa ciphertext Electron, đọc qua entry key cũ.
3. **Đóng gói** (`apps/shell/tauri.conf.json`, `tauri.release.json`, `nsis/hooks.nsh`, `build.rs`, `resources.rs`)
   - Version lấy từ `apps/desktop/package.json` (3.5.0): `tauri.conf.json` `"version": "../desktop/package.json"`; `build.rs` đọc cùng file → `ORRERY_VERSION` = `config::APP_VERSION` (desk `app_version`, pty `appVersion`, update so bản). Test `the_app_version_is_the_desktop_package_version`.
   - **Identifier**: dev giữ `dev.ccm.orrery.shell` (single-instance / socket không đụng bản cài thật); bản phát hành ghép `tauri.release.json` → `dev.ccm.monitor` = appId Electron: mac cùng bundle id (LaunchServices, quyền notification, deep link `agent-monitor`), Windows AUMID notification như cũ. `mainBinaryName` `Orrery` → `Orrery.exe` / `Orrery.app/Contents/MacOS/Orrery`.
   - **Đường dữ liệu**: data dir không đổi (`%LOCALAPPDATA%\ccm`, `~/Library/Application Support/ccm`, mặc định của `core-serve`): usage cache, alias, backup, `github-accounts`, layoutlib… dùng tiếp tại chỗ. Profile WebView = `<data>/shell-webview` (WebView2 / WKWebView không đọc được profile Chromium), nên localStorage đi qua bước 1. Thư mục userData `Agent Monitor` chỉ còn đọc (Local State, leveldb), không xoá.
   - NSIS: `installMode currentUser` (`%LOCALAPPDATA%\Orrery`), WebView2 `downloadBootstrapper` silent, lzma, icon `icons/icon.ico`, hook:
     - PREINSTALL: tìm Electron cũ ở `HKCU\…\Uninstall\ed94a9ba-cfb1-5245-8809-b9bd8561d205` (UUIDv5 của `dev.ccm.monitor` theo electron-builder, tính bằng `builder-util-runtime`), `InstallLocation` khác `$INSTDIR` → chạy uninstaller cũ `/S /KEEP_APP_DATA --updated _?=<dir>` (giữ dữ liệu, như electron-builder tự làm khi cập nhật), xoá `Uninstall *.exe` còn sót, `RMDir` nếu rỗng. Autostart Electron (`HKCU\…\Run` value `dev.ccm.monitor`) → ghi lại cho exe mới ở POSTINSTALL dưới tên `${PRODUCTNAME}`.
     - POSTINSTALL: có `--force-run` → chạy `Orrery.exe`.
   - dmg arm64 + x64, `minimumSystemVersion 11.0`, ad-hoc `signingIdentity "-"`, `hardenedRuntime false` (như Electron).
   - Ký: Windows `certificateThumbprint null` + `digestAlgorithm sha256` trong `tauri.release.json` (điền thumbprint hoặc thêm `signCommand`); mac đặt `APPLE_SIGNING_IDENTITY` / sửa `signingIdentity` khi có Developer ID (notarize: `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` theo Tauri).
   - Resources: `web/` (apps/web/dist), `layout-daemon/ccm-project-model.gradle`; sidecar `orrery-core` (pty) + `orrery-bridge` (statusline / hook bridge, `CCM_BRIDGE_BIN` mặc định cạnh exe đã khớp). `resources.rs` tìm `Contents/Resources` trên mac trước. `CCM_LAYOUT_*` đặt như `layoutDeskEnv` của Electron, kit nhúng qua `ORRERY_DAEMON_KIT` (dist script tính size + sha256 của `layout-daemon-<v>.zip`).
4. **Updater** (`update_host.rs`, `desk::wire`)
   - Vỏ trước đây không gọi `set_host` / `set_sink` của Desk: standing host (update, telegram…) và event nền không tới trang. `desk::wire` (gọi trong `setup`) nối cả hai, manage `Vault`, rồi `desk:update-start` (D8 đã ghép thêm `mcp::start` vào cùng hàm).
   - `update.env`: release Windows `nsis`, mac `mac`, debug `dev` (`assetKind` theo OS, `canDownload` khi có `CCM_UPDATE_FEED`). `update.prepare`: nsis giữ đường file; mac `hdiutil attach` + script port từ `install.ts` `macScript` (`mac-readonly` khi bundle không ghi được). `update.restart`: `desk:backup-write-auto` pre-update (kèm localStorage trang), ghi `pending.json`, sau 200 ms quitting + shutdown + chạy `Setup /S --updated --force-run` hoặc script mac rồi `exit(0)`; spawn lỗi → `restart()`. `update.open` (http(s) → trình duyệt, path tuyệt đối → open), `update.reveal`.
   - **Electron 3.4 → Tauri**: feed GitHub `releases/latest`, `pickAsset` tìm `Orrery-Setup-<v>.exe` / `Orrery-<v>-<arch>.dmg`; dist đổi tên artifact Tauri về đúng các tên đó (test `dist-tauri.test.ts` chạy `pickAsset` thật). Windows: 3.4 chạy Setup `--updated /S --force-run` → NSIS Tauri cài im lặng, hook gỡ bản Electron, `--force-run` mở app mới. Mac: script 3.4 `ditto` `Orrery.app` trong dmg đè `/Applications/Orrery.app` rồi `open`. Asset SHA-256 lấy từ `digest` GitHub như cũ.
   - **Portable**: Tauri không có bản portable 1 file → release Tauri không có `Orrery-Portable-<v>.exe`, user portable 3.4 sẽ thấy "không có bản cho máy này" (xem rủi ro).
5. **Script + CI**
   - `npm run dist` → `scripts/dist.mjs` → `dist:win` / `dist:mac` = `node scripts/dist-tauri.mjs --win|--mac`. Electron cũ: `npm run dist:electron` (`dist.mjs --electron`), `dist:electron:win`, `dist:electron:mac`.
   - `dist-tauri.mjs`: build web → `gen-desk-shim` → daemon zip (`build.mjs --daemon-zip-only` nếu chưa có) → mỗi target: `rustup target add`, `cargo build --release --locked -p core-bin -p core-bridge --target`, chép sidecar `apps/shell/binaries/<name>-<triple>`, `npx --yes @tauri-apps/cli@2.12.4 build --config tauri.release.json --target <triple> --bundles nsis|dmg --ci` (env `ORRERY_DAEMON_KIT`, `CARGO_BUILD_JOBS` ≤ `JIN_MAX_THREADS`/6) → `apps/shell/release/Orrery-Setup-<v>.exe`, `Orrery-<v>-arm64.dmg`, `Orrery-<v>-x64.dmg`; in size, cảnh báo Setup > 35 MB. CLI Tauri qua `npx` (không đổi `package-lock.json` đang có thay đổi của chủ nhân).
   - `.github/workflows/desktop.yml`: mặc định Tauri (`apps/shell/release/*`), `workflow_dispatch` input `electron: true` chạy bản Electron; rust-cache cả `apps/core` + `apps/shell`; bước node-pty x64 chỉ cho Electron. `release.yml` không đổi (vẫn `dist/*.exe dist/*.dmg dist/*.zip`).
   - `.gitignore`: `apps/shell/binaries/`, `apps/shell/release/`.
   - `cdp.cjs`: thêm cờ `--shell=tauri` (= `CCM_E2E_SHELL=tauri`); khi chạy vỏ đặt `CCM_E2E_DEVTOOLS=1`, `CCM_KEYRING_SERVICE=Orrery E2E`, `CCM_ELECTRON_USER_DATA=<tmp>/electron-ud` (E2E không đọc profile / Keychain Electron thật).
6. **Devtools** (`config.rs:38` `devtools_allowed`): chỉ debug build hoặc `CCM_E2E_DEVTOOLS=1`; áp cho cửa sổ chính (`window.rs` builder), child webview (`guest.rs` `.devtools(..)`) và F12 / `openDevTools` (`guest::devtools` trả false). CDP E2E Windows vẫn qua `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` (không phụ thuộc devtools UI).
7. **Size `orrery-core`** (`apps/core/Cargo.toml`): profile release `opt-level = "s"`, `lto = true` (fat), giữ `codegen-units = 1`, `panic = "abort"`, `strip = true`. Chưa đo (không build release theo đề, `cargo bloat` chưa cài). Dự kiến 8,1 → khoảng 6–6,5 MB (−20–25 %; vỏ shell cùng profile).
   - Đề xuất chưa làm: (a) `ureq` `native-tls` thay `rustls` + `platform-verifier` cho `core-desk` → bớt khoảng 1–1,5 MB mỗi binary, nhưng đổi hành vi TLS / proxy ngay trước release nên để sau. (b) Bản Tauri chứa core-serve + core-desk 2 lần (vỏ link thẳng, `orrery-core` sidecar chỉ dùng cho pty): thêm chế độ `orrery-shell --pty` (hoặc sidecar `orrery-pty` chỉ có `core-pty`) bỏ được khoảng 5 MB chưa nén. Ước Setup NSIS (lzma): vỏ khoảng 15–20 MB + core 6 MB + bridge < 1 MB + web khoảng 3 MB → nén khoảng 12–18 MB, dưới 35 MB.

## File

- Mới: `apps/shell/src/{vault,legacy,webstorage,update_host,resources}.rs`, `apps/shell/nsis/hooks.nsh`, `apps/shell/tauri.release.json`, `scripts/dist-tauri.mjs`, `apps/desktop/src/webstorageExport.ts`, `apps/desktop/test/{webstorage-export,dist-tauri}.test.ts`.
- Sửa: `apps/shell/{Cargo.toml,Cargo.lock,build.rs,tauri.conf.json}`, `apps/shell/src/{main,config,desk,hosts,strings,window,guest}.rs`, `apps/desktop/src/main.ts` (import + `exportPageStorage` + 1 phần tử `allSettled`), `apps/desktop/e2e/cdp.cjs`, `apps/core/Cargo.toml` (profile), `package.json` (scripts), `scripts/dist.mjs`, `.github/workflows/desktop.yml`, `.gitignore`, `.code_index/{shell,_meta}.md`, plan §4 dòng T2.
- Deps mới (shell): `keyring 3.6.3` (apple-native, windows-native), `aes-gcm 0.10`, `aes 0.8`, `cbc 0.1`, `pbkdf2 0.12`, `sha1 0.10`, `rusty-leveldb 3.0.3`; windows-sys `Win32_Security_Cryptography`.
- Không đụng `pty.rs`. `main.rs`: 5 dòng `mod`, `config::APP_VERSION` (2 chỗ), `webstorage::with_import`, `desk::wire`.
- Không comment trong code; file dài nhất mới `vault.rs` 307 dòng (một nửa là test).

## Số test thật

- `cargo clippy -j 4 --all-targets -- -D warnings` (apps/shell): sạch, cả trên bản sao `apps/core` HEAD lẫn cây thật sau khi D8 xong.
- `cargo test -j 4` (apps/shell, cây thật có D8): **77 passed**, 0 failed (bản sao HEAD: 75). Mới: vault 7 (round trip master key, giá trị hỏng / lạ → lỗi, ciphertext Electron Windows GCM + lưu key cũ vào keyring giả, mac CBC + PBKDF2, keyring từ chối ghi, encode key, cảnh báo 1 lần), legacy 3, webstorage 6 (decode latin1 / UTF-16, leveldb kiểu Chromium tự dựng, export thắng + marker, không nguồn, prelude, max), update_host 4, resources 3, config 1 (version). Keyring chỉ dùng `Mem` giả, không gọi keychain thật.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 166 file passed, 1 skipped; **1700 passed**, 29 skipped, 0 failed (gồm `webstorage-export` 5, `dist-tauri` 4).
- `npm run typecheck`: 0 lỗi.
- Chưa: `--target x86_64-pc-windows-msvc` (máy chỉ có target `aarch64-apple-darwin`; code Windows `dpapi`, `creation_flags` đọc lại theo chữ ký `windows-sys 0.61`), `tauri build`, E2E.

## Lệnh chủ nhân cần chạy

```sh
export PATH="$HOME/.cargo/bin:$PATH"
npm run dist:mac        # macOS: Orrery-3.5.0-arm64.dmg + Orrery-3.5.0-x64.dmg trong apps/shell/release
npm run dist:win        # Windows: Orrery-Setup-3.5.0.exe (xem dòng size, mục tiêu ≤ 35 MB)
CCM_E2E_SHELL_EXE="%LOCALAPPDATA%\Orrery\Orrery.exe" node apps/desktop/e2e/<bộ>.cjs --shell=tauri   # E2E bản cài (máy test)
```
CI: workflow `desktop` (tag `v3.5.0` hoặc dispatch) rồi `release` như cũ. Bản Electron: dispatch `desktop` với `electron: true`, hoặc `npm run dist:electron`.

## Checklist test tay

Windows (máy có Electron 3.4 đã cài, đã có tài khoản GitHub + localStorage tuỳ chỉnh):
- [ ] Electron 3.4 → Settings → Update thấy 3.5.0 (feed thật hoặc `CCM_UPDATE_FEED` loopback), tải + cài: Setup chạy im lặng, Electron cũ biến khỏi `%LOCALAPPDATA%\Programs\Orrery` và Apps & features, app Tauri tự mở.
- [ ] Shortcut Desktop / Start Menu "Orrery" trỏ `%LOCALAPPDATA%\Orrery\Orrery.exe`, icon taskbar đúng.
- [ ] Theme, layout, workspace, ngôn ngữ, tab… giữ như bản Electron (localStorage từ leveldb); mở lại lần 2 không bị ghi đè thay đổi mới.
- [ ] Settings → GitHub: tài khoản cũ còn, push / fetch được (token giải từ DPAPI); Credential Manager có `Orrery` / `safe-storage-key` + `electron-safe-storage-key`. Xoá `Local State` thử → notification nhập lại token, app không crash.
- [ ] Telegram token cũ còn chạy (nếu có).
- [ ] "Chạy cùng Windows" đã bật ở bản cũ → vẫn bật, khởi động lại máy app mở ẩn trong tray.
- [ ] Link `agent-monitor://open?...` mở pane; notification hiện tên Orrery.
- [ ] Máy không có WebView2 (Windows 10 sạch): Setup tải bootstrapper và cài.
- [ ] Setup ≤ 35 MB, RAM rảnh ≤ 200 MB.
- [ ] F12 / Ctrl+Shift+I không mở devtools ở bản cài.
- [ ] XML Layout Preview tải kit `layout-daemon-3.5.0.zip` (đã upload lên release) và render.
- [ ] Gỡ cài đặt Orrery Tauri: `%LOCALAPPDATA%\ccm` còn nguyên.

macOS (arm64 + x64 nếu có máy Intel):
- [ ] Electron 3.4 trong `/Applications` cập nhật lên dmg Tauri: app thay tại chỗ, tự mở, bỏ quarantine.
- [ ] Mở lần đầu: Keychain hỏi quyền đọc "Orrery Safe Storage" → Cho phép → tài khoản GitHub cũ chạy. Từ chối → notification nhập lại token.
- [ ] localStorage giữ như bản cũ.
- [ ] Traffic light + header, tray, dock, deep link `agent-monitor://`, Open at login.
- [ ] Terminal trong Workspace (sidecar `orrery-core` trong `Contents/MacOS`), statusline bridge (`orrery-bridge`) cài vào `~/Library/Application Support/ccm/bin`.
- [ ] Bản cài từ dmg mới (máy chưa có Electron): chạy bình thường, không có bước migrate.

## Rủi ro

- **Chưa build thật**: `tauri build` với `version` dạng đường dẫn package.json, `externalBin`, resources map, hook NSIS (`${GetParameters}` / `${GetOptions}` / `MAINBINARYNAME` / `PRODUCTNAME` của template Tauri, `Var /GLOBAL` trong macro) chưa chạy lần nào. Lỗi cú pháp NSIS chỉ hiện lúc `dist:win`.
- **GUID uninstall Electron** tính theo công thức electron-builder (UUIDv5 namespace `50e065bc-…`), chưa đối chiếu registry máy thật. Sai thì hook bỏ qua: hai bản cùng cài, shortcut Electron cũ còn (không mất dữ liệu).
- **Autostart**: tên value của `tauri-plugin-autostart` giả định = productName `Orrery`; khác thì checkbox tray hiện tắt dù Run có value.
- **Portable**: không có asset portable Tauri → user portable 3.4 kẹt ở 3.4 (cần tải Setup tay). Muốn giữ: thêm bản Electron portable vào release hoặc làm portable Tauri (zip thư mục).
- **Electron 3.4 không ghi export**: migrate localStorage dựa vào đọc leveldb của Chromium bằng `rusty-leveldb`; định dạng đã test bằng DB tự dựng, chưa test với profile Electron 44 thật (compaction / snappy). Đọc lỗi = bắt đầu với localStorage trống (không crash).
- **safeStorage mac**: app Tauri ký ad-hoc khác chữ ký Electron → Keychain luôn hỏi lần đầu. Electron có thể đã dùng tên Keychain khác nếu `app.name` khác `Orrery` (đặt `CCM_ELECTRON_KEYCHAIN_SERVICE` để thử). Windows: nếu có token Electron rất cũ không prefix `v10` (DPAPI trực tiếp) thì không giải được.
- **Data dir dùng chung**: chạy song song Electron và Tauri trên cùng máy sẽ tranh cổng 4317 (Tauri báo cổng bận và thoát); không phá dữ liệu.
- **Proxy**: update / GitHub qua `ureq` chỉ theo biến môi trường proxy (D9). Máy sau proxy công ty có thể không cập nhật được bản Tauri.
- **Cookie app trong BrowserPane** (rủi ro T1) vẫn còn ở bản phát hành.
- `opt-level "s"` cho `orrery-core` cũng áp cho bản Electron (`CCM_CORE=rust` serve): parse transcript có thể chậm hơn một chút; quay lại `3` nếu đo thấy chậm.

## Sửa sau review

Cổng: `cargo test -j 4` 86 pass, `cargo clippy -j 4 --all-targets -- -D warnings` sạch. Không build Tauri / E2E.

1. `config.rs:20` `try_random_hex` trả Err khi `getrandom` lỗi; `from_env` dùng `?` nên app dừng có thông báo (`fail`). `random_hex` còn lại (nonce guest, token pty) thoát process với lỗi, không còn seed theo giờ.
2. `update_host.rs:84` `accept_update_file`: canonicalize, file phải nằm đúng `<data_dir>/updates` (thư mục core `update/core.rs` tải về), tên `Orrery-Setup-*.exe` (Windows) / `Orrery-*.dmg` (mac), ký tự `[A-Za-z0-9._-]`. `prepare` nhận `data_dir`.
3. `update_host.rs:94` `inside_data_dir`: `pending.file` phải nằm trong `data_dir` sau canonicalize thư mục cha, không phải symlink.
4. `update_host.rs:154` `prepare_mac`: mount `updates/mnt-<pid>-<ms>` (mkdir 0700, không recursive nên trùng tên thì lỗi), script `updates/install-<pid>-<ms>.sh` qua `write_private_new` (`create_new`, 0600); không còn `temp_dir()`.
5. `webstorage.rs:27,43` chỉ nhận key prefix `ccm.` (export file và leveldb).
6. `webstorage.rs:141` `loaded` chạy thread, eval `localStorage.getItem(PAGE_MARKER)==='1'` qua `hosts::eval_value`, chỉ `mark_done` khi true. Không có nguồn để import thì marker không bao giờ ghi (mỗi lần mở quét lại, rẻ).
7. `webstorage.rs:62,119` scratch ở `<data_dir>/webstorage-scratch/<pid>` (cha 0700), `clear_scratch` dùng `symlink_metadata` (symlink thì `remove_file`, không theo), cha là symlink thì bỏ qua.
8. `config.rs:40` `devtools_gate`: bản release chỉ mở devtools khi có đủ `CCM_E2E_DEVTOOLS=1` + `CCM_PORT` số hợp lệ khác 0 + `CCM_SHELL_WEBVIEW_DIR` + `CCM_DATA_DIR` + `CCM_KEYRING_SERVICE` khác `Orrery`. Đó là đúng bộ env `apps/desktop/e2e/cdp.cjs` đặt cho `--shell=tauri` (cdp.cjs không đặt biến `CCM_E2E_PORT` vào env con nên không dùng được làm điều kiện; không sửa cdp.cjs). Debug build vẫn luôn mở. Hạn chế: kẻ kiểm soát env của process vẫn bật được; chặn tuyệt đối cần sửa cdp.cjs đặt thêm một secret.
9. `nsis/hooks.nsh`: chỉ chạy khi `UninstallString` bắt đầu bằng nháy, tách exe trong nháy, exe phải bắt đầu bằng `$R1\`; `$R1` và `$INSTDIR` cắt `\` cuối, so bằng `==` (không phân biệt hoa thường); `ExecWait '"$R5" /S ...'`.
10. `hooks.nsh`: xoá giá trị Run `dev.ccm.monitor`, xoá `Uninstall *.exe` / `RMDir` và đặt `OrreryHadLogin` chỉ khi `$R2 == "0"`.

Test mới (Rust): `update_names_follow_the_release_assets`, `only_a_named_file_inside_the_updates_folder_is_accepted` (2), `the_pending_file_must_stay_inside_the_data_dir` (3), `the_install_script_is_private_and_never_overwritten` (4), `only_ccm_keys_are_imported` (5), `a_symlinked_scratch_is_not_followed` (7), `the_confirmation_reads_the_page_marker` (6), `release_devtools_need_the_whole_e2e_environment` (8); `a_missing_file_is_not_prepared` và hooks test cập nhật. Hook NSIS chưa biên dịch thật (rủi ro `${Do}` / số register R0-R8 chỉ lộ lúc `dist:win`).

STATUS: DONE (chưa build / E2E)
