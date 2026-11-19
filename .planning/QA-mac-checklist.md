# Checklist test macOS (chủ nhân test sau khi xong hết trên Windows)

Chủ nhân 2026-11-19: mọi phần cần test trên Mac ghi hết vào file này; làm xong trên máy Windows thì chủ nhân cầm Mac test một lượt. Mỗi gói WP xong, main thêm dòng của gói vào đây (lấy từ mục "Bước QA tay" của báo cáo). Đánh dấu `[x]` khi chủ nhân báo OK, ghi lỗi ngay dưới dòng.

## Chuẩn bị
- [ ] `git pull` nhánh `feat/v98-pwa`, `npm install`, `npm run dist:mac` (hoặc `npm run desktop:dev`), mở app.
- [ ] Bản mới mặc định UI next (WP-70b); kiểm cả máy đang dùng classic → tự chuyển sang next một lần, Appearance vẫn chọn lại classic được.

## Merge master (a91daa12)
- [ ] UI next: dải trên cùng kéo được cửa sổ; 3 nút đèn đỏ / vàng / xanh nằm trong dải, không đè logo / rail; double click dải → phóng to / thu nhỏ như app Mac.
- [ ] Không có nút cửa sổ tự vẽ (chỉ Windows mới có).
- [ ] Click menu v100 + Disk v101 vẫn chạy trên Mac sau merge.

## Remote v97 / v98
- [ ] Settings → Điện thoại: bật remote, QR ghép, danh sách thiết bị (thiết bị iPhone cũ hiện "Web app (iPhone)"), chuông push, relay, chặn IP / tin cậy / nhật ký.
- [ ] Nút "Cập nhật relay" chạy được (bundle mới có `/app/*`).
- [ ] Nút `^C` từ điện thoại ngắt được lệnh đang chạy trong pane shell trên Mac (WP-64 chỉ sửa Windows, Mac kiểm không hỏng).
- [ ] Từ chối ghép trên hộp native → bảng QR hiện "đã từ chối" + nút tạo mã mới.
- [ ] Tắt relay rồi bật lại → desktop nối lại trong ≤ ~15 s.

## v103 UI next (điền dần khi từng gói xong)
- [ ] WP-70a: nút "Explorer" có chữ + icon cây thư mục; không còn hộp "Cần bạn xử lý"; session lùi vào dưới project; nút thu gọn đầu rail + Cmd+Shift+B (cả khi đang focus terminal), tắt mở app vẫn nhớ; footer hạn mức ở đáy (Claude Code + Codex, giờ reset theo locale 12h / 24h); không cuộn trang, đáy pane còn bo góc, footer không đè dải title bar Mac; cửa sổ hẹp (icon / overlay) vẫn đúng.
- [ ] WP-70b: menu "…" tắt khi click ra ngoài / Esc / Cmd+Tab sang app khác; Tray → Compact window rồi "Full view" → cửa sổ chính hiện, không sinh cửa sổ thứ 2; Esc thoát compact (không đụng Cmd+W), Cmd+K mở palette trong compact; nút "Đầy đủ" không bị vùng kéo title bar che; không còn Mission Control (Cmd+Shift+O, Cmd+K, chuột phải project); máy đang classic → lần đầu mở vào next, không nháy UI cũ, chọn lại classic thì restart vẫn classic.
- [ ] WP-71 (Explorer): Cmd+Shift+E bật / tắt (cả khi đang ghim); ghim → cột đặc bên phải, pane co lại, cả cửa sổ 1280–1440; kéo mép trái bằng trackpad, double click về 288, mở lại app nhớ width; cửa sổ hẹp → nút ghim bị chặn kèm tooltip; cây sâu cuộn ngang hai ngón (scrollbar overlay macOS), dòng tên project + nút đứng yên; tên dài đủ chữ với font Mac; cột ghim không làm trang cuộn.
- [ ] WP-72 (Git log): chọn commit thấy ngay diff preview dưới bảng; Alt+↑ / Alt+↓ đổi commit, j / k trong bảng; ảnh png / webp / svg trong commit: Side by side / Slider / Onion skin; ảnh đổi tên, ảnh thêm / xoá hiện 1 bên; Cmd thay Ctrl ở mọi phím tắt.
- [ ] WP-73 (Disk): tick checkbox trong cây / kết quả đổi ngay (cả phím Space); tích chỉ `/Volumes/X` → chỉ X được quét, `/` không quét lại; bỏ chọn ổ đã quét → dòng "Dữ liệu cũ được giữ", quét nền / snapshot / báo cáo Telegram chỉ có ổ đang chọn; chọn thư mục con của `/` vẫn tính theo ổ cha; nút "Quét siêu nhanh" ẩn trên Mac.
