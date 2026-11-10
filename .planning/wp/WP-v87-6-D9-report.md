# WP-v87-6-D9 — báo cáo (2026-11-10)

Port `update/` (kiểm bản mới, tải + SHA-256, state, timer) và `deepseek/` (cài, login, sync rules / skills / MCP) sang `core-desk`. Cờ `update` và `deepseek` nằm trong `DESK_NODE_DEFAULT` (mặc định node; twin bật rust).

## Đã làm

1. `core-desk/src/update/`: `semver`, `pick` (chọn asset theo kind + arch), `github` (feed `releases/latest`, allow-list host feed / download / page, `parse_release`, `parse_digest`), `download` (stream vào `.part`, SHA-256, tiến độ 250 ms + tốc độ cửa sổ 3 s, rename khi khớp size + hash, redirect đi tay từng hop và kiểm host từng hop), `net` (agent `ureq`), `core` (`Core` = `UpdateManager`: state, timer 30 s / 12 h, `update.json`, `pending.json`, dedupe `check`, huỷ tải). 9 kênh `ccm:update-*` + `desk:update-start`; event `ccm:update-event`.
2. `core-desk/src/deepseek/`: `parse`, `cli` (tìm codewhale / npm wrapper / winget, `install_plan`, `update_plan`), `sync` (managed block AGENTS.md, import inline, TOML `project_instruction_imports` + `[lifecycle_outbox]`, merge MCP, skill managed, fingerprint SHA-256, backup `.orrery-backup`, ghi qua file tạm), `templates` (sinh từ `templates.ts`, không gõ tay), `runner` (spawn có stdin, stream từng dòng, deadline, huỷ, kill cả process group), `Service` (busy, cache status 60 s, cancel). 9 kênh `ccm:deepseek-*`; event `ccm:deepseek-event`. Chỉ đăng ký khi `CCM_DEEPSEEK_FEATURE=1` (giữ gate tạm tắt). KHÔNG gọi LLM, không gửi prompt; API key chỉ đi qua stdin của `codewhale auth set`, thay bằng `••••` trong detail lỗi, không vào log.
3. TS: `update/deskHosts.ts` (`updateHosts`), `UpdateManager.hostEnv()`, `update/ipc.ts` + `deepseek/ipc.ts` đi qua `deskHandle` (deepseek hosts per request: `deepseek.confirm`), `forward.ts` thêm module `update`, `deepseek` + 2 event, `main.ts` ghép `updateHosts` vào `hosts` standing của `DeskHost`, rust mode khởi động update bằng `desk:update-start` (không trả `true` thì `updates.start()` TS).
4. Thuần vỏ (không port): `prepare` (`prepareNsis` / `preparePortable` / `prepareMac`), `restart` (`confirmQuit`, `writeAuto('pre-update')`, `app.quit`, chạy `launch` khi thoát), `openUrl`, `reveal`, dialog confirm DeepSeek.

## File

- Mới: `apps/core/crates/core-desk/src/update/{mod,core,semver,pick,github,download,net,tests}.rs`, `.../deepseek/{mod,parse,cli,sync,templates,runner,tests}.rs`, `apps/desktop/src/update/deskHosts.ts`, `apps/desktop/test/desk-update.twin.test.ts`, `apps/desktop/test/desk-deepseek.twin.test.ts`.
- Sửa nhỏ: `core-desk/src/lib.rs` (2 `pub mod` + 2 `register`), `apps/desktop/src/{update/ipc.ts,update/manager.ts,deepseek/ipc.ts,desk/forward.ts,main.ts}`, `.code_index/{core,desktop,_meta}.md`, plan §4 dòng D9.
- Không đụng file của D4b / D5.

## Số test thật

- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 723 passed, 0 failed. Mới: update 12 (semver, pick, github, download, 6 test luồng với server loopback tự dựng: check / tải / cài / từ chối restart / mac-readonly / digest sai / thiếu digest / không có bản mới / skip + auto lưu / huỷ giữa chừng / start dọn hoặc báo `incomplete` / kind dev), deepseek 16 (parse, cli, sync, 6 test với `codewhale` / `npm` giả).
- `cargo clippy -j 4 --all-targets -- -D warnings`: sạch phần D9; còn 2 lỗi ở file gói D5 (`core-desk/src/apk/axml.rs:70` `chunks_exact`, `:142` collapsible match), không sửa.
- Twin: `desk-update.twin` 4 passed, `desk-deepseek.twin` 5 passed, đều `twin.logs` và `twin.fellBack` rỗng.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 159 file passed, 1 skipped; 1653 passed, 29 skipped, 0 failed. `npm run typecheck`: 0 lỗi. Đường TS (cờ node) vẫn do `update.test.ts` và `deepseek-setup.test.ts` phủ, pass.

## E2E cần chạy (chưa chạy, theo đề)

`v74.cjs` có kiểm update qua `CCM_UPDATE_FEED` (feed loopback). Chạy sau `node apps/desktop/build.mjs` + `cargo build --release -j 4 -p core-bin`, instance cô lập `CCM_E2E_PORT=4531 CCM_E2E_DBG=9531`, thêm `CCM_DESK_UPDATE=rust`; lượt thứ hai không đặt cờ (đường TS). DeepSeek chưa có bộ E2E (feature tạm tắt): nếu muốn thử tay, `CCM_DEEPSEEK_FEATURE=1 CCM_DESK_DEEPSEEK=rust`, dùng binary giả, không đăng nhập thật.

## Callback host mới (vỏ Tauri làm theo)

Standing (update, `HostLink` req 0): `update.env` -> `{kind, assetKind, canDownload, arch}`; `update.prepare {file}` -> `{ok:true}` | `{ok:false, error}` (vỏ giữ `Prepared` cho lần `restart`); `update.restart {version, pending:{file,text}}` -> bool (vỏ `confirmQuit(true)`, `writeAuto('pre-update')`, đặt `launch` = ghi `pending.text` vào `pending.file` rồi chạy `Prepared.launch`, `app.quit`); `update.open {target}`; `update.reveal {file}`. Per request: `deepseek.confirm {kind:'install'|'download'|'update', commands}` | `{kind:'logout'}` | `{kind:'sync', files:[{file,action}], mcpAdded, outboxAdded}` -> bool. Tauri cũng phải gọi `desk:update-start` lúc boot (thay `updates.start()`).

## Khác biệt còn lại

- Proxy: Electron `net.fetch` theo proxy hệ thống, `ureq` chỉ đọc biến môi trường proxy. Máy sau proxy công ty có thể báo `offline` / `download` ở Rust; cách lui `CCM_DESK_UPDATE=node`. Cần xử lý trước khi bỏ Electron.
- Tải file không có timeout tổng (như TS); chỉ có timeout kết nối 15 s và chờ header 30 s. Mạng treo giữa chừng chờ tới khi TCP báo lỗi hoặc user bấm huỷ.
- Redirect tải: Rust kiểm host từng hop (chặt hơn TS chỉ kiểm host cuối). Link release GitHub (github.com -> `*.githubusercontent.com`) vẫn qua.
- `published_at` chỉ nhận dạng ISO (`Date.parse` của JS nhận nhiều dạng hơn); GitHub luôn trả ISO.
- Confirm host lỗi / vỏ không trả lời -> coi như huỷ (`cancelled`); TS sẽ reject.
- Lỗi ghi file khi sync: `detail` là câu lỗi của Rust (`Permission denied (os error 13)`), không phải mã `EACCES` của Node.
- Update ở rust mode: nếu desk chết giữa phiên, router chạy `UpdateManager` TS với state mới (idle), timer auto check TS không được bật lại tới lần mở app sau.
- `check` trùng nhau dùng Condvar (cùng kết quả như promise dùng chung của TS). State Rust nằm trong process desk: desk restart (tối đa 3 lần) mất state `phase`, giữ `update.json`.
- macOS chạy thật các test `cfg(unix)` (runner, fake codewhale); nhánh Windows (`taskkill`, `raw_arg`, `creation_flags`) chỉ compile theo `cfg`, chưa chạy.

STATUS: DONE
