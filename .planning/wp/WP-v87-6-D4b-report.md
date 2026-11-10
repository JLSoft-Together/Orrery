# WP-v87-6-D4b — báo cáo (2026-11-10)

Nối mọi consumer TS của `runManager` với state Rust, rồi đổi `run` / `flutterPreview` sang mặc định rust.

## Đã làm
- `apps/desktop/src/run/facade.ts` `RunFacade`: là `runManager` của `main.ts`, giữ đúng interface consumer dùng (`list`, `live`, `getSettings`, `onSessionState`, `configs`, `devices`, `start`, `stopAll`, `killAll`, `loadSettings`). `RunManager` TS nằm ở `.node`: ipc node handler và đường node dùng nó.
  - Rust mode: `list` / `live` mirror từ `ccm:run-event` (prune 20 ended như node). `onSessionState` lấy từ event desk-only `desk:run-state`. `getSettings` lấy từ `desk:run-settings`; trước event đầu tiên thì dùng settings node đọc từ cùng file `run-settings.json`. `configs` / `devices` / `start` (MCP run tools telegram) gọi desk, host `preview.allowed` = project.
  - `ingest(event, payload)`: `ccm:run-event` đi qua `deps.broadcast` cũ (runLogs, telegram `runState` / `runFailedReport`, gửi cửa sổ), nên consumer không phải sửa và không gửi trùng.
- Rust `core-desk/src/run/`:
  - `sessions.rs` `update`: mỗi lần state đổi thì emit `desk:run-state` `{session, config, device}`. `Live.device` mới giữ `RunDevice::to_value`.
  - `manager.rs`: thêm `STATE_EVENT`, `SETTINGS_EVENT`, `settings_changed`. `ipc.rs`: emit `desk:run-settings` sau `ccm:run-project-sdk-set`.
  - `settings.rs` `SettingsStore`: đọc lại file khi mtime / size đổi (một mutex, không deadlock). Nhờ vậy chế độ trộn run node + fwp rust vẫn thấy settings mới.
  - Payload trang `ccm:run-event` giữ nguyên.
- File chung, mỗi chỗ sửa nhỏ bằng Edit:
  - `main.ts`: import, kiểu `runManager: RunFacade`, `new RunFacade({...}, () => deskRouter)`, `registerRunIpc({ manager: runManager.node })`, `setupDesk` `onEvent` gọi `runManager?.ingest(event, payload)` trước khi forward.
  - `desk/forward.ts`: `invoke(channel, args, hosts = {})`; bỏ `run`, `flutterPreview` khỏi `DESK_NODE_DEFAULT`.
  - `test/desk-twin.ts`: router đọc `CCM_DESK_<MODULE>` từ env (mặc định rust) cho mọi module; thêm tham số `onEvent`.
  - `desk-run.twin.test.ts` / `desk-fwp.twin.test.ts`: skip khi flag module = `node`, vì 2 file này so TS với Rust.
- Không chạm `devices/`, `apk/`, `releaseCheck/`, `update/`, `deepseek/`. Consumer ngoài `run/` (layout model, telegram, runLogs, MCP, devices `getSettings`, layout `getJdk` / `getRunSettings`, fwp `getSettings`, `confirmQuit`, `running`, `androidDirs`) giữ nguyên code.

## Số test thật
- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 695 passed, 0 failed. Có test mới `run::settings::tests::get_picks_up_a_file_written_by_another_process`.
- `cargo clippy -j 4 --all-targets -- -D warnings`: lỗi chỉ ở file của gói khác: `apk/axml.rs` ×2 (D5), `deepseek/parse.rs`, `deepseek/tests.rs`, `update/github.rs` (D9). Không có lỗi nào ở `run/` hay `flutter_preview/`.
- `npm test -w @ccm/desktop -- --maxWorkers=4` (debug `orrery-core` mới build): 156 files passed, 1 skipped; 1635 tests passed, 29 skipped.
  - Mới: `test/run-facade.test.ts` có 6 test: mirror, prune, listener, settings, gọi desk, node mode.
  - Mới: `test/desk-run-facade.twin.test.ts` có 3 test. Cùng chuỗi thao tác (settings-set, project-sdk-set, configs, start ok / bad / ghost, output, exit, start slow, stop) thì `getSettings`, `list`, `live`, transitions `onSessionState` (session + config + device), log theo session và session event qua broadcast khớp giữa node và rust.
- `CCM_DESK_RUN=node CCM_DESK_FLUTTER_PREVIEW=node`: facade twin 3 passed; twin run / fwp TS-vs-Rust skip (10).
- `npm run typecheck`: 0 lỗi.

## E2E cần chạy
Chưa chạy E2E, chưa build release.

1. Build: `node apps/desktop/build.mjs` + `cd apps/core && cargo build --release -j 4 -p core-bin`.
2. Chạy: `CCM_E2E_PORT=4511 CCM_E2E_DBG=9511 CCM_CORE_BIN=<release> node apps/desktop/e2e/<bộ>.cjs`. Lấy danh sách bộ bằng `rg -l "ccm:run-|ccm:fwp-|run-start|fwp" apps/desktop/e2e`. Mặc định giờ là rust; chạy thêm 1 lần với `CCM_DESK_RUN=node CCM_DESK_FLUTTER_PREVIEW=node`.
3. Kiểm tay trong app:
   - Build Android chạy xong thì XML Preview refresh model.
   - Telegram nhận state run.
   - MCP `run_start` / `run_configs` chạy được.
   - Quit app khi có run đang chạy: hộp thoại liệt kê run, run Rust dừng.

## Khác biệt còn lại
- Chế độ trộn run rust + fwp node: fwp TS lấy settings qua facade (đúng). Chế độ trộn run node + fwp rust: Rust đọc lại file theo mtime (đúng).
- `RunFacade.killAll` chỉ kill TS. Run Rust dừng qua `stopRuns` (`desk:run-stop-all` rồi `desk:run-kill-all`); quit gấp thì `Desk::on_close` kill khi core tắt. Lý do: gọi kill-all trong `killAll` sẽ cắt ngang graceful stop của `stopRuns`.
- Desk chết giữa chừng mà router chưa đánh dấu fallback: facade rust trả `[]` / `{ok:false, error:{code:'spawn'}}` cho MCP, không chạy run ở TS.
- Còn nguyên từ D4: runner `start()` dạng thread, Windows `killTreeSparing` qua PowerShell, chưa test fvm-local-unverified (fwp), watcher trên FS thật, `run-devices` với daemon thật. `devices()` của facade chưa test với adb / flutter thật.

## Mặc định cuối cùng
- `run`: rust (`CCM_DESK_RUN=node` để về TS).
- `flutterPreview`: rust (`CCM_DESK_FLUTTER_PREVIEW=node` để về TS).

STATUS: DONE
