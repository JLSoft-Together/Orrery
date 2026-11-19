WP-21: ĐẠT. Tab Organize và Apps của Disk chạy được trên vỏ Tauri: 8 kênh `ccm:organize-*` + event `ccm:organize-changed`, 5 kênh `ccm:diskapps-*`.

## Nguyên nhân gốc
Giống WP18: `apps/shell/src/ccm.rs` `dispatch` không khớp `ccm:organize-*` / `ccm:diskapps-*`, rơi xuống `desk::dispatch`. Ở đó `core-desk` không có kênh nên trả `not ported: <kênh>`. Trên Tauri tab Organize không có nguồn, không xem trước được. Tab Apps trống. Electron chạy được vì dùng `apps/desktop/src/{organize,diskApps}/*` (TS).

Ghi chú: brief nói organize có 9 kênh, nhưng preload và shim chỉ có 8 kênh invoke (state, settings, add-source, remove-source, plan, run, undo, junk) cộng 1 event.

## Sửa (apps/shell)
- `disk_organize_rules.rs`: port `rules.ts`.
  - Bảng đuôi file, tên thư mục en / vi.
  - Lọc rule, patch, tên thư mục cấm (`con`, ký tự lạ, dấu chấm cuối).
  - `classify`, `with_suffix`. Tháng theo giờ máy (`core_desk::telegram::clock::local`).
- `disk_organize_log.rs`: port `log.ts`.
  - Journal `CCM_DATA_DIR/organize/<id>.jsonl` cùng format (`run` / `dir` / `intent` / `move` / `fail` / `back` / `undo`), nên Electron và Tauri undo được run của nhau.
  - Giữ 200 run.
- `disk_organize_fs.rs`: lstat, `touched` = max(mtime, ChangeTime, birth). ChangeTime trên Windows đọc qua `GetFileInformationByHandleEx`.
  - Move không ghi đè. Windows dùng `MoveFileExW` không cờ REPLACE; unix dùng `hard_link` rồi xoá.
  - Khác ổ thì copy với `create_new`, kiểm size, đặt mtime, kiểm nguồn không đổi rồi mới xoá nguồn.
  - Mã lỗi theo kiểu Node (EBUSY / EPERM / EEXIST / EXDEV...).
- `disk_organize.rs`: service.
  - Chỉ chạy plan đã tạo ở main (id ngẫu nhiên, TTL 30 phút, tối đa 5 plan). Từ chối nguồn ngoài danh sách.
  - `checkSource` / `fromOk` / `dirOk` dùng `disk_guard::resolve_deletable`, cộng thêm kiểm keepTree (thư mục app).
  - Thư mục tạo ra mà rỗng thì xoá lại.
  - Auto: watcher `notify` + 1 thread timer (Condvar) thay setTimeout. Có `since`, chờ file ổn định, retry khi file bị khoá, retry khi watcher lỗi.
  - Chọn thư mục: `blocking_pick_folder`.
- `disk_organize_junk.rs`: tìm thư mục rỗng và shortcut hỏng qua `disk.search` của sidecar.
  - Bỏ qua thư mục app / build / ẩn, riêng `%TEMP%` được phép.
  - `.lnk` đọc phần LinkInfo, không dùng COM. Không đọc được target thì không liệt kê.
  - macOS: symlink hỏng. Xoá junk vẫn qua `ccm:diskmap-delete` (WP18).
- `disk_apps_reg.rs`: port `registry.ts` + `uninstall.ts`.
  - Đọc qua `reg.exe export` 3 khoá như Electron (không thêm feature windows-sys), hoặc fixture.
  - Parser `.reg` (UTF-16, hex(2/7/b/4), dword), ẩn bản update / KB / SystemComponent, dedupe.
  - `parse_uninstall`: chặn `& | < > ^`, shell exe, chỉ nhận msiexec `/X{GUID}`.
  - Gỡ: `raw_arg` + DETACHED. Bị chặn quyền thì dùng PowerShell `Start-Process`.
- `disk_apps_left.rs`: port `leftovers.ts` (`app_key`, `publisher_key`, danh sách GENERIC, loại folder dùng chung / thuộc app khác) và `mac.ts` (Info.plist, plutil, mdls, du).
- `disk_apps.rs`: service.
  - Danh sách app và mục "đã gỡ" (`removed`). Size lấy từ index (`disk.sizes`).
  - Gỡ app (Windows): đọc registry lại, xác nhận bằng dialog native, rồi parse lại lệnh gỡ trước khi chạy.
  - Gỡ app (mac): chuyển bundle vào Thùng rác.
  - Xoá file sót: chỉ path đã quét. Qua guard (+ keepTree), `delete_box`, mặc định Thùng rác, xoá hẳn có dialog lần 2. Ghi `disk/history.jsonl`.
  - Settings: mở `ms-settings:appsfeatures`.
- Nối dây (sửa nhỏ):
  - `ccm.rs` thêm 2 nhánh, `main.rs` thêm 8 `mod` + `disk_organize::start`, `lifecycle::clean` thêm `disk_organize::stop`.
  - `disk_map.rs`: `trash_path` / `remove_path` / `measure_rest` đổi thành `pub`, `organize::app_dirs` để `pub`.
  - `Cargo.toml` thêm `regex = "1"` (đã có 1.13.1 trong lock).
- E2E: `CCM_E2E_ORGANIZE_{STABLE_MS,DOWNLOADS,DESKTOP,PICK}` và `CCM_E2E_DISKAPPS_{REG,ROOTS,CONFIRM}` chỉ được nhận ở build debug hoặc `config::e2e_isolated()`.
- `apps/desktop/e2e/v101-organize.cjs`, `v101-apps-ram.cjs`: khi chạy `--ui=next` thì mở trang Settings `disk` thay vì `data`, giống WP18 làm với diskmap-ui. Không có file web nào bị sửa.

## Kiểm
| Lệnh | Kết quả |
|---|---|
| `cargo test -j 18` (apps/shell) | 279 pass, 0 fail (có 14 test mới: rules, journal, plan/run/undo, auto pass, no-overwrite, .lnk, junk filter, .reg, uninstall parse, leftovers, plist/mdls) |
| `CCM_E2E_SHELL=tauri node v101-organize.cjs` | 27/27 |
| `CCM_E2E_SHELL=tauri node v101-organize.cjs --ui=next` | 27/27 |
| `CCM_E2E_SHELL=tauri node v101-apps-ram.cjs` | 26/26 (phần RAM chạy nhờ WP20) |
| `CCM_E2E_SHELL=tauri node v101-apps-ram.cjs --ui=next` | 26/26 |
| `CCM_E2E_SHELL=tauri node v101-qa.cjs` | 19/19 |
| Electron dev `v101-organize.cjs` / `v101-apps-ram.cjs` | 27/27, 26/26 |

Mọi lệnh E2E đều chạy với `CCM_E2E_OUT=…/ccm-e2e-wp21`, `CCM_E2E_PORT=4493`, `CCM_E2E_DBG=9428`.

E2E chỉ đụng dữ liệu fixture:
- Thư mục tạm.
- Registry fixture `k2.reg`.
- "Uninstaller" là `node` chạy file js ghi marker.
- Leftover vào Thùng rác.

Registry thật chỉ được đọc, không ghi.

Không build release.

## Còn thiếu / rủi ro
- `.lnk` không có LinkInfo (shortcut chỉ có IDList, ví dụ trỏ tới item shell) thì không liệt kê. Electron đọc được qua `shell.readShortcutLink`. Chọn bỏ qua là hướng an toàn.
- Chưa chuẩn hoá NFC tên file như TS. Chỉ ảnh hưởng macOS (cùng gap với WP18).
- Bản macOS (`read_mac_apps`, trash bundle, symlink hỏng) mới có unit test parse, chưa chạy trên máy Mac.
- `catch_unwind` trong dispatch organize không có tác dụng ở release (`panic = "abort"`). Lỗi I/O đã trả về giá trị rỗng như TS, không panic.
