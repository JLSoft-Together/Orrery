# WP-v87-6-D1 — báo cáo (2026-11-10, dừng 21h theo lệnh chủ nhân)

Port D1 sang `core-desk` + binary `orrery-bridge` xong phần code, twin test qua hết. **Chưa chạy E2E** → các module D1 để mặc định `node` (`DESK_NODE_DEFAULT` trong `desk/forward.ts`); bật Rust bằng `CCM_DESK_<MODULE>=rust`.

## Đã làm

1. **Crate mới `apps/core/crates/core-bridge`** → binary `orrery-bridge` (chỉ dùng serde_json): `orrery-bridge statusline-bridge [--chain-b64 …]` và `orrery-bridge hook-bridge`, port 1:1 `scripts/statusline-bridge.mjs` / `hook-bridge.mjs` (số in như JS, `toFixed`, `Math.round`, ISO, chain qua `cmd /d /s /c` / `/bin/sh -c`). Lệnh trong settings.json: `"<dataDir>/bin/orrery-bridge.exe" statusline-bridge` — vẫn chứa mark `statusline-bridge` / `hook-bridge` nên mọi chỗ nhận diện "ours" giữ nguyên.
2. **core-desk, module mới** (đăng ký trong `Desk::standard`):
   - `claude/` (dùng chung): `Sys` (home, `CLAUDE_CONFIG_DIR`), realpath native, `is_within`, `same_dir`, `check_dir`, ghi atomic + `.ccm-backup`, `diff.rs` (`indentOf` / `formatSettings` / `jsonDiff`), `mask.rs` (`maskHookText`, dùng `core_sources::mask::mask_secrets`), `proc.rs` (chạy process có timeout, kill cây: `taskkill /T /F` / `kill(-pgid)`).
   - `claude_hooks/` → `ccm:hooks-list|toggle|test` (sha256 id như TS, stash `hooks-disabled.json`, payload giả, chọn shell như Claude Code).
   - `extensions/` → `ccm:ext-status|mcp|install|remove|source|login`; catalog copy `extensions/catalog.json` (sinh từ `packages/shared` `CATALOG`, test chống lệch); URL chuẩn hoá bằng crate `url`.
   - `bridge/` → `ccm:statusline-state|enable` (module `statusline`), `ccm:hooks-state|set` (module `hooks`), `desk:bridge-install`, `desk:bridge-migrate` (module `bridge`).
   - `layouts.rs` → `ccm:layouts-list|save|delete|start` (giữ cả thứ tự key `id` / `openOnStart` như object JS có slot `undefined`).
   - `restore.rs` → `ccm:run-state|dirs-exist|prompts-export|prompts-import`, thêm `desk:run-boot` / `desk:run-clean` cho Tauri.
   - Phần không phải Electron của `appearance` / `pages` / `editors`: `appearance.rs` (parseLook, zoom, appearance.json; `desk:appearance-read|save|look|zoom`), `pages.rs` + `pages_style.rs` (trang loading / lỗi data: URL, byte-identical với TS; `desk:pages-loading|error`), `editors.rs` (`cleanDirty` + tracker; `desk:editor-dirty|forget|state`).
3. **Desktop (Electron)**:
   - `claudeHooks/ipc.ts`, `extensions/ipc.ts`, `layouts.ts` (store TS tạo lười, đọc file mới khi fallback), `restore/ipc.ts`: `deskHandle` + hosts.
   - `claudeSettings.ts` mới: 4 handler statusline / hooks bridge chuyển từ `main.ts` sang đây (logic giữ nguyên) + hosts.
   - `bridgeRust.ts` mới: `installRustBridge` (copy `orrery-bridge` vào `<dataDir>/bin`, giữ bản cũ nếu exe đang bị khoá), `migrateBridges` (đổi entry `node "<dataDir>/bin/statusline-bridge.mjs"`, `"…/<name>.cmd"`, `"…/<name>.sh"` sang lệnh mới, giữ phần đuôi `--chain-b64 …`, backup `.ccm-backup`, giữ indent / EOL). `main.ts`: khi `CCM_DESK_BRIDGE=rust` và có binary → dùng orrery-bridge, rồi chạy migrate qua router (fallback TS); ngược lại dùng bridge Node như cũ.
   - `desk/forward.ts`: thêm module vào `DESK_MODULES`, thêm `DESK_NODE_DEFAULT`.
   - `build.mjs`: build `-p core-bin -p core-bridge`, stage thêm `orrery-bridge`; `electron-builder.yml`: đóng gói `core/orrery-bridge(.exe)` (win + mac).

## File

- Mới Rust: `apps/core/crates/core-bridge/**`, `apps/core/crates/core-desk/src/{claude/{mod,diff,mask,proc}.rs, claude_hooks/{mod,model,manager,payload}.rs, extensions/{mod,commands,parse,status}.rs, extensions/catalog.json, bridge/{mod,settings,install}.rs, layouts.rs, restore.rs, appearance.rs, pages.rs, pages_style.rs, editors.rs}`.
- Sửa Rust: `apps/core/Cargo.toml` (member `core-bridge`), `core-desk/Cargo.toml` (core-sources, core-transcript, sha2 0.11, url 2), `core-desk/src/lib.rs` (mod + register), `Cargo.lock`.
- Mới TS: `apps/desktop/src/{claudeSettings,bridgeRust}.ts`, `apps/desktop/test/desk-{claude-hooks,extensions,claude-settings,layouts-restore}.twin.test.ts`.
- Sửa TS: `apps/desktop/src/{main.ts, desk/forward.ts, claudeHooks/{ipc,manager}.ts, extensions/ipc.ts, layouts.ts, restore/ipc.ts}`, `apps/desktop/test/desk-twin.ts` (router twin bật `rust` cho module mặc định node), `apps/desktop/{build.mjs,electron-builder.yml}`.
- Không đụng: `apps/shell`, `preview/` (D3), `packages/shared`, `apps/web`, `.code_index/*` (chưa cập nhật — xem việc còn lại).

## Số test thật

- `cargo test -j 4 -p core-desk -p core-bridge`: 40 passed, 0 failed. `cargo clippy -j 4 -p core-desk -p core-bridge --all-targets -- -D warnings`: sạch. `cargo test -j 4` cả workspace: dừng ở `core-http --test cookie` (`the_key_in_the_url_becomes_a_cookie…` 403 ≠ 200) — crate không thuộc D1 (cookie auth của T0/T1, đang sửa song song); các crate chạy trước đó pass. Cần chạy lại `--no-fail-fast` sau khi T1 xong.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 143 file, 1595 passed, 18 skipped, 0 failed (trước khi thêm `DESK_NODE_DEFAULT`); sau khi thêm: `vitest run test/desk-` 11 file / 54 test pass. Typecheck desktop: 0 lỗi.
- Twin mới (đều qua orrery-core desk thật, `twin.logs` / `fellBack` rỗng): claudeHooks 5 (list / toggle off-on / từ chối / test-run thật qua Git Bash + node, so cả file settings, backup, stash, text confirm), extensions 4 (CLI giả = node.exe đổi tên + script `plugin` / `mcp`, so output, argv từng lần gọi, plan confirm; + catalog drift), settings 5 (state / enable / hooks set × 7 kiểu file × đồng ý / huỷ, migrate 5 kiểu file, install, orrery-bridge vs 2 script .mjs: file ghi ra + dòng status + chain), layouts/restore 5 (layouts + limit 20, run-state, dirs-exist, prompts export/import, appearance, pages byte-identical en/vi, editors).

## E2E

Chưa chạy (dừng 21h). Bộ cần chạy với `CCM_E2E_PORT=4491`, `CCM_E2E_DBG=9491`, cờ `CCM_DESK_CLAUDE_HOOKS=rust CCM_DESK_EXTENSIONS=rust CCM_DESK_STATUSLINE=rust CCM_DESK_HOOKS=rust CCM_DESK_LAYOUTS=rust CCM_DESK_RESTORE=rust CCM_DESK_BRIDGE=rust`: `v74` (hooks manager), `extensions`, `tier2` (layouts), `run-pane` / `v77b` (layouts / restore), rồi chạy lại 1 lần với cờ `node`. Lưu ý: `cdp.cjs` không cô lập `CLAUDE_CONFIG_DIR` — đặt `CLAUDE_CONFIG_DIR` tạm khi chạy (migrate chỉ đổi entry trỏ vào `<CCM_DATA_DIR>/bin`, nên với data dir tạm không đụng settings thật, nhưng vẫn nên cô lập).

## Callback host mới (vỏ Tauri phải trả)

- `claudeHooks.confirmWrite` {file, action, event, diff} → bool; `claudeHooks.confirmTest` {event, text, cwd} → bool.
- `extensions.confirm` {kind, commands[], scope, dir} → bool.
- `statusline.command` → string|null (lệnh bridge đã cài); `statusline.confirm` {file, planned, other} → bool.
- `hooks.command` → string|null; `hooks.confirm` {file, planned, on} → bool.
- `restore.saveDialog` / `restore.openDialog` → path|null; `restore.previous` → {unclean} (Tauri: gọi `desk:run-boot` lúc khởi động thì không cần).
- Kênh nội bộ `desk:*` (bridge-install / bridge-migrate / run-boot / run-clean / appearance-* / pages-* / editor-*) chỉ cho vỏ gọi; vỏ Tauri không được cho trang gọi `desk:*`.

## Thuần vỏ, không port

- `appearance`: `ccm:window-bg`, `ccm:locale` (đổi `t`), `ccm:appearance` (setBackgroundColor / titleBarOverlay mọi cửa sổ), `ccm:zoom` (zoom webContents) — vỏ dùng `desk:appearance-*` cho phần file / tính toán.
- `editors`: `ccm:editor-dirty` / `ccm:editor-save-reply` gắn webContents, hỏi trang lưu khi thoát — vỏ giữ, có thể dùng `desk:editor-*` làm sổ.
- `pages`: nạp data: URL vào BrowserWindow, `LEAVE_SCRIPT` — vỏ; HTML lấy từ `desk:pages-*`.

## Khác biệt còn lại / rủi ro

- E2E chưa chạy → mặc định `node` cho 7 module D1 (bridge cũng node: Electron vẫn cài `statusline-bridge.mjs` / `hook-bridge.mjs` như cũ).
- Biên đã biết: regex `\b` / `\s` / `i` Unicode khác JS ở ký tự ngoài ASCII (mask), `localeCompare` thay bằng `core_transcript::collate::locale_cmp`, cắt UTF-16 giữa cặp surrogate, stdout hook test giải mã UTF-8 cả khối (Node giải theo chunk), extensions không giết khi vượt `maxBuffer` 48 MB.
- `orrery-bridge` đọc data dir như script Node (env `CCM_DATA_DIR` hoặc mặc định) — không suy ra từ vị trí exe.
- Exe đang bị Claude chạy (khoá file trên Windows) → bản mới không ghi được, giữ bản cũ.

## Bước tiếp theo

1. `cargo test -j 4 --no-fail-fast` cả workspace + `cargo build --release -j 4 -p core-bin -p core-bridge`, `node apps/desktop/build.mjs`.
2. E2E như mục E2E (rust rồi node), ghi pass/fail.
3. Qua E2E → bỏ module khỏi `DESK_NODE_DEFAULT` (mặc định rust), bỏ phần bật cờ trong `desk-twin.ts` nếu set trống.
4. Cập nhật `.code_index/{core,desktop,_meta}.md` + plan §4 ghi chú D1.

STATUS: PARTIAL
