## WP-v87-1-idle: Bước 1 — dừng việc nền khi cửa sổ ẩn / thu nhỏ / rảnh (timer, polling, throttling)
- Mục tiêu: khi cửa sổ Orrery bị thu nhỏ, ẩn xuống tray hoặc bị che, renderer gần như không tốn CPU; khi đang hiện mà rảnh, không có timer chạy dày vô ích.
- Việc:
  1. Electron main (`apps/desktop/src/main.ts`, `createMainWindow` ~481 + compact window ~525): kiểm `webPreferences.backgroundThrottling` (không được `false`) và chỗ nào ép renderer chạy nền (`setBackgroundThrottling(false)`, `powerSaveBlocker`, `paintWhenInitiallyHidden`…). Gửi trạng thái cửa sổ (minimize / restore / hide / show) xuống renderer qua kênh preload nếu cần.
  2. Renderer: composable dùng chung (vd `composables/usePageActive.ts`, hoặc mở rộng cái đã có) = `document.visibilityState` + trạng thái minimize từ main. Rà mọi `setInterval` / `setTimeout` lặp / polling trong `apps/web/src` (clock `now`, poll usage, git, resources, attention, peers, toast…): trang không active thì dừng hoặc giãn mạnh (vd ≥ 30 s); active lại thì chạy bù 1 lần ngay. Timer nào đã nhẹ / đã gate thì để nguyên, ghi vào report.
  3. Collector WS event khi cửa sổ ẩn: nếu gọn, renderer báo collector `client.active { on }` để collector gộp event thưa hơn (vd 1 s) cho client ẩn; nếu chạm protocol nhiều thì chỉ ghi đề xuất vào report.
  4. List dài: liệt kê list nào có thể render > 200 item mà không virtualize (activity, history, session list, git log, explorer). Chỉ virtualize nếu repo đã có thư viện / composable; không thì ghi đề xuất.
- Phạm vi file: `apps/desktop/src/main.ts` (chỉ vùng window / webPreferences / IPC trạng thái cửa sổ), `apps/desktop/src/preload.ts` (nếu thêm kênh), `apps/web/src/composables/*`, `<script>` của component / store có timer, `apps/web/src/lib/desktop.ts`, test tương ứng; `apps/collector` + `packages/shared` chỉ khi làm mục 3 gọn. Cấm đụng: CSS / class trong `<template>` / `<style>` (WP-v87-1-gpu), `scripts/`, `docs/perf/`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Bảng timer trong report: file:line, chu kỳ, trước / sau khi ẩn.
  - [ ] Test composable active / inactive (fake timers, mock visibility).
  - [ ] `npm test -w @ccm/web -- --maxWorkers=6`, `npm test -w @ccm/desktop -- --maxWorkers=6`, typecheck các workspace đã đổi pass.
- Nhìn thấy được: không (khi đang hiện, hành vi giữ nguyên).
- Báo cáo: `.planning/wp/WP-v87-1-idle-report.md`

## Nhật ký
- 1 AUDIT: main.ts khong co backgroundThrottling:false / paintWhenInitiallyHidden; powerSaveBlocker chi cho awake (chu y: dung y). visibilitychange da du cho minimize/tray, khong them IPC.
- 2 Them composables/usePageActive.ts (+test); FlipClock, HistoryView poll 30s, toolSpans prune 60s gate theo visibility.
- 3 Test/typecheck bi resource-guard chan (CPU 84-99%); bao CHUA KIEM. Report da ghi.
- 4 Chay lai khi CPU thap: web test 1827 pass, typecheck sach. DONE.
