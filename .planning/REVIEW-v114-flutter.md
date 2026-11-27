# Review v114 Flutter + shell prompts (sonnet, chỉ đọc, 27/11)

Commit: d1000472 (G1 phone), 54b88ea7 (G2 phone), 1cb63ab4 (G5). 1 CAO, 4 VỪA, 5 THẤP.

1. CAO, CHẮC. `ui/queue/queue_model.dart:160-163` `QueueFlow._update` trả false cho phase `sending`, `pendingUndo`, `snoozed`. Sau `snoozeUndo` 5 s driver `_retire` đặt `removed = true` nhưng phase vẫn `snoozed`; nhánh thêm slot mới chỉ có ở `sent` / `gone`. Hết hoãn 15 phút pane không quay lại queue. Cùng gốc: `undoSnooze` lỗi gọi `_retire` khi phase `sending`, slot kẹt cả phiên. Sửa: xử lý `removed` ở mọi phase: `removed && isQueueCandidate && !pane.snoozed` thì thay slot mới.
2. VỪA, CHẮC (luồng). `queue_driver.dart reply`: `ws.reply` timeout 8 s / lỗi mạng thì `_fail` gọi `flow.resume`, nút Gửi hiện lại dù desktop có thể đã submit. Bấm lại = gửi 2 lần. Sửa: timeout / offline giữ phase "chưa chắc", chờ snapshot (pane thôi asked / signature đổi) hoặc hiện "Có thể đã gửi" + Gửi lại.
3. VỪA, NGHI. `queue_driver.dart reply`: `await confirmDanger` chạy trước `flow.begin(sending)`, slot vẫn `ready`, bấm lần 2 thành 2 `ws.reply`. Sửa: cờ in-flight hoặc begin trước confirm, huỷ thì về ready.
4. VỪA, CHẮC. `remote/away_tracker.dart:37-40` `leave()` xoá summary nhưng `awayFilterProvider` không clear (chỉ `dismiss` clear); lần sau chip lọc cũ được chọn sẵn. Sửa: clear filter mọi máy trong `leave()`.
5. VỪA, CHẮC. `away_tracker.dart leave`: `store.save(marks)` ghi đè mark pending cũ (máy chưa fresh); summary chưa "Đã xem" bị clear, không thành pending. Sửa: giữ mark cũ (min leftAt, statuses cũ), summary chưa dismiss đưa lại pending.
6. THẤP, NGHI. `away_summary.dart:259` "Xong" gồm pane đang chạy lại; "Lỗi" gồm pane đã phục hồi. Đối chiếu spec G5.
7. THẤP, CHẮC. `ui/away/away_controller.dart:57` `_watchMachines` chỉ đăng ký máy có lúc `back()`; máy ghép sau không evaluate. Sửa: đăng ký lại khi danh sách máy đổi.
8. THẤP, NGHI. `prompts/prompts_controller.dart save/remove`: ViewerReply `pending` / `cancelled` coi là thành công. Sửa: coi là không thành công.
9. THẤP. `queue_model.dart:70 position` tăng khi skip (hiện 3/3 dù xử lý 1); `skippedCount` đếm slot đã `gone`; `_retire` dispose draft khi stage còn fade (rò nhẹ); `Dismissible` gỡ khi sending làm TextField mất focus.
10. THẤP, NGHI. `app/menu_action.dart runMenuAction`: chỉ reason `action` huỷ; Back hoặc toast khác đè (hide/remove) thì `ws.menu` vẫn gửi sau 3 s. Sửa: `hideCurrentSnackBar` khi dispose QueuePage, hoặc coi hide/remove là huỷ.

Rust `apps/shell/src/remote/prompts.rs`: không lỗi nặng. Nhẹ: `shaped` filled giống secret thì phone chèn body thô còn `{cwd}` không có `missing`; race fetch → set ở save/delete (bỏ qua được).

Đã soát không thấy lỗi: G1 pick qua undo 3 s, menuItemFor chặn other/submit/checked, danger guard + needsAuth retry 1 lần, vuốt không gây trả lời, desktop cũ không cap; vòng đời driver; G2 chèn không tự gửi, không cắt cụt, secret lọc ở list/save, cap prompts; G5 gap âm, ghi tmp rename, leftAt lúc paused, guard `_left`, evaluate chỉ feed fresh.
