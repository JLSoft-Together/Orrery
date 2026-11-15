## WP-2sec: vá guard xoá (review opus 15/11)

- Nguồn: review chỉ đọc đường xoá của Ổ đĩa. Kết luận "chưa an toàn để ship". WP này sửa phần **diskMap guard + main + delete**; phần `diskClean/**` giao cho WP-2b (đang sửa cùng thư mục).
- Phạm vi file: `apps/desktop/src/diskMap/guard.ts`, `delete.ts`, `ipc.ts`, `map.ts` (chỉ chỗ gọi guard), khối `setupDiskMap` + `setupDiskClean` trong `apps/desktop/src/main.ts` (chỉ `autoConfirm` và danh sách keep), test `apps/desktop/test/diskMap-*.test.ts`. Được export guard mới để `diskClean` dùng (WP-2b sẽ gọi).
- Cấm đụng: `apps/desktop/src/diskClean/**` (WP-2b), file WP-2c đang sửa (`diskMap/**` phần op `sizes` nếu có: đọc lại trước khi Edit), `apps/shell/**`.
- Model: opus.

## Danh sách

1. HIGH `guard.ts:20-26,62-72`: so path dạng chuỗi; Windows bỏ dấu `.` / khoảng trắng cuối segment và nhận tên 8.3, nên `C:\Users\dauph.`, `C:\Users\dauph `, `C:\Users\DAUPH~1`, `C:\Users.`, `C:\PROGRA~1` lọt guard rồi `trashItem` / `rm` xoá thư mục thật. Sửa: win32 từ chối segment kết thúc bằng `.` hoặc space, chứa `~` (sau khi đã thử resolve), hoặc `:` sau ổ; và kiểm trên path canonical: `fs.realpath.native(dirname(p))` + basename, so lại với guard.
2. MEDIUM `guard.ts` + `ipc.ts:137`: không realpath thư mục cha nên path đi qua junction / symlink ở giữa thoát deny-list (`D:\x\lnk\me\.ssh`, `lnk` → `C:\Users`). Sửa: realpath cha rồi kiểm lại; bắt buộc path là node có trong index (`disk.children` / op tra node) thay vì chỉ so với `roots`.
3. MEDIUM `main.ts` keep list + `guard.ts:13-16`: thiếu bảo vệ `~/AppData` (cả thư mục), `~/AppData/Roaming`, `~/AppData/Local` gốc, `~/.ssh`, `~/OneDrive` gốc, macOS `~/Library` gốc, `~/Library/Mobile Documents` (iCloud Drive), `~/Library/CloudStorage`, `C:\Users\Public`, `C:\Users\Default`, mọi con trực tiếp của `C:\Users` và `/Users`, mount point `/Volumes/*` và mọi root ổ từ `disk.volumes`. Thêm vào danh sách bảo vệ (chính thư mục đó, không phải con của nó, trừ khi là gốc cloud / `.ssh` / `Library` thì bảo vệ cả cây).
4. MEDIUM `main.ts:1671,1702`: `autoConfirm` đọc env `CCM_E2E_DISKMAP_CONFIRM` cả ở bản đóng gói; user / agent ghi được env (`setx`) → bỏ qua cả 2 confirm. Sửa: `!app.isPackaged && ...`. Rà các env `CCM_E2E_*` khác trong khối diskMap / diskClean / menuToast cùng kiểu.
5. LOW `delete.ts:71-79`: `rm` dừng giữa chừng → báo failed, freed 0, bỏ `disk.forget`, index còn size cũ. Sửa: forget / đo lại khi lỗi không phải EACCES / EPERM; ghi lịch sử phần đã xoá.
6. LOW (đường diskMap): trước khi xoá, sau confirm, lstat + realpath cha lại (TOCTOU parent bị đổi thành junction trong lúc dialog mở).

## Tiêu chí

- [ ] Test tái hiện mục 1-4 (fail trước, pass sau): từng biến thể path Windows (chạy được trên Windows; macOS ghi CHƯA KIỂM), junction ở giữa (tạo junction thật trong thư mục tạm), danh sách bảo vệ mới, `autoConfirm` false khi `isPackaged`.
- [ ] Export hàm guard canonical dùng chung (`checkDeletableCanonical` hoặc tên tương tự) + ghi vào báo cáo để WP-2b gọi trong `diskClean`.
- [ ] `npm test -w @ccm/desktop -- diskMap` pass; typecheck desktop sạch; E2E `v101-diskmap-host.cjs` + `v101-diskmap-ui.cjs` vẫn pass trên bản dev tách biệt (cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173, chỉ thư mục tạm).
- [ ] Không comment trong code.
- Báo cáo: `.planning/wp/WP-2sec-report.md`.

## Nhật ký
- 15/11 AUDIT: đọc guard/delete/ipc/main + diskClean/service (dùng chung runDelete). Đo thật: realpath expand 8.3 (PROGRA~1 -> Program Files), trailing dot/space -> ENOENT ở Node nhưng shell vẫn chuẩn hoá, nên phải chặn lexical.
- 15/11 Sửa guard.ts (lexical win, canonical, list bảo vệ mới), delete.ts (canonical trước/sau confirm, known, partial rm, e2eAutoConfirm, measureRest), ipc.ts (known, volumes, measure), main.ts autoConfirm. Test diskMap pass; diskClean 2 fail do WP-2b đang sửa (ctx.keep thiếu, 5 kênh).
- 15/11 Xong: diskMap 65 test pass, typecheck sạch, E2E host 18/18 + ui 30/30. Báo cáo .planning/wp/WP-2sec-report.md.
