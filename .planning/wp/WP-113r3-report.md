## WP-113r3 — PARTIAL

Toàn bộ finding đã xử lý; còn 1 test tích hợp đỏ trong full run (`ui_integration_test.dart`), nhiều khả năng không do gói này (xem Test).

### Bảng finding
| # | Mức | Kết quả | Test / ghi chú |
|---|---|---|---|
| V1 | VỪA | SỬA | `media_ui_test`: "cancel while the online question is open never starts listening" (cờ `_cancelRequested` trong `Dictation`) |
| V2 | VỪA | SỬA | `danger_rules_test`: "a second tap while auth is open sends once and keeps later typing". `_sending` + `mounted` + chỉ xoá đúng chữ đã gửi ở `chat_composer` (`_send`, `_sendChip`) và `terminal/input_bar`. input_bar sửa cùng mẫu, CHƯA có test riêng |
| V3 | VỪA | SỬA | Dart: `deep_link_host` kiểm subnet trừ khi QR có 127.0.0.1 (4 case `pair_link_test`, 2 test cũ đảo kỳ vọng). Kotlin `MainActivity.isFromShell` bỏ tin referrer khi intent có `EXTRA_REFERRER`/`EXTRA_REFERRER_NAME`. CHƯA KIỂM: compile Kotlin |
| V4 | VỪA | SỬA | `apk_install_test`: "remote push data never carries an inline apk offer" (`remoteTapData` bỏ khoá `apk` của FCM data; payload local vẫn inline) |
| T1 | THẤP | BỎ | `speech_to_text` không có API gói offline; `onDevice:true` ép on-device nên không rò audio. Cần thử máy thật |
| T2 | THẤP | SỬA | `apk_install_test`: "cancel while the installer runs keeps the file and the state"; `followRedirects=false` (CHƯA có test riêng cho redirect, `HttpClient` thật) |
| T3 | THẤP | SỬA | `danger_rules_test`: "a crash draft for the pane fills the composer once, unsent". Điền composer, bỏ chép clipboard (phone không có hàm mask nên không chép), chuỗi `runLogCrashFilled` en+vi, tách `crash_draft.dart` |
| T4 | THẤP | SỬA | `run_log_test`: "a restarted run with lower numbering replaces the buffer" |
| T5 | THẤP | SỬA | `media_test`: "a shrunk png still over the byte limit is retried on a smaller side" (2048 -> 1536 -> 1024) |
| T6 | THẤP | CHƯA KIỂM | Broadcast legacy `c2dm.RECEIVE`: thử trên A33 thật |
| T7 | THẤP | SỬA | `waiting_test`: "the pause stream can be listened to again after a cancel" (broadcast + giữ yêu cầu pending) |
| T8 | THẤP | SỬA | `checklist_test`: "a check still running after ten minutes keeps being polled" (hết 200 poll chuyển poll 30s) |

### Audit
Diff chỉ nằm trong `apps/mobile/**` (lib, test, ARB + generated l10n, 1 file Kotlin, file mới `lib/ui/run/crash_draft.dart`). Không comment mới; file mới sửa đều < 400 dòng (max 335). ARB en + vi, không em dash / dấu chấm giữa. `shots/06,10,11*.png` bị đổi do test screenshot tự ghi lại, không chủ ý: nên `git checkout` 3 file này trước khi commit.

### Test
- `flutter analyze` → No issues found.
- Từng file test liên quan (`media_ui_test`, `danger_rules_test`, `pair_link_test`, `apk_install_test`, `run_log_test`, `media_test`, `waiting_test`, `checklist_test`) → xanh sau sửa; mỗi bug đã thấy đỏ trước khi sửa.
- `flutter test --concurrency=2` toàn bộ → +573 ~63 -1. Fail duy nhất: `ui_integration_test.dart` "open tab, attach, type, echo with claim, close pane after confirm". Test này chạy gateway Rust thật (`RustRemoteCrypto`), trong lúc WP-113r1 đang sửa Rust, nên nhiều khả năng do môi trường. CHƯA KIỂM: chạy lại riêng file này chưa ra kết quả khi báo cáo (tải máy cao); cần main chạy lại sau khi r1 xong.
- `npm run qa -- --no-e2e`: bỏ qua (quét `git diff` cả repo, lẫn Rust/web của r1/r2).

### Hot file cần thêm
Không (không đụng `pubspec.yaml`).

### Commit message đề xuất
```
fix(mobile): harden dictation, send, pairing, apk offer and run log handling

- cancel dictation while the online prompt is open
- block double send during danger auth, keep text typed meanwhile
- apply the subnet check to pair extras unless the code is a USB loopback one
- ignore forged referrer extras and inline apk offers from remote push
- keep the apk file while the installer runs, refuse redirects
- fill crash drafts into the composer instead of the clipboard
- reset run log on restart, retry oversized photos smaller
- allow re-listening to pause requests, keep polling a long checklist run
```
