## WP-114r1 — DONE

11/11 finding SỬA (không BỎ). Mỗi lỗi có test tái hiện; F1, F2, F8, F9, F11 đã chạy fail trước rồi pass (F5/F4 Rust fail do compile trước khi có `dropped`); F3, F7, F10 đổi test cũ đang khẳng định hành vi sai.

### Bảng finding

| # | Mức | Kết quả | Sửa ở đâu | Test |
|---|---|---|---|---|
| 1 | CAO | SỬA | `useRemotePhone.ts reply()`: từ chối `useMenu` khi có BẤT KỲ entry queue; `permission` -> `useMenu`; thiếu `question` -> `notWaiting`; `statusSince` thiếu hoặc < 1.5 s (`REPLY_STABLE_MS`) -> `busy`. `TermPane.vue prefillReady()` + watch: không gõ khi `termMenu.menu` / `termMenu.busy`. Shell: `workspace.rs reply_refusal` + `phone_ws.rs` từ chối `ws.reply` khi summary của pane không có `question` hoặc có `menu` | `useRemotePhone.test.ts` (reply never types while any menu entry exists..., reply refuses a permission wait a fresh wait...), `ws_tests.rs a_reply_needs_a_pane_with_a_question_and_no_menu` |
| 2 | VỪA | SỬA | `replied()` trả `sent / gone / replaced / timeout`: pane mất hoặc không còn `running` -> `notFound`; prefill bị thay -> `busy` (không xoá prefill của người khác) | `reply is not sent when the pane vanishes or another prefill replaces its text`, `...pane ended and its prefill was dropped` |
| 3 | VỪA | SỬA | `insertPrompt.ts`: pane `waiting` -> `reason: 'waiting'`, `error` / `unknown` -> `busy` (chỉ `idle` / `working` / chưa có session được chèn). `useInsertPrompt.ts` bỏ toast cũ, `saved-prompts.ts` đổi chữ `insert.waiting` (en + vi, key có sẵn nên snapshot i18n không đổi) | `insertPrompt.test.ts` (refuses an agent that waits..., refuses an agent in a state that never types) |
| 4 | VỪA | SỬA | Rust `clean_body` loại mọi ký tự điều khiển trừ `\n` `\t` (sau khi chuẩn `\r`); web `promptModel.ts` `hasControlChars` / `stripControlChars`, `fitsDraft` (kéo theo `parseImport`) từ chối; `TermPane.vue sendPrefill` strip trước khi `conn.write` / `paste` | `prompts_tests.rs control_characters_are_refused...`, `promptModel.test.ts control characters` |
| 5 | VỪA | SỬA | `prompts.rs`: `parse` đếm `dropped` (mục lỗi + trùng + vượt max); `set` copy file gốc sang `prompts.json.bak-<ms>` trước khi ghi, copy lỗi -> `io`; reply có `recovered: true`. File hỏng hẳn: rename sang `.bak-<ms>` (không đè bản cũ) | `dropping_rows_keeps_the_original_file_as_a_timestamped_backup...`, `a_second_broken_file_never_overwrites_the_first_backup`, `a_broken_file_is_moved_to_bak...` (cập nhật) |
| 6 | THẤP | SỬA | `util.rs write_atomic`: `File::create` + `write_all` + `sync_all` trước `rename`; `.bak` có timestamp (F5) | gộp trong test F5; `sync_all` không kiểm hành vi được ngoài ghi đúng nội dung và không sót tmp (`writes_are_atomic_and_leave_no_temp_file`) |
| 7 | THẤP | SỬA | `savedPrompts.ts migrate()`: merge `ccm.prompts` cũ theo `title + body`, cắt theo chỗ trống `MAX_ITEMS`, trùng hết thì chỉ đánh dấu đã migrate | `savedPrompts.test.ts` (merges the old list..., marks the old list migrated...) |
| 8 | THẤP | SỬA | `inbox.ts snoozePane`: không còn mục inbox thì lấy kind từ terminal (`error` -> `error`, permission -> `permission`, còn lại `question`) | `useRemotePhone.test.ts a repeated snooze of a failed pane keeps the error max...` |
| 9 | THẤP | SỬA | `useRemotePhone.ts snoozeOp`: lưu undo của lần hoãn gần nhất theo `paneKey` (`railSnooze.undo` + khôi phục mục core trước đó theo số phút còn lại); `minutes: null` trong 30 s dùng undo, quá 30 s hoặc chưa có thì xoá như cũ | `ws.snooze hides the waiting pane...` (cập nhật), `undo ... long after the snooze clears` |
| 10 | THẤP | SỬA | `push_extra.rs queue_pane`: chỉ chặn `attention` / `question` / `permission` của pane `snoozed`; `done`, `error`, `open` đi bình thường | `push_watch_tests.rs a_snoozed_pane_gets_no_reminder_push_but_done_error_and_open_still_go_out` |
| 11 | THẤP | SỬA | `RailRowItem.vue onKey`: `preventDefault` rồi bỏ qua khi `e.repeat` | `RailPeek.test.ts ignores the auto repeat of a held i key...` |

### Audit

- Không comment mới trong code; file không vượt 400 dòng (`useRemotePhone.ts` ~310, `prompts.rs` ~340).
- Không đụng `apps/shell/src/remote/prompts.rs`, `viewer_map.rs`, `apps/mobile/*`.
- Thay đổi hành vi cần chủ nhân biết: (a) chèn prompt vào pane đang `waiting` bị từ chối thay vì xếp hàng (F3); (b) `ws.reply` giờ cần summary có `question` và pane chờ đủ 1.5 s, nên phone có thể nhận `busy` / `notWaiting` ngay sau khi câu hỏi vừa hiện, gửi lại sau ít giây là được (F1); (c) toast hoãn "Undo" từ phone khôi phục mức hoãn trước (F9); (d) `recovered` của prompts giờ cũng trả ở reply của `ccm:prompts-set`, chữ `savedPrompts.recovered` đã sửa nhắc `prompts.json.bak-<time>`.
- Giới hạn F1: không có tín hiệu "termMenu đang scan" ra ngoài `TermPane`; phần này chặn ở `TermPane.prefillReady` (menu / busy) cộng cửa sổ ổn định 1.5 s, không làm ở handler. Core principle: queue entry + permission + thiếu question đều từ chối, không đoán.
- Flaky đã thấy: `useRemotePhone.test.ts` test đầu hay quá hookTimeout 10 s khi máy bận (cần `--hookTimeout=60000`), 1 lần `rename and move change state only` fail rồi pass lại 3 lần liền (không liên quan thay đổi).

### Test

- `npm test -w @ccm/web -- src/features/remote src/features/prompts src/features/click-menu src/features/next src/features/workspace --hookTimeout=60000` -> 123 file pass, 2 fail do timeout 5 s khi máy tải (`RemotePairPanel.test.ts`, `rowWindow.test.ts`); chạy lại 2 file đó + `RailPeek.test.ts` với `--testTimeout=60000` -> 3 file, 22 test pass.
- `npm test -w @ccm/web -- src/i18n src/importRules.test.ts` -> 4 file, 10 test pass (snapshot i18n không đổi).
- `npm run typecheck` -> sạch (không lỗi).
- `cd apps/core; cargo test --locked -j 4 -p core-desk prompts` -> 19 passed (chạy lại sau khi viết lại test Rust, exit 0).
- `cd apps/shell; cargo test --locked -j 4 remote` -> 385 passed, 3 failed trước khi sửa test F10 (1 của mình, đã sửa); lần chạy sau: chỉ còn `viewer_map::prompts::tests::*` fail (WP-114g, ngoài phạm vi: `a_big_library_is_cut...`, `op_arguments_are_checked...`, `a_phone_manages_prompts_end_to_end...`). Chạy riêng: `a_snoozed_pane` 1 pass, `a_reply_needs` 1 pass, `ws_tests` 12 pass, `danger` 6 pass.
- Lưu ý chéo WP-114g: các test `viewer_map::prompts` đọc `prompts.json`; sau thay đổi của mình `recovered` và `.bak-<ts>` có thể ảnh hưởng `a_phone_manages_prompts_end_to_end...` (assert `prompts_tests.rs:333` thấy reply `{"ok":true,"id":...}`), nên main nên chạy lại sau khi 114g xong.

### Hot file cần thêm

- Không cần thêm. Đã sửa ngoài danh sách phạm vi chỉ: `apps/shell/src/remote/workspace.rs` (thêm `reply_refusal`), `ws_tests.rs` (thêm test), `push_watch_tests.rs` (đổi test cũ của F10) - cả ba thuộc chỗ xử lý ws.reply / test push mà brief cho phép.

### Commit message đề xuất

```
fix(remote): reply refuses any menu, fresh or question-less waits; undo restores prior snooze; push only skips reminders for snoozed panes
fix(prompts): strip control chars, keep timestamped backup when rows are dropped, merge legacy list, refuse insert into waiting pane
```
