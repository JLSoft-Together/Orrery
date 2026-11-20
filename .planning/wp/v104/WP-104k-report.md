# WP-104k: Bảng Tauri vs Electron + E2E mặc định Tauri

Trạng thái: xong phần code + bảng. E2E Tauri chưa 0 FAIL: còn 18 FAIL, đều có sẵn từ trước (không do Tauri); khoảng trống Tauri thật đã gắn cờ SKIP / ELECTRON_ONLY và ghi vào bảng. Không xoá Electron.

## Thay đổi
- `apps/desktop/e2e/cdp.cjs`: `SHELL` mặc định Tauri; Electron khi `--shell=electron` hoặc `CCM_E2E_SHELL=electron`; `launch` bỏ `exe` dạng `--flag` (nhiều file truyền `process.argv[2]` làm exe); `opts.exe` là exe Tauri khi ở chế độ Tauri; thêm `secondInstance(link, exe)` (Electron: spawn lần 2; Tauri: trả `null`).
- `apps/desktop/e2e/run.cjs`: ghi rõ build `apps/shell` debug trước; `ELECTRON_ONLY` = v74, v75, v76, v76b, v77a tự chạy với `CCM_E2E_SHELL=electron`.
- `desktop-only.cjs`, `v49-open.cjs`: bước "link qua instance thứ 2" bỏ trên Tauri (in `SKIP`). `v73.cjs`: bước "shell marks" bỏ trên Tauri (in `SKIP`).
- `.planning/AUDIT-v104-tauri-parity.md` (44 dòng, không còn "chưa rõ"), `.code_index/desktop.md` (mục E2E harness), `.code_index/_meta.md` (1 dòng), `CLAUDE.md` (dòng `npm run e2e -w @ccm/desktop`).

## Build
`npm run build -w @ccm/web`, `node scripts/gen-desk-shim.mjs`, `cd apps/shell && cargo build -j 4`. Bản `orrery-shell.exe` debug đang bị 1 instance dev của chủ nhân (pid 50760) khoá nên link lỗi `Access is denied`: đã đổi tên file bị khoá thành `target/debug/orrery-shell.locked-<ts>.exe` (Windows cho đổi tên exe đang chạy) rồi build lại. File `.locked-*.exe` này xoá được khi instance đó tắt.

## Kết quả E2E Tauri (cổng 4501 / 9501 lượt đầy đủ, 4502 / 9502 lượt chạy lại), pass / fail

Lượt đầy đủ `run.cjs` (trước khi sửa `desktop-only`, `v49-open`, `v73`): 24 FAIL. Sau chạy lại riêng + chuyển ELECTRON_ONLY: 18 FAIL, toàn bộ có sẵn.

| File | Pass | Fail | Ghi chú |
|---|---|---|---|
| p2 | 14 | 0 | |
| p3 | 15 | 0 | |
| p4 | 11 | 0 | |
| p5 | 6 | 1 | `reopen button on external session`: có sẵn (danh sách của chủ nhân) |
| p6 | 12 | 0 | |
| csp | 4 | 0 | |
| desktop-only | 5 | 0 | bước link instance thứ 2 SKIP (xem dưới) |
| split | 24 | 0 | |
| browser | 20 | 0 | |
| tabs-cwd | 17 | 0 | |
| tier2 | 28 | 0 | classic |
| explorer | 30 | 0 | |
| v25 | 27 | 1 | `main agent node on the map`: có sẵn |
| minimap | 5 | 0 | |
| v27 | 17 | 0 | |
| extensions | 14 | 0 | classic |
| diff | 30 | 0 | |
| log | 27 | 0 | |
| sidebar | 0 | 0 | tự SKIP (sidebar classic đã thay bằng rail) |
| rail | 19 | 0 | |
| explorer-menu | 11 | 0 | |
| split-diff | 29 | 0 | classic |
| flutter-preview | 21 | 0 | |
| run-pane | 64 | 0 | |
| drawable-preview | 6 | 0 | |
| file-editor | 22 | 0 | |
| loose-kit | 8 | 0 | |
| layout-speed | 11 | 0 | |
| agent-map | 1 | 3 | có sẵn |
| pane-drag | 13 | 0 | |
| project-panes | 23 | 0 | |
| v49-open | 31 | 0 | 2 check deeplink SKIP |
| v50-groups | 27 | 0 | |
| v51-compact | 16 | 0 | |
| v52-polish | 34 | 0 | |
| push | 14 | 0 | classic |
| links | 22 | 0 | |
| devices | 8 | 0 | |
| v67-ide | 5 | 4 | có sẵn |
| v69-ide | 22 | 0 | |
| v70-tools | 43 | 0 | classic |
| whats-new | 20 | 0 | |
| v72 | 41 | 0 | |
| v73 | 37 | 0 | bước "shell marks" SKIP |
| v74 | 0 | 1 | `no main inspector`: ELECTRON_ONLY, Electron 69 pass 0 fail |
| v75 | 0 | 1 | như v74; Electron 90 pass 0 fail |
| v76 | 67 | 2 | ELECTRON_ONLY; Electron 83 pass 0 fail (Tauri: `block next time` cần inspector, `keep awake` thiếu thật) |
| v76b | 0 | 2 | ELECTRON_ONLY; Electron 34 pass 0 fail |
| v77a | 0 | 1 | ELECTRON_ONLY; Electron 2 pass 4 fail, 4 fail đó có sẵn (danh sách của chủ nhân) |
| v77b | 25 | 0 | |
| v78 | 16 | 1 | `Shift+Del forgets a closed project`: fail y hệt trên Electron, có sẵn, không do Tauri |
| v80-mcp | 23 | 0 | |
| v80-toasts | 8 | 0 | |
| agent-api | 20 | 0 | |
| shell-agent | 3 | 2 | có sẵn |
| v79-shots | 44 | 2 | Cần `docs/qa/v79/` (đã bị xoá cùng `docs/qa/**`: tạo tạm, chạy, xoá lại). 2 fail `cards counted + heights` và `narrow sheet is a bottom sheet`: y hệt trên Electron, có sẵn |

Tổng FAIL còn lại 18: danh sách chủ nhân đã nêu 15 (p5 1, shell-agent 2, agent-map 3, v67-ide 4, v77a 4 chạy trên Electron, v25 1) + 3 có sẵn chưa có trong danh sách (v78 1, v79-shots 2; cả hai fail y hệt trên Electron).

Chưa chạy lại toàn bộ `run.cjs` lần thứ hai sau khi sửa (mỗi lượt 35 phút). Các file đã sửa (`desktop-only`, `v49-open`, `v73`) và `v76`, `v78`, `v79-shots` đã chạy lại riêng như bảng.

## File / bước chỉ chạy được trên Electron
| File / bước | Lý do | Cách |
|---|---|---|
| v74, v75, v76, v76b, v77a | patch `dialog.showMessageBox` bằng Node inspector của main process Electron; Tauri không có main Node | `ELECTRON_ONLY` trong `run.cjs` |
| v76 bước `keep awake` | `apps/shell/src` không gọi `desk:awake-busy` / `-power` / `-dispose`: keep awake không giữ máy trên Tauri (khoảng trống thật, bảng dòng 43) | chạy cả v76 trên Electron |
| desktop-only, v49-open (link qua instance thứ 2) | `config::e2e_isolated` tắt single-instance trong E2E, instance thứ 2 không chuyển link được (khi bản dev khác đang chạy cùng identifier, link tới nhầm bản đó) | bỏ bước trên Tauri, in `SKIP` |
| v73 bước `shell marks` | Tauri chưa có shell integration OSC 633 | bỏ bước trên Tauri, in `SKIP` |

## Lưu ý quan trọng
- Lúc thử `secondInstance` lần đầu (trước khi phát hiện `e2e_isolated`), con đã spawn `orrery-shell` thứ 2 không có env E2E: single-instance chuyển các link `agent-monitor://open?agent=shell&cwd=<repo>/docs` và `focus?k=wrong` vào bản dev đang chạy của chủ nhân (pid 50760), có thể đã mở thêm vài pane shell ở `docs` trong bản dev đó. Đã sửa script để Tauri không spawn nữa. Chủ nhân đóng các pane đó nếu thấy.
- Gap mới tìm thấy (không sửa, ngoài phạm vi): keep awake chưa nối trên Tauri (bảng dòng 43); toast OS + toast menu v100 chưa có (dòng 33, 34).

## Test macOS (main chép vào `.planning/QA-mac-checklist.md`, mục "Tauri (WP-104k)")
- [ ] `npm run desktop` (hoặc bản dist Tauri) chạy; cửa sổ chính, đèn đỏ / vàng / xanh, tray, đóng cửa sổ thì ẩn vào tray.
- [ ] Terminal: Claude, Codex, Shell mở được, resize, `cd` đổi nhánh pane, đóng pane, `--resume`.
- [ ] Explorer, Git Log, Changes, mở file, kéo thả file từ Finder vào pane.
- [ ] Deep link `agent-monitor://open?agent=shell&cwd=<thư mục>` mở đúng pane; `agent-monitor://focus` đưa app lên.
- [ ] Chụp ảnh child webview (Browser pane) và hiệu ứng dust (`takeSnapshot`, chưa từng biên dịch chạy trên Mac).
- [ ] Keychain: token GitHub / Telegram từ bản Electron cũ đọc được (`Orrery Safe Storage`).
- [ ] Không có (đã ghi thiếu, đừng báo lỗi): menu app macOS, PATH login shell, "Move to Applications", dock badge, toast OS khi Orrery ở nền, keep awake.
- [ ] Cập nhật: `hdiutil` + thay app (`update_host.rs` `mac_script`).
