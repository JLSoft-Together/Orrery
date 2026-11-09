## WP-v87-1-load: Bước 1 — renderer CPU / GPU khi nhiều agent đang chạy
- Số chuẩn 3.4.0 (`docs/perf/3.4.0-win32.json`, báo cáo `.planning/wp/WP-v87-0-report.md`): demo-6x3 renderer 16.6 % + GPU 14.5 % một core; demo-30x6 renderer **116.7 %** (539 MB) + GPU 25.3 %, main thread renderer bận 767 ms/s, FunctionCall 203/s, DOM 6 472 node. Khi có session busy: 35 frame/s renderer, 48 DrawAndSwap/s GPU. Lúc rảnh gần như không vẽ.
- Nguồn đã thấy trong trace: (1) pulse dot chạy dọc edge `components/TreeEdge.vue:~50-80` (rAF mỗi frame, `setAttribute('transform')`, có `runners.take()` giới hạn số pulse); (2) Vue `Transition` / `TransitionGroup` onLeave: 2 rAF + timer 1 400 ms mỗi lần thêm / xoá row; (3) spinner SVG `ccm-spin`; (4) WS `onmessage` → `stores/connection.ts:~320` `onMessage` cập nhật store theo từng message.
- Mục tiêu: demo-30x6 renderer CPU giảm ≥ 50 %, demo-6x3 renderer + GPU giảm rõ, không mất animation theo rule chủ nhân (mọi thay đổi vẫn có chuyển động; được giảm khi dồn dập — vd một đợt nhiều thay đổi trong cùng frame thì animate gộp / rút gọn).
- Việc (đo trước khi sửa từng mục nếu được, chọn việc theo lợi lớn nhất):
  1. WS → store: gom message theo frame (`requestAnimationFrame` hoặc micro-batch ~16–50 ms), áp một lần; store danh sách lớn dùng `shallowRef` / `markRaw` / cập nhật theo id thay vì thay mảng; tránh `computed` quét toàn bộ agent trên mỗi event. Snapshot đầu và thứ tự event phải giữ đúng.
  2. TreeEdge pulse: chuyển sang animation chạy trên compositor (CSS `offset-path` + `offset-distance` hoặc SMIL / Web Animations trên `transform`) để không chạy JS mỗi frame; hoặc giữ JS nhưng tính sẵn mẫu, giới hạn pulse đồng thời thấp hơn khi nhiều edge; không chạy khi Agent Map không hiện / cửa sổ ẩn.
  3. Transition row: bỏ timer 1 400 ms + rAF trong JS hook nếu có thể dùng CSS transition thuần; khi một đợt > N thay đổi (vd 20) trong một frame thì dùng animation gộp nhẹ thay vì từng row.
  4. Spinner `ccm-spin`: chỉ render khi đang hiện (đã v-if) — kiểm số spinner đồng thời ở demo-30x6; nếu nhiều, dùng 1 animation CSS trên `transform` (compositor) và không có layout.
  5. cursorBlink xterm (`TermPane.vue:~907`, `DropTerminal.vue:~194`): chỉ blink ở pane đang focus và khi cửa sổ đang focus.
  6. DOM 6 472 node ở demo-30x6: nếu list session / agent / activity là nguồn chính, đề xuất virtualize (ghi vào report, chưa làm nếu chưa có thư viện).
- Phạm vi file: `apps/web/src/stores/*` (connection, agents, terminals, activity, toolSpans…), `components/TreeEdge.vue` + component Agent Map / list liên quan, `lib/*` transition helpers, `components/workspace/TermPane.vue` + `DropTerminal.vue` (chỉ cursorBlink), test tương ứng. Cấm đụng: `apps/collector`, `apps/desktop`, `packages/`, `scripts/`, `docs/perf/`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Mỗi mục làm / không làm có lý do + ước lợi trong report.
  - [ ] Test cho batching store (thứ tự event, snapshot sau đó event, nhiều event cùng id trong 1 batch).
  - [ ] `npm test -w @ccm/web -- --maxWorkers=6` + `npm run typecheck -w @ccm/web` pass.
  - [ ] Đo lại: CHƯA KIỂM trong WP này (main build + chạy `scripts/perf-bench.mjs` sau).
- Nhìn thấy được: có (animation edge / row) → CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-1-load-report.md`

## Nhật ký
- 5 cursorBlink: TermPane/DropTerminal blink only while textarea focused (done)
- 1 connection.ts: receive()/flush() batch rAF+50ms fallback, snapshot/direct flush immediately, connect/onclose flush; onMessage kept sync for tests
- 2 TreeEdge: no pulse start when page hidden (pageActive); pulse already precomputed samples + max 6 runners (done)
- 3,4,6 khong doi code (ly do trong report); test+typecheck pass; report ghi
