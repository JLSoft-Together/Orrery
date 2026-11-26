# Review v113 Flutter / Kotlin (reviewer sonnet, chỉ đọc, 27/11)

0 CAO, 4 VỪA, 8 THẤP.

## VỪA
1. CHẮC. Mic còn nghe sau khi rời màn: `lib/ui/chat/chat_media.dart` ~232, `lib/media/dictation.dart` ~58-70. `MicButton.dispose` gọi `Dictation.cancel()` khi `start()` còn await (`engine.init` / hộp thoại `askOnline`), `_listening` chưa bật nên cancel không làm gì; sau đó `start()` vẫn `engine.listen` tới 60s không có UI. Sửa: cờ `_cancelRequested`, kiểm trước `engine.listen`.
2. CHẮC. Gửi 2 lần khi đang xác thực danger: `chat_composer.dart` `_send`, `_sendChip`; `terminal/input_bar.dart` `_send`. Không cờ in-flight quanh `await confirmDanger`; sau await `_text.clear()` xoá chữ user gõ thêm; `_sendChip` không kiểm `mounted`. Sửa: `_sending` + `mounted`, chỉ xoá đúng chữ đã gửi.
3. NGHI. Extra `pairQr` từ app khác bỏ qua kiểm mạng: `MainActivity.kt` `takePairIntent` / `isFromShell`, `deep_link_host.dart`. Activity exported nhận extra từ bất kỳ app; `fromLink=false` nên không kiểm subnet; `referrer` lấy từ `EXTRA_REFERRER` giả được "com.android.shell" → nhãn "USB". Vẫn cần user bấm xác nhận + desktop "Cho phép". Sửa (main chốt): bỏ tin `EXTRA_REFERRER`, chỉ dùng `Activity.getReferrer()` để chọn nhãn; extra chỉ được miễn kiểm subnet khi danh sách LAN trong QR có `127.0.0.1` (đường USB adb reverse), còn lại áp kiểm subnet như link.
4. NGHI. Offer APK không niêm phong vẫn nhận: `push/push_apk.dart` ~22-26, `push_service.dart` `_onOpened`. `apkTapOf` nhận `data['apk']` nguyên văn từ FCM, không cần `e` (sealed). Sửa: offer từ FCM bắt buộc sealed; chỉ payload local tự tạo được inline.

## THẤP
1. NGHI. `dictation.dart` `offlineFor` dựa `speech.locales()` (ngôn ngữ của recognizer), không phải gói offline đã tải → có thể bỏ qua hộp thoại đồng ý online.
2. CHẮC. `apk_flow.dart` `ApkInstallFlow.cancel()` xoá file khi đang `installer.install`; `HttpClient` chưa tắt `followRedirects` (redirect ra ngoài LAN, SHA vẫn chặn).
3. CHẮC. `run/crash_send.dart` `crashDraftProvider.put` không có nơi `take` → draft không vào composer; text crash chưa che secret nằm lại state / clipboard.
4. NGHI. `RunLogBuffer.add` lọc `l.n > seen`; desktop reset đánh số khi chạy lại → bỏ hết dòng mới.
5. NGHI. `shrinkToPng` biến ảnh quá cỡ thành PNG → dễ vượt 8 MB.
6. NGHI. `WaitingPushReceiver` dựa broadcast legacy `c2dm.RECEIVE` (thử trên máy thật).
7. NGHI. `waiting_sink.dart` `ChannelWaitingSink._requests` single-subscription không đóng → mount `PausePromptHost` lần 2 ném `StateError`.
8. CHẮC. Checklist ngừng poll sau 10 phút dù state vẫn `running`.

## Đã soát, không thấy lỗi
Action thông báo `showsUserInterface:true`, chờ app lock; SHA-256 + size kiểm trước `install`; `FileProvider` chỉ lộ `cache/apk_offer/`, `inside()` kiểm canonical; PendingIntent `FLAG_IMMUTABLE`, API 34 `startActivityAndCollapse` đúng; dictation không tự gửi; MethodChannel bắt lỗi; dispose / `mounted` của run log, checklist, clip, `ApkInstallHost`.
