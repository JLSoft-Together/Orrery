## WP-1c-search: tìm kiếm + filter trong view Ổ đĩa + E2E toàn view trên Electron dev

- Mục tiêu: trong view Ổ đĩa (`apps/web/src/components/disk/`) thêm ô tìm kiếm toàn index (kiểu Everything) + filter, gọi `diskMapSearch` (đã có trong preload / host, xem `.planning/wp/CONTRACT-v101-diskmap.md`, `packages/shared/src/diskMap.ts` `DiskSearchQuery`). Kết quả là list ảo (đường dẫn rút gọn có tooltip, size, ngày sửa, loại), chọn nhiều, dùng chung thanh hành động xoá + menu chuột phải (Mở trong Explorer/Finder) đã có. Có tab / chế độ **File lớn** (sort size, filter minSize) và **File cũ** (before = N tháng, chọn 3 / 6 / 12 / 24 tháng).
- Filter: chuỗi tên (có bỏ dấu, glob `*.mp4`), loại (image / video / audio / doc / archive / installer / code), size min / max (preset 100 MB, 1 GB, 10 GB + nhập tay), ngày sửa trước / sau, chỉ file / chỉ thư mục, giới hạn trong thư mục đang xem. Gõ thì debounce ~120ms, huỷ kết quả cũ về trễ (giữ seq). Hiện tổng khớp + thời gian ms từ engine.
- Đọc `.planning/PLAN-v101-disk-manager.md`, `.planning/wp/WP-1b-ui-report.md`, `.planning/wp/WP-1b-host-report.md`, `.planning/COORD-2026-11-15-v100-v102.md` (chỉ token + utility có sẵn), CLAUDE.md hard rules.
- Phạm vi file: `apps/web/src/components/disk/**`, `apps/web/src/stores/diskMap.ts` (+test), `apps/web/src/lib/diskMap.ts` (+test), `apps/web/src/i18n/disk-v101.ts`, E2E mới `apps/desktop/e2e/v101-diskmap-ui.cjs`.
- Cấm đụng: `TermPane.vue`, `lib/termMenu*`, `composables/useTermMenu*` (WP-0c đang làm), `apps/desktop/src/**`, `apps/core/**` (WP-1a-fix đang làm), `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`.
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] Test store: query → debounce → kết quả; kết quả cũ về trễ bị bỏ; filter map đúng sang `DiskSearchQuery`; File lớn / File cũ preset.
- [ ] `npm test -w @ccm/web` pass; `npm run typecheck -w @ccm/web` sạch.
- [ ] E2E `v101-diskmap-ui.cjs` trên **bản dev tách biệt** qua `apps/desktop/e2e/cdp.cjs` (đã cô lập userData / CCM_PORT / CCM_DATA_DIR / APPDATA; env `CCM_E2E_DISKMAP_CONFIRM=1` tự đồng ý dialog): tạo cây thư mục tạm có file lớn / file cũ (đặt mtime cũ), mở view Ổ đĩa qua UI (Settings > Data > Ổ đĩa hoặc menu `⋯`), quét đúng thư mục tạm đó (không quét ổ thật), treemap + cây có dữ liệu, click đi vào thư mục, tìm theo tên + filter loại / size ra đúng file, chọn rồi Vào Thùng rác, lịch sử có dòng. Đo `document.documentElement.scrollWidth <= clientWidth` ở 900px. Chụp `docs/qa/v101/WP-1c/` (view thật trên Electron, tìm kiếm, File lớn, Settings > Data > Ổ đĩa). Dọn item E2E khỏi Thùng rác sau khi xong. Không kill Orrery/Electron đang chạy, không dùng cổng 4317 / 5173, không `npm run desktop:dev`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Lệnh test: `npm test -w @ccm/web && npm run typecheck -w @ccm/web`; build web + desktop; `node apps/desktop/e2e/v101-diskmap-ui.cjs`.
- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-1c-search-report.md`.

## Nhật ký
- 1: lib+store+DiskResults/DiskSearch/DiskView + i18n viet xong, con test
- 2: test store/lib/view viet, pass; tiep: E2E
- 3: E2E v101-diskmap-ui.cjs viet, chay lan 1
- 4: E2E that bug: store.scan truyen Proxy roots qua IPC -> clone loi, scan UI khong chay; sua [...roots]
