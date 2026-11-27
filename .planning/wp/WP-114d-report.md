## WP-114d — DONE (phần Dart; chưa nhìn trên máy)

### Đã đổi
- `lib/ui/model/workspace_model.dart`: `PaneInfo.question` (max 300) và `snoozed` (parse field mới).
- `lib/remote/queue_cap.dart`: cap `queue`, mức hoãn 15 / 60.
- `lib/ui/queue/`: `queue_model.dart` (QueueFlow thuần: thứ tự theo `since`, state thẻ ready / sending / pendingUndo / snoozed / sent / stale / gone, resume, refresh, skip, review), `queue_driver.dart` (pick qua `runMenuAction` + `ws.menu`, `ws.reply` qua danger guard + retry `needsAuth`, `ws.snooze` + undo 5 s, xử lý lỗi), `queue_card.dart`, `queue_stage.dart` (vuốt, footer Hoãn / Bỏ qua / Mở pane), `queue_page.dart`, `queue_nudge.dart` (nút "Còn N đang chờ").
- `inbox_page.dart`: nút "Trả lời từng cái". `chat_home_page.dart`: nút "Chờ bạn (N)" ở header nhóm chờ (file thuộc `ui/chat`, thêm ~12 dòng vì đây là màn machine thật).
- `pane_link.dart` (`queue` flag, `m/<m>/queue`), `deep_link_host.dart` (mở queue; bọc route pane từ link bằng `QueueNudge`).
- ARB en + vi (28 khoá `queue*`) + `flutter gen-l10n`.
- Test: `queue_model_test.dart` (15), `queue_ui_test.dart` (28), `pane_link_test.dart` (+1).

### Tiêu chí
- Thứ tự since, mọi state + undo, stale, gone, ẩn theo cap, vuốt, deep link, nút Còn N, ô trả lời chữ + danger + lỗi: đạt (test).
- analyze sạch; không comment; file mới ≤ 400 dòng (page 248, stage 229).

### Audit
- Mất kết nối giữa thẻ: sync chỉ chạy khi link ready + có workspace + không stale; thẻ giữ nguyên, nút mờ; trước lần sync đầu hiện spinner (không báo "Xong hết" nhầm).
- Desktop cũ không cap: ẩn Hoãn, vuốt phải, ô chữ (hiện "Chọn trong pane"); menu vẫn trả lời.
- Pane đóng / hết chờ: ready và stale thành gone (1.5 s rồi bỏ); pendingUndo, sending, snoozed không bị coi là gone.
- Undo rồi pane hết chờ: `resume` ra gone; menu đổi: stale. Undo hoãn đặt `ignoreSnoozed` để cờ `snoozed` trễ không làm thẻ thành gone.
- Câu hỏi đổi khi đang gõ: stale, nháp giữ nguyên, "Xem câu hỏi mới" nạp lại.
- Giới hạn: hint vuốt chỉ nhớ trong 1 lần chạy app (chưa lưu bền, tránh sửa view_prefs); pane bị hoãn từ desktop khác cũng hiện "Đã trả lời trên máy tính".

### Review
- Không tự gửi gì: chỉ gửi khi bấm; undo 3 s dùng `runMenuAction` có sẵn.

### Test
- `flutter analyze` -> No issues found.
- `flutter test --concurrency=2` toàn bộ -> 615 pass, 1 fail (`narrow_layout_test` chat home 360dp x1.3: nút "Chờ bạn (N)" tràn 57 px). Đã sửa (Flexible + ellipsis); chạy lại các file liên quan (queue_ui, queue_model, narrow_layout, pane_link, chat_ui, inbox_model) -> 81 pass. Chưa chạy lại toàn bộ sau sửa.
- Gate `npm run qa`: bỏ qua, chỉ sửa Dart.

### Nhìn: CHƯA NHÌN
- Thẻ queue, vuốt, nút nổi "Còn N" (đặt top-right dưới header, có thể đè nội dung chat), nút ở Inbox / machine screen. Cần xem trên A33.

### Hot file cần thêm
- `.code_index/mobile.md` + `_meta.md`: module `ui/queue/*`, `remote/queue_cap.dart`, PaneInfo field mới.
- Docs remote-integration: phone dùng `ws.snooze`, `ws.reply`, cap `queue`, link `m/<m>/queue`.
- `shots/06-tablet-split.png`, `08-inbox.png`, `10-confirm.png`, `11-pane-actions.png` bị test ghi đè: main revert (08-inbox đổi thật vì có nút mới nếu muốn giữ).

### Ngoài phạm vi
- Rust, web, `pubspec.yaml`; lưu bền hint; E2E.

### Commit message đề xuất
feat(mobile): add the one-by-one waiting queue with snooze, text reply, deep link and more-waiting nudge
