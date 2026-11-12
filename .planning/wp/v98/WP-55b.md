## WP-55b: Web app — Workspace A/B, terminal xterm.js (Ctrl / Alt dính), viewer file / diff / git / run, xác nhận thao tác nguy hiểm

- Mục tiêu: phần "điều khiển" của web app ngang app Android: xem và chọn pane, terminal gõ được (kể cả tiếng Việt trên bàn phím iOS), phím phụ, viewer, xác nhận trên điện thoại cho thao tác nguy hiểm; chạy dưới CSP chặt của relay.
- Spec: `.planning/PLAN-v98-ios-pwa.md` #9, #10, #12, WP-55. Mockup đã duyệt: `docs/design/v97-mobile-ui/png/{ws-a,ws-b,term,term-pick,file,diff,detail}-*.png`, `section-components.png`, `section-motion.png`; lựa chọn chủ nhân: Workspace A mặc định (1 pane + chip) + nút sang B (lưới Mission Control), nhớ lựa chọn; terminal / code luôn nền tối (`.planning/PLAN-v97-mobile-ui.md` mục "Chủ nhân duyệt mockup").
- Hợp đồng: `.planning/wp/v98/WP-55a-report.md` mục "Hợp đồng cho WP-55b" (route `workspace {machine, pane?}` trong `src/router.ts`, thay `views/WorkspaceStub.vue` ở nhánh `master.name === 'workspace'` của `App.vue`, `PageBar`, `hub.viewOf(machine).workspace`, `rt.hub.session(id)?.request`); `.planning/wp/v98/WP-54-report.md` (store, kết nối, rủi ro xterm.js chèn `<style>`); `.code_index/web-pwa.md`. Giao thức: `docs/remote-integration.md` §3 (frame pty_in / pty_out, `FLAG_CLAIM`), §4.3 Workspace, §4.4 PTY (attach / detach / resize / claim, max `REMOTE_ATTACH_MAX`), §4.5 Viewer, §4.6 Xác nhận, §5 giới hạn; kiểu `packages/shared/src/remote.ts`. Tham khảo Android (chỉ đọc): `apps/mobile/lib/ui/terminal/**` (`sticky_modifiers.dart`, `input_bar.dart`, `terminal_pane.dart`), `apps/mobile/lib/ui/workspace/**`, `apps/mobile/lib/ui/viewers/**`, `apps/mobile/lib/remote/pty_stream.dart`. Desktop dùng xterm.js: `apps/web` (đọc cách cấu hình, IME tiếng Việt `apps/web/src/lib/termIme.ts`).
- Phạm vi file được đụng: `apps/pwa/**`; `apps/relay/src/app.ts` CHỈ phần CSP `style-src` nếu không còn cách khác (ghi lý do, chạy `cd apps/relay && npm test`); `.code_index/web-pwa.md` + `_meta.md`; `docs/remote-integration.md` §7.5 nếu đổi CSP (đọc lại ngay trước khi sửa); `docs/qa/v98/WP-55b/**`; báo cáo. Cấm đụng: `apps/shell/**`, `apps/web/**`, `apps/mobile/**`, `packages/shared/**`, các file relay khác (WP-59 đang sửa relay + shell).
- Model: sonnet. Song song với WP-59 (opus).

### Việc
0. Đọc `CLAUDE.md` Hard rules; nạp skill `frontend-design:frontend-design` + `ui-ux-pro-max:ui-ux-pro-max` trước khi làm UI.
1. CSP + xterm.js trước tiên (chặn mọi thứ): dựng 1 trang terminal tối thiểu chạy dưới đúng CSP relay (E2E đã có static server áp CSP), đo có vi phạm `style-src` không trên WebKit + Chromium. Thứ tự giải pháp: (a) import CSS xterm qua Vite, chặn / thay chỗ xterm tạo `<style>` (vd. dùng renderer / addon không chèn style, hoặc `adoptedStyleSheets` — CSSOM được CSP cho phép); (b) nếu buộc phải nới: hash / nonce không dùng được với file tĩnh đổi theo build → cân nhắc `style-src 'self' 'unsafe-inline'` CHỈ cho style (script vẫn chặt), ghi đánh đổi; (c) cuối cùng mới tự vẽ. Chọn cách, ghi lý do + kết quả đo vào báo cáo.
2. Workspace A (1 pane lớn + chip pane, chuyển pane bằng chip / vuốt), B (lưới Mission Control, chạm mở), nút đổi mode nhớ lựa chọn; pane hiện trạng thái (icon + chữ + màu), agent (Claude / Codex / shell màu theo token `agent-*`), tool đang chạy; iPad: lưới rộng hơn / terminal bên phải.
3. Terminal: attach / detach / resize theo §4.4 (id attach do phone chọn, max attach, detach khi rời màn / nền), luồng pty_out ghi vào xterm, gõ → pty_in có `claim` chỉ khi người dùng thật gõ / dán (câu trả lời query của terminal không bật claim — xem cách Android làm); thanh phím phụ Esc / Tab / Ctrl / Alt (dính: off → once → locked như Android) / mũi tên / Enter / Ctrl-C; ô nhập dòng (TextArea) cho bàn phím iOS + tiếng Việt Telex / VNI (gửi cả dòng khi Gửi, hoặc từng ký tự khi bật "gõ trực tiếp" — chọn như Android), dán; font mono hệ thống; nền tối luôn; giữ vị trí cuộn khi output chảy; nối lại sau nền thì attach lại.
4. Xác nhận thao tác nguy hiểm (§4.6): hộp xác nhận trên điện thoại cho lệnh desktop yêu cầu (`RemoteNeedsConfirm` / `confirm`), hết hạn `REMOTE_CONFIRM_MS`; nút approve / deny / stop / esc / option trên pane đang hỏi (`RemoteKeyPress`), gửi prompt text ≤ `REMOTE_TEXT_MAX`. Quyền "điều khiển từ xa" tắt trên desktop → hiện rõ "chỉ xem".
5. Viewer: file (chỉ đọc, highlight nhẹ hoặc không — chọn bản nhỏ), diff (thêm / xoá màu token `diff-*`), git (log / thay đổi theo §4.5), run (trạng thái + log); nền code tối; lỗi / rỗng theo mockup.
6. Deep link `/app/m/<machineId>/p/<paneKey>` (WP-55a để stub) → mở thẳng pane; máy chưa ghép → thông báo "Chưa ghép máy này"; pane không còn → "Pane này không còn mở".
7. Test: Vitest (sticky modifiers, ánh xạ phím → byte, claim, ghép khúc pty_out, reducer workspace patch nếu thêm). E2E mở rộng (`npm run e2e -w @ccm/pwa`): desktop giả (`apps/pwa/e2e/peer`, được sửa) phát workspace + pty_out + nhận pty_in; kịch bản: Workspace A ↔ B nhớ lựa chọn, mở terminal, nhận output 64 KiB đúng, gõ "echo xin chào" → peer nhận byte đúng (UTF-8) + `claim`, Ctrl dính 1 lần, xác nhận approve, viewer diff, deep link mở pane; KHÔNG vi phạm CSP (sau khi chốt bước 1). Ảnh 390×844 + 820×1180 sáng / tối các màn mới → `docs/qa/v98/WP-55b/` (chỉ WebKit, trừ ảnh riêng Chromium), chụp SAU lần sửa cuối.

### Tiêu chí nghiệm thu
- [ ] `npm run typecheck -w @ccm/pwa`, `npm test -w @ccm/pwa` (số), `npm run build -w @ccm/pwa` sạch; E2E WebKit + Chromium pass (số bước), 0 vi phạm CSP.
- [ ] Báo cáo ghi rõ cách xử lý CSP / xterm + kết quả đo; nếu đổi `apps/relay/src/app.ts` thì relay `npm test` pass.
- [ ] Ảnh khớp mockup (main so); text đủ vi + en; không file mới > ~400 dòng.

### Luật (bắt buộc)
- Không viết comment trong code (kể cả `/** */`, `//`, TODO); lý do ghi báo cáo hoặc `.code_index/`.
- Không `git commit` / `push`. Không build release Tauri / APK.
- Text UI qua vue-i18n `vi` + `en`; màu qua token; trạng thái = icon + chữ + màu; không blur, không gradient; font hệ thống; không tải gì từ mạng.
- Tool nặng ≤ 80% CPU / RAM; kiểm `tasklist | grep -i cargo` trước khi build cargo (peer E2E), có agent khác build thì chờ. Không dùng cổng 4320–4322.
- Bash: `rtk` có thể rewrite `grep` → dùng tool Grep hoặc `rtk proxy grep`.
- Tắt mọi tiến trình nền khi xong.

### Báo cáo
`.planning/wp/v98/WP-55b-report.md`: cách xử lý CSP (đo trước / sau), màn đã làm (file + symbol), test / E2E chạy thật (số), ảnh, khác mockup + lý do, việc dở (gồm việc cần desktop), rủi ro iOS thật (bàn phím, IME, cuộn, nền).

### Nhật ký
- Bước 1 xong. Đo trên trang terminal tối thiểu dưới CSP relay (`style-src 'self'`): xterm 6.0.0 thuần = WebKit 12 vi phạm `style-src-elem inline`, Chromium 8, chữ không có màu (DomRenderer + Viewport tạo `<style>`). Cách chọn (a): `src/terminal/xtermStyles.ts` chặn `document.createElement('style')`, trả `div` ẩn có setter `textContent` nạp vào `CSSStyleSheet.replaceSync` + `adoptedStyleSheets`. Sau: 0 vi phạm cả 2 trình duyệt, màu đúng (#e5e5ea / đỏ). Không đổi `apps/relay`.
- Xong: typecheck sạch, 156 test pass, build sạch, E2E WebKit 28/28 + Chromium 28/28, 0 vi phạm CSP. Báo cáo: `.planning/wp/v98/WP-55b-report.md`.
