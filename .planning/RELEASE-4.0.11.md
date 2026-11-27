# RELEASE 4.0.11

Release 2026-11-27 (tag v4.0.11): gói v113 (điện thoại và thiết bị, 12 tính năng) và v114 (UI/UX đợt G, 5 tính năng). Bộ cài kèm sẵn app Orrery Remote 4.0.11 (APK ký cố định).

## Tiếng Việt

### Mới
- Kết nối điện thoại 3 bước trong Settings: cài qua cáp USB hoặc quét QR, kiểm kết nối, phần Nâng cao gập lại.
- Bộ cài có sẵn app Orrery Remote: cài qua cáp bằng 1 nút hoặc lưu file APK.
- Settings, Thiết bị: điện thoại, máy adb và emulator chung 1 danh sách; thu hồi có Hoàn tác.
- Trả lời câu hỏi của agent ngay trên thông báo điện thoại; thông báo "đang chạy" có đồng hồ; mở pane trên điện thoại từ menu pane.
- Widget màn hình chính và ô Quick Settings hiện số session đang chờ, giữ ô để tạm dừng thông báo 1 giờ.
- Hàng đợi "Chờ bạn" trên điện thoại: trả lời từng cái, hoãn 15 phút hoặc 1 giờ, trả lời chữ.
- "Từ lúc bạn đi vắng" trên điện thoại: số xong, chờ bạn, lỗi sau 30 phút vắng.
- Prompt đã lưu: gõ `;` trong Ctrl+K, menu pane, Settings; chèn vào ô nhập, không tự gửi; dùng chung với điện thoại.
- Xem nhanh session khi rê chuột trên rail: tool đang chạy, thời gian, câu hỏi đang chờ.
- Mật độ giao diện Gọn, Vừa, Thoáng.
- Lệnh nguy hiểm gửi từ điện thoại cần vân tay hoặc PIN.
- Checklist trước merge trên điện thoại.
- Chạy trên nhiều máy Android cùng lúc, biến thể sáng, tối, chữ lớn, lưới ảnh chụp.
- Gửi APK vừa build lên điện thoại; xem log Run và gửi crash cho agent trên điện thoại.
- Clipboard chung giữa điện thoại và máy tính, chỉ khi bấm, chặn chữ giống secret.
- Nói ra chữ hoặc chụp ảnh gửi vào pane từ điện thoại.

### Sửa lỗi và an toàn
- Thu hồi thiết bị luôn có hiệu lực ngay cả khi kết nối đang bận.
- Trang tải APK trong mạng nhà: mã ghép đôi dùng 1 lần, chỉ cho đúng máy vừa tải.
- Trả lời chữ từ điện thoại không bao giờ gõ vào pane đang có menu.

## English

### New
- Three step phone connect in Settings: install over USB or scan a QR, a connection test, Advanced settings folded away.
- The installer ships Orrery Remote: install over USB in one click or save the APK.
- Settings, Devices: phones, adb devices and emulators in one list; revoke with Undo.
- Answer agent questions right from the phone notification; a running notice with a timer; open a pane on the phone from the pane menu.
- Home screen widget and Quick Settings tile with the number of waiting sessions; long press pauses notifications for an hour.
- "Waiting for you" queue on the phone: answer one at a time, snooze 15 minutes or 1 hour, reply in text.
- "Since you were away" on the phone: done, waiting and failed counts after 30 minutes away.
- Saved prompts: type `;` in Ctrl+K, pane menu, Settings; inserted into the input, never sent by itself; shared with the phone.
- Peek a session from the rail: running tool, run time, pending question.
- Interface density: compact, regular, comfortable.
- Dangerous commands sent from the phone need fingerprint or PIN.
- Pre-merge checklist on the phone.
- Run on several Android devices at once with light, dark and large font variants and a screenshot grid.
- Send a freshly built APK to the phone; read the Run log and hand a crash to an agent from the phone.
- Shared clipboard between phone and computer, only on a tap, blocks text that looks like a secret.
- Dictate or take a photo into a pane from the phone.

### Fixes and safety
- Revoking a device always takes effect, even when its connection is busy.
- LAN APK download page: the pairing code works once and only for the device that downloaded the APK.
- A text reply from the phone never types into a pane that shows a menu.

## Chưa có trong bản này
- macOS: chưa thử bản 4.0.11 trên Mac (checklist `.planning/QA-mac-checklist.md`).
- Chưa thử trên A33: widget / ô Quick Settings, bấm action thông báo, cài APK từ điện thoại, hàng đợi, thẻ đi vắng, mic, ảnh.
- Spike F13 (mirror màn hình) và F14 (dev server trên điện thoại) để bản sau.
