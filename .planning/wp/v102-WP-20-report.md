WP-20: ĐẠT. diskclean (6 kênh + event `ccm:diskclean-progress`), diskram (2 kênh) và MCP `disk_search` / `disk_usage` / `disk_propose_delete` đã chạy trên Tauri.

## Nguyên nhân gốc
Giống WP18. `ccm.rs` không có nhánh cho `ccm:diskclean-*` / `ccm:diskram-*`, nên kênh rơi xuống `desk::dispatch` và bị reject `not ported`. MCP disk: `core-desk` `mcp_desk/deps.rs` gọi host `mcpDesk.disk`, nhưng `hosts.rs` không có method này, nên luôn trả `unavailable`.

Brief ghi diskclean có "7 kênh". Thực tế Electron `DISKCLEAN_CHANNELS` có 6 kênh: categories, scan, cancel, delete, empty-bin, command. Kênh thứ 7 là event progress.

## Sửa (apps/shell)

### Module mới
- `disk_clean.rs`: port `diskClean/service.ts` + `ipc.ts`.
  - Kiểu `Item` / `Member`, `Ctx` cho provider (junk tính lười, dùng chung cho node_modules và build).
  - scan có huỷ, mỗi lần chỉ chạy 1 scan.
  - `remove`: chỉ xoá path có trong lần scan cuối. Kiểm lock open / run / server / self, `live_lock` (avd / browser / appcache), `system_safe`, member `files`.
  - `precheck` ngay trước khi xoá: lstat đúng kiểu, realpath của thư mục cha không đổi, lock làm mới sau khi xác nhận.
  - One tap: chỉ `temp` / `browser` xoá hẳn, phần còn lại vào Thùng rác.
  - `fold` gộp kết quả giống TS. Có `run_command` (docker / xcrun) và `empty_bin`.
  - `run_delete` có hook, port từ `diskMap/delete.ts`. Guard ctx: `roots` = chính các path, `volumes` rỗng.
  - `AppDeps` lấy:
    - keep: `disk_map::keep_of`.
    - self: thư mục exe, data_dir, repo khi debug.
    - running: run session còn sống + server từ resources report (không tính daemon).
    - index size: dùng sidecar khi đã index.
    - history: ghi chung `disk/history.jsonl`.
- `disk_clean_dev.rs`:
  - Walk junk: marker, `git ls-files` cho build / dist / out / target, bỏ `buildResources` của electron-builder.
  - Project, restore, lastTouched.
  - Provider xcode, cache.
  - SDK: platforms, build-tools, system-images, AVD có lock, Gradle dists, iOS.
  - Docker: probe, `system df`, prune.
  - Helper `run` có timeout. Không dùng regex: `compileSdk` / `buildToolsVersion` / `distributionUrl` đọc tay.
- `disk_clean_sys.rs`:
  - Provider Temp / browser / crash / logs / thumbs / installers / bin / update / appcache.
  - `is_forbidden` giống `system.ts`.
  - File bận: mở read+write, lỗi 32 / 33 / 5 coi là bận.
  - Thùng rác Windows gọi thẳng `SHQueryRecycleBinW` / `SHEmptyRecycleBinW` (crate `windows`, feature đã có sẵn). macOS dùng `.Trash` + osascript. Fake dùng thư mục `Bin`.
- `disk_clean_text.rs`: hộp thoại one tap / bin / command / agent / kill (en + vi, cùng chữ với Electron).
  - `ask_buttons`: 2 nút qua `hosts::confirm`, 3 nút qua `YesNoCancelCustom`.
- `disk_ram.rs`: port `diskRam/sampler.ts` + `ipc.ts`.
  - Đọc process bằng `core_procs::table` + `usage` (working set, cpu ms).
  - Cache snapshot 900 ms. Gắn nhãn self / parent / system.
  - Kill: so `start` trước và sau khi xác nhận, rồi `core_desk::resources::kill_tree`.
  - RAM máy: `GlobalMemoryStatusEx` khai báo tay, không thêm feature vào Cargo.toml.
- `disk_agent.rs`: port `diskMap/agent.ts` (kiểm tham số, ready / not-indexed / scanning, not-in-index, busy, `display_text`).

### Nối dây (sửa nhỏ)
- `ccm.rs`: thêm 2 nhánh.
- `main.rs`: khai 6 mod.
- `lifecycle::clean`: gọi `disk_clean::stop`.
- `hosts.rs`: thêm `"mcpDesk.disk"`.
- `disk_map.rs`: thêm `remove_with(confirm, first_count)` + `is_deleting`. `remove` cũ gọi qua `remove_with`. `trash_path` / `remove_path` / `measure_rest` đổi thành `pub`.
- `disk_move.rs`: `clean_known` đọc `disk_clean::known_plain`.
- `core-desk/src/resources.rs`: `kill_tree` đổi thành `pub`.
- Không sửa Cargo.toml, web hay TS.

### Bảo mật
- Mọi xoá đi qua guard canonical + dialog native, mặc định vào Thùng rác.
- Kill chỉ chạy sau dialog; process self / parent / system bị chặn trước khi hỏi.
- Env E2E chỉ nhận ở build debug hoặc `config::e2e_isolated()`: `CCM_E2E_DISKMAP_CONFIRM`, `CCM_E2E_DISKCLEAN_{SYS,SDK}`, `CCM_E2E_DOCKER` (chạy `node <script>`), `CCM_E2E_DISKRAM_CONFIRM`.
- E2E chỉ thao tác trên fixture tạm. Lượt test RAM chỉ kill process do test tự spawn; số mục trong Thùng rác thật không đổi (34 -> 34).

## Kiểm
| Lệnh | Kết quả |
|---|---|
| `cargo test` shell (lọc disk_clean / disk_ram / disk_agent / disk_map / disk_move / hosts) | 10 / 4 / 3 / 4 / 13 / 5 pass |
| `npm run qa -- --no-e2e` | 3/4. FAIL duy nhất là `disk_organize::auto_pass_moves...` (WP21); chạy riêng pass 2/2, chỉ hỏng khi chạy song song vì timing |
| `cargo test` shell, cả crate | 278/279, cùng test WP21 ở trên |
| Tauri classic: v101-diskclean / -diskclean-system / -sdk-move / -mcp-disk / -apps-ram / -projsize | 22/22, 26/26, 28/28, 26/26, 26/26, 17/17 |
| Tauri `--ui=next`: cùng 6 file | 22/22, 26/26, 28/28, 26/26, 26/26, 16/17 |
| Electron dev classic: cùng 6 file | 22/22, 26/26, 28/28, 26/26, 26/26, 17/17 |
| Electron dev `--ui=next` v101-projsize | 16/17, FAIL giống Tauri |

v101-projsize next FAIL ở check "rail project tooltip" (tooltip rỗng). Electron cũng FAIL đúng chỗ này, nên đây là chỗ UI next chưa có, không do port.

Build E2E dùng target riêng `apps/shell/target-wp20` (lúc đó exe debug trong `target/` đang bị gói khác chạy), đã xoá sau khi test xong.

## Còn thiếu
- `openNow` (cwd của pane PTY): Tauri chỉ dùng danh sách `open` mà trang gửi lên. Trang luôn gửi, nên E2E vẫn khoá đúng.
- macOS chưa test:
  - `allowTree` cho Temp chưa có. `/private/var/folders` sẽ bị guard trả `system`, nên tab Temp trên macOS Tauri sẽ báo lỗi thay vì xoá.
  - xcode / ios / appcache đã port nhưng chưa chạy thử.
- Báo cáo Telegram sau quét nền (`diskMap/report.ts`) vẫn chưa có, như WP18.
