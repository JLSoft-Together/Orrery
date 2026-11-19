# WP-66 report: Android hiện "đã lược bớt" (`trimmed`) và lỗi `limit`

## Thay đổi
- `apps/mobile/lib/remote/size_notes.dart` (mới): `SnapshotTrim.parse` (đếm > 0 hoặc `text: true`, còn lại null), `SizeLimit.parse` (cần `what` chuỗi, `max` mặc định 1 MiB).
- `apps/mobile/lib/remote/store.dart`: `MachineView.trimmed` / `limit` (copyWith dùng sentinel để xoá được), `_onSnapshot` đọc `trimmed` và xoá `limit`, case `limit` (`_onLimit`, không `_scheduleSave`), `MachineStore.resync()` công khai: reply `RemoteCommandException('limit')` thành `limit(what: snapshot)`; `_requestResync` dùng lại.
- `apps/mobile/lib/ui/machine_api.dart`: `MachineApi.resync` -> `store.resync()`.
- `apps/mobile/lib/ui/monitor/size_notice.dart` (mới): `SizeNotice` banner lỗi (`monitor-limit`, nút `monitor-limit-retry`) hoặc dòng nhỏ `monitor-trimmed`; `monitor_page.dart`: đặt `SizeNotice` trên danh sách; có `limit` mà chưa có session thì không hiện "Máy tính chưa có session nào".
- ARB en + vi sửa tay (`monLimitTitle`, `monLimitBody {size} {max}`, `monTrimmed`); `flutter gen-l10n` (check.mjs tự chạy) sinh lại `app_localizations*.dart`. Không chạy `gen_arb.mjs`.
- Test: `test/size_notes_test.dart` (5 ca: parse trimmed, limit event + snapshot xoá, reply limit, widget banner + Thử lại, widget dòng trimmed); `test/support/ui_fakes.dart` thêm `FakeMachineApi.resync` và tham số `view(trimmed, limit)`.
- Docs `docs/remote-integration.md` §4.2 (câu về Android), `apps/mobile/.code_index/mobile.md`.

## Test (lệnh thật)
- `node apps/mobile/scripts/check.mjs`: gen-l10n OK, format OK, analyze sạch, `flutter test` +300 ~63 pass (so 295: +5), không file hand-written > 400 dòng.

## QA A21s
Chưa chạy (không build APK). Bước kiểm: mở app với desktop có > 700 sub-agent (fixture `scripts/demo-fixture.mjs --subagents=700`), vào Monitor: thấy dòng "Đã lược bớt dữ liệu cũ để vừa giới hạn" cùng danh sách; tạo dữ liệu không cắt vừa được: thấy banner đỏ + Thử lại; đóng bớt session trên desktop, bấm Thử lại: banner mất khi snapshot về.

## Rủi ro
- Chỉ màn Monitor hiện notice; Workspace không có (không cần: dùng `ws.*`).
- Thử lại gửi `resync`; desktop trả `limit` lần nữa thì banner giữ nguyên (đúng ý). `trimmed` / `limit` không lưu cache nên máy offline mở từ cache không có dòng này.
- `bytes` của reply `limit` không có, banner ghi "> 1 MB" khi `bytes` = 0.
- Không commit; không tiến trình nền nào đang chạy.
