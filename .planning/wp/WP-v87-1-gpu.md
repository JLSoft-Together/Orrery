## WP-v87-1-gpu: Bước 1 — bỏ blur, không vẽ lặp khi rảnh (CSS / template / canvas)
- Quyết định chủ nhân 2026-11-09: được bỏ blur (`backdrop-filter`) trên toolbar + sidebar để giảm GPU. Rule cũ "Translucent material (blur) only on chrome layers" trong `CLAUDE.md` project → WP này sửa đúng dòng đó (chrome layer dùng nền opaque theo token, không blur, vì GPU; owner 2026-11-09) và `.planning/AUDIT-v18-deck-reference.md` §4 nếu có nhắc blur.
- Mục tiêu: GPU lúc rảnh gần 0: không còn layer blur, không có animation CSS chạy lặp vô hạn khi không có thay đổi, không có rAF loop chạy liên tục khi rảnh. Giữ rule chủ nhân: mọi thay đổi (thêm / xoá / đổi tên / đổi trạng thái) vẫn có animation — chỉ bỏ animation lặp vô hạn chạy khi rảnh, hoặc đổi sang chạy hữu hạn / chỉ khi trạng thái đang đổi.
- Việc:
  1. Tìm mọi `backdrop-filter` / `backdrop-blur` / class material (`ccm-material` …) trong `apps/web/src` (css, tailwind config, .vue). Thay bằng nền opaque theo theme token (giữ tương phản, đúng `.planning/COLOR-SYSTEM.md`, đủ cho mọi theme trong `themes.css`; `lib/themes.test.ts` phải pass). Popover / menu / dialog vốn opaque, không đổi.
  2. Tìm `@keyframes` + `animation: … infinite`, `animate-spin|pulse|ping` của Tailwind, `animation-iteration-count: infinite`. Với từng cái: chỉ hiện khi đang có việc (spinner loading, status working) thì giữ nhưng đảm bảo không render khi ẩn (`v-if`, không chỉ `opacity: 0`); chạy cả khi rảnh (glow, shimmer, pulse trang trí) thì đổi thành chạy hữu hạn hoặc bỏ. Liệt kê trong report.
  3. rAF loop / canvas vẽ liên tục trong component (Agent Map, Mission Control, dust, office…): đảm bảo dừng khi không có thay đổi / view không hiện. Chỉ sửa phần vẽ; timer / interval thuần JS để WP-v87-1-idle lo.
  4. `will-change` để lâu dài, `filter: blur/drop-shadow` trên phần tử lớn đang animate: gỡ nếu không cần.
- Phạm vi file: `apps/web/src/**/*.css`, `apps/web/tailwind.config.*`, phần `<template>` class và `<style>` của `.vue`, code vẽ canvas / rAF trong component; `CLAUDE.md` project (đúng dòng blur); `.planning/AUDIT-v18-deck-reference.md`. Cấm đụng: `apps/desktop`, `apps/collector`, timer / interval / polling trong `<script>` (WP-v87-1-idle), `scripts/`, `docs/perf/`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] `rg -n "backdrop" apps/web/src` không còn chỗ nào áp blur (liệt kê chỗ còn lại + lý do).
  - [ ] Danh sách animation infinite trước / sau trong report; chỗ nào giữ thì vì sao và đảm bảo không render khi ẩn.
  - [ ] `npm test -w @ccm/web -- --maxWorkers=6` (gồm `lib/themes.test.ts`) + `npm run typecheck -w @ccm/web` pass.
- Nhìn thấy được: có → CHƯA NHÌN (main QA sau bằng instance riêng).
- Báo cáo: `.planning/wp/WP-v87-1-gpu-report.md`

## Nhật ký
- Audit: backdrop chỉ ở style.css .ccm-material + CodeEditor .cm-panels; infinite anim đều gắn state (v-if / class có điều kiện).
- Sửa: style.css (bỏ backdrop-filter, 2 token material opaque), themes.css (material = raised), CodeEditor.vue (bỏ blur), CLAUDE.md, AUDIT-v18 §4.
- Test: 213 file / 1823 pass; vue-tsc sạch. Xong, đã ghi report.
