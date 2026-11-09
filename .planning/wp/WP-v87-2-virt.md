## WP-v87-2-virt: Rail + Agent Map chỉ vẽ phần đang nhìn thấy
- Bối cảnh: profile demo-30x6 (`.planning/wp/WP-v87-1-prof-report.md`, `docs/perf/v87-prof-win32.json`): DOM 6 425 node (rail 2 019 ≈ 67 node / card, Agent Map 4 201 ≈ 20 node / agent), ~36 % main thread renderer là style / layout / paint native. Sau bước 1 renderer vẫn 98.1 % một core. Cần giảm việc trình duyệt tính layout + vẽ cho phần không nhìn thấy.
- Mục tiêu: rail và Agent Map không tốn layout / paint cho phần ngoài viewport; ưu tiên cách ít rủi ro, giữ đủ hành vi hiện có.
- Việc (chọn cách theo thứ tự rủi ro, ghi lý do trong report):
  1. Rail (`components/SessionList.vue`, `SessionCard*.vue`): bước A — CSS `content-visibility: auto` + `contain-intrinsic-size: auto <chiều cao ước lượng>` trên mỗi card (và `contain: layout paint style` nếu hợp), card ngoài viewport bị trình duyệt bỏ qua layout / paint mà không unmount → giữ TransitionGroup / FLIP move, `frozen` order, `scrollIntoView` theo `ui.revealRequest`, focus / keyboard, rename. Bước B (chỉ khi A không đủ và an toàn): windowing thật (mount card trong viewport + 1 màn đệm). Nếu không chắc giữ được các hành vi trên → dừng ở A, ghi đề xuất B.
  2. Agent Map (`components/AgentMap.vue`, `MainNode.vue`, `SubPill.vue`, `TreeEdge.vue`, `PeerEdge.vue`): thử `only-render-visible-elements` của Vue Flow. Rủi ro đã biết: node chưa mount thì không có `dimensions` → `fitView`, minimap, `ui.nodeSizes` / layout có thể sai. Nếu bật: phải giữ kích thước đã đo (cache theo id, hoặc kích thước cố định / ước lượng cho node chưa đo) để layout và fit view không nhảy. Nếu rủi ro quá → thay bằng `content-visibility: auto` trong nội dung node (không trên wrapper Vue Flow dùng để đo) hoặc giảm DOM mỗi node (gộp wrapper, bỏ phần tử ẩn còn mount, `v-if` thay `v-show` cho phần hiếm hiện). Ghi rõ đã chọn gì.
  3. Đếm DOM trước / sau nếu có thể bằng test (vd mount SessionList với 30 terminal, đếm node card); nếu không, ghi CHƯA KIỂM.
  4. Giữ rule chủ nhân: mọi thay đổi vẫn có animation (enter / leave / move row, capsule, pulse edge); không blur; màu qua theme token; không cuộn ngang.
- Phạm vi file: `apps/web/src/components/SessionList.vue`, `SessionCard.vue`, `SessionCardHeader.vue`, `SessionCardSignals.vue`, `SubPill.vue`, `MainNode.vue`, `AgentMap.vue`, `TreeEdge.vue`, `PeerEdge.vue`, `components/map/*`, test tương ứng; `apps/web/src/style.css` chỉ khi thật cần (ghi lý do). Cấm đụng: `stores/*` (WP-v87-2-cap đang sửa `stores/connection.ts` song song), `lib/motion.ts`, `components/workspace/*`, `apps/collector`, `apps/desktop`, `packages/`, `scripts/`, `docs/`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Rail: card ngoài viewport không layout / paint (content-visibility hoặc windowing), các hành vi liệt kê ở mục 1 còn nguyên (đọc code + test hiện có pass; mục nào không kiểm được ghi CHƯA KIỂM).
  - [ ] Agent Map: đã chọn và áp một cách giảm cost phần ngoài viewport, fit view / layout không phụ thuộc node đã mount (hoặc lý do không làm).
  - [ ] `npm test -w @ccm/web -- --maxWorkers=6` + `npm run typecheck -w @ccm/web` pass.
  - [ ] Đo lại: CHƯA KIỂM trong WP (main build stage + perf-bench).
- Nhìn thấy được: có (rail cuộn, Agent Map pan / zoom / fit view) → CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-2-virt-report.md`

## Nhật ký
- Rail: thêm `content-visibility: auto` + padding/margin 4px lên li (SessionList.vue)
- Agent Map: KHÔNG áp (xem report); test 1845 pass, typecheck pass
