# RELEASE 4.0.8

Release 2026-11-20 (tag v4.0.8): Orrery Remote (điện thoại), Inbox, giao diện mới mặc định, sửa lỗi macOS.

## Tiếng Việt

### Orrery Remote (điện thoại)
- App Android và web app iPhone / iPad: theo dõi và điều khiển session từ điện thoại.
- Màn chat: hộp thư ưu tiên session đang chờ bạn, gửi thẳng vào session (kể cả khi agent đang chạy), nút Dừng, chạm để trả lời menu chọn và xin quyền, xem terminal.
- Kết nối qua LAN hoặc relay Cloudflare của chính bạn, mã hoá đầu cuối (Noise). Dựng relay 1 chạm bằng wrangler.
- Push có nội dung (agent, session, câu hỏi), secret đã che; mỗi điện thoại tắt được nội dung.
- Remote có trang riêng trong Cài đặt; thẻ Cài app lên điện thoại bằng QR. Remote tắt mặc định.

### Workspace
- Inbox (`Ctrl+Shift+A`) gom menu chọn, xin quyền, câu hỏi, đụng file, lỗi, ghi chú và việc xong; `Ctrl+Alt+Q` tới mục kế.
- Cảnh báo khi 2 session sửa cùng 1 file; Changes xem được phần sửa của từng session.
- Tin nhắn từ điện thoại và Telegram vào thẳng session, kể cả khi agent đang chạy.
- MCP & plugin: đánh dấu thay đổi rồi áp dụng một lần.

### Giao diện
- Giao diện mới (Sương kính) là mặc định: rail hiện trạng thái, tool đang chạy và thời gian; pane file, Changes, log nằm trong nhóm Công cụ thu gọn được.
- Cài đặt gom lại còn 8 trang, có ô tìm kiếm.
- Màu: thêm bảng Đen trắng và Ngẫu nhiên (mỗi lần mở app một bảng màu mới, có Trộn lại / Giữ màu này). Giao diện cũ còn Orrery sáng, Orrery tối, Ngân hà; các theme cũ khác tự chuyển sang các theme này.
- Giao diện cũ sẽ bỏ ở bản 4.1.0.

### Disk
- Quét và quét lại khi rảnh chỉ trên các ổ đã chọn; dữ liệu của ổ bỏ chọn được ghi rõ là cũ.

### macOS
- Cmd+Q hỏi trước khi đóng terminal đang chạy và lưu vị trí cửa sổ; bấm icon Dock mở lại cửa sổ đã đóng.
- Esc đóng được hộp thoại (cả hộp xác nhận tiếng Việt); Cmd+K dùng được trong terminal.

### Sửa lỗi khác
- Windows: Ctrl+C tới được chương trình đang chạy trong pane terminal.
- Thanh tab không còn treo (vòng đo độ rộng tab vô hạn) làm pane file trống.

## English

### Orrery Remote (phone)
- Android app and iPhone / iPad web app to follow and drive sessions from your phone.
- Chat: an inbox with waiting sessions first, send straight into the session (even while the agent works), Stop, tap to answer choice menus and permissions, terminal view.
- Connects over LAN or your own Cloudflare relay, end-to-end encrypted (Noise). One tap relay setup with wrangler.
- Push notifications with content (agent, session, question), secrets masked; each phone can turn content off.
- Remote has its own Settings page, with a QR card to install the phone app. Remote is off by default.

### Workspace
- Inbox (`Ctrl+Shift+A`) gathers choice menus, permissions, questions, file clashes, errors, notes and done; `Ctrl+Alt+Q` jumps to the next item.
- A warning when two sessions edit the same file; Changes can show one session's edits.
- Phone and Telegram messages go into the session even while the agent is working.
- MCP & plugins: tick your changes, then apply them all at once.

### Interface
- The new UI (Mist) is the default: rail rows show status, running tool and time; file, Changes and log panes sit under a collapsible Tools line.
- Settings regrouped into 8 pages, with search.
- Colours: plain Đen trắng (black and white) palette and Ngẫu nhiên (random), a new palette at every launch with Mix again / Keep. The classic UI keeps Orrery light, Orrery dark and Ngân hà (Galaxy); other classic themes move to these.
- The classic UI will be removed in 4.1.0.

### Disk
- Scans and idle rescans cover only the drives you picked; data of unpicked drives is marked as old.

### macOS
- Cmd+Q asks before closing running terminals and saves the window position; the Dock icon reopens a closed window.
- Esc closes dialogs (including localized confirms); Cmd+K works inside terminals.

### Other fixes
- Windows: Ctrl+C reaches programs running in terminal panes.
- The tab bar no longer freezes (endless tab width measuring) leaving a file pane empty.

## Kiểm tra trước release
- macOS arm64: full E2E Tauri 1356 PASS; file lỗi chạy lại đều pass (trừ test Electron cũ, đã bỏ khỏi bộ chạy). `npm run qa -- --no-e2e` pass, web 2814/2814.
- Bản cài arm64: mở, Cmd+Q hỏi / Huỷ / Esc, Dock mở lại. Ký ad-hoc, chưa notarize.
- Chưa thử: bản x64 qua Rosetta, Windows NSIS.
