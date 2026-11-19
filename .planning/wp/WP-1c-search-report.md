## WP-1c-search: DONE

- Đã đổi:
  - Mới: `components/disk/DiskSearch.vue` (ô tìm, filter, tổng khớp + ms), `components/disk/DiskResults.vue` (list ảo, path rút gọn + tooltip, loại, ngày sửa, size, chọn nhiều, menu chuột phải), `lib/diskMap.search.test.ts`, `stores/diskMap.search.test.ts`, E2E `apps/desktop/e2e/v101-diskmap-ui.cjs`.
  - Sửa: `lib/diskMap.ts` (`diskMapSearch`, `buildSearchQuery`, preset 100 MB / 1 GB / 10 GB, 3 / 6 / 12 / 24 tháng, helper ngày / MB / `shortPath`), `stores/diskMap.ts` (mode browse / search / big / old, filters, debounce 120ms, seq huỷ kết quả cũ, chọn / xoá dùng chung trên list kết quả, giữ selection chỉ với dòng còn hiện), `components/disk/DiskView.vue` (tab Duyệt / Tìm kiếm / File lớn / File cũ), `i18n/disk-v101.ts` (en + vi), `components/disk/DiskView.test.ts` (+3 test).
  - Lỗi thật tìm được nhờ E2E: `store.scan()` truyền `roots.value` (reactive Proxy) qua IPC nên structured clone lỗi, nút Quét không chạy trong app thật, unit test không thấy vì mock bridge. Đã sửa `[...roots.value]`.
- Tiêu chí 3/3 (store test, web test + typecheck, E2E, không comment / không "trần" / em dash / "·").
- Audit: diff chỉ trong phạm vi brief; không đụng style.css / themes.css / en.ts / vi.ts / apps/core / apps/desktop/src.
- Review edge: index chưa có (tab ẩn khi chưa index, trừ khi đang ở chế độ tìm), query rỗng ở tab Tìm kiếm (không gửi request, hiện gợi ý), 0 kết quả (note), kết quả nhiều (limit 500, hiện "hiện N đầu tiên" khi total > N), gõ nhanh (debounce + seq), bridge thiếu hoặc `under` ngoài index (hiện "tìm kiếm lỗi", xoá kết quả), xoá xong tự tìm lại, đổi tab huỷ query chờ, ô số MB dùng `change` để không bị reset khi gõ "0.5".
- Test:
  - `npm test -w @ccm/web -- --maxWorkers=6` -> 258 file, 2327 test pass.
  - `npm run typecheck -w @ccm/web` -> sạch. `npm run build -w @ccm/web` -> ok.
  - `node apps/desktop/e2e/v101-diskmap-ui.cjs` (bản dev tách biệt, cổng 4472 / CDP 9472, chỉ quét thư mục tạm) -> 30/30 PASS: mở view từ Settings > Data, quét thư mục tạm, treemap có pixel, tree 3 dòng, click vào thư mục, glob `*.mkv`, hoa thường, bỏ dấu, 0 kết quả, loại video, min size, only dir, File lớn sort size, File cũ 3 / 6 / 12 / 24 tháng ra đúng file (mtime 30 và 8 tháng), chọn + Vào Thùng rác (file nằm trong `$Recycle.Bin`, lịch sử có dòng), scrollWidth <= clientWidth ở 900px cả 4 tab. Item E2E đã dọn khỏi Thùng rác (đếm lại = 0), thư mục tạm xoá, Orrery đang chạy không bị đụng.
- Nhìn: `docs/qa/v101/WP-1c/` gồm settings-data-disk, browse-treemap, search-glob, search-selected, big-files, old-files, big-files-900 (Electron dev thật, sau lần sửa cuối chỉ chụp lại bằng lần chạy E2E cuối). Đã xem search-selected, old-files, big-files-900. Chưa xem tay: hover tooltip path, menu chuột phải trên dòng kết quả (test logic có, chưa chụp).
- Hot file cần thêm: không. Lưu ý nhỏ: filter dùng chung giữa 3 tab nên chuỗi tên đã gõ ở Tìm kiếm còn lại khi sang File lớn (có nút Xoá bộ lọc).
- Ngoài phạm vi: không thấy lỗi ở host / engine. Ô size max chỉ ở tab Tìm kiếm.
- Commit message đề xuất: feat(web): add Disk search with filters, Big files and Old files tabs, debounced seq-guarded store, E2E on isolated Electron dev, fix scan roots IPC clone
