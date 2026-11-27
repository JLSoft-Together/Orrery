## WP-114r2 — DONE
- Đã đổi:
  - apps/mobile/lib/ui/queue/queue_model.dart — phase `unsure`, `canResend`, `_replace` (thay slot removed mọi phase), `position` đếm slot đã xử lý, `skippedCount` bỏ slot gone
  - apps/mobile/lib/ui/queue/queue_driver.dart — `_replying` chặn double-submit, timeout/offline vào `unsure`, draft retire dispose muộn, truyền `proceed: isMounted`
  - apps/mobile/lib/ui/queue/queue_card.dart — body cho phase `unsure` (dùng errTimeout/errOffline sẵn có + ô trả lời + nút Gửi)
  - apps/mobile/lib/ui/queue/queue_stage.dart — Dismissible luôn có (direction none khi không vuốt được), nút mở pane cho `unsure`
  - apps/mobile/lib/ui/queue/queue_page.dart — gỡ snackbar (scheduleMicrotask, removeCurrentSnackBar) khi rời trang
  - apps/mobile/lib/app/menu_action.dart — tham số `proceed`, huỷ gửi khi page đã dispose
  - apps/mobile/lib/remote/away_tracker.dart — `leave()` giữ mark pending (min leftAt, statuses cũ), summary chưa dismiss thành pending (`_shown`)
  - apps/mobile/lib/ui/away/away_controller.dart — `leave()` invalidate awayFilterProvider; `ref.listen(machinesProvider)` đăng ký lại máy mới
  - apps/mobile/lib/prompts/prompts_api.dart — pending/cancelled ném ViewerFailure
  - test: queue_model_test, queue_ui_test, away_summary_test, prompts_test, away_filter_test (mới)
- Finding:
  | # | Kết quả |
  |---|---|
  | 1 | SỬA (queue_model_test: snoozed card retired is replaced, retired while sending is replaced; queue_ui_test: snoozed card that expires) |
  | 2 | SỬA (queue_model_test: unsure card waits; queue_ui_test: reply that times out is unsure) |
  | 3 | SỬA (queue_ui_test: tapping send twice during the danger check sends once) |
  | 4 | SỬA (away_filter_test) |
  | 5 | SỬA (away_summary_test: leaving again before fresh keeps old mark, unseen card pending again, dismissed card) |
  | 6 | BỎ (cần đối chiếu spec G5, không đủ bằng chứng là lỗi) |
  | 7 | SỬA (chưa có test: cần fake RemoteHost đổi danh sách máy) |
  | 8 | SỬA (prompts_test: writes that the desktop did not finish) |
  | 9 | SỬA (queue_model_test: position, skipped count; test cũ đổi position 2 thành 1; Dismissible và draft sửa không test riêng) |
  | 10 | SỬA (queue_ui_test: leaving the queue page drops a pending menu answer) |
- Test: `flutter analyze` sạch; `flutter test --concurrency=4` → 686 pass / 0 fail (63 skip có sẵn). Lưu ý: chưa chạy bản code cũ để xác nhận từng test đỏ trước.
- Hot file cần thêm: key `queueMaybeSent` en "It may have been sent. Send again only if it did not arrive." / vi "Có thể đã gửi. Chỉ gửi lại nếu chưa tới." (tạm dùng errTimeout/errOffline ở queue_card.dart phase unsure; đổi sang key mới nếu main thêm)
- Commit message đề xuất: fix(mobile): queue slot replace after snooze, unsure reply state, double-submit guard, away mark and filter on leave, prompts write failures, snackbar cleanup
