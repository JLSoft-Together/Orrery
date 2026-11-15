## WP-4c-snap: snapshot so sánh + quét nền + gộp cảnh báo ổ đầy + Telegram

- Mục tiêu (PLAN v101 Q5, Q28, Q31, Q10):
  1. **Snapshot**: sau mỗi lần quét xong (thường / MFT / nền), main lưu snapshot gọn = size theo thư mục tới độ sâu 4 dưới mỗi root (lấy từ index qua op `disk.children` lặp hoặc op mới `disk.summary {roots, depth}` trong `core-disk` nếu cần cho nhanh) vào `CCM_DATA_DIR/disk/snapshots/<ts>.json.gz`, giữ 30 bản (xoá cũ nhất). Không lưu tên file, chỉ thư mục + size.
  2. **So sánh**: trong tab Duyệt có chọn "So với: <ngày>" (danh sách snapshot), cây thư mục hiện cột chênh lệch (+X GB / −Y GB, mũi tên + chữ, không chỉ màu), sort theo chênh lệch; có danh sách "Phình ra nhiều nhất" (top 20 thư mục tăng).
  3. **Quét nền định kỳ**: mặc định 1 lần/ngày, chỉ khi máy rảnh (`powerMonitor.getSystemIdleTime()` ≥ 10 phút) và cắm sạc (`powerMonitor.isOnBatteryPower() === false`), level `light`, chỉ các root người dùng đã quét trước đó; Settings > Data > Ổ đĩa: tần suất (tắt / hằng ngày / hằng tuần), hiện lần quét nền cuối. Đang quét nền mà người dùng bắt đầu dùng máy (idle < 1 phút) hoặc rút sạc → pause, rảnh lại → resume. Không chạy khi chưa từng quét.
  4. **Gộp cảnh báo ổ đầy v77** (`apps/desktop/src/disk/{watch,ipc}.ts`, web `DiskBadge.vue`, `DiskSettings.vue`, `HeaderStatusPopover.vue`, i18n `disk-v77.ts`): mục Settings cảnh báo ổ đầy chuyển vào cùng mục Settings > Data > Ổ đĩa (giữ setting cũ, giữ key lưu cũ để không mất cấu hình); badge header khi click mở view Ổ đĩa (thay vì chỉ popover) và popover có nút "Dọn dẹp" mở tab Dọn dẹp. Giữ nguyên logic đo free space của v77.
  5. **Telegram** (Q31): khi cảnh báo ổ đầy bắn hoặc quét nền xong mà dung lượng trống giảm > 5 GB so với snapshot trước, gửi qua đường Telegram báo cáo app sẵn có (tìm module gửi Telegram của app: `telegram_notify` / bot-tele-send-notify, chỉ gửi khi người dùng đã bật báo cáo Telegram) nội dung: ổ, dung lượng trống, thay đổi, **top 5 thư mục lớn nhất / phình nhiều nhất (tên thư mục + size)**, không bao giờ gửi tên file. Có công tắc "Gửi báo cáo Ổ đĩa qua Telegram" trong mục Ổ đĩa (mặc định bật nếu Telegram đã bật).
- Bảo mật / quyền riêng tư: snapshot chỉ thư mục; Telegram chỉ tên thư mục đầu (không full path dưới home: rút gọn `~\...`); env E2E chỉ khi `!app.isPackaged`.
- Phạm vi file: `apps/desktop/src/diskMap/**` (service snapshot + scheduler, đọc lại file trước Edit), mới `apps/desktop/src/diskMap/snapshots.ts`, `schedule.ts`; `apps/desktop/src/disk/**` (v77); khối riêng `main.ts` + `preload.ts`; `packages/shared/src/diskMap.ts`; `apps/core/crates/core-disk/**` nếu thêm op `disk.summary`; web `components/disk/**` trừ `apps/`, `ram/`, `clean/`, `move/`, `organize/`, `dupes/` thư mục con; `components/settings/SettingsDiskMap.vue`, `DiskSettings.vue`, `DiskBadge.vue`, `HeaderStatusPopover.vue`, `stores/diskMap.ts`, `lib/diskMap.ts`, i18n `disk-v101.ts` + `disk-v77.ts`; module Telegram của desktop (chỉ thêm hàm gửi báo cáo ổ đĩa, không đổi luồng khác); test + E2E `apps/desktop/e2e/v101-snap.cjs`.
- Cấm đụng: `apps/desktop/src/diskApps/**`, `diskRam/**`, `components/disk/apps|ram/**`, `stores/diskApps|diskRam`, `lib/diskApps.ts`, `i18n/disk-apps-v101.ts`, `packages/shared/src/diskApps.ts`, `DiskView.vue` (WP-4b đang sửa; cần gắn gì thì ghi "Hot file cần thêm"); `apps/core/crates/core-desk/**` (session khác); `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `apps/shell/**`.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] Test desktop: snapshot ghi / đọc / xoay vòng 30 bản, không chứa tên file; diff 2 snapshot; scheduler (idle / pin / tần suất / chưa từng quét / pause-resume) với clock + powerMonitor giả; tin Telegram dựng đúng (top 5, rút gọn path, không tên file), chỉ gửi khi bật.
- [ ] Test web: cột chênh lệch + sort, chọn snapshot, Settings mới giữ cấu hình cảnh báo cũ.
- [ ] `npm test` desktop + web liên quan (`--maxWorkers=6`) pass; typecheck sạch; nếu thêm op Rust: `cargo test -p core-disk -j 8` + clippy.
- [ ] E2E trên **bản dev tách biệt** (cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): quét thư mục tạm → thêm file → quét lại → so sánh hiện +size đúng; scheduler chạy bằng clock / idle giả qua env bản dev; Telegram bằng sink giả (env bản dev ghi ra file), **không gửi Telegram thật**. Chụp `docs/qa/v101/WP-4c/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·". Message vue-i18n escape `|`, `{`, `}`, `@`.

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-4c-snap-report.md`.

## Nhật ký
- 15/11 19:50 Rust op `disk.summary {roots, depth, minSize}` + test `engine_summary_lists_dirs_to_depth` pass (target-dir riêng `apps/core/target-4c`).
- 15/11 19:55 desktop: `diskMap/{snapshots,report,schedule,snap}.ts`, hook `onDone`/`beforeScan` trong ipc.ts, khối `setupDiskSnap` trong main.ts, 6 method preload; 4 file test `test/diskSnap-*.test.ts` viết xong. Chặn: `packages/shared/src/mcpDesk.ts` (WP khác, MCP) đang lỗi cú pháp (U+2028 trong regex), vitest/tsc không chạy được; đợi.
- 15/11 20:00 web: lib/stores diskSnap, DiskTree (cột chênh lệch, picker, top 20), SettingsDiskMap (quét nền + Telegram + gộp DiskSettings), DiskBadge/HeaderStatusPopover, i18n; 3 file test web + 4 desktop + Rust pass. Còn: full suite, E2E, chụp.
- 15/11 20:15 E2E 36/36, ảnh docs/qa/v101/WP-4c/, .code_index cập nhật. Viết báo cáo.
