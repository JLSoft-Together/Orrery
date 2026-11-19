# WP-73 report: Disk, chọn item và chọn ổ để quét

## Nguyên nhân

1. Chọn item không đổi trạng thái (tab Duyệt, `DiskTree.vue`): checkbox của dòng gọi `e.preventDefault()` trong `@click` trong khi `:checked` bind từ `selected`. Trình duyệt hoàn tác trạng thái checkbox sau khi handler chạy (canceled activation) và đè lên giá trị Vue vừa set, nên store đã chọn (nền dòng đổi, đếm "đã chọn N") nhưng ô tick vẫn trống, lần click sau Vue không patch lại vì prop không đổi. Các danh sách khác (Dọn dẹp, Dupes, Apps, Organize, Kết quả tìm) dùng `@change` + Set mới nên không dính. jsdom không tái hiện được lỗi này (test checkbox vẫn pass khi đặt lại `preventDefault`), nên chưa xác nhận bằng E2E trên app thật.
2. Chọn ổ quét không đúng: backend (`core-disk` `scan`, shell `clean_scan`, MFT) chỉ quét đúng `roots` được gửi; lỗi nằm ở store `diskMap`:
   - `init()` tự tick lại toàn bộ `status.roots` (kể cả root cũ đã merge vào index như ổ D, hoặc thư mục không phải ổ nên không có checkbox) rồi gửi hết đi quét, người dùng tích C vẫn quét thêm D / root ẩn.
   - Sau khi quét xong, view mở `status.roots[0]`, index merge giữ root cũ đứng trước nên hiện ổ D thay vì C vừa quét.
   - Lựa chọn ổ không được nhớ.
   Quét nền (`disk_snap.rs`) rescan mọi root đang có trong index theo thiết kế, chưa đổi.

## File đổi

- `apps/web/src/components/disk/DiskTree.vue`: bỏ `preventDefault` ở `check`; dòng có `tabindex=0` và Space (`@keydown.space.self.prevent`) đổi chọn.
- `apps/web/src/components/disk/DiskResults.vue`: cùng Space + `tabindex`.
- `apps/web/src/lib/diskMap.ts`: `loadRoots`, `saveRoots` (`ccm.diskRoots`), `onlyMounts`.
- `apps/web/src/stores/diskMap.ts`: `scanRoots` (chỉ root là ổ đang có, dùng cho quét thường, quét siêu nhanh `fastDrive`), nhớ lựa chọn ổ trong `toggleRoot`, `init` khôi phục lựa chọn đã lưu (lọc theo volumes) thay vì tick hết `status.roots`, `pickStart` mở root vừa quét sau khi xong.
- `apps/web/src/components/disk/DiskScanBar.vue`: nút Quét disabled theo `scanRoots`.
- Test: `apps/web/src/stores/diskMap.test.ts` (2 ca mới), `apps/web/src/components/disk/DiskView.test.ts` (2 ca mới), `apps/core/crates/core-disk/tests/scan.rs` (`scan_covers_only_the_picked_roots`).

## Test

- `npx vitest run src/components/disk src/stores/disk src/lib/diskMap` (apps/web): 14 file, 143 test pass.
- `npm run typecheck -w @ccm/web`: sạch.
- `cargo test -j 4 -p core-disk`: toàn bộ pass (scan.rs 26/26 gồm ca mới).
- Shell `cargo test -j 4 disk_` (apps/shell): không sửa file shell; xem kết quả trong báo cáo cuối.
- Không chạy E2E, không có ảnh `docs/qa/v103/WP-73/`.

## QA tay (Windows + macOS)

1. Tab Duyệt: bấm ô tick một dòng, ô phải hiện dấu tick cùng lúc nền dòng đổi; bấm lại bỏ tick. Tab dòng bằng Tab, Space đổi chọn. Shift-click chọn dải.
2. Có 2 ổ đã quét: tích chỉ C, bấm Quét: tiến độ chỉ chạy trên C, xong view mở C. Mở lại tab: vẫn chỉ C được tick. Quét siêu nhanh chỉ hiện khi đúng 1 ổ NTFS được tick.

## Việc dở

- Chưa xác nhận lỗi checkbox trên Electron / Tauri thật (cần E2E CDP).
- Index vẫn merge: quét C không xoá dữ liệu D đã quét trước đó (thiết kế); quét nền rescan mọi root trong index, chưa theo lựa chọn ổ.
- Chưa có ảnh QA sáng / tối.

## Bổ sung: quét nền, snapshot, Telegram chỉ theo ổ đang chọn

- Nguyên nhân: `disk_snap.rs` (`capture`, idle rescan) lấy mọi root trong index, nên ổ D đã quét trước đó vẫn bị quét lại và vào snapshot / báo cáo Telegram.
- Sửa: module mới `apps/shell/src/disk_pick.rs` (`Picked` lưu `picked-roots.json` trong thư mục `disk`, `restrict` lọc root theo ổ đã chọn, khớp không phân biệt hoa thường / dấu `\`). `DiskMap.scan` lưu roots người dùng chọn, `fast_scan` lưu ổ tương ứng, `DiskMap::scheduled_roots` dùng cho `capture` và idle scan trong `disk_snap.rs`. Chưa có file lưu (máy cũ) thì giữ hành vi cũ (mọi root trong index). Không thêm IPC mới: lựa chọn đi theo yêu cầu quét thật.
- Dữ liệu cũ của ổ bỏ chọn: giữ trong index (ẩn đi phải gỡ khỏi index core và mất dữ liệu đã quét lâu), nhưng `DiskScanBar.vue` hiện dòng `data-disk-stale` ("Dữ liệu cũ được giữ, không quét lại: ..., quét nền và snapshot chỉ quét các ổ đang tick"). i18n `diskMap.scan.stale` en + vi trong `apps/web/src/i18n/disk-v101.ts`.
- Test: Rust `disk_pick::tests` (3 ca: lọc root Mac, ổ Windows, lưu / đọc); vitest `DiskView.test.ts` ca "says which indexed drives are kept old and not rescanned".

## Kết quả test (cập nhật)

- vitest disk (apps/web): 14 file, 144 pass; `npm run typecheck -w @ccm/web` sạch.
- `cargo test -j 4 -p core-disk`: pass hết (scan.rs 26/26).
- `cargo test -j 4 disk_` (apps/shell): 73 pass trước khi thêm `disk_pick`; kết quả sau khi thêm ghi ở cuối mục này.
- `npm run qa -- --no-e2e`: 3/5; 2 bước fail ngoài vùng (git preview twin timeout, `RemotePairPanel` lệch 1 giây khi máy tải).

## Test macOS

1. Volume trên Mac là `/` và `/Volumes/<tên>`; ổ ngoài hiện checkbox riêng. Tích chỉ `/Volumes/X`, quét: chỉ X có tiến độ; `/` không được quét lại.
2. Tích `/`, bỏ `/Volumes/X` (đã quét trước đó): dòng "Dữ liệu cũ được giữ" liệt kê `/Volumes/X`; chờ quét nền (hoặc ép lịch) và kiểm snapshot / báo cáo Telegram chỉ có `/`.
3. Chọn lại thư mục con của `/` (index merge): root vẫn được tính theo ổ cha, không bị lọc nhầm.
4. Quét siêu nhanh không có trên Mac (chỉ Windows NTFS), nút phải ẩn.

Kết quả shell sau khi thêm `disk_pick`: `cargo test -j 4 disk_` (apps/shell) 76 pass, 0 fail (73 cũ + 3 ca mới).
