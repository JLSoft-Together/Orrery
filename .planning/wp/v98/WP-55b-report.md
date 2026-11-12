# WP-55b report

## CSP + xterm
- Đo (trang terminal tối thiểu dưới CSP relay `style-src 'self'`): xterm 6.0.0 thuần = WebKit 12 / Chromium 8 vi phạm `style-src-elem inline` (DomRenderer + Viewport tạo `<style>`), chữ mất màu.
- Chọn (a): `apps/pwa/src/terminal/xtermStyles.ts` thay `document.createElement('style')` bằng div ẩn có setter `textContent` -> `CSSStyleSheet.replaceSync` + `document.adoptedStyleSheets`. Sau: 0 vi phạm cả hai trình duyệt, màu đúng; E2E toàn bộ 0 vi phạm. Không đổi `apps/relay`. DOM renderer (không WebGL). Đánh đổi: patch toàn cục `document.createElement` chỉ cho tag style; sheet dọn khi terminal đóng.

## Màn
- `views/WorkspaceView.vue` (A 1 pane + chip + vuốt, B lưới + lọc, nhớ mode `prefs.workspaceMode`, Pane mới `ws.open`, gửi prompt `ws.send`), `views/TerminalView.vue` + `components/TerminalSurface.vue` (attach/detach/resize, claim chỉ khi gõ thật, ghép pty_out, nền tối), `KeyBar` (Esc Tab Ctrl Alt ⇧Tab ^C mũi tên Enter; Ctrl/Alt off/once/locked), `InputBar` (textarea dòng + gõ trực tiếp), `PermissionBar` (`ws.keys` + `since`), `RemoteConfirmHost` (`confirm.ask`/`confirm.answer`, đếm 60 s), `ViewerPage` + Files/Changes(diff)/Log/Run, deep link + thông báo "Chưa ghép máy này" / "Pane này không còn mở", chế độ chỉ xem (khi desktop trả `denied`).
- Logic: `terminal/{ptyStreams,ptyRegistry,modifiers,keys}.ts`, `lib/{workspaceModel,viewerModel,viewerApi,useAsync}.ts`, hub thêm emitter `pty`/`device`.

## Test thật
- `npm run typecheck`, `npm run build` sạch; `npm test -w @ccm/pwa`: 15 file, 156 pass / 0 fail (thêm 27: sticky, key bytes, PtyStreams claim/gộp/limit/reattach, model workspace + viewer).
- `npm run e2e -w @ccm/pwa`: WebKit 28/28 + Chromium 28/28 = 56 pass / 0 fail. Gồm: A<->B nhớ sau reload, attach theo cỡ phone, 64 KiB pty_out đúng từng byte (checksum), "echo xin chào" -> peer nhận UTF-8 + claim, Ctrl dính once/locked/off, gõ trực tiếp có claim, đổi pane detach/attach, approve/deny `ws.keys`, xác nhận confirm.answer, viewer file/diff/git/run, deep link 3 trường hợp, `ws.send`, view-only.
- Peer E2E mở rộng (`e2e/peer/src/data.rs`, `conn.rs`); sửa 1 bug peer (seal sai thứ tự nonce).

## Ảnh
`docs/qa/v98/WP-55b/` : ws-a, ws-b, confirm, term, term-pick, file, diff x {phone 390x844, iPad 820x1180} x {sáng, tối} (WebKit); term + ws-a phone sáng Chromium.

## Khác mockup
- Thẻ pane không có xem trước output thật (cần attach): hiện tool / waitText. File viewer không highlight cú pháp. Terminal khi không gõ vẽ theo cỡ desktop, font co tối thiểu 9px (cuộn ngang trong terminal nếu rộng hơn). Viewer là overlay trong màn pane, không route riêng.

## Việc dở / cần desktop
- Desktop chưa báo "điều khiển từ xa tắt" trong hello; web chỉ biết sau khi lệnh bị `denied`. Nên thêm cờ vào hello.
- Chưa làm: ws.close/rename/move/compact, sửa file (fs.write), stage/commit/branch, run.start.

## Rủi ro iOS thật (chưa thử máy thật)
- Bàn phím iOS + Telex/VNI trong textarea dòng (nên ổn) và gõ trực tiếp qua textarea ẩn của xterm (IME composition chưa kiểm). `inputmode=none` để chặn bàn phím khi chạm terminal. Cuộn xterm khi bàn phím mở / visualViewport chưa xử lý; vuốt đổi pane có thể đụng cử chỉ back của Safari; `adoptedStyleSheets` cần Safari >= 16.4; nền: pty detach khi ẩn trang, attach lại khi quay lại.
