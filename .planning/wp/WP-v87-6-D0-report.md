# WP-v87-6-D0 — báo cáo (2026-11-10)

Khung `core-desk` + `orrery-core desk` + forward trong Electron main + twin test; port xong `backup`, `buildClean`, `libs`, mặc định `rust`.

## Đã làm

1. **Crate `apps/core/crates/core-desk`** (lib, không dính Electron / Tauri):
   - `Desk::dispatch(channel, args)` / `dispatch_with(&Ctx, …)`, đăng ký theo module `desk.on(module, channel, handler)`, `Desk::standard(&DeskEnv)` gọi `backup::register`, `build_clean::register`, `libs::register`.
   - `Ctx::emit(event, payload)` (sink event) và `Ctx::host(method, params)` (gọi ngược về vỏ cho việc chỉ vỏ làm được: dialog, localStorage trang, confirm có text i18n, thùng rác, dữ liệu main). Trait `DeskIo` → Tauri cài trực tiếp trong process, Electron qua NDJSON.
   - Helper dùng lại cho D1–D9: `js.rs` (độ dài / cắt UTF-16, thứ tự key như JS, số in như JS, `JSON.stringify(v, null, n)`), `util.rs` (ISO time, ngày local, uuid, ghi atomic, `path.resolve` / `isInsideOrSame`, `fmtBytes` + `toFixed(1)`), `fs_tree.rs`, `git.rs`.
2. **`orrery-core desk`** (core-bin): NDJSON `{id, channel, args}` → `{id, ok, value|error}`, event `{event, payload}`, gọi ngược `{call, for, method, params}` ↔ `{reply, ok, value|error}`, dòng đầu `{ready, pid, channels}`. Đọc env `CCM_DATA_DIR`, `CCM_APP_VERSION`, `CCM_DOCUMENTS_DIR`, `CCM_USER_DATA`, `CCM_LIBS_REGISTRY` / `CCM_OSV_URL` (chỉ loopback). Không mở cổng mạng. Stdin EOF / `shutdown` → exit 0.
3. **Desktop**: `src/desk/host.ts` `DeskHost` (spawn, chờ `ready` timeout 10 s, restart max 3 — reset sau 60 s ổn định, quá max / thiếu binary / không boot → `dead`, kill sau 2 s khi thoát, child tự thoát khi stdin đóng nên không mồ côi). `src/desk/forward.ts` `DeskRouter.route(...)`: kênh chạy Rust khi `CCM_DESK_<MODULE>` ≠ `node`; frame lạ / cờ `node` → handler TS; `DeskUnavailable` → chạy handler TS lượt đó, desk `dead` → fallback cả phiên + Notification 1 lần (`coreFallbackTitle` + string mới `deskFallbackBody`). Diagnostics thêm `desk: backup|buildClean|libs rust|node`, `desk.log` vào báo cáo lỗi.
4. **Port 1:1** (đọc TS + test hiện có): `backup` (`format.ts` + `BackupService` + 4 kênh, thêm `desk:backup-apply-pending` / `desk:backup-write-auto` cho Tauri / D9), `buildClean` (shared `buildClean.ts` + `scan.ts` + `service.ts`, event `ccm:build-event`), `libs` (shared `libVersions.ts` + `registry.ts` + `osv.ts` + `checker.ts`, cache `lib-versions-cache.json` cùng format với TS nên đổi mode không mất cache). Mặc định `rust` cả 3.
5. **Twin test** TS ↔ Rust qua `orrery-core desk` thật: cùng input qua đúng `ipcMain` handler (một bên không desk, một bên có `DeskRouter`) → so output, file ghi ra, event cuối, cache; còn kiểm router không phải fallback (`logs` / `fellBack` rỗng).

## File

- Mới: `apps/core/crates/core-desk/**` (`Cargo.toml`, `src/{lib,js,util,fs_tree,git,stdio}.rs`, `src/backup/{mod,format,service}.rs`, `src/build_clean/{mod,rules,scan,service}.rs`, `src/libs/{mod,versions,registry,osv,checker,http}.rs`, `tests/desk_modules.rs`), `apps/desktop/src/desk/{host,forward}.ts`, `apps/desktop/test/{desk-twin.ts,desk-host.test.ts,desk-backup.twin.test.ts,desk-build-clean.twin.test.ts,desk-libs.twin.test.ts}`.
- Sửa: `apps/core/Cargo.toml` (member), `Cargo.lock` (regex, ureq 3.5.1 + rustls / ring / platform-verifier, getrandom 0.4), `crates/core-bin/{Cargo.toml,src/main.rs}` (arm `desk`), `apps/desktop/src/{backup,buildClean,libs}/ipc.ts`, `main.ts` (`setupDesk`, deps cho buildClean tách ra để dùng chung TS + host, `deskHost?.stop()` lúc thoát, Diagnostics, `desk.log`), `strings.ts` (`deskFallbackBody` en/vi), `.code_index/{core,desktop,_meta}.md`, plan §4 ghi chú D0.
- Không đụng: `apps/collector/src`, `packages/shared`, `apps/web`, `core-http`, file của T0.

## Số test thật

- `cargo test --locked -j 4` (cả workspace core): 483 passed, 0 failed (core-desk: 12 unit + 4 integration). `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 136 file, 1565 passed, 18 skipped, 0 failed. Mới: `desk-host` 9, `desk-backup.twin` 6, `desk-build-clean.twin` 5, `desk-libs.twin` 3 (chạy lại với binary release: 23/23). Typecheck desktop: 0 lỗi.

## E2E (instance cô lập `cdp.cjs`, `CCM_E2E_PORT=4471`, `CCM_E2E_DBG=9471`, out `%TEMP%\ccm-e2e-d0`; build `node apps/desktop/build.mjs` + `cargo build --release -p core-bin`)

| Bộ | Kênh | Kết quả |
|---|---|---|
| `v74` (backup export / import, libs check) | `ccm:backup-*`, `ccm:libs-check` | 69 PASS / 0 FAIL |
| `v75` (OSV advisories) | `ccm:libs-vulns` | 90 PASS / 0 FAIL |
| `v77a` (build clean) | `ccm:build-*` | 17 PASS / 0 FAIL |
| `v77a` với `CCM_DESK_*=node` (đường TS) | | 17 PASS / 0 FAIL, không spawn desk |

`desk.log` mỗi lượt: `orrery-core desk ready`, không dòng fallback. Sau khi app thoát không còn `orrery-core.exe` nào.

## Mẫu port 1 module cho D1–D9 (≤ 10 dòng)

1. `apps/core/crates/core-desk/src/<module>/` port 1:1 file TS (dùng `js::` cho độ dài / JSON / thứ tự key, `util::` cho path / thời gian), `pub fn register(desk, env)` gọi `desk.on("<module>", "ccm:<kênh>", |ctx, args| …)`; thêm vào `Desk::standard`.
2. Việc chỉ vỏ làm được → `ctx.host("<module>.<việc>", json!({...}))`, gửi dữ liệu thô (không gửi câu i18n); event → `ctx.emit("ccm:<event>", payload)` và thêm tên vào `DESK_EVENTS`.
3. TS `<module>/ipc.ts`: `const handle = deskHandle(ipcMain, isAppFrame, desk ? { router, module: '<module>', hosts: (event) => <module>Hosts(deps, event) } : undefined)`, đổi `ipcMain.handle` → `handle`; thêm tên vào `DESK_MODULES`.
4. `main.ts`: truyền `desk: deskRouter ?? undefined` (+ deps của hosts).
5. Twin: copy `test/desk-libs.twin.test.ts` (`startTwin({ CCM_DATA_DIR })`, 2 `fakeIpc`, gọi cùng input, `swapRoot` cho path, chuẩn hoá thời gian / token, cuối cùng `expect(twin.logs).toEqual([])`).
6. Rust unit test cho phần thuần + `cargo clippy -D warnings`; E2E bộ có kênh đó.

## Rủi ro / khác biệt còn lại

- **Proxy**: Electron `net.fetch` đi theo proxy hệ thống (WinINET / PAC); `ureq` chỉ đọc biến môi trường proxy. Máy sau proxy công ty có thể check thư viện lỗi `network` ở Rust trong khi TS chạy được → cách lui: `CCM_DESK_LIBS=node`. Cần xử lý trước khi bỏ Electron (D7 / D9 cũng gặp).
- **Kích thước**: `orrery-core.exe` release 5,0 → 8,1 MB (rustls + ring + regex + platform verifier). Nếu cần gọn: `native-tls`.
- Khác biệt biên đã biết (không gặp ở dữ liệu thật): `JSON.parse` chuỗi có surrogate lẻ (serde từ chối), `\s` Unicode của JS vs Rust (U+FEFF), `pruneList` dùng so byte thay `localeCompare` khi trùng mtime, confirm host lỗi → coi như huỷ (TS sẽ reject).
- Boot `applyPending` và `writeAuto` (update) vẫn chạy TS ở Electron.
- macOS: code theo `cfg` (`libc::localtime_r`, `git` PATH thêm `/usr/bin` …), chưa compile / chạy trên Mac.

STATUS: DONE
