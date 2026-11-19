# WP19 report: 5 E2E fail có sẵn

Kết luận: cả 3 file là test lệch môi trường / UI hiện tại, không có lỗi app. Chỉ sửa test, không sửa `src/`.

## 1. e2e/v101-snap.cjs "scan did not start"
- Nguyên nhân: e2e/v101-snap.cjs dòng 36-37 mặc định `CCM_CORE_BIN=apps/core/target-4c/debug/orrery-core.exe` và `CCM_WEB_DIST=%TEMP%/ccm-web-4c`. Thư mục `target-4c` không còn. App log: "orrery-core not found at ...target-4c...", core serve rơi về Node nên `disk.scan` không có, `DiskMapService.scan` (src/diskMap/ipc.ts:85) trả `{started:false}`. Gọi `diskMapScan` thủ công với core thật trả `started:true`, nên `cleanScan` và main process đúng.
- Sửa: test tự chọn bin mới nhất trong `apps/core/target/{debug,release}`, không ép `CCM_WEB_DIST` khi không set; chỉ truyền env khi có giá trị.
- Kết quả: 36/36 PASS.

## 2. e2e/v76.cjs "card lines", "card badge" x2
- Nguyên nhân: Monitor hiện chỉ render 1 `SessionCard` của tab đang chọn (MonitorView.vue:43,119, panel Session chỉ hiện khi `ui.sessionTab`). Test cũ chờ >= 2 card cùng lúc và mở panel mà không chọn session tab. `SessionList` không còn được import ở đâu.
- Sửa: test gọi `ui.openSessionTab(id)` trước khi đọc `[data-card-resources]` (A rồi B, kiểm 528 MB / 300 MB + forgotten như cũ) và trước `[data-card-sensitive]` (B) ở cả 2 chỗ. Assertion không nới.
- Kết quả: 83 PASS, 0 FAIL ở UI mặc định và `CCM_E2E_UI=next`.

## 3. e2e/v73.cjs "PowerShell pane: exit 3 gets a ✗ mark"
- Nguyên nhân (đã bắt stream PTY và log marks tạm trong useShellMarks, đã gỡ): backend đúng, 633;E/C/D;3 về đủ, `tracker.finished` trả lệnh, `registerDecoration` thành công nhưng `onRender` không bao giờ chạy. Prompt đầu nằm line 0 còn `baseY=1`: ConPTY khởi tạo in 23 dòng trống rồi pane tách đôi (17 hàng) đẩy 1 dòng vào scrollback, nên decoration của lệnh đầu tiên nằm ngoài viewport. Là artifact của layout test (2 pane từ bước "limit chip"), mark vẫn hiện khi cuộn lên.
- Sửa: test chạy `Clear-Host` (reset scrollback) trước `cmd /c exit 3`; `openMarkMenu` lấy mark cuối (`.at(-1)`) vì `Clear-Host` cũng để lại 1 mark ok (ở `CCM_E2E_UI=next` mark đó làm "Copy output" lấy nhầm).
- Kết quả: 45 PASS, 0 FAIL ở UI mặc định và `CCM_E2E_UI=next`.

## Kiểm
- `npm run typecheck -w @ccm/desktop` sạch. Không sửa `apps/web` hay `apps/desktop/src` (đã dựng lại web sau khi gỡ debug).
- Không chạy Vitest vì không đổi mã nguồn app.
- Mọi lần chạy E2E dùng `CCM_E2E_OUT=.../ccm-e2e-wp19 CCM_E2E_PORT=4496 CCM_E2E_DBG=9431` (trừ 3 lần debug v73 đầu quên export, ghi ra thư mục `ccm-e2e` mặc định ở Temp).

## File đổi
- apps/desktop/e2e/v101-snap.cjs
- apps/desktop/e2e/v76.cjs
- apps/desktop/e2e/v73.cjs
