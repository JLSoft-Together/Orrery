## WP-1b-ui: DONE (còn 3 việc main gộp, xem "Hot file cần thêm")

- Đã đổi:
  - Mới: `apps/web/src/lib/treemap.ts`, `lib/sunburst.ts`, `lib/diskMap.ts` (type cục bộ khớp contract, bridge bọc an toàn, helper size / path / nhóm file), `stores/diskMap.ts`, `components/disk/` (DiskView, DiskScanBar, DiskVolumeRow, DiskTreemap, DiskSunburst, DiskTree list ảo + menu chuột phải, DiskActions, DiskHistory, DiskTip, useCanvas), `components/settings/SettingsDiskMap.vue`, `i18n/disk-v101.ts` (namespace `diskMap.*` vì `disk.*` đã là v77).
  - Sửa: `stores/ui.ts` (View `disk`), `App.vue`, `AppHeader.vue` (menu `...`), `CommandPalette.vue`, `components/settings/SettingsData.vue` (nhúng mục Ổ đĩa, trang Data).
  - Test mới: `lib/treemap.test.ts`, `lib/sunburst.test.ts`, `lib/diskMap.test.ts`, `stores/diskMap.test.ts`, `components/disk/DiskView.test.ts`.
- Tiêu chí: 5/5.
- Audit / Review / Test:
  - `npm run typecheck -w @ccm/web` sạch.
  - `npm test -w @ccm/web` 252 file, 2271 test pass (trước khi chỉnh màu/alpha cuối); chạy lại lib + store diskMap + components/disk + components/settings: 1376 pass.
  - Edge đã phủ bằng test: bridge thiếu method (view báo "chưa hỗ trợ", Settings nút disabled), index rỗng, listing null, thư mục 0 byte (thông báo thay vì canvas trống), 2000 con (cap, gộp "Khác"), xoá một phần lỗi (giữ lại mục lỗi đang chọn, xem chi tiết), native confirm bị huỷ (không đổi gì), listing cũ về trễ bị bỏ, resize + đổi theme (ResizeObserver + MutationObserver vẽ lại), DPR cao.
  - Không comment trong code; chuỗi vi không có "trần", em dash, "·".
- Nhìn: `docs/qa/v101/WP-1b-ui/` (Edge headless, bản dev Vite cổng 5199 + bridge giả, đã xoá file harness và tắt server): treemap-light-1400, treemap-galaxy-1400, sunburst-galaxy-1400, sunburst-light-1400, selection-delete-900, scanning-900, empty-1400, none-1400, big-2000-items-1400. Ảnh trước chỉnh màu dir/media: treemap-light-1400 và sunburst-galaxy-1400 (chụp lại bản treemap-galaxy sau chỉnh). CHƯA NHÌN: mục Settings > Data > Ổ đĩa (chỉ test logic), hover tooltip, menu chuột phải. Không đo scrollWidth thật ở 900px, chỉ xem ảnh (không thấy tràn ngang).
- Hot file cần thêm (main gộp):
  1. `apps/web/src/i18n/en.ts`: `import diskV101 from './disk-v101'` và `...diskV101.en,` cuối block spread; `i18n/vi.ts`: tương tự `...diskV101.vi,`. Thiếu dòng này thì runtime hiện key `diskMap.*` thô (test đã tự nạp message riêng).
  2. `apps/web/src/components/settings/index.ts` mảng `entries`: `{ page: 'data', id: 'diskMap', labelKey: 'diskMap.settings.title', hintKey: 'diskMap.settings.hint' },` (cho ô tìm Settings). Mục đã hiện sẵn trong trang Data.
  3. `packages/shared/src/diskMap.ts` đã có (WP-1b-host): đổi type cục bộ trong `lib/diskMap.ts` sang `import type {...} from '@ccm/shared'` khi chốt (tên khớp contract; `DiskSearch*` chưa dùng).
- Ghi chú: `diskMapStatus.roots` quyết định giới hạn nút "Lên cấp" và breadcrumb; tìm kiếm (`diskMapSearch`) ngoài phạm vi WP này. Chọn level lưu `ccm.diskLevel`, chart lưu `ccm.diskChart` (localStorage, try/catch).
- Commit message đề xuất: feat(web): add Disk view with treemap and sunburst canvas, virtual tree, delete flow, history and Settings section
