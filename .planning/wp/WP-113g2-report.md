## WP-113g2 — DONE (phần nhìn trên máy thật: CHƯA NHÌN)

### Đã đổi
Phone (`apps/mobile`):
- `lib/push/push_menu.dart` (mới): parse `menu` (max 3 mục, bỏ trùng / index sai, nhãn bỏ ký tự điều khiển + bidi, cắt 24), action id `menu:<i>` / `open`, `tapDataWithAction`.
- `lib/push/local_notifier.dart`: action của thông báo hỏi (tối đa 3, `showsUserInterface: true`, `cancelNotification: false`); `running` = ongoing + `usesChronometer` từ `t0` + channel `running` (low, không rung, không âm) + `timeoutAfter` 6h; `done` có pane có nút Mở; tạo channel `running` ở `init`; `clearMachine` huỷ thông báo có tag `<m>:` (bỏ qua channel running); `onTap` / `launchPayload` gộp `actionId` vào payload (kể cả app bị kill lúc bấm).
- `lib/push/push_notice.dart`, `push_seal.dart`: thêm kind `running`, `open`; `PushNotice.menu / startedAt / labels / ongoing`; seal đọc `menu` và `wl`; menu chỉ giữ cho `question|permission|attention`; payload thêm `sig`.
- `lib/push/push_present.dart`: `clear` gọi `gateway.clearMachine`; `wl` trong seal đưa vào widget (113k chỉ đọc `wl` ngoài seal); `open` luôn show (kể cả đang mở session và đang tạm dừng).
- `lib/push/push_background.dart`, `push_gateway.dart`: `clear` huỷ thông báo hỏi của máy; `running` kèm `notification` (plain) vẫn được show lại local để thành ongoing + chronometer; `open` không bị `pauseUntil` chặn.
- `lib/push/push_host.dart::pushLink`: tap action menu thành `orrery-remote://m/<m>/p/<p>?menu=<sig>&i=<n>` (không có action thì giữ link cũ).
- `lib/app/pane_link.dart`: `PaneLink.menuSig / menuIndex` (so sánh + dedupe tính cả hai).
- `lib/app/menu_action.dart` (mới) + `lib/app/deep_link_host.dart`: sau khi qua khoá app (held link cũ) và pane hiện ra: kiểm `sig` với menu hiện tại của pane (`PaneInfo.menu`); lệch / mất menu thì snackbar "Câu hỏi đã đổi, mở pane để xem" (pane đã được mở); khớp thì snackbar "Đã chọn X" + Hoàn tác 3s (dùng lại `chatPicked`, `chatUndo`), hết 3s mới gửi `ws.menu {paneKey, sig, index}` qua `machineApiProvider`. Lỗi `stale` / `busy` từ desktop cũng ra thông điệp stale. `persist: false` bắt buộc (SnackBar có action mặc định không tự đóng, test đã bắt lỗi này).
- ARB en + vi + `flutter gen-l10n`: `pushFixedRunning*`, `pushFixedOpen*`, `pushRunningChannel`, `pushActionOpen`, `pushMenuStale`.
- Test: `test/push_actions_test.dart` (24 test), `test/support/seal_maker.dart` (tạo seal thật để test menu / wl), fakes `clearMachine` trong `test/support/push_fakes.dart`, `test/push_present_test.dart`.

PWA (`apps/pwa`):
- `public/push-sw.js`: `actions` (tối đa 3, theo `Notification.maxActions`) chỉ khi payload có `menu` hợp lệ và kind attention/question/permission, không phải WebKit; `data.sig`; click action: app đang mở thì `postMessage {type:'orrery-open', url, menu:{sig,index}}`, chưa mở thì `openWindow(url?from=push&menu=<sig>&i=<n>)`; `running` / `open` hiện như thông báo thường.
- `src/lib/deepLink.ts`, `src/stores/push.ts`, `src/views/ChatView.vue` (ngoài danh sách file ban đầu, cần để PWA gửi được `ws.menu`): giữ lựa chọn chờ, khi pane có menu cùng `sig` thì gọi `pick` có sẵn (undo 3s của chat PWA); lệch `sig` thì nói "Câu hỏi đã đổi".
- Test: `src/push/push.test.ts` (+3), `src/lib/chat.test.ts` (+1).

### Tiêu chí
- [x] Dart test: parse menu (max 3, nhãn cắt), action thành ws.menu sau undo (fake), stale (sig lệch, không có menu, lỗi `stale`), running ongoing + chronometer, done thay running cùng tag, open thành link, clear huỷ (foreground + background).
- [x] PWA: vitest; `actions` chỉ khi có menu (kiểm cả done, sig sai, WebKit).
- [x] `flutter analyze` sạch; ARB en + vi, `arb_no_dots_test` xanh; không comment; file mới / sửa đều < 400 dòng (`push_service.dart` 419 dòng không đụng).
- [ ] Bấm thật trên A33: CHƯA KIỂM.

### Audit
Diff chỉ đụng: `apps/mobile/lib/push/*`, `lib/app/{pane_link,menu_action,deep_link_host}.dart`, ARB + l10n sinh lại, test mobile; `apps/pwa/public/push-sw.js`, `src/{lib,stores,views,push}`. Không đụng Rust, `apps/web`, `lib/ui/chat/*`, `input_bar.dart`, `lib/security/*`, `lib/ui/checklist/*`, Kotlin, `pubspec.yaml`. `git status` có file của WP khác (shell, web) không phải của gói này.

### Review (edge case)
- Desktop cũ thiếu field: không `menu` / `t0` / `wl` / `open` thì đi đường cũ (test "old push").
- Seal tắt hoặc content tắt: không có menu thì không có action; body chạm vẫn mở pane.
- Khoá app: action chỉ thành link giữ trong `_held`, gửi sau khi mở khoá; link `expired` thì khoá lại trước. Khoá bị huỷ / không mở thì không gửi gì. Không gửi từ nền (action `showsUserInterface: true`, gửi ở main isolate sau khi UI mở).
- App bị kill khi bấm: `launchPayload` mang `actionId`, đi qua `initialOpen` thành link có menu.
- Menu stale: kiểm `sig` trước khi hiện Hoàn tác, và kiểm lại bằng lỗi `stale`/`busy` của desktop (contract: desktop hiện trả `busy`).
- Running không có done: `timeoutAfter` 6h tự gỡ; `clear` không huỷ thông báo running.
- Menu nhiều ô (`checked`) / `other` / `submit`: bị bỏ khỏi `menuItemFor`.
- Giới hạn đã biết: (1) `running` gửi dạng plain mà app bị kill hẳn thì hệ thống tự hiện bản thường (không chronometer); (2) snackbar bị snackbar khác ẩn sớm vẫn gửi `ws.menu` (chỉ bấm Hoàn tác mới huỷ); (3) bấm action không xoá thông báo (`cancelNotification: false`), nó ở lại tới khi `clear` / thay thế / vuốt bỏ.

### Test
- `cd apps/mobile; flutter analyze` → No issues found.
- `flutter test --concurrency=2 test/push_actions_test.dart` → 24 xanh.
- `flutter test --concurrency=2` (toàn bộ) → All tests passed (475 xanh, 63 skip); đã trả lại các ảnh `apps/mobile/shots/*.png` bị test ghi đè.
- `cd apps/pwa; npx vitest run` → 310 passed; `npx vue-tsc --noEmit -p tsconfig.json` → không lỗi; `node --check public/push-sw.js` → ok.

### Nhìn
CHƯA NHÌN: thông báo thật trên A33 (3 nút hỏi, chronometer ongoing, nút Mở sau done, snackbar Hoàn tác sau khi mở khoá), và Chrome Android cho PWA actions. Cần desktop 113g1 gửi push thật.

### Hot file cần thêm
- `.code_index/mobile.md` (`apps/mobile/.code_index/mobile.md`) và `_meta.md`: thêm `push_menu.dart`, `app/menu_action.dart`, `PaneLink.menuSig`.
- `docs/` (push / remote-integration): link `orrery-remote://m/<m>/p/<p>?menu=<sig>&i=<n>`, action `menu:<i>`, channel `running`.
- Desktop 113g1: trả `stale` thay `busy` khi `sig` lệch (phone đã chấp nhận cả hai).

### Ngoài phạm vi
`push_service.dart` 419 dòng (không đụng). Không build APK / E2E.

### Commit message đề xuất
feat(remote): phone and PWA answer menu questions from notification actions, ongoing running notice, open and clear pushes
