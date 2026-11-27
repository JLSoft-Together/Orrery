## WP-114e — DONE (chưa nhìn trên máy thật)

### Đã đổi
- Mới `apps/mobile/lib/remote/away_summary.dart`: `AwayMark`, `markFromSnapshot`, `summarizeActivity` (port web), `computeAway`, `awayDue`, `AwaySummary` (giữ tên trường web + `failed`, `partial`).
- Mới `lib/remote/away_store.dart` (file `away_marks.json` trong app support, ghi tmp + rename, cùng cơ chế `view_prefs`, không thêm package) và `lib/remote/away_tracker.dart` (leave / back / evaluate / dismiss, IO tuần tự).
- Mới `lib/ui/away/`: `away_controller.dart` (Riverpod `awayProvider`, `awayFilterProvider`, `awayIdsProvider`), `away_card.dart`, `away_chips.dart`, `away_slot.dart`.
- Sửa `lib/ui/chat/chat_home_page.dart` (+9 dòng: `AwaySlot` dưới tiêu đề, lọc hàng theo `awayIds`), `lib/main.dart` (+~14 dòng: `leave()` ở paused / hidden / detached, `back()` ở resumed, khởi tạo notifier sau frame đầu để xử lý mark sau khi app bị kill).
- Mới test `test/away_summary_test.dart`, `test/away_card_test.dart`; cập nhật `apps/mobile/.code_index/mobile.md`.

### Tiêu chí (mục Test G5 của spec)
- Dưới 30 phút không hiện: PASS (`awayDue`, tracker "back within 30 minutes", mark bị xoá).
- Đủ 30 phút, số đúng từ activity mẫu: PASS (`computeAway exact numbers`).
- Buffer bị cắt, nhánh chênh lệch + nhãn: PASS logic (`partial` + done từ chênh lệch); nhãn UI `awayPartial` CHƯA KIỂM (chờ ARB).
- `leftAt` qua kill app: PASS (tracker mới đọc mark từ store).
- Chip lọc: logic `awayIdsProvider` + lọc ở `chat_home_page`; widget test viết sẵn, CHƯA CHẠY (chờ ARB).
- Ẩn sau khi xem: PASS logic (`dismiss`, leave mới xoá thẻ cũ); nút "Đã xem" test UI chờ ARB.
- "Cuộn qua thì ẩn": KHÔNG làm (xem Ngoài phạm vi).
- Nhiều máy: mỗi máy 1 thẻ, PASS (tracker 2 máy).

### Audit
- PHÁT HIỆN QUAN TRỌNG: spec / review V11 nói `terminal.status` có `data.from/to` cho working→idle / error. Thực tế core chỉ phát `terminal.status` với `to: "waiting"` (`core-monitor/src/monitor/reconcile.rs:199`; golden fixture chỉ có `to: waiting`). Activity chỉ có `turn.completed` / `turn.failed` cho DeepSeek. Hệ quả: Xong / Lỗi của Claude / Codex không suy được từ activity. Con làm: Xong = (activity `from working to idle` hoặc `turn.completed`) ∪ (chênh lệch status lúc rời / lúc về: working→idle, hoặc idle có `statusSince` sau mốc rời); Lỗi tương tự với `error`. Code đọc cả activity nên tự đúng nếu sau này core phát thêm.
- `AwayMark.ref` = thời điểm mới nhất (giờ desktop) trong snapshot lúc rời; so activity với `ref` thay vì giờ điện thoại, nên lệch / đổi đồng hồ điện thoại không làm sai số đếm.
- Thẻ chỉ tính khi snapshot tươi (`!stale && !fromCache`); cache cũ không bị lấy làm số liệu.

### Review (edge case)
- App bị kill nền: mark ghi bền khi pause, đọc lại khi khởi động (test PASS). Nếu bị kill trước khi pause kịp ghi file: mất thẻ lần đó (chấp nhận, ghi async).
- Nhiều máy: mỗi máy mark + thẻ riêng (PASS).
- Buffer cắt: `partial` khi mục cũ nhất mới hơn mốc rời; buffer rỗng không coi là cắt.
- Đồng hồ máy đổi: lùi lại (gap âm) = bỏ mark (PASS); nhảy tới trước thì thẻ có thể hiện "Từ" lệch, số đếm vẫn đúng nhờ `ref`.
- Mở lại < 30 phút: mark bị xoá, không thẻ (PASS). Pause 2 lần không resume: giữ mark đầu (PASS).
- Máy offline lúc mở lại: mark giữ tới khi có snapshot tươi (PASS); máy đã quên thì mark mồ côi nằm trong file (nhỏ, chưa dọn).
- Chạm: chip cao >= 48 dp, nút "Đã xem" >= 48 dp; motion 160 / 220 ms qua `motion()` / `QuietSize` (tôn trọng giảm chuyển động); 360 dp dùng `Wrap`.
- Không comment trong code; file đều < 300 dòng (`chat_home_page.dart` ~300).

### Test
- `flutter test --concurrency=2 test/away_summary_test.dart` → All tests passed (31 test).
- `flutter analyze lib/remote/away_*.dart test/away_summary_test.dart` → No issues found.
- `flutter analyze lib/ui/away lib/ui/chat/chat_home_page.dart lib/main.dart` → 9 lỗi, tất cả là thiếu getter l10n `away*` (chờ main gộp ARB), không lỗi khác.
- `test/away_card_test.dart`: CHƯA CHẠY: thiếu ARB `away*`.
- Chưa chạy test hiện hữu (`ui_integration_test`, `chat_ui_test`): CHƯA KIỂM: chờ ARB mới compile được; cần chạy lại vì `main.dart` / `chat_home_page.dart` đổi (notifier mới đọc `remoteHostProvider`, `FileAwayStore` lỗi path_provider trong test được nuốt bằng try/catch).

### Nhìn: CHƯA NHÌN
Cần nhìn trên A33: tắt app 30 phút rồi mở, thẻ đầu màn machine, chip lọc, "Đã xem", 360 dp, dark / light.

### Hot file cần thêm (ARB key: en / vi), thêm vào `app_en.arb` + `app_vi.arb`, rồi `flutter gen-l10n`
Placeholders: `n` kiểu int cho các key đếm; `time`, `ago` kiểu String cho `awayTitle`.
- awayTitle: "Since {time} ({ago} ago)" / "Từ {time} ({ago} trước)"
- awayDone: "Done {n}" / "Xong {n}"
- awayWaiting: "Waiting for you {n}" / "Chờ bạn {n}"
- awayFailed: "Failed {n}" / "Lỗi {n}"
- awayAgentsDone: "Sub-agents done {n}" / "Agent con xong {n}"
- awayAgentsFailed: "Sub-agents failed {n}" / "Agent con lỗi {n}"
- awayCompact: "Compact {n}" / "Compact {n}"
- awaySeen: "Seen" / "Đã xem"
- awayPartial: "Numbers may be incomplete" / "Số liệu có thể thiếu"
Ví dụ metadata: `"@awayDone": {"placeholders": {"n": {"type": "int"}}}`, `"@awayTitle": {"placeholders": {"time": {"type": "String"}, "ago": {"type": "String"}}}`.

### Ngoài phạm vi
- "Cuộn qua thì ẩn" (spec G5 nói "Đã xem hoặc cuộn qua"): chưa làm; không có detector hiển thị trong ListView lazy, đoán sai sẽ làm thẻ biến mất khi user chưa đọc. Chỉ có "Đã xem". Cần chủ nhân chốt nếu muốn.
- Chip "Agent con xong / lỗi" và "Compact" chỉ lọc được khi event có `terminalId`; thiếu thì chip hiện số nhưng không chạm được.
- Số "Xong" có thể thiếu với pane idle→working→idle mà `statusSince` không cập nhật (core chưa phát activity idle); đề nghị gói Rust riêng nếu muốn chính xác (phát `terminal.status` từ / đến cho mọi chuyển trạng thái).
- Không đụng Rust, web, pubspec, ARB, `chat_composer.dart`, `lib/prompts/*`.

### Commit message đề xuất
feat(mobile): away summary card on the machine screen with persistent leave marks, status-diff based done and failed counts, and chip filters for the pane list

### Cập nhật sau khi main gộp ARB
- Sửa 6 test chết `remoteHostProvider must be overridden`: `ui/away/away_controller.dart` nay bắt lỗi khi thiếu host (không thẻ, tracker rỗng, không subscribe); harness `test/support/ui_fakes.dart` thêm override `awayStoreProvider` = `MemoryAwayStore`.
- `flutter analyze` (toàn app) → No issues found.
- `flutter test --concurrency=2 --exclude-tags integration` (toàn bộ) → All tests passed (+665, ~63 skipped là các file `*_shots_test.dart` tự skip). `integration_test` bỏ qua vì cần build Rust.
- `test/away_card_test.dart` chạy trong đợt này, pass.
- `apps/mobile/shots/*.png` (06-tablet-split, 08-inbox, 10-confirm, 11-pane-actions) đang ở trạng thái modified; có thể do test ghi đè, main revert nếu không chủ ý.
- Còn CHƯA NHÌN trên A33.
