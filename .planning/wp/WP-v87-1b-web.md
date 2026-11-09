## WP-v87-1b-web: giảm size web dist trong bộ cài
- Bối cảnh: `dist:win` 2026-11-09: Setup 104.7 MB, cài xong 336 MB, `resources/web` 29 MB (assets 19 MB, file-icons 9.5 MB). 188 chunk JS; grammar shiki có 2 bản cho cùng ngôn ngữ (vd `cpp-jvSrPFZo.js` + `cpp-DddhVWhK.js` mỗi bản ~0.8 MB; tương tự astro, bat, c, csharp, css, dart, diff, dockerfile, go, groovy, html, ini, java, javascript, json, jsonc, jsx…). Chunk lớn: `WorkspaceView` 1.5 MB, `index` 1.4 MB. PLAN-v87 §6 bước 1b còn mục "chỉ đóng gói grammar shiki hay dùng, phần còn lại tải lười trong cùng app (không lấy từ network)".
- Mục tiêu:
  1. Tìm vì sao shiki bị nhân đôi (2 import path khác nhau: `shiki` vs `shiki/bundle/*` / `@shikijs/langs` / `@shikijs/core` hay 2 version trong lock?) — dồn về 1 nguồn, mỗi grammar còn 1 chunk. Không đổi màu / theme / ngôn ngữ hỗ trợ.
  2. `file-icons` 9.5 MB: xem gồm gì (`scripts/gen-file-icons.mjs`), có thể gộp / nén (sprite SVG, bỏ icon trùng, minify SVG) mà giữ đủ icon Atom Material đang dùng (`apps/web/src/lib/explorer/fileIconRules.ts`). Không lấy từ network.
  3. Kiểm chunk lớn `WorkspaceView` / `index`: thư viện nặng nào nằm sẵn trong chunk đầu mà chỉ cần khi mở tính năng (vd Office iso, editor) → tách lazy nếu an toàn. Không đổi hành vi / animation.
  4. Đo trước / sau: `npm run build -w @ccm/web`, tổng size `apps/web/dist` (raw + zip), số chunk, size chunk đầu tải khi mở app.
- Phạm vi file: `apps/web/**` (vite config, import shiki, file icons loader), `scripts/gen-file-icons.mjs` nếu cần, `package.json` / lock chỉ khi phải bỏ một bản shiki trùng (ghi lý do). Cấm đụng `apps/core`, `apps/collector`, `apps/desktop/src`, `scripts/{demo-fixture,perf-bench}.mjs` (WP khác).
- Skill: `frontend-design:frontend-design` không cần (không đổi UI). Theo CLAUDE.md: UI string qua i18n, không đổi màu.
- Model: sonnet.
- Được build: `npm run build -w @ccm/web` (đo size). Không `dist:*` (main đo bộ cài sau).
- File nhỏ: file mới ≤ ~400 dòng.
- Lệnh test: `npm test -w @ccm/web -- --maxWorkers=4`, `npm run typecheck -w @ccm/web`.
- Tiêu chí:
  - [ ] Mỗi grammar shiki 1 chunk; highlight code / diff / editor vẫn đúng (test hiện có pass).
  - [ ] file-icons nhỏ hơn rõ rệt, đủ icon (test / so danh sách tên trước sau).
  - [ ] Bảng size trước / sau (raw, zip, chunk đầu).
  - [ ] Test + typecheck sạch.
- Nhìn thấy được: có (icon explorer, highlight) → CHƯA NHÌN nếu không chạy app; ghi rõ cách main mở để xem.
- Báo cáo: `.planning/wp/WP-v87-1b-web-report.md`

## Nhật ký
- 17:10 audit: shiki nhân đôi vì `highlight.ts` import động `highlightCore` (fallback main-thread) + `highlight.worker.ts` import cùng file → Vite bundle worker riêng, copy mọi grammar lần 2. Sửa: fallback chỉ khi không PROD.
- 17:20 file-icons: nén lossless path `d` (gen script + 2341 file), -91 KB; qrcode lazy trong PhoneSettings / DeviceQrPopover.
- 17:27 test UI desktop 218 file / 1865 pass, typecheck sạch, build đo xong. Report ghi.
