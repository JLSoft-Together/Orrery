# WP-v87-6-D6b — báo cáo (2026-11-10)

Port model service layout (Gradle model, source model, loose kit, cache đĩa, watch res / code) sang `core-desk` (module `layout`, cùng cờ `CCM_DESK_LAYOUT`, vẫn trong `DESK_NODE_DEFAULT`). Ở rust mode `model.prepare` chạy trong Rust, render dùng model đó; vỏ Tauri không Node render đủ.

## Đã làm

1. **Rust `apps/core/crates/core-desk/src/layout/`** (không comment):
   - `model.rs` `ModelSvc` (port `LayoutModelService`): `resolve` (TTL 10s) / `pick_module` (library mượn app model, `preferred-apps.json`), `entry` (hash = sha1(`<rootKey>\0<module>`)[..16] nên cùng tên file cache `<dataDir>/layoutlib/models/<hash>.json` với TS), `load_disk` (cache đĩa phiên trước), `info`, `pick_app`, `prepare` → `{key, frameworkOnly, builtinLibs, sourceOnly, prebuiltAt?}`; trait `ModelDaemon` (impl cho `LayoutDaemon`, test dùng daemon giả); 1 `Mutex<St>` + `serial` Mutex thay chuỗi promise.
   - `model_load.rs`: `ensure_loaded` (Gradle model, source model prebuilt, source model res-only, loose kit, framework), `load` / `load_source` / `load_kit` (`projectId`, `classesChanged`, event `loading`, `restartSuggested`), `source_for` (TTL, key `source:<hash>:<sha1(payload)[..12]>`), loose kit đọc `<CCM_LAYOUT_LOOSEKIT>/kit.json`.
   - `model_run.rs`: `refresh` (1 job / root, caller sau join), `run_refresh` = `android_env_with` (SDK / JDK / gradlew, issue chặn `sdk-missing|jdk-missing|jdk-old|gradlew-missing|gradlew-not-executable`) + `gradlew_spawn` + đúng lệnh TS `-q --init-script <init> :<module>:ccmProjectModel -Pccm.out=<hash>.next.json -Pccm.variant=<v> [-Pccm.compile=true]`, `JAVA_HOME` / `ANDROID_HOME`, timeout 5 phút (refresh) / 15 phút (compile), `kill_tree_sparing` /GradleDaemon/, đọc output → `parse` → rename sang `<hash>.json`, reload hoặc `project/changed` res + classes khi payload không đổi; `auto_compile` (1 lần / root / phiên, theo setting `layoutAutoCompile`), `after_build`, `on_daemon_state`, `kill_now`.
   - `model_watch.rs`: notify recursive: res nguồn, res generated (watch module dir, lọc file trong res generated), `.kt` / `.java` → `code-changed`; debounce 300 ms (1500 ms khi generated).
   - `model_ipc.rs`: `ccm:layout-model-status|refresh|pick` (gate = explorer + `is_layout_target`, module ≤ 300 ký tự) + `desk:layout-after-build` [root, module, variant].
   - `project_model.rs` (`parse_project_model`, `parse_loose_kit`, `project_model_args`, `Payload` key đúng thứ tự TS), `source_model.rs` (`build_source_model`, `app_r_jar`, `module_classes`, `script_dependencies`, AAR extract 4 thread), `gradle_graph.rs`, `gradle_cache.rs` (`resolve_cached`, `extract_aar` marker `.ccm-ready` = 2 cùng TS), `zip.rs` thêm `extract_zip_filtered`.
   - `ipc.rs`: `Svc.model` tạo cùng `Svc`, `DaemonCfg.on_state` gọi `model.on_daemon_state`, `SvcCfg` thêm `loose_kit_dir` / `init_script` / `model_timing`, env `CCM_LAYOUT_INIT` + `CCM_LAYOUT_LOOSEKIT` + `GRADLE_USER_HOME`; `ipc_render.rs` dùng `svc.model.prepare` (render, drawable, warmup, agent), `auto_compile` chạy thread riêng, `desk:layout-stop` kill cả Gradle; `agent_render` lúc daemon chưa ready giờ chạy `prepare` song song với khởi động JVM như TS (đóng khác biệt D6).
2. **TS**: `layout/ipc.ts` 3 kênh model qua `handle` (deskHandle) + `LayoutIpc.afterBuild` (rust → `desk:layout-after-build`, node → `model.afterBuild`), `layout/deskHosts.ts` bỏ `layout.prepare` / `layout.autoCompile` / dep `model`, `main.ts` env `CCM_LAYOUT_INIT` + `CCM_LAYOUT_LOOSEKIT` và `layoutIpc?.afterBuild(...)`. Logic TS giữ nguyên làm fallback (cờ `node`).
3. **Test**: `core-desk/tests/layout_model.rs` (12), unit trong `project_model` (4), `gradle_graph` (2), `gradle_cache` (+ AAR extract, versions, pom), `tests/layout.rs` sửa theo model nội bộ (loose kit thay cho host `layout.prepare`), `desktop/test/desk-layout.twin.test.ts` (+4 `it`, `twinModel` riêng có kit + init).

## File

- Mới: `core-desk/src/layout/{model,model_load,model_run,model_watch,model_ipc,project_model,source_model,gradle_graph,gradle_cache}.rs`, `core-desk/tests/layout_model.rs`.
- Sửa: `core-desk/src/layout/{mod,ipc,ipc_render,zip}.rs`, `core-desk/tests/layout.rs`, `apps/desktop/src/layout/{ipc,deskHosts}.ts`, `apps/desktop/src/main.ts`, `apps/desktop/test/desk-layout.twin.test.ts`, `.code_index/{core,desktop,_meta}.md`, plan §4 dòng D6. Không sửa `Cargo.toml` / `Cargo.lock` / `lib.rs` (dep có sẵn).

## Số test thật

- `cargo test -j 4 --no-fail-fast` (cả `apps/core`): 791 passed, 0 failed, 6 ignored. `cargo clippy -j 4 --all-targets -- -D warnings`: sạch (cả workspace).
- Twin `desk-layout.twin.test.ts`: 11/11 (7 cũ + 4 mới), `twin.logs` / `fellBack` rỗng cho `twin`, `twinMissing`, `twinModel` (chạy với `CCM_CORE_BIN=.../target/debug/orrery-core`). Twin mới so TS và Rust cùng input: gate (guest, file không phải layout, `/etc/hosts`), library layout + candidates + pick (hợp lệ, `:nope`, module quá dài), refresh qua `gradlew` giả (`ready`, resCount / classpathCount / warnings, themes), render có model (`frameworkOnly: false`, cache hit, refresh đổi payload thì render mới), refresh lỗi (`gradle-failed` + detail), file cache `<hash>.json` + `preferred-apps.json` giống hệt TS từng byte, source model (không Gradle model: `sourceOnly: true`, rồi `prebuiltAt` khi có R.jar + classes), `afterBuild`, chuỗi event `model` / `loading` / `project-changed` bằng nhau.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 1688 passed, 18 skipped, 2 failed, cả 2 ở `desk-telegram.twin.test.ts` (gói D7 đang làm, không đụng). `npm run typecheck`: 0 lỗi (shared, collector, desktop, web).
- Chạy lại với `CCM_DESK_LAYOUT=node`: chưa chạy riêng (twin skip khi `node`; đường node do `layout*.test.ts` cũ phủ, pass trong lượt full).

## E2E cần chạy (chưa chạy: không build release / không E2E / không `gradlew` thật theo brief)

`node apps/desktop/build.mjs` + `cargo build --release -j 4 -p core-bin`, rồi `CCM_E2E_PORT=4541 CCM_E2E_DBG=9541 CCM_DESK_LAYOUT=rust` bộ có kênh layout (`rg "layout-model|layout-render|drawable-render" apps/desktop/e2e`; `loose-kit.cjs`, `layout-speed.cjs`, `drawable-preview.cjs`). Cần JDK ≥ 17, Android SDK, 1 project Android thật có `gradlew`. Kiểm: nút Refresh model chạy Gradle thật và ra `ready`; mở lại app dùng cache `models/<hash>.json` của bản TS (đổi mode không mất cache); render project có custom view sau Refresh; Run pane `running` kéo theo refresh (`desk:layout-after-build`); sửa file res thì preview cập nhật (`project/changed`); sửa `.kt` thì hiện `codeChanged`; thoát app không còn tiến trình Gradle / JVM mồ côi (`pgrep -f ccmProjectModel`, `pgrep -f dev.ccm.layoutdaemon`).

## Callback host mới cho vỏ Tauri

- Không thêm callback host. Bỏ `layout.prepare` và `layout.autoCompile` (model đã chạy trong Rust). Còn `preview.explorerRoot`, `preview.sender`, `layout.urlFor`, `layout.confirm` như D6.
- Cấu hình env desk child thêm: `CCM_LAYOUT_INIT` (đường dẫn `ccm-project-model.gradle`), `CCM_LAYOUT_LOOSEKIT` (thư mục có `kit.json`), `GRADLE_USER_HOME` (nếu có). Vỏ phải gọi channel `desk:layout-after-build` [root, module, variant] khi Run pane Android / Flutter-on-Android `running` (thay `layoutIpc.afterBuild`). `afterRunTarget` (chọn root / module / variant từ run config) còn ở TS `layout/model.ts`; vỏ Tauri cần bản tương đương (hàm nhỏ, ~12 dòng) khi hết Node.

## Khác biệt còn lại

- `afterRunTarget` chưa port (nằm ở phía caller, không phải model). TS `LayoutModelService` giữ làm fallback node; trong rust mode nó ngủ (không ai gọi `prepare`).
- Watch: Rust dùng `notify` recursive + debounce 50 ms riêng, TS dùng `RootWatcher` (có thể lọc đường dẫn khác); trên macOS fsevent báo realpath.
- `prebuiltAt` / `at` lấy mtime ns / 1e6 (f64), TS dùng `mtimeMs`; trùng khi mtime tròn ms, có thể lệch phần lẻ sau dấu phẩy khi không tròn.
- `compare_versions` (so version Gradle cache) thay `localeCompare` bằng so chữ thường rồi chữ hoa; chỉ khác với phần chữ trộn hoa / thường lạ.
- AAR extract chạy 4 thread (TS concurrency 4 trên promise); thứ tự warning `AAR … unreadable` theo thứ tự lib, không theo thứ tự hoàn thành.
- Kill Gradle timeout trên Unix = kill process group (như Run pane Rust); TS `killTreeSparing` trên Unix tương tự. Nhánh Windows (`taskkill`, `gradlew.bat` qua `cmd_shim`) chỉ compile theo `cfg`, chưa chạy.
- `ensure_loaded` / `prepare` giữ `serial` Mutex suốt `build_source_model` (có thể vài giây lần đầu: giải nén AAR), TS cũng nằm trong chuỗi `serial` nên cùng hành vi.
- Chưa chạy `gradlew` thật; test dùng `gradlew` sh giả ghi JSON model.

STATUS: DONE
