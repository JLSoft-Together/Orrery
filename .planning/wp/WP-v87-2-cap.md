## WP-v87-2-cap: Giới hạn tần suất cập nhật hình khi nhiều agent
- Bối cảnh: sau bước 1 (`docs/perf/v87-step1b-win32.json`), demo-30x6 renderer 98.1 % một core. Profile (`.planning/wp/WP-v87-1-prof-report.md`): ~41 % main thread nằm trong flush scheduler Vue sau mỗi lô store, ~36 % là style / layout / paint native. `stores/connection.ts` hiện flush WS event mỗi frame (`schedule()` rAF + fallback 50 ms, ~dòng 310-345). Mỗi lô → patch store → Vue re-render → layout / paint.
- Mục tiêu: khi tải cao, gom event thành lô thưa hơn (4–5 lần mỗi giây) để số lần re-render + layout giảm mạnh; khi tải thấp giữ cảm giác tức thì như hiện tại.
- Việc:
  1. `stores/connection.ts`: lịch flush thích ứng. Đo tốc độ event (cửa sổ trượt ~1 s, hoặc EWMA). Dưới ngưỡng (vd < 20 event/s) → giữ rAF như cũ. Trên ngưỡng → flush theo khoảng `HEAVY_FLUSH_MS` (~200–250 ms); có hysteresis để không nhảy qua lại. Đặt hằng số có tên rõ ràng, export để test.
  2. Giữ nguyên: snapshot / message `immediate` flush ngay; `onMessage` sync cho test; `connect` / `onclose` flush; thứ tự event trong lô; `document.hidden` thì có thể dùng khoảng dài hơn (vd 1 s) nhưng không được mất event.
  3. Event cần phản hồi ngay vẫn đi nhanh khi tải cao: event của terminal / agent đang được chọn (selected) hoặc chuyển sang trạng thái cần người dùng (waiting / permission / error / done) → flush lô ở frame kế, không chờ 250 ms. Tra tên status thật trong `packages/shared/src/types.ts`.
  4. Test (`stores/connection.batch.test.ts` hoặc file test mới cạnh đó, fake timers): tải thấp flush theo frame; vượt ngưỡng chuyển sang khoảng dài; hết tải quay về rAF (hysteresis); snapshot giữa lúc tải cao flush ngay và đúng thứ tự; event status cần chú ý flush sớm; không mất / lặp event.
- Phạm vi file: `apps/web/src/stores/connection.ts`, `apps/web/src/stores/connection.batch.test.ts` (hoặc test mới cạnh nó). Cấm đụng: component, `stores/agents.ts`, `terminals.ts`, `lib/*`, `apps/collector`, `apps/desktop`, `packages/`, `scripts/`, `docs/`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Tải thấp: hành vi như cũ (test).
  - [ ] Tải cao: số lần `apply()` ≤ ~5/giây (test với fake timers, vd bơm 200 event/s trong 2 s → ≤ ~12 lần apply).
  - [ ] Event cần chú ý / selected đi trong ≤ 1 frame (test).
  - [ ] `npm test -w @ccm/web -- --maxWorkers=6` + `npm run typecheck -w @ccm/web` pass.
  - [ ] Đo lại: CHƯA KIỂM trong WP (main build stage + perf-bench).
- Nhìn thấy được: có nhẹ (trạng thái cập nhật trễ tối đa ~250 ms khi nhiều agent) → CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-2-cap-report.md`

## Nhật ký
- connection.ts: lich flush thich ung (rate window 1s, burst 250ms, slowTimer 220ms / 1s hidden, urgent bypass)
- connection.batch.test.ts: +9 test adaptive flush; web test 1854 pass, typecheck sach
