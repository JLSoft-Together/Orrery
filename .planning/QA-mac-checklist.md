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
- [ ] WP-70b: menu góc phải tắt khi click ra ngoài; chế độ gọn về lại chế độ đầy đủ được; không còn Mission Control.
- [ ] WP-71: Explorer ghim cố định bên phải, kéo width bằng chuột (trackpad), nhớ sau khi mở lại app; cây sâu cuộn ngang được bằng trackpad.
- [ ] WP-72: Git log xem diff dễ dùng; bấm ảnh png / webp trong thay đổi của commit xem được trước / sau.
- [ ] WP-73: Disk trên Mac: chọn ổ / volume nào thì chỉ quét volume đó; chọn item đổi trạng thái ngay.
