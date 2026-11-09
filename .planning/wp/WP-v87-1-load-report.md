## WP-v87-1-load — PARTIAL
- Đã đổi:
  - `apps/web/src/stores/connection.ts` — `accept()` (parse + kiểm seq, giữ nguyên logic resync), hàng đợi `queue`, `receive()` (WS path: gom event, flush bằng rAF, deadline chờ 50 ms nếu rAF không chạy), `flush()`. Snapshot và message `direct` flush hàng đợi rồi áp ngay (đúng thứ tự). `connect()` và `ws.onclose` flush trước khi reset `lastSeq`. `onMessage()` giữ nghĩa đồng bộ (cho test cũ), export thêm `receive`, `flush`. `ws.onmessage` gọi `receive`.
  - `apps/web/src/components/TreeEdge.vue` — không bắt đầu pulse khi cửa sổ ẩn (`pageActive`), tránh giữ runner budget khi rAF bị treo.
  - `apps/web/src/components/workspace/TermPane.vue`, `DropTerminal.vue` — `cursorBlink` mặc định false, bật khi textarea focus, tắt khi blur (blur cửa sổ cũng blur textarea).
  - Test mới `apps/web/src/stores/connection.batch.test.ts` (8 test).
- Từng mục:
  1. WS → store: LÀM. Nhiều message trong cùng frame giờ thành 1 lần `apply` (1 lần `activity.push`, 1 lượt flush render của Vue thay vì mỗi message). Ước lợi: giảm số lần recompute `agents.byTerminal`/`list`/`terminals.list` (sort toàn bộ) và patch DOM từ N/frame xuống 1/frame; chưa đo. Không đổi store sang shallowRef: store dùng `reactive` theo id, patch theo id sẵn, đổi sẽ vỡ nhiều consumer (rủi ro > lợi, chưa có số đo).
  2. TreeEdge pulse: LÀM một phần. Code sẵn đã tính trước mẫu path (`samplePath`), chỉ `setAttribute` mỗi frame, max 6 runner đồng thời (`MAX_RUNNERS` trong AgentMap). Thêm chặn khi cửa sổ ẩn. Không chuyển sang CSS `offset-path`: dot dựng trong SVG, `offset-path` trên phần tử SVG kém ổn định + mất đồng bộ với edge đổi hình (`pathRev`); ước lợi nhỏ vì đã có max 6 pulse. Agent Map ẩn: edge unmount nên `onBeforeUnmount(stop)` đã dừng.
  3. Transition row: KHÔNG làm. Row dùng Vue TransitionGroup + CSS class (`ccm-list`, `pinLeaving`); 2 rAF là nội bộ Vue, không có JS hook 1 400 ms trong row (1 400 ms là `LINGER_MS` của AgentMap, timer theo id, đã tôn trọng reduced motion). Batching ở mục 1 đã gộp mọi thay đổi cùng frame vào một lượt animate. Animation không bị bỏ.
  4. `ccm-spin`: KHÔNG đổi. Đã là CSS `transform: rotate` (compositor), đều `v-if`/class theo trạng thái busy; spinner ở các dialog chỉ có khi dialog mở. `StatusIcon` dùng `ccm-spin-once` (0.9 s một lần, không lặp). CHƯA ĐẾM số spinner đồng thời ở demo-30x6 (cần chạy app).
  5. cursorBlink: LÀM. Ước lợi nhỏ (xterm vốn chỉ blink khi focus), nhưng chắc chắn không còn timer blink ở pane không focus / cửa sổ blur. CHƯA KIỂM bằng trace.
  6. DOM 6 472 node: CHỈ ĐỀ XUẤT. Chưa có thư viện virtualize trong apps/web. Đề xuất: virtualize danh sách activity (cap 1000 + 500 tool) và danh sách session rail khi > 50 hàng; Agent Map đã chỉ render node đang hiện. Cần đo node theo vùng UI trước khi chọn.
- Tiêu chí: 3/4 đạt (mỗi mục có lý do + ước lợi; test batching; test + typecheck pass). Đo lại: CHƯA KIỂM (main chạy `scripts/perf-bench.mjs`).
- Audit: working tree có sẵn thay đổi của gói khác (usePageActive, toolSpans, FlipClock…), không đụng. Chỉ sửa 4 file + 1 test mới trong phạm vi.
- Review: snapshot khi batch chờ → flush trước rồi hydrate; reconnect giữa batch → `connect()`/`onclose` flush, `lastSeq` reset sau đó; removed→created cùng id → áp tuần tự đúng thứ tự (có test); cửa sổ ẩn rAF không chạy → deadline chờ 50 ms áp batch (có test); reduced motion không ảnh hưởng batching; Agent Map ẩn không ảnh hưởng. Rủi ro còn lại: độ trễ tối đa 1 frame (~16 ms) cho event incremental, 50 ms khi cửa sổ ẩn.
- Test: `npm test -w @ccm/web -- --maxWorkers=6` → 1835 pass / 0 fail (215 file); `npm run typecheck -w @ccm/web` → pass.
- Nhìn: CHƯA NHÌN: không được chạy app (animation edge / row / blink).
- Commit message đề xuất:
  Renderer: batch WebSocket events per frame, pause edge pulses while hidden, blink terminal cursor only when focused
