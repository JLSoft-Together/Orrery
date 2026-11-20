# PLAN v107: điện thoại dạng chat, bớt ngộp

Chủ nhân chốt 2026-11-20 (grilling 2 vòng).

## Quyết định
1. Bấm session agent (Claude / Codex) mở **chat** làm mặc định: bong bóng câu hỏi / trả lời, ô nhập dưới, nút phụ "Xem terminal". Pane shell mở thẳng terminal.
2. Nội dung chat **gọn**: câu chủ nhân hỏi + câu trả lời của agent (markdown, code block); tool call gộp 1 dòng mở ra được ("Đã sửa 3 file, chạy 2 lệnh"); không thinking, không kết quả tool thô; che secret như push.
3. Xin quyền / menu: **bong bóng có nút** (Có / Không, từng lựa chọn menu). Thêm nhãn menu vào giao thức remote. Phím chỉ gửi sau khi chủ nhân bấm, Hoàn tác 3 giây.
4. Gửi tin: **gửi thẳng vào session, bỏ luật chỉ submit khi agent rảnh**, áp cả Telegram. Có nút Dừng (Esc). Cần sửa CLAUDE.md (ngoại lệ 3, 5, 7) và docs khi làm.
5. Màn chính: **hộp thư chat**, mỗi session 1 dòng (tên, câu cuối của agent, trạng thái, thời gian), Chờ bạn lên đầu. Gộp Giám sát + Workspace + Inbox. Quota, file, diff, git, run vào menu "..." / Cài đặt.
6. Bỏ khỏi màn chính: hoạt động thô / tool, nhóm theo project / tab (danh sách phẳng theo thời gian). Giữ: subagent, model, context %, quota ở chỗ phụ.
7. Làm **web app và Android cùng lúc**.
8. Đợt 3: U14, U16 (Shell mới), U17 (mosh) gộp vào thiết kế chat; làm luôn U11, U12, U13 (desktop); gác U15. Thêm vòng nghiên cứu "dọn bớt thông tin" cho desktop.

## Bước
- [x] Mockup chat điện thoại → orrery.jinvu.vn/chat/ (deploy 2026-11-20), chờ chủ nhân duyệt.
- [x] Nghiên cứu dọn bớt desktop (128 phần tử: Giữ 56, Ẩn 28, Gộp 32, Bỏ 12, .planning/RESEARCH-v107-declutter.md) → orrery.jinvu.vn/gon/, chủ nhân bấm Giữ / Ẩn / Gộp / Bỏ.
- [ ] U11, U12, U13 đã duyệt, chờ nghiên cứu xong hết mới code (chủ nhân 2026-11-20).
- [ ] Chủ nhân duyệt mockup, rồi chia gói: giao thức nội dung chat + nhãn menu (desktop), web app, Android, bỏ luật idle (desktop + Telegram + docs).
- Trang nghiên cứu UI từ nay làm đơn giản, ít chữ (chủ nhân 2026-11-20).
- [x] Chủ nhân chọn xong dọn bớt desktop: `.planning/DECISION-v107-declutter.md`. Giao 6 gói G1 rail, G2 tab + toolbar + Inbox + U11, G3 đầu pane, G4 footer + Ctrl+K + màn trống + U12, G5 Giám sát + U13, G6 gộp trang Cài đặt.
- [~] Mockup chat v2 (chủ nhân 2026-11-20): kiểu app Claude (nền ấm, phẳng, agent không bong bóng), thêm tên project trên dòng, context % / quota, tool đang chạy, thẻ file đã sửa + diff.
- [x] G4 về (footer ẩn track mức ok, bỏ placeholder; Ctrl+K bỏ Esc / số đếm / dải prefix; U12 LaunchPicker). G1 về (rail), đang sửa lại: Telegram + điện thoại phải Giữ. G5 về (Giám sát: menu Hiển thị, ẩn cột 0, card click mở Workspace; U13 hoa văn + "làm N%" + gộp idle).
- [x] G3 về (đầu pane: REVEAL hover cho 4/5/14/15/26/27, context chỉ từ 70%, SubagentOrbit flat, peer/coord badge trên chip trạng thái; 7 overlap chỉ thu icon, 18 giữ). 25 test.
- [x] G2 về (bỏ chip project / chuông / '2 pane', Lỗi + chờ thành chấm, Bố cục ẩn hover, màn trống 1 dòng, U11 chip project chỉ khi nhiều project; Inbox phím ẩn, dòng 2 chỉ thời gian). G1 sửa xong Telegram/điện thoại luôn hiện. Chờ G6 + mockup v2.
- [x] Gate + E2E desktop v107: 24 file, pass hết trừ v101-diskmap-ui (lỗi có từ trước: test gán roots nhưng UI không chọn ổ; không đụng code Disk). v89-pane sửa theo quyết định bỏ số token. Mockup chat v2 kiểu app Claude đã deploy.

## Chốt mockup (chủ nhân 2026-11-20)
- Chọn **bản lai** `/chat-lai/` (GPT + Claude + Gemini), nhưng bớt gradient và bỏ icon sparkle kiểu Gemini. Accent màu đặc.

## Gói (manager)
| WP | Model | Việc | Phụ thuộc |
|---|---|---|---|
| C0 | haiku | Mockup lai: bỏ gradient + sparkle | - |
| C1 | opus | Spec giao thức chat (`.planning/SPEC-v107-chat-protocol.md`) + type TS `packages/shared/src/remote.ts` + Dart model | - |
| C2 | sonnet | Bỏ luật chỉ submit khi rảnh (remote + Telegram), sửa CLAUDE.md ngoại lệ 3/5/7 + docs | - |
| C3 | opus | Desktop: phát nội dung chat + nhãn menu theo spec (`apps/shell/src/remote`) | C1 |
| C4 | opus | PWA: hộp thư chat + màn chat theo mockup lai, U14, U16, U17 bước 1 | C1 |
| C5 | opus | Android: như C4 | C1 |

## Mặc định đang giữ (C1 hỏi, chủ nhân chưa trả lời)
- Thẻ file đã sửa: +/- ước lượng từ input tool, bấm vào xem `git.diff` hiện tại của file (không phải diff riêng từng lượt).
- Menu mục "Other": bản 1 chỉ nút mở terminal.
- Câu cuối trên hộp thư: đoạn cuối câu trả lời đã xong, max 160 ký tự; agent đang chạy thì hiện tool đang chạy.
| C6 | opus | Relay cài 1 chạm: wrangler (login OAuth / dùng login sẵn) + dự phòng link token điền sẵn quyền, tự đọc clipboard, 1 account tự deploy (chủ nhân chọn cả 2, 2026-11-20) | - |
| C8 | opus | Bảng MCP / plugin ở header session: checklist + 1 nút Áp dụng, bỏ chờ 3-5 s, lưới nhiều cột | - |
| T1 | opus | Mockup 3-5 bảng màu hài hoà + danh sách theme Giữ / Bỏ + chế độ Ngẫu nhiên (`/mau/`), chờ chủ nhân chọn rồi mới code | - |
| R1 | - | **Task cuối**: viết lại README (cũ, dài), làm sau khi xong hết | mọi gói |
