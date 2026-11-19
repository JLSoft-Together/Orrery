# PROGRESS 2026-11-15: click menu (v100) + Ổ đĩa (v101)

## Tóm tắt hiện tại

- Nhánh `feat_clean_memory`. Plan: `.planning/PLAN-v100-click-menu.md`, `.planning/PLAN-v101-disk-manager.md`.
- Commit: từng phần sau mỗi WP đạt, không push, hiệu lực cả phiên.
- Build/test: repo cho phép typecheck + vitest + cargo test + E2E (memory "Build + test after code").
- Release (orrery-4e báo, chủ nhân chốt): v100 + v101 + bản sửa Tauri gom thành **4.0.6**, release 1 lần khi xong hết. Khi v100/v101 xong: thêm mục en + vi + dòng "Changelog chờ" vào `.planning/RELEASE-4.0.6.md`; không bump version / `changelog.ts`, không build / cài trước khi chủ nhân bảo. 4.0.6 còn chờ cả v102 (orrery-e6). v100/v101 xong vẫn merge vào `master` trước (COORD) nhưng **merge master cần chủ nhân cho phép rõ** (rule git), hỏi khi tới lúc.
- **Test app chỉ trên bản dev tách biệt** (chủ nhân 15/11), không đụng Orrery đang chạy (cổng 4317, userData `Agent Monitor`, `%LOCALAPPDATA%/ccm`): E2E qua `apps/desktop/e2e/cdp.cjs` (đã tự cô lập `--user-data-dir`, `CCM_PORT`, `CCM_DATA_DIR`, `APPDATA` tạm). Chạy tay: `electron apps/desktop/stage --user-data-dir=<tmp>/ud --remote-debugging-port=<khác>` với env `CCM_PORT=4417 CCM_DATA_DIR=<tmp>/data APPDATA=<tmp>/appdata CCM_TOAST=0`. Không `npm run desktop:dev` (dùng cổng 5173/4317 mặc định). Dev không đăng ký `agent-monitor://` (`registerLinks` chỉ khi packaged). Không kill process Orrery/Electron đang chạy.
- Đợt đóng 15/11 20:44: 23 WP ĐẠT, HEAD 60438421, QA tổng xanh. Chờ chủ nhân: merge master, mft-smoke UAC, Codex 0.169.0. Handoff: `.planning/HANDOFF-2026-11-15-2044-click-disk.md`.

## Bảng WP

| WP | Model | Trạng thái | Vòng | Test | Review | Commit | Ghi chú |
|----|-------|------------|------|------|--------|--------|---------|
| WP-0a | sonnet | ĐẠT | 1 | 27 pass, tsc sạch | main kiểm | 1e84e87d | 3 fixture synthetic (permission, plan, codex approval) chụp thật ở 0c |
| WP-0b | sonnet | GIAO | 1 | | | | thanh nút từ parser màn + công tắc Settings + E2E dev tách biệt |
| WP-1a | opus | ĐẠT | 1 | 17 pass, clippy sạch | sonnet review FFI đang chạy | fcf8e0c3 | bench 2.5M entry: 12-40s, search 4-26ms, RAM 180MB; macOS CHƯA KIỂM |
| WP-0b | sonnet | ĐẠT | 1 | 2177 pass, E2E 15/15 dev | main xem ảnh | 04e390e1 | gửi phím qua conn.write, đọc lại màn trước khi gửi |
| WP-0c | sonnet | ĐẠT | 1 | 99 menu test, E2E 25/25 dev | main xem ảnh | 45e558ee | sự cố: Codex toàn cục bị update 0.162.1 → 0.169.0 lúc chụp, đã báo chủ nhân |
| WP-1a-fix | sonnet | ĐẠT | 1 | 9 unit + 23 integration, clippy sạch | main kiểm | 1e6643c5 | 17/17 mục; phần mac CHƯA KIỂM |
| WP-1d-mft | opus | ĐẠT (chờ UAC thật) | 1 | cargo 63 pass, desktop 37, web 2368, E2E fixture 13/13 | main kiểm | fd87fcbc | chủ nhân chạy `node scripts/mft-smoke.mjs C`; rủi ro: bản cài per-user, binary ghi được trước lúc bấm UAC → việc sau: kiểm hash/chữ ký binary trước khi runas; Tauri shim chưa có diskMapFastScan (luồng Tauri) |
| WP-0d | sonnet | ĐẠT | 1 | 2327 web, E2E 17/17 dev | main kiểm | 6f766885 | main sửa: wsAsked vào dialog phím tắt + i18n en/vi + .code_index/web.md |
| WP-0e | sonnet | ĐẠT | 1 | desktop 20, web 2368, E2E 14/14 dev | main kiểm | ce9a9709 | main sửa: tách hunk main.ts/preload.ts khỏi WP-2a, index notes 9bb22a0d; macOS + bấm toast thật CHƯA KIỂM; 2 test fail ngoài WP (desk-agent-data.twin, collector prompt-search timeout) kiểm cuối đợt |
| WP-1b-host | opus | ĐẠT | 1 | 33 desktop + 19 cargo, E2E 19/19 dev | main kiểm | 34b9fca0 | type `DiskScanLevel` (trùng v77) |
| WP-1b-ui | sonnet | ĐẠT | 1 | 2271 web pass | main xem ảnh | 42f9788d | main sửa: import i18n + entry Settings + type sang @ccm/shared; E2E Electron thật dồn sang WP-1c |
| WP-1c-search | sonnet | ĐẠT | 1 | 2327 web, E2E 30/30 dev | main xem ảnh | 6bb96d9e | E2E bắt lỗi thật: roots Proxy không clone qua IPC; việc nhỏ sau: ô ngày theo locale OS |
| WP-2a-devjunk | opus | ĐẠT | 1 | desktop 70, web 2369, E2E 22/22 dev | review opus đường xoá đang chạy | 59f51156 | kênh xoá riêng `ccm:diskclean-delete`; cache toàn cục đo khi bấm; main sửa: import i18n; v77 xoá được sau (BuildCleanDialog, buildClean/service+ipc, Rust build_clean) |
| WP-2b-sysjunk | opus | GIAO | 1 | | | | rác hệ thống + Dọn 1 chạm; + 6 mục bảo mật diskClean từ review (gửi SendMessage) |
| WP-2b-sysjunk | opus | ĐẠT | 1 | desktop 120, web 2385, E2E 26/26 + 22/22 dev | main kiểm | 1ba491b6 | + 6 mục bảo mật diskClean; main sửa: tách dòng main/preload khỏi WP-3a/3b; **lỗi đã biết macOS**: TMPDIR `/private/var/folders` bị guard trả `system` → sửa guard cho phép đúng TMPDIR hoặc ẩn nhóm Temp trên mac; empty Recycle Bin thật CHƯA KIỂM |
| WP-4a-sdk-move | opus | ĐẠT | 1 | desktop 148, web 2426, E2E 27/27 dev | main kiểm | fd211c67 | main sửa: tách hunk main/preload/index khỏi WP-4b; chưa: lối vào chuyển ổ từ tab Duyệt + menu chuột phải, action `move` trong lịch sử, Flutter engine, simctl runtime; E2E v101-diskclean(-system) chọn tab theo vị trí → hỏng khi thêm tab, sửa ở QA |
| WP-4c-snap | sonnet | GIAO | 1 | | | | snapshot + so sánh + quét nền + gộp cảnh báo v77 + Telegram |
| WP-4c-snap | sonnet | ĐẠT | 1 | desktop 321 / web 449 (gộp), E2E 36/36 dev | main kiểm | e0bb18c5 | commit chung với 4c-mcp; Telegram thật + đường v77 → onAlert thật CHƯA KIỂM |
| WP-QA | sonnet | ĐẠT | 1 | typecheck sạch, web 2464, desktop 1993 (+1 fail có từ trước EPERM symlink), cargo pass, qa 9/9, E2E 17/17 script PASS | main kiểm | 60438421 | `docs/qa/v101/RESULT.md` |
| WP-DOC | sonnet | ĐẠT | 1 | glossary sạch | main kiểm | 2eb6afe0 | CLAUDE.md Since v100/v101 + sixth exception, docs §11as/§11at; RELEASE-4.0.6.md đã thêm mục nhưng file là của orrery-4e (untracked) nên không commit; dòng _meta.md chờ gộp (trong WP-DOC-report) |
| WP-4c-mcp | sonnet | ĐẠT (commit e0bb18c5) | 1 | cargo mcp 57, desktop 239, E2E 26/26 dev | main kiểm | | registry.rs chỉ hunk `desk_reply_ms` (lọc 2 khối orrery-4e); Tauri hosts.rs thiếu `mcpDesk.disk` (luồng Tauri); `desk-mcp.twin` cần rebuild core; `apps/core/target-4c/` là thư mục build tạm, xoá trước commit |
| (cũ) WP-4c-mcp | sonnet | GIAO | 1 | | | | 3 tool MCP disk_search / disk_usage / disk_propose_delete; orrery-4e cho sửa registry.rs trừ 2 khối `reset` + `mod tests` của họ (lọc hunk khi commit) |
| WP-3a-dupes | sonnet | ĐẠT | 1 | cargo pass, web 129, desktop 25, E2E 20/20 dev | main kiểm | 9a613568 | bench F:\work 79 GB hash 104s CPU 3.4%, 1864 nhóm phí 42.8 GB; DiskView tab dupes + organize trong commit này |
| WP-4b-apps-ram | opus | ĐẠT | 1 | desktop 159, web 2427, E2E 26/26 dev | main kiểm | d729fab1 | main sửa: import i18n + bỏ messages.ts tạm; QA sau: mode bar 9 tab bị cắt ở 760px; Tauri shim thiếu diskApps*/diskRam*/diskMapFastScan (luồng Tauri) |
| WP-3b-organize | opus | ĐẠT | 1 | desktop 116, web 389, E2E 27/27 dev | main kiểm | 7d1ea54b | main sửa: import i18n + bỏ mergeLocaleMessage tạm + escape `|` trong message (messages.test bắt); DiskView tab organize commit cùng WP-3a |
| WP-2sec | opus | GIAO | 1 | | | | review xoá: 1 HIGH (Windows dấu `.`/space cuối, 8.3 lọt guard) + autoConfirm env ở bản đóng gói + junction giữa path + thiếu danh sách bảo vệ → ĐẠT 07260d5d (65 test diskMap, E2E host 18/18 + ui 30/30); main sửa: chặn env CCM_E2E_MENU_TOAST_* ở bản đóng gói; macOS CHƯA KIỂM |
| WP-2c-projsize | sonnet | ĐẠT | 1 | web 2378, cargo pass, E2E 16/16 dev | main kiểm | 0514ae7c | op `disk.sizes`; main sửa: tách hunk preload; Cleanable cộng cả mục bị khoá (việc nhỏ sau) |

## Hot file (chủ = main)

`apps/web/src/i18n/en.ts`, `vi.ts`, `apps/desktop/src/preload.ts`, `apps/desktop/src/main.ts`, `apps/web/src/stores/ui.ts`,
`App.vue`, `AppHeader.vue`, `CommandPalette.vue`, `packages/shared/src/index.ts`, `style.css`, `themes.css`.
Lượt 1: `apps/core/Cargo.toml` + `apps/core/crates/core-bin/src/main.rs` giao cho WP-1a.

## Nhật ký

- 15/11 grill xong, lập plan, mở progress, giao lượt 1.

- [x] 2026-11-15 20:44 Handoff mới: `.planning/HANDOFF-2026-11-15-2044-click-disk.md` (đợt v100/v101 đóng, chờ chủ nhân quyết merge master)
