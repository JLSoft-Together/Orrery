WP-18: ĐẠT với diskmap + diskmove. Còn 4 nhóm Disk chưa port: diskclean, organize, diskapps, diskram.

## Nguyên nhân gốc
Bản Tauri chưa từng có handler cho kênh v101. `apps/shell/src/ccm.rs` `dispatch` không khớp `ccm:diskmap-*`, nên rơi xuống `desk::dispatch` (`apps/shell/src/desk.rs:114`). Ở đó `core_desk` `Caller::call` (`apps/core/crates/core-desk/src/lib.rs`) không có kênh, trả `UnknownChannel`, và `desk.rs:95` `error_text` đổi thành `not ported: ccm:diskmap-scan`. Cả `core-desk` lẫn `shell/src` không có kênh `diskmap` nào. Tìm `rg diskmap --type rust` trước khi sửa: 0 file.
- Trang gọi `diskMapScan` thì bị reject, không có scan và không có event `ccm:diskmap-done`, nên E2E chờ "scan done" tới timeout.
- Lỗi không ghi ra `electron.err`, vì lỗi trả thẳng về renderer.
- Electron chạy được vì dùng `apps/desktop/src/diskMap/*` (TS, sidecar `orrery-core disk`).
- Các nhóm `diskclean-*`, `diskmove-*`, `organize-*`, `diskapps-*`, `diskram-*` hỏng cùng nguyên nhân.
- Binary không thiếu: `orrery-core.exe` đã có trong bundle.

## Sửa (apps/shell)
- `disk_host.rs`: chạy sidecar `orrery-core disk` (đường dẫn core như PTY), giao tiếp NDJSON giống `host.ts`, boot max 10s, call max 120s, có `stop()`. Env `CCM_DATA_DIR`. Helper MFT/UAC vẫn là `orrery-core`.
- `disk_map.rs`: 19 kênh `ccm:diskmap-*`: volumes, status, scan, pause, resume, cancel, children, search, delete, reveal, history, fastscan, sizes, dupes, dupes-cancel, snap-list, snap-compare, schedule-get, schedule-set.
  - Sanitize request và reply giống `map.ts`.
  - History dùng chung `disk/history.jsonl`.
  - Xoá theo `delete.ts`: guard, `known`, dialog native (`hosts::confirm` + `disk_text.rs`), resolve lại sau confirm, trash bằng crate `trash`, xoá hẳn có dialog lần 2 và không follow link.
  - Event `ccm:diskmap-progress|done|dupes-*` gửi qua `desk::emit_app`.
- `disk_guard.rs`: port `guard.ts`. Phần lexical: root, system, protected, 8.3, `C$`. Phần canonical: `canonicalize`, bỏ tiền tố `\\?\`.
- `disk_snap.rs`: snapshot gzip, cùng tên file và format với Electron, giữ 30 bản, so sánh có cache 60s. Scheduler tick 60s: idle từ `host_extra::idle_seconds`, kiểm pin bằng `core_desk::power`, có pause/resume, preempt khi user scan. Event `ccm:diskmap-schedule` / `-snap-new`.
- `disk_move*.rs` (subagent làm): 10 kênh `ccm:diskmove-*` + progress, journal `disk/moves.jsonl` cùng format. Junction tạo bằng `FSCTL_SET_REPARSE_POINT`. Holders lấy từ `core_procs`, kèm suggest SDK/AVD/gradle/pub/npm/docker. Bảo mật giữ như TS. `cleanKnown` trả None (cần diskClean).
- Nối dây:
  - `ccm.rs`: thêm 2 nhánh trước khi rơi xuống desk.
  - `main.rs`: khai mod + gọi `disk_map::manage` trong setup.
  - `lifecycle::clean`: dừng sidecar.
  - `host_extra::idle_seconds` đổi thành pub.
  - `Cargo.toml`: thêm flate2 (đã có trong lock), core-procs, 2 feature windows-sys.
- E2E auto confirm: `CCM_E2E_DISKMAP_CONFIRM=1` (và fake `CCM_E2E_DISKSNAP_*`, `CCM_E2E_DISKMOVE`) chỉ nhận khi build debug hoặc `config::e2e_isolated()`. Exe thường không bị nới.
- `apps/desktop/e2e/v101-diskmap-ui.cjs`: thêm `waitFor` chờ các dòng của thư mục con. Trước đó test đọc ngay khi `path` đổi, nên trên Tauri fail 1/3 lần vì IPC chậm hơn Electron vài ms.
- Không sửa file web.

## Kiểm
- `cargo test` shell (qua `npm run qa -- --no-e2e`): PASS, 248 test. `qa`: 3/3 pass (cargo shell, vitest related web, typecheck web).
- Tauri debug (`CCM_E2E_SHELL=tauri`):
  - `v101-diskmap-ui.cjs` 30/30, chạy thêm `--ui=next` 30/30.
  - `v101-qa.cjs` 19/19, chạy thêm `--ui=next` 19/19.
  - `v101-sdk-move.cjs` 1/2 (fail ở `diskclean-categories`). Bản bỏ phần diskClean chạy 16/16.
- Electron dev: `v101-diskmap-ui` 30/30, `v101-qa` 19/19.
- Release (`npm run dist:win` ra `Orrery.exe` + NSIS 4.0.5): `v101-qa` 19/19, `v101-diskmap-ui` 30/30 (`CCM_E2E_SHELL_EXE`=exe release).

## Còn thiếu
- `diskclean-*` (7 kênh), `organize-*` (9), `diskapps-*` (5), `diskram-*` (2): cùng nguyên nhân `not ported` nhưng lớn, khoảng 200 KB TS ở `apps/desktop/src/{diskClean,organize,diskApps,diskRam}`. Trên Tauri các tab Clean / Organize / Apps / RAM vẫn trống hoặc báo lỗi.
- MCP `disk_search` / `disk_usage` / `disk_propose_delete` trên Tauri trả `unavailable`: `core-desk` `mcp_desk/deps.rs` `disk()` dùng mặc định, chưa nối vào `disk_map` (port `diskMap/agent.ts`).
- Chưa có báo cáo Telegram sau quét nền (`diskMap/report.ts`).
- macOS: key path chưa chuẩn hoá NFC như TS.
