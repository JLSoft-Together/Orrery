## WP-v87-1b-web — PARTIAL
- Nguyên nhân shiki nhân đôi: `lib/highlight.ts` có fallback `await import('./highlightCore')` trên main thread, còn `highlight.worker.ts` cũng import `highlightCore`; worker `{format:'es'}` được Vite bundle riêng nên mỗi grammar (`shiki/langs/*`) có 1 chunk cho main + 1 chunk cho worker. Không phải 2 version shiki (lock chỉ 1 `shiki` 4.x).
- Đã đổi: `apps/web/src/lib/highlight.ts` (fallback main-thread chỉ khi `!import.meta.env.PROD`; test vẫn chạy được fallback; ở bản đóng gói worker chết thì render plain, không còn chạy highlight trên main thread); `scripts/gen-file-icons.mjs` (+`compactPath`, nén số thừa trong path `d`, lossless) + 2341 file `apps/web/public/file-icons/**` đã nén tương ứng; `components/PhoneSettings.vue`, `components/devices/DeviceQrPopover.vue` (`qrcode` import động, đã dùng async sẵn).
- Tiêu chí 2/4 đạt, 1 chưa đo thêm 1 phần: (1) đạt, mỗi grammar shiki 1 chunk (chunk trùng tên còn lại là CodeMirror legacy-modes, khác nội dung); (2) CHƯA ĐẠT "rõ rệt" — chỉ -1.9%; (3) bảng size có; (4) test + typecheck sạch.
| | trước | sau |
|---|---|---|
| dist raw | 23,093,253 B | 20,220,514 B (-12.4%) |
| dist zip | 11,858,318 B | 11,476,701 B (-3.2%) |
| số file | 2991 | 2951 |
| JS chunk | 188 | 148 |
| JS tổng | 11,337,103 B | 8,555,469 B |
| assets/ | 18,200,809 B | 15,419,175 B |
| file-icons | 4,865,043 B | 4,773,938 B |
| chunk đầu (index) | 1,414,620 B | 1,389,099 B |
- Audit: diff 4 file src/scripts + svg icons; số trong `d` so khớp từng file trước/sau (script kiểm số học, 0 lệch). Backup icon gốc: `/tmp/file-icons-backup`.
- Review: file-icons "9.5 MB" trong `du` chủ yếu là cluster 4 KB x 2341 file; byte thật 4.8 MB, đã minify sẵn, chỉ ~11 file > 20 KB (koka 130 KB x2, lex 81 KB). Giảm thật cần (a) làm tròn toạ độ các icon tracing nặng (lossy, cần nhìn), (b) gộp 2341 file thành 1 bundle / sprite hoặc đóng asar (đổi loader `lib/explorer/fileIcons.ts`, ngoài mức an toàn của gói này), (c) bỏ 185 file trùng nội dung (cần bảng alias). Chunk `index` 1.39 MB: i18n en+vi 552 KB inline, components 766 KB; `WorkspaceView` 1.5 MB: components 1.4 MB + xterm 0.5 MB (đã lazy theo view). Tách thêm i18n lazy đổi hành vi boot, chưa làm.
- Test: `npm test -w @ccm/web -- --maxWorkers=4` = 218 file, 1865 test pass. `npm run typecheck -w @ccm/web` sạch. `npm run build -w @ccm/web` OK.
- Nhìn: CHƯA NHÌN (explorer icon, highlight code/diff). Main mở app (`npm run desktop`), vào Workspace > Explorer xem icon, mở file .cpp / .dart / diff xem tô màu.
- Ngoài phạm vi: bộ cài (`dist:win`) chưa đo; không đổi package.json / lock.
- Commit message đề xuất: ♻️ web: drop duplicate shiki grammar chunks (worker-only highlight in prod), compact file-icon path data, lazy qrcode
