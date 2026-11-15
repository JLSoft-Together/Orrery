## WP-1b-ui: view Ổ đĩa (treemap / sunburst + cây + xoá + lịch sử) + mục Settings

- Mục tiêu: view `disk` toàn màn hình mở từ Settings > Ổ đĩa và từ menu `⋯` của header (giống Devices, không chiếm tab chính) + Command Palette. Gọi bridge theo `.planning/wp/CONTRACT-v101-diskmap.md` (bridge do WP-1b-host làm song song; web phải chạy được khi bridge thiếu method: ẩn / hiện trạng thái "chưa hỗ trợ"). Đọc `.planning/PLAN-v101-disk-manager.md`, `.planning/COORD-2026-11-15-v100-v102.md` (chỉ dùng token + utility có sẵn, không thêm màu / CSS), CLAUDE.md hard rules (i18n, theme token, không horizontal scroll, status = icon + text + màu, Apple style dense calm), `.code_index/web.md`. Mẫu thêm view: commit `cb5f80a8` (Devices tab) + `apps/web/src/lib/devices.ts`.
- Nội dung:
  1. Mục Settings "Ổ đĩa": danh sách ổ (`diskMapVolumes`: dung lượng dùng / trống, thanh %), trạng thái index, nút "Mở Ổ đĩa", chọn mức Nhẹ / Cân bằng / Tối đa (`ccm.diskLevel`, mặc định balanced).
  2. View: thanh trên chọn ổ / thư mục gốc + Quét / Pause / Resume / Cancel + progress (số file, GB, thư mục đang đi, thời gian). Breadcrumb đường dẫn hiện tại.
  3. Bản đồ: treemap squarified vẽ **canvas** (hit-test khi hover, tooltip tên + size + %, click thư mục để đi vào, nút lên cấp) + sunburst canvas, nút chuyển qua lại (`ccm.diskChart`). Màu theo nhóm loại file lấy từ token có sẵn (`agent-*`, `diff-*`, ink ladder... đọc `getComputedStyle`), không hardcode màu. Vẽ lại khi resize / đổi theme; chỉ vẽ top N con (vd 2000) gộp phần còn lại thành "Khác". DPI cao sắc nét.
  4. Cây bên cạnh: con của thư mục hiện tại sắp theo size, thanh %, list ảo khi nhiều dòng, chọn nhiều (checkbox, Shift-click), chuột phải / nút: Mở trong Explorer/Finder (`diskMapReveal`), Vào Thùng rác, Xoá hẳn.
  5. Thanh hành động khi có mục chọn: tổng size, nút Vào Thùng rác (chính) + Xoá hẳn (phụ) → `diskMapDelete` (dialog native do main hiện) → hiện kết quả (đã giải phóng X GB, lỗi N mục, xem chi tiết) và tải lại cây.
  6. Panel Lịch sử dọn (`diskMapHistory`): danh sách lần dọn + tổng đã giải phóng.
- Phạm vi file: mới `apps/web/src/lib/diskMap.ts` (+test), `apps/web/src/stores/diskMap.ts` (+test), `apps/web/src/components/disk/**` (DiskView, DiskTreemap, DiskSunburst, DiskTree, DiskActions, DiskHistory, DiskSettings section...), `apps/web/src/lib/treemap.ts` (thuật toán squarified thuần + test) và `apps/web/src/lib/sunburst.ts` (+test), `apps/web/src/i18n/disk-v101.ts` (key namespace `disk.*` cho en + vi theo mẫu `i18n/devices-v61.ts` / `disk-v77.ts`). Sửa đăng ký view: `stores/ui.ts`, `App.vue`, `AppHeader.vue`, `CommandPalette.vue`, component Settings (thêm mục). Kiểm `disk.*` không trùng key namespace `disk` của v77 (`i18n/disk-v77.ts`); trùng thì dùng `diskMap.*`.
- Cấm đụng: `apps/web/src/i18n/en.ts` + `vi.ts` (WP-0b đang sửa; ghi dòng import cần thêm vào "Hot file cần thêm", main gộp), `TermPane.vue`, store settings của WP-0b nếu cùng file (để setting riêng trong store diskMap), `style.css`, `themes.css`, `apps/desktop/**`, `packages/shared/**` (WP-1b-host tạo `diskMap.ts`; tạm khai báo type cục bộ trong `lib/diskMap.ts` khớp contract, ghi vào báo cáo để main đổi sang import shared).
- Model: sonnet.
- Phụ thuộc: contract. Song song với WP-1b-host và WP-0b.

## Tiêu chí nghiệm thu

- [ ] Test thuần: squarified (tổng diện tích = khung, tỉ lệ cạnh hợp lý, input rỗng / 1 phần tử / size 0), sunburst (góc cộng đủ 360°), store (scan → progress → done, chọn nhiều, xoá cập nhật), lib bọc bridge thiếu method.
- [ ] View chạy với bridge giả (test component hoặc fixture), không horizontal scroll ở 900px rộng, text i18n en + vi đủ.
- [ ] `npm test -w @ccm/web` pass; `npm run typecheck -w @ccm/web` sạch.
- [ ] Nhìn: sau khi WP-1b-host xong main sẽ chạy E2E UI; ở WP này chụp view với bridge giả (dev tách biệt qua `apps/desktop/e2e/cdp.cjs` nếu chạy được, hoặc ghi "CHƯA NHÌN"). Không kill Orrery/Electron đang chạy, không dùng cổng 4317 / 5173, không `npm run desktop:dev`.
- [ ] Không comment trong code.

- Lệnh test: `npm test -w @ccm/web && npm run typecheck -w @ccm/web`.
- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-1b-ui-report.md`.

## Nhật ký
- 17:22 WP-1b-ui: viết xong lib treemap/sunburst/diskMap, store, components/disk/*, SettingsDiskMap, i18n disk-v101 (namespace diskMap.*), đăng ký view disk (ui/App/AppHeader/CommandPalette). Còn: test + typecheck + chụp.
- 17:24 test lib/store/DiskView viết xong, đang typecheck
- 17:30 XONG: typecheck sạch, test pass, đã chụp docs/qa/v101/WP-1b-ui/, báo cáo WP-1b-ui-report.md
