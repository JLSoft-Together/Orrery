# WP-26 QA máy thật A21s (R58N661JMPA) - KHÔNG ĐO ĐƯỢC

Kết quả: 0/5 lần đo. Không có số dòng thật. Dừng trước khi chạy `1..3000` vì không có cách đọc buffer terminal điện thoại mà không sửa code / build.

## Đã kiểm
- App dev.ccm.orrery.remote foreground, đã mở khoá, ghép DESKTOP-SMCL7FD (LAN, 1 pane `ccm-device-proj-BrcNjh`), mở được màn pane bằng `adb input tap`.
- `uiautomator dump` (script `dump.sh`) đọc được cây UI: nút, tiêu đề, thanh phím. Nội dung terminal KHÔNG có trong cây (xterm TerminalView vẽ canvas, không có Semantics) -> không đọc được dòng nào.
- `screencap` / `screenrecord`: ảnh rỗng do FLAG_SECURE (đã nêu trong brief).
- Nút "Đo terminal" (`home_page.dart:46`, `onOpenSpike`) mở `src/spike/spike_page.dart`, là spike cũ: chỉ `debugPrint('SPIKE ...')` về luồng riêng của trang spike, không đếm buffer của `terminal_pane`.
- Không có log đếm dòng trong app (logcat sạch về pty/overflow); gateway `phone_pty.rs:63,102` chỉ gửi `pty.overflow` qua kênh, không log ra file.
- Pane hiện "Chỉ xem": gõ từ điện thoại không dùng được (không đổi remoteEnable theo luật), phải gõ `1..3000` từ phía dev app.
- App build debug nhưng chạy bằng adb nên không có Dart VM service URI (log khởi động đã trôi); mở lại app sẽ bật khoá app, không thử.

## Cần để đo thật (main chọn)
1. Thêm hook tạm trong `terminal_pane.dart`: sau khi stream kết thúc, log (`debugPrint`) số lần mỗi số 1..3000 xuất hiện trong `terminal.buffer` + số `pty.overflow` / reattach; build APK debug, cài, rồi đọc `adb logcat -s flutter`. Hoặc
2. Cho một cờ debug tắt FLAG_SECURE tạm để dump được ảnh từng trang cuộn, hoặc
3. Chạy dev (`flutter run`) để dùng VM service đọc buffer.
Không ai trong 3 cách làm được dưới luật "không sửa code / không build" của lượt này.

## Lượt 2: hook đếm dòng
- Thêm `apps/mobile/lib/ui/terminal/wp26_probe.dart` (`LineProbe`, chỉ chạy `kDebugMode`): log `WP26 event <type>` mỗi event pty.* (gồm pty.overflow / pty.ready) và 2 s sau lần ghi cuối `WP26 lines=.. numbers=.. first=.. last=.. missing=[..] dup=[..]`. Nối vào `terminal_pane.dart` (`_flushOut`, `_onEvent`, `dispose`). Test `test/line_probe_test.dart`. analyze sạch.
- Build debug arm64, `adb install -r` lên A21s xong. Chưa mở app (lúc đó máy đang ở Cài đặt Wi-Fi, app khác foreground); app khởi động lại sẽ ở màn khoá -> cần chủ nhân mở khoá trước khi đo 5 lần.
- Đọc log: `adb -s R58N661JMPA logcat -s flutter | grep WP26`.

## Lượt 3: đo thật qua Relay (A21s, debug APK có LineProbe)
- Điều kiện: app Orrery Remote đã mở khoá, desktop `state=connected`, kết nối Relay (màn home ghi "Relay"), pane shell PowerShell `Chỉ xem`.
- Cách đo: `trial.sh N` - `cls`, rồi gõ lệnh vào pty của pane từ desktop (`window.__wp26` qua `/eval`, ws attach own + byte input), trong lúc chảy chạy 12 (lần 1) / 40 (lần 2-5) cặp `adb shell input swipe` lên/xuống trên vùng terminal, đợi 6 s, đọc `LineProbe` trong logcat (`trialN.log`).
- Lần 1: `1..3000` thẳng. Lần 2-5: `1..3000|%{$_;if($_%25 -eq 0){Start-Sleep -m 15}}` (chảy chậm hơn để swipe chồng lên lúc output).
- Kết quả (mỗi lần): `lines=3005 numbers=3000 first=1 last=3000 missing=[] dup=[]` -> 5/5 đủ 3000 dòng, không thiếu, không trùng (`trial1.log`..`trial5.log`).
- `pty.overflow`: 0 lần (logcat không có event `WP26 event pty.overflow`, grep overflow = 0); không có `pty.ready` giữa chừng.
- Giới hạn: chưa gây được overflow thật, nên đường `_reattach` vẫn chỉ được chứng minh bằng test giả; ảnh màn hình không có (FLAG_SECURE), bằng chứng là log đếm buffer.
- Kết luận: giữ `LineProbe` (chỉ `kDebugMode`, 1 file + test).
