WP-22: ĐẠT. Vỏ Tauri gửi báo cáo Telegram sau lần quét nền ổ đĩa, cùng nội dung, định dạng và sink E2E như Electron. Thêm luôn báo cáo sau cảnh báo ổ đầy (Electron `diskSnap.onAlert`).

## Nguyên nhân gốc
`apps/shell/src/disk_snap.rs` (WP18) chỉ ghi snapshot trong `on_done`, bỏ qua kết quả `note_done` (lần quét nền hay không) và không có phần port `diskMap/report.ts`. Không có gì ghi vào `CCM_E2E_DISKSNAP_TG_SINK`, nên 4 check Telegram của `v101-snap` FAIL (0 dòng).

## Sửa (apps/shell)
- `src/disk_snap_report.rs` (mới, module con `report` của `disk_snap.rs`, đọc được `Compared` / `Node` private): port `fmtBytes` (số theo locale: vi `2,5 MB` / `1.000 MB`, en `2.5 MB` / `1,000 MB`, 0 số lẻ khi >= 100), `shortPath` (`~` cho home, rút gọn `a\...\y\z` khi > 4 phần), `freeDrops` (> 5 GB), `largestDirs`, `buildReport` (text vi / en giống hệt), ngày `dd/mm/yyyy` theo giờ local (`core_transcript::time::local_offset_ms`). Gửi: sink E2E nếu có, ngược lại `desk:tg-state` (enabled + configured) rồi `desk:tg-report` với route đầu, giống `main.ts` `setupDiskSnap`.
- `disk_snap.rs`: `on_done` đọc snapshot mới nhất trước khi ghi, rồi gửi báo cáo `bgscan` chỉ khi `note_done` trả true (scheduler đang chạy). Thêm `on_alert` (so với snapshot mới nhất, không ghi) và `report` (kiểm switch Telegram của lịch, gap 6 giờ cho alert qua `alert_at`). `Store::latest` bỏ `#[cfg(test)]`.
- `episodes.rs`: phone episode `reason: disk` gọi `DiskSnap::on_alert` trong thread riêng (ngoài `desk:tg-alert` sẵn có).
- `Cargo.toml`: thêm `core-transcript` (path crate, đã nằm trong graph qua core-desk).
- Sink `CCM_E2E_DISKSNAP_TG_SINK` / `CCM_E2E_DISKSNAP_TG_OFF` chỉ nhận khi `debug_assertions` hoặc `config::e2e_isolated()` (cùng cổng với FAKE / TICK). Không gửi Telegram thật trong test.

## Kiểm
- `cargo test -j 18` crate shell: 284 pass (5 test mới: số byte, rút gọn path, ngày, báo cáo đủ vi / en / không drop, largest bỏ thư mục cha).
- `cargo build` debug: không warning mới.
- E2E `CCM_E2E_SHELL=tauri node e2e/v101-snap.cjs` (exe debug mặc định, cổng 4492 / 9427): 36/36.
- Electron dev `node e2e/v101-snap.cjs`: 36/36.
- Chưa chạy lại exe release (main build).

## Còn lại
- Không có. Không sửa file e2e, không đụng Electron.
