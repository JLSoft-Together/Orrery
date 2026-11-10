# WP-v87-6-D2 — báo cáo (2026-11-10)

Port D2 sang `core-desk`: `github`, `phone`, `capture`, `power` (keep awake), `disk`, `agentStatus`, `resources` (`ccm:res-kill`). Twin test TS ↔ Rust qua hết. **Chưa chạy E2E** (chủ nhân tự build) → cả 7 module nằm trong `DESK_NODE_DEFAULT` (mặc định `node`), bật bằng `CCM_DESK_<MODULE>=rust`.

## Đã làm

1. **Khung**: `Emitter` trong `core-desk/src/lib.rs` (`desk.emitter()`, `desk.set_sink(sink)`) để module gửi event từ thread nền (timer, fs watch); `stdio::run` nối sink vào pipe NDJSON. `Ctx::emit` cũ giữ nguyên.
2. **`keychain.rs`**: seal / unseal / available đều qua host (`keychain.*`), Rust không giữ khoá. Token GitHub, URL / topic ntfy, webhook Discord chỉ đi qua pipe tới vỏ để seal / unseal; không trả về trang, không ghi log (lỗi trả về chỉ là mã lỗi).
3. **`github/`** (`store.rs`, `api.rs`, `mod.rs`): `ccm:gh-accounts|gh-account-add|gh-account-remove|gh-account-default`; store đọc-sửa-ghi `github-accounts.json` mỗi lượt (cùng format, cùng thứ tự key, file 0600), kiểm token `GET /user` (scopes từ `x-oauth-scopes`), max 20 tài khoản, đổi token khi trùng login. Thêm cho vỏ Tauri: `desk:gh-list`, `desk:gh-resolve`, `desk:gh-env` (= `envFor`, `GIT_CONFIG_*` + Basic), `desk:gh-set-choice` (cache quyền 10 phút như TS). `CCM_GH_API` (chỉ loopback) cho test.
   TS: `GitHubAccounts.reload()`; khi module chạy rust, `github/ipc.ts` đọc lại file sau mỗi lượt để push / fetch (preview hosts `preview.ghEnv` …) thấy thay đổi.
4. **`phone/`** (`send.rs`, `mod.rs`): `ccm:phone-state|set|test` + `desk:phone-episode` (main chuyển episode collector vào đây khi rust); URL sạch bằng crate `url` (WHATWG), text en / vi + `mask_secrets`, rate 20 / giờ, POST ureq timeout 10 s, `CCM_PHONE_URL_OVERRIDE`; event `ccm:phone-event` như TS (`failed` 1 lần mỗi chuỗi lỗi, rồi `state`).
5. **`capture/`** (`files.rs`, `folder.rs`, `mod.rs`): `ccm:capture-get|pick|set|shot|open` + `desk:capture-sweep`; tên shot local time, ghi `create_new` dup 1..50, retention (chỉ file tên Orrery, bỏ file đang giữ). TS `CaptureFolder.reload()` sau lượt rust (recorder / MCP vẫn dùng object TS).
6. **`power/`** (`awake.rs`, `native.rs`, `mod.rs`): `ccm:awake-state|mode` + `desk:awake-busy|power|dispose`, event `ccm:awake-event`. Giữ máy thức trong Rust: macOS `/usr/bin/caffeinate -i -w <pid desk>`, Windows thread riêng `SetThreadExecutionState(ES_CONTINUOUS|ES_SYSTEM_REQUIRED)`; pin: `pmset -g ps` / `GetSystemPowerStatus`. `CCM_E2E_POWER`, `CCM_E2E_AWAKE_GRACE_MS`, mới `CCM_E2E_AWAKE_LOG=<file>` (ghi start / stop thay vì giữ máy thật). TS `power/ipc.ts` `awakeControl` (busy từ collector, on-battery / on-ac, dispose lúc thoát).
7. **`disk/`** (`space.rs`, `mod.rs`): `ccm:disk-state|refresh|set|projects|open` + `desk:disk-start|stop|poke`; `DiskWatch` 10 phút + poke gộp, alert 1 lần mỗi lần vượt ngưỡng, re-arm +1 GB. statfs (`libc::statfs` / `GetDiskFreeSpaceW`), mount (`dev` đi lên / `GetVolumePathNameW`). Nơi đo: Claude, Codex, data, capture (đọc `capture.json`, `CCM_VIDEOS_DIR`), project, Android (`run-settings.json` `androidSdk` → `ANDROID_HOME` → `ANDROID_SDK_ROOT` → mặc định, AVD home). `CCM_E2E_DISK`. Event `ccm:disk-event` + `desk:disk-alert` (main chuyển cho collector `host.post`). TS `diskControl`.
8. **`agent_status/`** (`status.rs`, `watch.rs`, `mod.rs`): `ccm:agent-status-watch`, event `ccm:agent-status`; parse như `parseAgentStatus` (bỏ ký tự điều khiển / bidi, max 40 / 80 code point, progress kẹp 0–1), realpath chống thoát thư mục, max 4096 byte. Watch bằng `notify` trong 1 worker thread (lệnh qua channel, callback watcher không giữ lock → không deadlock khi bỏ watcher); debounce 300 ms, recheck 30 s, hold 5 s, max 64 key.
9. **`resources.rs`**: `ccm:res-kill` (port `resKill` của `main.ts` + `resources/kill.ts`): parse target, plan theo report mới nhất (host), chặn daemon bận < 2 phút, so start time bằng `core_procs::table()`, confirm qua host (vỏ dựng câu i18n như cũ), emulator qua host `avdStop`, kill cây (`taskkill /T /F` / TERM rồi KILL sau 5 s), `CCM_RES_FIXTURE` + `CCM_E2E_RES_CONFIRM`.
10. **Desktop**: `desk/forward.ts` thêm 7 module vào `DESK_MODULES` + `DESK_NODE_DEFAULT`, `DESK_EVENTS` += `ccm:phone-event|awake-event|disk-event|agent-status`; `main.ts`: truyền `desk` cho github / capture / agentStatus / disk / phone, `phoneEpisode`, `awakeCtl`, `diskCtl`, `resKillOpts` + `resKillHosts` + `ccm:res-kill` qua router, env desk `CCM_VIDEOS_DIR`, event `desk:disk-alert` → `host.post`.

## Thuần vỏ, không port

- `attention.ts`: dock badge / bounce trên macOS, nhận toast từ collector — không có kênh `ccm:*`, chỉ là API dock.
- `capture.ts` `dustSize` + `ccm:capture-rect` / `ccm:capture-guest`: `capturePage` của BrowserWindow / webContents (T1 đã làm ở vỏ Tauri).
- `resources/deviceUse.ts` (gửi `device.used` cho collector từ adb spawn) và `resources/agentPorts.ts` (MCP `free_port` / `list_servers`, thuộc D8): không có kênh `ccm:*` riêng. Đường MCP `resKill(..., ask)` vẫn TS.

## File

- Mới Rust: `apps/core/crates/core-desk/src/{keychain.rs, resources.rs, github/{mod,store,api}.rs, phone/{mod,send}.rs, capture/{mod,files,folder}.rs, power/{mod,awake,native}.rs, disk/{mod,space}.rs, agent_status/{mod,status,watch}.rs}`.
- Sửa Rust: `core-desk/src/lib.rs` (mod + register + `Emitter`), `core-desk/src/stdio.rs` (`set_sink`), `core-desk/Cargo.toml` (`base64` 0.23, `notify` 8.2, `core-procs`, windows-sys `Win32_System_Power` + `Win32_Storage_FileSystem`), `Cargo.lock`.
- Mới TS: `apps/desktop/src/power/ipc.ts`, `apps/desktop/test/desk-{github,phone,capture,disk,agent-status,resources}.twin.test.ts`.
- Sửa TS: `apps/desktop/src/{main.ts, desk/forward.ts, github/{accounts,ipc}.ts, phone/ipc.ts, capture/{folder,ipc}.ts, disk/ipc.ts, agentStatus/ipc.ts}`.
- Doc: `.code_index/{core,desktop,_meta}.md`, `.planning/PLAN-v87-step6-tauri.md` §4 ghi chú D2.
- Không đụng: `preview/**` (D3b), `run/**` / `flutter_preview/**` (gói khác), `apps/shell`, `packages/shared`, `apps/web`.

## Số test thật

- `cargo test -j 4 --no-fail-fast` cả workspace `apps/core`: lượt 1 (trước khi gói `run/` / `flutter_preview/` đổi) 548 passed, 0 failed, 6 ignored; lượt cuối 652 passed, 13 failed, 6 ignored — cả 13 fail đều ở `core-desk/src/flutter_preview/**` (gói khác đang làm), không test D2 nào fail. Unit test mới của D2: github 3, phone 2, capture 1, power 2, disk 3, agent_status 1, resources 1.
- `cargo clippy -j 4 --all-targets -- -D warnings`: file của D2 sạch; lỗi còn lại đều ở file gói khác: `flutter_preview/{events,host,service}.rs`, `run/{android/catalog,manager,sessions,types,configs}.rs`, `lib.rs:160` (field `closers` do gói khác thêm).
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 150 file passed + 1 skipped, 1607 passed, 29 skipped, 0 failed.
- Twin mới (qua `orrery-core desk` debug thật, `twin.logs` / `fellBack` rỗng): github 3, phone 2, capture + awake 3, disk 2, agentStatus 2, resources 1 = 13 test, 6 file.
- `npm run typecheck` (mọi workspace): 0 lỗi.

## E2E cần chạy (chủ nhân, sau build)

Build: `node apps/desktop/build.mjs` + trong `apps/core` `cargo build --release -j 4 -p core-bin`. Cờ bật Rust:
`CCM_DESK_GITHUB=rust CCM_DESK_PHONE=rust CCM_DESK_CAPTURE=rust CCM_DESK_POWER=rust CCM_DESK_DISK=rust CCM_DESK_AGENT_STATUS=rust CCM_DESK_RESOURCES=rust`, instance riêng `CCM_E2E_PORT=4492 CCM_E2E_DBG=9492`.

| Bộ | Kênh D2 | Lệnh (từ repo root) |
|---|---|---|
| `github-accounts.cjs` | `ccm:gh-account*` (+ push dùng `preview.ghEnv`) | `<cờ> CCM_E2E_PORT=4492 CCM_E2E_DBG=9492 node apps/desktop/e2e/github-accounts.cjs` |
| `push.cjs` | git push qua tài khoản (store TS đọc lại sau lượt Rust) | `<cờ> … node apps/desktop/e2e/push.cjs` |
| `v76.cjs` | keep awake, capture folder, phone (`CCM_PHONE_URL_OVERRIDE`, `CCM_E2E_IDLE_S`), `ccm:res-kill` (`CCM_RES_FIXTURE`) | `<cờ> … node apps/desktop/e2e/v76.cjs` (thêm `CCM_E2E_AWAKE_LOG=%TEMP%\awake.log` nếu không muốn giữ máy thức thật) |
| `v77a.cjs` | disk (`CCM_E2E_DISK`) | `<cờ> … node apps/desktop/e2e/v77a.cjs` |
| `agent-api.cjs` | `ccm:agent-status-watch` | `<cờ> … node apps/desktop/e2e/agent-api.cjs` |

Chạy lại 1 lần mỗi bộ với cờ `=node` (hoặc bỏ cờ, mặc định node). Qua E2E → bỏ module khỏi `DESK_NODE_DEFAULT`.

## Callback host mới (vỏ Tauri phải trả)

- `keychain.available` → bool; `keychain.seal {plain}` → chuỗi base64 (lỗi = không seal được); `keychain.unseal {sealed}` → chuỗi (lỗi = coi như chưa cấu hình). Tauri: `keyring` + migrate safeStorage (T2).
- `github.confirmRemove {login}` → bool.
- `phone.idle` → giây máy rảnh; `phone.locale` → `en` | `vi`.
- `capture.pick {near}` → path | null; `capture.open {dir}` → bool; `capture.missing {dir}` → (hiện Notification); `capture.held` → string[] (file recording đang giữ token).
- `disk.open {dir}` → bool.
- `resources.report` → ResourceReport | null (report mới nhất từ collector); `resources.confirm {items}` → bool (dựng câu `resKillTitle` / `resKillDetail` + `confirmLines`); `resources.avdStop {avd}` → bool.
- Event: `ccm:phone-event`, `ccm:awake-event`, `ccm:disk-event`, `ccm:agent-status` → cửa sổ; `desk:disk-alert` → gửi collector (`disk.alert`). Vỏ Tauri phải gọi `desk.set_sink(...)` thì mới nhận được event nền (awake / disk / agent status).
- Kênh nội bộ chỉ cho vỏ (không cho trang gọi): `desk:gh-list|gh-resolve|gh-env|gh-set-choice` (dùng cho host `preview.ghEnv|ghResolve|ghSetChoice` và `backup.github`), `desk:phone-episode`, `desk:capture-sweep` (timer 6 giờ), `desk:awake-busy|awake-power|awake-dispose`, `desk:disk-start|disk-stop|disk-poke`.

## Khác biệt còn lại / rủi ro

- **State 2 bên** (Electron, rust mode): github / capture lưu file, TS đọc lại sau mỗi lượt Rust; `capture.missing` có thể báo 2 lần cho 1 thư mục (1 từ Rust, 1 từ object TS dùng cho recorder). Desk restart làm mất state trong RAM (rate limiter / episode đã gửi của phone, cảnh báo disk đã gửi, watch agent status, trạng thái giữ máy thức → nhả, chờ `busy` kế tiếp).
- **power**: blocker do process `orrery-core desk` giữ (caffeinate / thread), không phải Electron `powerSaveBlocker`; mỗi `apply` có release time tạo 1 thread ngủ (huỷ theo generation).
- **phone**: seal lỗi → TS ném lỗi IPC, Rust trả `{ok:false, error:'failed'}`; locale khác `en` → `vi`; Rust chỉ kiểm tối thiểu episode (parse đầy đủ `parsePhoneEpisode` vẫn ở vỏ); lượt gửi chạy tuần tự (khoá giữ trong lúc POST, max 10 s).
- **capture**: TS sweep sau `set` chạy nền, Rust sweep xong rồi mới trả lời; `sweepOrphans` (lúc khởi động, cần `fileComplete` của recorder — D5) và timer 6 giờ vẫn TS ở Electron; base64 của Rust bỏ ký tự lạ nhưng không dừng ở `=` như Node.
- **disk**: Windows mount bằng `GetVolumePathNameW` (thư mục mount NTFS có thể ra mount khác cách đi lên theo `dev` của Node); đo song song 2 lượt → lượt sau trả state vừa đo; Android SDK đọc `run-settings.json` trực tiếp.
- **agentStatus**: `notify` (FSEvents / ReadDirectoryChangesW) thay `fs.watch`; `set` trả `true` trước khi event gửi lại (worker).
- **github**: `GIT_CONFIG_COUNT` parse chặt hơn `parseInt` ("3abc" → 0); resolve gọi API tuần tự thay vì `Promise.all`.
- **resources**: liveStarts chỉ từ `core_procs::table()` (TS còn fallback PowerShell CIM / `ps` khi core procs tắt).
- macOS: chạy + twin trên máy Mac arm64; Windows code theo `cfg` (`SetThreadExecutionState`, `GetSystemPowerStatus`, `GetDiskFreeSpaceW`, `GetVolumePathNameW`, taskkill) chưa biên dịch trên Windows.

STATUS: PARTIAL
