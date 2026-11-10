# WP-v87-6-D5b — báo cáo (2026-11-10)

Port nốt 4 nhóm kênh của `devices/` sang `core-desk`, cùng cờ `CCM_DESK_DEVICES` (vẫn trong `DESK_NODE_DEFAULT`, mặc định node, twin bật rust): `ccm:dev-rec-*` (5), `ccm:devfs-*` (7), `ccm:perf-*` (5), `ccm:appdata-*` (8). 25 kênh, có twin test.

## Đã làm

1. `adb_io.rs`: thêm vào `Adb` các lệnh còn thiếu: `exec_out` (bytes, `overflow`, `timed_out`), `shell_input` (stdin), `spawn_shell`, `spawn_adb`, `pull`. Kill bằng pid như `killAdbClient`.
2. `recorder.rs`: `Recorder` (screenrecord + thread frame + token tối đa 3, `thin_frames` / `pick_even` / `mp4_complete` giống TS). Kênh `ccm:dev-rec-start|stop|cancel|save|discard`. `stop` / `cancel` nối tiếp qua mutex của session (`stop` đang chạy thì `cancel` chờ rồi discard token). deadline chờ orphan `(30+60)s` tự cancel. Static `held_files()` + event `ccm:dev-rec-held` để retention của capture không xoá clip đang giữ token (`capture/mod.rs` `held()` gộp thêm).
3. `devfs.rs` + `files.rs`: parse `ls -la` (toybox / toolbox / busybox), path / scope, argv, `local_safe_name`, `unique_name`, `safe_to_open`; `DeviceFiles` list / mkdir / remove (confirm) / pull (file stream + `.orrery-part`, dir qua `adb pull`) / push (stdin, replace confirm, kiểm size) / cancel / open. Progress throttle 120ms.
4. `perf.rs`: `Perf` (parse gfxinfo / meminfo / `am start -W`, history `perf/<pkg>.json` indent 1, reset map, busy per device, confirm cold).
5. `appdata.rs` + `sqlguard.rs` + `sqlwal.rs` + `prefs.rs`: `AppData` (list, pull db / wal / prefs qua `exec-out run-as cat`, `apply_wal` giống TS, mở DB bằng `rusqlite` `bundled` + `serialize`: `deserialize_read_exact` read-only + `PRAGMA query_only`, tối đa 3 handle LRU, rows 200 / page, query 1000 hàng, guard read-only, sửa / restore prefs có original + confirm + force-stop + đọc lại). Chỉ đọc DB như TS.
6. `ipc_ext.rs` đăng ký 25 kênh (cùng validate với TS); `ipc.rs` gọi `ipc_ext::register`.
7. TS: `devices/ipc.ts` rec qua `deskHandle` + host `devices.recSave`; `filesIpc.ts` / `perfIpc.ts` / `appDataIpc.ts` nhận `desk` và đi qua `deskHandle`; `deskHosts.ts` standing thêm `devices.recDir|recKeeps` + `devices.confirm` kind mới; `main.ts` thêm `devicesConfirm`, `rustRecHeld`, xử lý event `ccm:dev-rec-held`, truyền `desk` cho 3 module.

## File

- Mới: `apps/core/crates/core-desk/src/devices/{adb_io,recorder,devfs,files,perf,appdata,prefs,sqlguard,sqlwal,ipc_ext}.rs`; `apps/desktop/test/desk-devices-ext.twin.test.ts`.
- Sửa: `core-desk/Cargo.toml` (+`rusqlite` 0.41 `bundled`, `serialize`), `Cargo.lock`, `devices/{mod,ipc,manager}.rs` (`ready_check` / `notify` pub), `capture/mod.rs` (`held`), `apps/desktop/src/devices/{ipc,filesIpc,perfIpc,appDataIpc,deskHosts}.ts`, `main.ts`, `.code_index/{core,desktop,_meta}.md`, plan §4 dòng D5.

## Số test thật

- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 790 passed, 1 failed. Cái fail là `layout::project_model::tests::loose_kit_paths_stay_inside_the_kit` (file của D6, không đụng). core-desk lib 295 passed + 1 fail đó; mới của D5b: `recorder` 2, `devfs` 2, `perf` 2, `sqlguard` 1, `sqlwal` 1, `prefs` 1, `appdata` 1.
- `cargo clippy -j 4 --all-targets -- -D warnings`: file D5b sạch; còn 3 lỗi ở `layout/{gradle_graph,project_model,source_model}.rs` (D6, ghi nhận, không sửa).
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 161 file passed (+1 skipped), 1676 passed, 29 skipped, 0 failed. Twin mới `desk-devices-ext.twin.test.ts`: 11 test pass, kết thúc `twin.logs` / `twin.fellBack` rỗng.
- `npm run typecheck`: 0 lỗi.
- Twin dùng adb giả (script sh trong `sdk/platform-tools`, `LC_ALL=C` cho `ls`), DB tạo bằng sql.js trong thư mục tạm, data dir tạm; không chạm adb server, thiết bị, `~/.android`. Thư mục tạm xoá cuối test. Không build release, không E2E.

## E2E cần chạy (chưa chạy theo yêu cầu gói)

Build trước: `node apps/desktop/build.mjs` + `cargo build --release -j 4 -p core-bin`. Instance cô lập `CCM_E2E_PORT=4521 CCM_E2E_DBG=9521`:

- `CCM_E2E_PORT=4521 CCM_E2E_DBG=9521 CCM_DESK_DEVICES=rust node apps/desktop/e2e/devices.cjs` (rec / devfs / perf / appdata trong Devices tab; dùng `CCM_E2E_RECORD_FILE`, `CCM_E2E_PULL_DIR`, `CCM_E2E_PUSH_FILE` như bản TS).
- Chạy thêm 1 lượt với `CCM_DESK_DEVICES=node`.

## Callback host mới (vỏ Tauri làm theo)

- Standing: `devices.recDir` `{}` -> đường dẫn thư mục clip (= `CaptureFolder.recordingsDir()`, fallback `<tmp>/orrery-rec`); `devices.recKeeps` `{file}` -> bool (= `keeps`); `devices.confirm` thêm kind `fs-delete` `{name, where, dir}`, `fs-replace` `{name, where}`, `perf-cold` `{pkg}`, `pref-write` `{pkg, file, key, before, after}`, `pref-restore` `{pkg, file}` -> bool.
- Theo request: `devices.recSave` `{defaultName}` -> đường dẫn | null (dialog Save); `devfs.pickFolder` -> đường dẫn | null; `devfs.pickFiles` -> mảng đường dẫn | null; `devfs.progress` `{transfer, name, total, index, count, done}` (gửi tới cửa sổ gọi); `devfs.open` `{local, mode: 'open'|'reveal'}` -> bool; dùng lại `gate.dir`.
- Event từ Rust: `ccm:dev-rec-held` (mảng đường dẫn clip đang giữ token), vỏ nhớ lại để `capture.held` của TS / retention không xoá.

## Khác biệt còn lại

- Text lỗi SQLite lấy từ `rusqlite` (message gốc của `SQLITE_*`), có thể khác sql.js ở phiên bản SQLite (vd `EXPLAIN QUERY PLAN`); test chỉ so các câu lệnh có message ổn định. Số nguyên > 2^53 trong ô: Rust giữ đủ i64, sql.js làm tròn thành số JS.
- `apply_wal` chỉ có unit test (không có twin WAL thật; fixture dùng DB không WAL).
- `record-failed` / `write-failed` detail khi lỗi OS (mkdir / copy) dùng text lỗi của Rust thay vì mã `ENOENT…` của Node.
- `devfs-progress` gửi qua host round trip (mỗi 120ms), không phải event broadcast.
- JS `trim()` vs Rust `trim()` khác ở vài ký tự khoảng trắng hiếm (BOM) trong `clean_pref_value`.
- Chưa chạy trên Windows thật (`cfg`: test skip trên win32); macOS chạy thật.

STATUS: DONE
