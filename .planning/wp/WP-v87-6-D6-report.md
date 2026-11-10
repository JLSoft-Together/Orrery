# WP-v87-6-D6 — báo cáo (2026-11-10)

Port phần chạy daemon của `apps/desktop/src/layout/` sang `core-desk` (module `layout`, mặc định `node`, bật bằng `CCM_DESK_LAYOUT=rust`). Model service Gradle còn TS (PARTIAL).

## Đã làm

1. **`apps/core/crates/core-desk/src/layout/`** (không comment):
   - `rpc.rs` frame `Content-Length` + `RpcClient` (pending map, timeout), `jdk.rs` (settings → JAVA_HOME → Android Studio JBR → PATH, ≥ 17, dùng lại `run::android::env`), `cds.rs` (AppCDS record → snapshot → dump → `.jsa`, marker `.nocds`), `proc.rs` (args JVM, spawn, `kill_pid` cả cây, dump), `daemon.rs` `LayoutDaemon` (khởi động lười, deadline `initialize` 60s, slot cancel, deadline render 15s → kill, crash → restart max 3 / 5 phút, idle 10 phút, `request_restart` chờ hết in-flight, `stop` shutdown + EOF + kill sau 3s, `kill_now`), `download.rs` `Bundle` (Fetcher trait + `UreqFetcher` không follow redirect, allow-list `dl.google.com` / github, size + SHA-256 pin, `.partial-*`, kit + prune), `zip.rs` (stored + deflate, chặn zip slip / zip64 / max 600 MB), `cache.rs` (LRU 32 + `render_key` sha1), `xml.rs` (cleaner `device` / `scale` / `tree` / `diagnostics` / `swatches`, `drawable_kind`, `is_layout_target`), `ipc.rs` + `ipc_render.rs` (handler).
   - Kênh: `ccm:layout-status|download|render|warmup`, `ccm:drawable-render`, desk-only `desk:layout-request|restart|stop|kill|agent-render`.
2. **TS**: `layout/deskHosts.ts` (host per request), `layout/ipc.ts` (`deskHandle` cho 5 kênh; `modelDaemon` đưa `project/load` của model TS sang daemon Rust; `onDeskEvent`; `agentRender` / `stop` / `killNow` qua desk), `wire.ts` (`desk`), `main.ts` (`layoutDeskEnv()` env `CCM_LAYOUT_{JAR,OVERRIDE,KIT,LOG}`, forward `ccm:layout-event` cho model, `desk: deskRouter`), `desk/forward.ts` (module `layout` trong `DESK_MODULES` + `DESK_NODE_DEFAULT`, event `ccm:layout-event` trong `DESK_EVENTS`).
3. Test + fixture: `core-desk/tests/layout.rs`, `desktop/test/desk-layout.twin.test.ts`, `desktop/test/fixtures/fake-layout-daemon.cjs` (daemon giả JSON-RPC stdio: init, render, drawable, project/load, cancel, shutdown; XML chứa `SLOW` / `ERR` / `CRASH` / `HANG` kích hoạt lỗi).

## File

- Mới: `apps/core/crates/core-desk/src/layout/{mod,rpc,jdk,cds,proc,daemon,download,zip,cache,xml,ipc,ipc_render}.rs`, `apps/core/crates/core-desk/tests/layout.rs`, `apps/desktop/src/layout/deskHosts.ts`, `apps/desktop/test/{desk-layout.twin.test.ts,fixtures/fake-layout-daemon.cjs}`.
- Sửa nhỏ: `core-desk/src/lib.rs` (mod + register), `apps/desktop/src/{layout/ipc.ts,layout/wire.ts,desk/forward.ts,main.ts}`, `.code_index/{core,desktop,_meta}.md`, plan §4 dòng D6. Không sửa `Cargo.toml` / `Cargo.lock` (dep đã có).

## Số test thật

- `cargo test -j 4 --no-fail-fast` (cả `apps/core`): 744 passed, 0 failed, 6 ignored (core-desk lib 261 gồm 14 test `layout::*`; `tests/layout.rs` 6). `cargo clippy -j 4 --all-targets -- -D warnings`: sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 1660 passed, 29 skipped, 1 failed lượt đầu là `run-gaps.test.ts` (flaky, của gói run; chạy riêng 15/15 pass). Twin mới `desk-layout.twin.test.ts` 7/7, `twin.logs` / `fellBack` rỗng (cả twin có bundle và twin thiếu bundle). `npm run typecheck`: 0 lỗi (desktop + web).
- Twin bao: status (có / thiếu bundle, frame lạ), render denied / bad-params / too-large / missing / bundle-missing, render + cache hit + text khác + lỗi daemon, drawable (state lọc), warmup, download (ready + cancelled), `agentRender`, event daemon `starting → jvm → setup → ready` giống TS, model service TS tải loose kit qua daemon Rust (`builtinLibs: true`, event `loading kit`, `model-status`).
- Chạy lại với `CCM_DESK_LAYOUT=node`: twin skip như các module khác (đường node do các `layout*.test.ts` cũ phủ, pass trong lượt full).

## E2E cần chạy (chưa chạy: không build release / không E2E theo brief)

`CCM_E2E_PORT=4541 CCM_E2E_DBG=9541 CCM_DESK_LAYOUT=rust`, sau `node apps/desktop/build.mjs` + `cargo build --release -j 4 -p core-bin`; bộ có kênh layout (`rg "layout-render|drawable-render" apps/desktop/e2e`), cần JDK ≥ 17 + `apps/layout-daemon/build/layoutlib-dist` (hoặc download). Kiểm: render XML thật, drawable, cache hit lần 2, đổi file → render mới, thoát app không còn JVM mồ côi (`pgrep -f dev.ccm.layoutdaemon`).

## Callback host mới (cho vỏ Tauri)

- `preview.explorerRoot` {root} → real path hoặc null (đã có từ D3), `preview.sender` → id người gọi (slot key).
- `layout.prepare` {file} → `{key, frameworkOnly, builtinLibs, sourceOnly, prebuiltAt?}` | `{ok:false}` (model lỗi) | lỗi / null (không có model → Rust dùng framework-only).
- `layout.autoCompile` {file} (không chờ), `layout.urlFor` {file, roots[]} → `{url}` | null (Rust tự thêm `?r=<tag>`), `layout.confirm` {bytes, host} → bool (câu i18n ở vỏ).
- Cấu hình qua env desk child: `CCM_LAYOUT_JAR`, `CCM_LAYOUT_OVERRIDE` (rỗng = dùng download), `CCM_LAYOUT_KIT` (JSON `{version,size,sha256,url}`), `CCM_LAYOUT_LOG`.

## Khác biệt còn lại

- **Model service chưa port** (`model.ts`, `sourceModel.ts`, `gradleCache.ts`, `gradleGraph.ts`, `projectModel.ts`: 3 kênh `ccm:layout-model-status|refresh|pick`): vẫn TS, dùng daemon Rust qua `desk:layout-request`. Vỏ Tauri không Node thì không có model → `layout.prepare` lỗi → render framework-only (không resolve class / res của project). Cần gói riêng (~1.7k dòng TS + gọi Gradle) trước khi bỏ Electron.
- `agentRender` lúc daemon chưa ready: TS chạy `model.prepare` song song với khởi động JVM; Rust chỉ khởi động JVM (host không gọi được ngoài request), `prepare` chạy ở lượt render sau. Chậm hơn một nhịp cho lần gọi MCP đầu.
- `ensure` giữ `start_lock` suốt `initialize` (caller khác chờ lock thay vì chờ chung promise); init lỗi RPC thì kill JVM luôn (TS để JVM sống nhưng `ready` đã reject).
- Rust tạo `cacheDir` trước khi spawn (TS chỉ tạo qua `planCds` / `render`): tránh `spawn-failed` khi JDK không có file `release`.
- Slot key dùng `preview.sender` (cùng ý nghĩa `event.sender.id`); `Math.round(mtimeMs)` ở key AppCDS tính từ ns nên khác key TS 1 lần đầu (archive record lại 1 lần, rồi dùng chung).
- Proxy: `ureq` chỉ đọc env proxy (như D0); kit github + layoutlib Maven sau proxy công ty cần `CCM_DESK_LAYOUT=node`.
- Code `cfg(unix)` chạy thật trên máy này (twin + test Rust dùng `java` giả là shell script); nhánh Windows (`taskkill`, `;` classpath, `creation_flags`) chỉ compile theo `cfg`, chưa chạy.

STATUS: PARTIAL
