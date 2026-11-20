# WP-104m: Tauri lấp keep awake, toast OS, toast menu v100

Trạng thái: xong trên Windows. Bảng `AUDIT-v104-tauri-parity.md` dòng 33, 34, 43 đã đổi (33, 43 đủ; 34 đủ trên Windows, macOS còn thiếu). Không xoá Electron, không đụng file web.

## 1. Keep awake (dòng 43)
- `apps/core/crates/core-serve/src/config.rs`: `ServeConfig.host_events` + `posts_to_host()` (`host_rpc || host_events`); `toast_supported()` dùng `posts_to_host()`.
- `core-serve/src/lib.rs` `serve`: chạy nhúng (`run_with` / `run_with_tap`, chỉ vỏ Tauri dùng) thì bật `host_events`; `runtime.rs` `Setup.busy_events` thay `host_rpc` cho `BusyTracker`. Nên core-serve phát `busy` dù `CCM_HOST_RPC` tắt. Đường Electron (`CCM_HOST_RPC=1`) không đổi.
- `core-desk/src/power/native.rs` `on_battery_now()` (pub, theo `CCM_E2E_POWER` nếu có).
- `apps/shell/src/power.rs`: `busy` (tin `busy` -> `desk:awake-busy`, bỏ khi `Shell.stopped`), `idle` (`collector::stop` -> busy 0, như `onExit` của Electron), `watch` (thread `power-watch`, đọc pin mỗi 30 s, đổi trạng thái -> `desk:awake-power`), `dispose` (`main.rs` `before_stop`, chạy trong `Shell::shutdown` -> `desk:awake-dispose`).
- `episodes.rs` `sink`: `busy` / `toast.*` vào hàng đợi, thread `host-messages` xử lý riêng.
- Khác Electron: đổi nguồn điện thấy sau max 30 s (Electron có event `powerMonitor`).

## 2. Toast OS Windows (dòng 33)
- `apps/shell/src/toast.rs`: port `serve-toast.ts` (`parse_full`, cùng giới hạn độ dài) + script PowerShell của `collector/src/toast.ts` sang WinRT qua crate `windows` đã có (thêm feature `UI_Notifications`, `Data_Xml_Dom`, không thêm crate mới). `xml()` ra đúng XML cũ (`scenario='reminder'` khi persistent, nút trả lời max 3, Open, Snooze, Dismiss `activationType='system'`), tag + group `ccm`, hide = `History.RemoveGroupedTagWithId`, health = `ToastNotifier.Setting()` -> `ok` / `appOff` / `globalOff` / `unknown`, gửi `toast.health` về serve lúc `desk::wire`, sau `collector::start`, sau toast test. App id: exe trong `target/debug|release` dùng AUMID PowerShell (như Electron), bản cài dùng identifier (như `tauri-plugin-notification`). Lệnh WinRT chạy tuần tự trên thread `toasts`.
- `config.rs`: Windows đặt `CCM_TOAST_LINK=agent-monitor://` nếu chưa có (như Electron), nút Open / Snooze đi qua `links.rs` `focus` / `snooze` đã có.
- `CCM_TOAST=0` -> `serve_cfg.toast` None -> không toast, không health.

## 3. Toast menu v100 (dòng 34)
- `apps/shell/src/menu_toast.rs`: `Core` port `createMenuToasts` (`sync`, `settle`, `clear`, `expire` theo grace, `activate`, `link`), `parse_sync` giữ giới hạn của `menuToast.ts`, tag `waiting:<terminal>` hoặc `menu:<pane>`, key 16 byte mỗi lần chạy, so key không rò thời gian.
- `ccm.rs`: kênh `ccm:menu-toast-sync` (chỉ cửa sổ `main`, thêm vào `LOCAL`). Shim đã có `menuToastSync` / `onMenuToast` từ `preload.ts`, không phải sinh lại.
- `links.rs`: `Link::Menu` (`agent-monitor://menu?...`, không chờ `LinkGate`, như Electron gọi `handleMenuLink` trước) -> `menu_toast::activate` -> emit `ccm:menu-toast` `{type, paneKey, sig, index}` tới `main`; `open` hiện cửa sổ. App không tự gõ phím: chỉ trang gõ phím sau click nút của chủ nhân (ngoại lệ thứ 6 v100 giữ nguyên).
- `window.rs`: `Focused(true)` của `main` -> `settle` (ẩn toast đang hiện, như `browser-window-focus`). `focused` = visible, không minimize, focus.
- `drawer_from`: `CCM_TOAST=0` tắt hẳn; `CCM_E2E_MENU_TOAST_LOG` / `_GRACE_MS` / `_REAL` chỉ ở bản debug hoặc `config::e2e_isolated()` (Electron: chỉ bản chưa đóng gói).

## 4. Lỗi tìm thêm và đã sửa: thu nhỏ cửa sổ làm hỏng terminal
Thu nhỏ Orrery Tauri trên Windows co webview về 144x19 px: xterm fit lại, PTY resize còn khoảng 1 cột, màn TUI agent vỡ, menu không đọc được (toast menu không bao giờ hiện khi thu nhỏ). `window.rs` `keep_size_when_minimized` / `minimized_size`: khi `Resized` lúc `is_minimized`, đặt lại webview chính về size thường cuối cùng. Probe: lúc thu nhỏ trang vẫn 1360x860, menu đọc được.

## 5. Test
- `cargo test -j 4 --bin orrery-shell`: 529 pass, 4 ignored (mới: `toast` 5 + 1 ignored, `menu_toast` 8, `power` 2, `window` 1, `links` thêm case `menu`).
- `cargo test -j 4 --bin orrery-shell -- --ignored a_real_toast_shows_and_hides`: pass, 1 toast thật hiện 3 s rồi ẩn, health `ok` (toast thật duy nhất của gói).
- `cargo test -j 4 -p core-serve`: 14 pass (mới `an_embedding_host_gets_toasts_and_busy_without_host_rpc`).
- Chưa chạy `core-bin --test serve`: `apps/core/target/debug/orrery-core.exe` đang bị process khác khoá (Access denied khi link). Đường `CCM_HOST_RPC=1` của binary không đổi hành vi.
- E2E Tauri (cổng 4521 / 9521, `CCM_TOAST=0` nên không có toast thật):
  - `v76`: 83 pass, 0 fail (trước: 67 pass 2 fail; bằng số của Electron). Đã bỏ v76 khỏi `ELECTRON_ONLY` trong `run.cjs`. `v76.cjs` trên Tauri dùng `shellConfirms` (đọc `<data>/e2e-confirms.log`, đổi câu trả lời qua `<data>/e2e-confirm-answer`) thay inspector Node; `hosts.rs` `e2e_answer` đọc file đó trước `CCM_E2E_CONFIRM` và log `message detail`.
  - `v100-menu-toast` (không nằm trong `run.cjs`): 7 pass, 0 fail, 1 SKIP (các bước link qua instance thứ 2: E2E Tauri chạy cách ly, không single-instance; unit test `menu_toast` + `links` phủ key, sig, option, hide). Trên Tauri gõ lệnh fixture lúc cửa sổ hiện rồi mới thu nhỏ (CDP không gõ được vào cửa sổ đang thu nhỏ).
  - Chạy lại sau sửa thu nhỏ: xem mục 7.
- Sự cố trong lúc làm: con lỡ chạy `run.cjs` đầy đủ (cổng 4521) khoảng 3 phút, đã dừng ở `tabs-cwd` (kill `run.cjs` + con của nó). Không đụng instance dev của main (pid 50760) hay cổng cấm.

## 6. Mục treo
- macOS: toast menu có nút (Electron mac dùng `Notification` có action) và dock badge / bounce (dòng 35) chưa có; `toast.rs` chỉ Windows.
- Phía web (WP-104e đang dời `useMenuToast`): không cần đổi; trang vẫn gọi `desktop.menuToastSync` và nghe `onMenuToast`, nhớ giữ hai lời gọi này khi dời sang `InboxChip.vue` / `App.vue`.
- `qa-fast.mjs` `E2E_MAP` chưa map `apps/shell/src/{toast,menu_toast,power}.rs` sang `v76` / `v100-menu-toast`.

## 7. Chạy lại sau sửa thu nhỏ
Bản build cuối (`cargo build -j 4`, link được, không phải đổi tên exe khoá): `v76` 83 pass 0 fail, `split` 24 pass 0 fail, `desktop-only` 5 pass 0 fail (1 SKIP có sẵn từ WP-104k), `v100-menu-toast` 7 pass 0 fail 1 SKIP. Chưa chạy lại `run.cjs` đầy đủ (35 phút).

## Test macOS
- [ ] Keep awake: agent chạy -> badge `[data-awake]` "working", `pmset -g assertions` có `caffeinate`; hết việc -> grace 2 phút rồi nhả; rút sạc với chế độ "Chỉ khi cắm sạc" -> nhả sau max 30 s.
- [ ] Thoát app khi đang giữ máy thức -> không còn process `caffeinate` của Orrery.
- [ ] Không có (đừng báo lỗi): toast OS khi Orrery ở nền, toast menu có nút, dock badge.
