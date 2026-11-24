# WP bug: hide / minimize rồi mở lại, cửa sổ bị che, không click được

## Root cause

`apps/shell/src/window.rs::keep_size_when_minimized` (d7a678be, 20/11) gọi `webview.set_size(keep)` trên webview chính lúc `Resized` khi đang minimize.

- Shell bật feature `unstable` của tauri, nên webview chính của `WebviewWindow` là **child webview** có autoresize theo tỉ lệ (`WebviewBounds { width_rate, height_rate, ... }`, `tauri-runtime-wry-2.12.3/src/lib.rs:4917` và handler `TaoWindowEvent::Resized` ở `lib.rs:4273`).
- `WebviewMessage::SetSize` (`lib.rs:3741`) tính lại `width_rate = size / window.inner_size()`. Lúc minimize client chỉ còn **144x19**, nên rate = 1360/144 = **9.44**, 860/19 = **45.26**.
- Listener của app chạy **trước** autoresize của tauri (`lib.rs:4211-4235`), nên lúc minimize trick này vẫn giữ webview 1360x860 (PTY không còn 1 cột). Nhưng khi restore, autoresize nhân client 1360x860 với rate hỏng: webview thành **12844x38926**.
- Trang render theo viewport khổng lồ đó: chỉ góc trái trên hiện trong cửa sổ (rail rỗng, tab strip, vùng pane tối), header / nút min-max-close / dialog nằm ngoài màn hình nên trông như "bị phủ tối, không click được". Rate hỏng giữ nguyên qua mọi lần hide / show, chỉ hết khi tắt app.
- Lần minimize **đầu tiên** sau khi mở app chưa có size ghi nhớ nên không gọi `set_size` (webview co về 144x19 rồi về đúng); bug xuất hiện từ lần minimize thứ 2. Close (ẩn vào tray) không tự gây bug, nhưng sau khi rate đã hỏng thì show lại vẫn sai.
- Blur (898b7a49) không liên quan. Guest webview (Browser pane) không có autoresize (`bounds = None`), không bị ảnh hưởng.

## Bằng chứng (native, Win32 `GetClientRect` + `EnumChildWindows` `WRY_WEBVIEW` + `CopyFromScreen`)

Trước fix (bản dev, `CCM_E2E_PORT=4471`, CDP 9471):

| bước | client | child WRY_WEBVIEW | innerWidth x innerHeight |
|---|---|---|---|
| start | 1360x860 | 1360x860 | 1360x860 |
| min 1 / restore 1 | 144x19 / 1360x860 | 144x19 / 1360x860 | ok |
| min 2 | 144x19 | 1360x860 | 1360x860 |
| restore 2 | 1360x860 | **12844x38926** | **12844x38926** |
| hide / show | 1360x860 | **12844x38926** | **12844x38926** |

Sau fix: mọi bước restore / show đều client = child = viewport = 1360x860; minimize 2 vẫn giữ 1360x860 (mục tiêu fb09bf46 còn nguyên).

Ảnh: `docs/qa/bug-hide-restore/before-4-restore2.png`, `before-6-show.png` (UI bị che, chỉ còn rail + vùng tối như ảnh chủ nhân), `after-4-restore2.png`, `after-6-show.png`, `after-e2e-cdp.png`, `before-/after-0-start.png`, `before-/after-2-restore.png`.

## Fix

`apps/shell/src/window.rs`
- `:228-234` thêm `enum Fit { Keep, Fill }` và `SizeMemory` (label, size thường cuối, cờ đã keep lúc minimize).
- `:236-256` `minimized_size` trả `Fit::Keep(last)` khi minimize (đặt cờ), và `Fit::Fill(size)` đúng 1 lần ở `Resized` không minimize đầu tiên sau đó.
- `:258-269` `keep_size_when_minimized`: `Keep` vẫn `set_size` như cũ; `Fill` gọi `webview.set_bounds(0,0,size)`, chạy trước autoresize nên `SetBounds` reset rate về 0/0/1/1 theo client vừa restore, autoresize sau đó áp `size x 1`.
- `:351-381` unit test cũ cập nhật + `the_webview_fills_the_window_again_once_after_every_restore` (3 vòng minimize/restore, restore thẳng sang size maximize).

Test / tooling:
- `apps/shell/e2e/window-restore.cjs` (mới): Win32 thật (`ShowWindow` 6/9/0/5) + `Browser.setWindowBounds` minimized/normal; 3 vòng minimize/restore, minimize+hide+show+restore, hide+show; kiểm child `WRY_WEBVIEW` == client, `innerWidth*dpr` == client, click ở tâm cửa sổ trúng tâm trang. Chỉ chạy Windows (macOS in SKIP).
- `apps/shell/e2e/run.cjs:9` thêm `window-restore` vào bộ chạy toàn bộ.
- `scripts/qa-fast.mjs:46` map `apps/shell/src/window.rs` → E2E `window-restore`.
- `.code_index/shell.md` (Bất biến), `.code_index/_meta.md` 1 dòng; `node scripts/index-check.mjs`: ok 39 file, 0 lỗi.

## Test đã chạy

- `cargo test --locked -j 4` (apps/shell): **604 passed, 0 failed, 4 ignored**.
- E2E `window-restore` với fix: **13/13 PASS**.
- E2E `window-restore` với fix tạm tắt (nhánh `Fill` không làm gì, đã trả lại): **8 FAIL** (restore 2/3, tauri unminimize, hide/show, click lệch: gửi 680,430, tâm trang 6422,19463), xác nhận test bắt đúng bug.
- `npm test -w @ccm/web -- src/features/workspace/lib/splitLayout.test.ts`: 24/24 pass.
- `npm run typecheck`: pass.
- Không còn process `orrery-shell` / cổng 4471, 9471 sau khi chạy; không đụng app thật (4317).

## Rủi ro còn lại

- Lần minimize đầu tiên sau khi mở app vẫn co webview về 144x19 (chưa có `Resized` thường nào để ghi nhớ size); web đã bỏ refit khi viewport nhỏ (fb09bf46) nên PTY không về 1 cột. Có thể seed size từ `create_main` sau này nếu cần.
- Fix dựa trên thứ tự: listener app chạy trước autoresize của tauri-runtime-wry 2.12.x. Nâng tauri thì chạy lại E2E `window-restore`.
- Chưa kiểm multi-monitor khác DPI và restore sang maximize bằng Win32 thật (unit test có case size maximize).
- App đang chạy của chủ nhân (4.0.9) vẫn mang bug cho tới khi cài bản mới; tạm thời tắt / mở lại app là hết.

## Commit message đề xuất

```
fix(shell): refit the main webview after restoring a minimized window

Keeping the webview size while minimized went through set_size, which made
tauri's autoresize store a 9x / 45x rate against the 144x19 minimized client.
The next restore blew the webview up to 12844x38926, so the window looked
covered and clicks missed. The first Resized after a minimize now sets the
bounds back to the client area, which also resets the rate. Adds the
window-restore E2E (Win32 minimize / restore / hide / show).
```
