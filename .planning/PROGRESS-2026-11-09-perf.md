# PROGRESS 2026-11-09 — tối ưu RAM + CPU (PLAN-v86)

## Tóm tắt hiện tại
- 6/6 WP xong (perf-1..6), chưa commit — message ở COMMITS.md. Test: collector 417, web 1823, desktop 1414 pass; typecheck toàn repo sạch (đã npm install cho package telegram).
- Chờ chủ nhân chạy app kiểm: Office tắt mặc định + switch Settings → General; Monitor mở lần đầu; chuyển tab pane đang stream; boot.
- Đổi hành vi: chip dev server chỉ ở pane shell; Office mặc định tắt (DECISIONS 2026-11-09).
- Việc tiếp (chưa làm): PTY flush leading edge (gõ phím), intro boot không chờ cố định 2.45 s, đo lại RAM/CPU trên app thật + macOS.

## Bảng WP
| WP | Model | Trạng thái | Vòng | Test | Review | Commit | Ghi chú |
|----|-------|------------|------|------|--------|--------|---------|
| perf-1 | sonnet | ĐẠT | 2 | 410 pass (main chạy lại) | main: usage+hub đạt | | gitstat: bỏ hit cache 3s (stale), miss cache chỉ khi not-a-repo |
| perf-2 | sonnet | ĐẠT | 1 | web 1803 pass, typecheck ok (main chạy lại) | main | | đổi hành vi: chip dev server chỉ còn ở pane shell |
| perf-3 | sonnet | ĐẠT (chờ xem) | 1 | 371 pass liên quan, typecheck ok | main | | main sửa: bỏ prefetch Monitor lúc idle (mất lợi RAM). Ngoài phạm vi: habblaud board.ts close(). CHƯA NHÌN Monitor mở lần đầu |
| perf-6 | sonnet | ĐẠT (chờ xem) | 1 | collector 417 pass, web liên quan 136 pass | main | | Office default off; switch ở Settings → General; CHƯA NHÌN |
| perf-4 | opus | ĐẠT (chờ xem) | 2 | web 1823 pass, typecheck ok (main chạy lại) | opus reviewer: 0 high, 4 low | | vòng 2: giữ carry, bỏ truncate → flush khi vượt 2 MB, refit chờ parse |
| perf-5 | sonnet | ĐẠT (PARTIAL) | 1 | desktop 1414 pass | main | | chỉ dời tạo window + spawn collector sớm; không lazy import (main không có module nặng để lười). main sửa: dời createMainWindow sau backups.applyPending |

## Nhật ký
- [~] perf-1/2/3 giao sonnet, chạy nền.
- [~] perf-6 (Office tắt mặc định, quyết định chủ nhân) giao sonnet.
- Thống kê: 6 WP ĐẠT (perf-1, perf-4 cần vòng 2; perf-5 PARTIAL do main không có module nặng để lazy). Model: 5 sonnet + 1 opus (perf-4) + reviewer opus + index haiku. Không escalate.
