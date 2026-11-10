# WP-v87-6-D4 — báo cáo (2026-11-10)

Port `apps/desktop/src/run/` (23 kênh `ccm:run-*`) và `flutterPreview/` (5 kênh `ccm:fwp-*`) sang `core-desk`. Cờ `CCM_DESK_RUN` / `CCM_DESK_FLUTTER_PREVIEW` nằm trong `DESK_NODE_DEFAULT` (mặc định node, chưa qua E2E); twin test bật rust. Logic TS giữ làm fallback.

## Đã làm
- `run/` Rust: types / args / parse / proc / collect / before (spawn, kill cây, line splitter), configs + config_store (personal + shared, trust hash) + importers (IntelliJ / VS Code) + command (preview) + toolchain (doctor / android checks) + watch (notify, debounce 500ms) + settings; `flutter/` (env, detect, machine JSON-RPC, devices daemon, runner), `android/` (detect, env, jdk_pick, adb_watch, devices, runner, spare), `custom.rs`; manager (`manager*.rs`, `sessions.rs`, `pubget.rs`, `ipc.rs`).
- `flutter_preview/` Rust: scanner Dart, host Dart, service, watcher, 5 kênh, event `ccm:fwp-state`.
- Hạ tầng: `Desk::host_link` (gọi host ngoài request, `for: 0`), `Desk::on_close` (kill run / flutter con khi core tắt), `DeskHost opts.hosts` (host standing), `DeskRouter.invoke`.
- TS: `run/ipc.ts` + `run/deskHosts.ts`, `flutterPreview/{ipc,wire}.ts`, `main.ts` (standing hosts, `runConfirm`, `stopRuns` dừng run Rust trước khi dừng desk), `desk/forward.ts`.

## Số test thật
- `cargo test -j 4 --no-fail-fast` workspace `apps/core`: 694 passed, 0 failed. `cargo clippy -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4` (debug `orrery-core`): 152 files passed, 1628 tests passed, 18 skipped. Twin mới: `desk-run.twin.test.ts` 7 test (settings, templates, detect, configs, save / delete personal + shared, import, preview, env, toolchain, pub get, chạy custom ok / fail / before-launch / spawn lỗi / stop / trust confirm, so event TS vs Rust), `desk-fwp.twin.test.ts` 3 test; `twin.logs` và `twin.fellBack` rỗng.
- `npm run typecheck`: 0 lỗi.
- Chưa chạy twin với `CCM_DESK_RUN=node` (router twin không đọc env; đường node có test TS cũ).
- Không build release / E2E.

## E2E cần chủ nhân chạy
Build: `node apps/desktop/build.mjs` + `cd apps/core && cargo build --release -j 4 -p core-bin`. Instance cô lập `CCM_E2E_PORT=4511 CCM_E2E_DBG=9511 CCM_CORE_BIN=<release> CCM_DESK_RUN=rust CCM_DESK_FLUTTER_PREVIEW=rust node apps/desktop/e2e/<bộ>.cjs`. Bộ: `rg -l "ccm:run-|ccm:fwp-|run-start|fwp" apps/desktop/e2e` (Run pane v31, Flutter preview), chạy thêm 1 lần với `=node`.

## Callback host mới
- Per request: `run.owner {key}` -> chuỗi owner `${sender.id}:${key}` (vỏ phải gọi `desk:run-release-owner(prefix)` khi cửa sổ đóng / điều hướng); `preview.allowed` (dùng lại, cho cả `fwp`).
- Standing (ngoài request): `run.confirm {code: 'reinstall'|'trust-config', detail}` -> bool; `run.adbUse {serial}`.

## Khác biệt còn lại
- Rust mode: session / settings do Rust giữ; consumer TS đọc `runManager` (layout model `onSessionState`, telegram `runState`, `runLogs`, MCP run tools, diagnostics, `getSettings` của layout / devices) không thấy dữ liệu Rust. Cần cầu nối (mirror từ `ccm:run-event` + `desk:run-settings`) trước khi bật rust mặc định.
- `start()` của runner chặn tới khi run kết thúc; thread thay async; `on_change` không có unsubscribe.
- Windows: `killTreeSparing` qua PowerShell CIM (không có snapshot native); nhánh Windows chỉ test chuỗi.
- `wire.ts` vẫn tạo `FlutterPreviewService` TS (không spawn tới khi gọi). Request flutter `show` / `restart` có deadline 30s / 120s.
- Chưa test: fvm-local-unverified (fwp), watcher notify trên FS thật, `run-devices` / `run-emulators` với daemon thật.

STATUS: PARTIAL
