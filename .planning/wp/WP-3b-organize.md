## WP-3b-organize: sắp xếp file (xem trước + tự động + Undo) + thư mục rỗng / shortcut hỏng

- Mục tiêu (PLAN v101 Q7, Q17, Q29 + "Lịch sử dọn"):
  1. **Sắp xếp**: tab **Sắp xếp** trong view Ổ đĩa. Chọn thư mục nguồn (mặc định Downloads; thêm được Desktop / thư mục khác), chế độ **Theo loại** (Ảnh / Video / Nhạc / Tài liệu / Nén / Cài đặt / Code / Khác, tên thư mục đích theo locale) hoặc **Theo tháng** (`YYYY-MM`), chọn như filter; rule thêm: đuôi → thư mục, tên chứa, size > N. **Xem trước**: bảng file sẽ chuyển (nguồn → đích), bỏ tick từng dòng, rồi "Sắp xếp N file". Chỉ xếp file ở cấp ngay trong thư mục nguồn (không đi sâu thư mục con, không đụng thư mục đích đã tạo). Bỏ qua `.crdownload`, `.part`, `.tmp`, `.download`, file đang bị khoá, file mới sửa dưới 2 phút, file ẩn / hệ thống (`desktop.ini`, `.DS_Store`). Trùng tên ở đích → thêm hậu tố ` (2)`, không ghi đè. Di chuyển bằng `rename` (cùng ổ); khác ổ thì copy + verify size + xoá nguồn.
  2. **Undo**: mỗi lần xếp ghi log `CCM_DATA_DIR/organize/<id>.jsonl` (from, to, size, mtime); Undo chuyển ngược nếu file ở `to` còn đúng size + mtime, xung đột thì bỏ qua và báo; xoá thư mục đích rỗng do chính lần xếp đó tạo. Lần xếp xuất hiện trong **Lịch sử dọn** có sẵn (`diskMapHistory` hoặc lịch sử riêng hiển thị chung) với nút Undo.
  3. **Tự động** (công tắc, mặc định tắt): khi app đang chạy (kể cả tray) theo dõi thư mục nguồn (`fs.watch` / chokidar nếu repo đã có), file mới đủ điều kiện (đã ổn định ≥ 2 phút, không phải file đang tải) thì xếp theo rule đã lưu; mở app lại thì xử lý file vào trong lúc app tắt (mtime > mốc chạy cuối). Không cài tiến trình khởi động cùng máy. Ghi log Undo như trên.
  4. **Thư mục rỗng / shortcut hỏng**: trong tab Dọn dẹp **không** (WP-2b đang sửa) → đặt ở tab Sắp xếp một mục "Dọn thư mục rỗng & shortcut hỏng": thư mục rỗng lấy từ index (dir không có con, bỏ thư mục hệ thống / app / `.git` / thư mục có file ẩn quan trọng), shortcut hỏng: Windows `.lnk` dùng `shell.readShortcutLink` kiểm target tồn tại (target ổ mạng / ổ rút ra → nhãn "không kiểm được", không tick), macOS symlink hỏng (alias Finder: CHƯA KIỂM được thì bỏ). Xoá qua `diskMapDelete` (guard canonical).
- Bảo mật / an toàn: mọi path qua guard canonical của `apps/desktop/src/diskMap/guard.ts` (`resolveDeletable` / `checkDeletableCanonical`, xem `.planning/wp/WP-2sec-report.md`) trước khi move; nguồn không được là root ổ / home / thư mục hệ thống; đích luôn nằm trong nguồn. IPC chỉ nhận id kế hoạch đã tạo ở main (renderer không gửi được cặp from/to tuỳ ý). E2E env chỉ có tác dụng khi `!app.isPackaged`.
- Phạm vi file: mới `apps/desktop/src/organize/**` (+ test `apps/desktop/test/organize*.test.ts`), khối riêng trong `main.ts` + `preload.ts`, `packages/shared/src/organize.ts` (+ export index), web `apps/web/src/components/disk/organize/**`, `stores/organize.ts` (+test), `lib/organize.ts`, i18n riêng `apps/web/src/i18n/disk-organize-v101.ts` (main gộp import vào en/vi), gắn tab ở `DiskView.vue` (1 chỗ), E2E `apps/desktop/e2e/v101-organize.cjs`.
- Cấm đụng: `apps/desktop/src/diskClean/**`, `components/disk/clean/**`, stores/lib diskClean, `i18n/disk-clean-v101.ts` (WP-2b); `apps/core/**`, `components/disk/dupes/**`, `stores/diskDupes.ts`, `diskMap/**` trừ việc import guard (WP-3a); `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `apps/shell/**`.
- Model: opus (di chuyển file người dùng, watcher nền).

## Tiêu chí nghiệm thu

- [ ] Test desktop: kế hoạch theo loại / theo tháng / rule; bỏ file đang tải / mới / ẩn; trùng tên thêm hậu tố; khác ổ copy + verify; Undo đúng và bỏ qua xung đột; watcher chờ file ổn định; bắt kịp file vào lúc app tắt; renderer không gửi được path tuỳ ý; guard chặn nguồn nguy hiểm; thư mục rỗng + `.lnk` hỏng (tạo `.lnk` thật bằng `shell.writeShortcutLink` trong test Electron hoặc fixture).
- [ ] `npm test -w @ccm/desktop -- organize`, web test (`--maxWorkers=6`) pass; typecheck sạch.
- [ ] E2E trên **bản dev tách biệt** (cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): **thư mục nguồn là thư mục tạm giả**, không bao giờ đụng Downloads / Desktop thật. Xem trước → xếp → Undo trả đúng; bật tự động, thả file vào → được xếp sau khi ổn định (dùng env rút ngắn 2 phút xuống vài giây chỉ ở bản dev). Chụp `docs/qa/v101/WP-3b/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-3b-organize-report.md`.

## Nhật ký
- 18:59 desktop organize/{rules,plan,move,log,junk,service,ipc}.ts + shared organize.ts + main/preload khối riêng, typecheck sạch
- 19:01 test desktop organize + organize-junk: 20/20 pass
- 19:06 web lib/store/i18n/components organize + tab DiskView; store test 5/5; typecheck web/desktop/shared sạch
- 19:09 E2E v101-organize.cjs (4481/9481): 27/27 PASS, ảnh ở docs/qa/v101/WP-3b/
- 19:09 code_index desktop/web cập nhật; audit comment / text sạch
