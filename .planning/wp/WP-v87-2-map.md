## WP-v87-2-map: Agent Map chỉ mount node / edge đang nhìn thấy
- Bối cảnh: WP-v87-2-virt (report `.planning/wp/WP-v87-2-virt-report.md`) làm rail, bỏ qua Agent Map vì rủi ro `dimensions`. Map là vùng DOM lớn nhất ở demo-30x6: 4 201 node (~20 / agent, ~210 agent). Phải giảm.
- Hướng: bật `only-render-visible-elements` của Vue Flow trên `<VueFlow>` (`components/AgentMap.vue` ~dòng 1286), kèm bảo đảm mọi chỗ cần kích thước node không phụ thuộc node đang mount.
- Việc:
  1. Đọc mã Vue Flow trong `node_modules/@vue-flow/core` (bản đang cài): cách lọc node khi `onlyRenderVisibleElements` (node chưa đo có được coi là visible không), node bị unmount có giữ `dimensions` không, edge bị lọc thế nào. Ghi kết luận vào report.
  2. Kích thước: cache theo id (dùng `ui.nodeSizes` nếu đã là cache phù hợp — đọc `stores/ui.ts` chỉ đọc; nếu cần đổi store thì ghi vào "Ngoài phạm vi") hoặc fallback kích thước ước lượng cho node chưa từng đo. Sửa các chỗ đọc `node.dimensions` trong AgentMap / `useOffscreenCapsules` / layout (`~442`, `goToPeer ~751`, `~909`, fit view) để dùng kích thước đã cache khi node không mount.
  3. `fitView` lần đầu và sau tab switch, minimap, capsule offscreen, peer edge bay, pulse edge, resize handle, setCenter tới node ngoài viewport: phải chạy đúng. Node vừa vào viewport khi pan: mount lại không flash, không chạy animation enter như node mới (animation enter chỉ cho agent mới thật sự).
  4. Test: thêm test cho phần thuần logic (vd hàm lấy kích thước node có fallback cache, `useOffscreenCapsules` với node không mount).
- Phạm vi file: `apps/web/src/components/AgentMap.vue`, `MainNode.vue`, `SubPill.vue`, `TreeEdge.vue`, `PeerEdge.vue`, `components/map/*`, `composables/useOffscreenCapsules*` (nếu có), lib helper mới cho map + test. Cấm đụng: `stores/*` (WP-v87-2-cap đang sửa `stores/connection.ts`), `SessionList.vue`, `lib/motion.ts`, `components/workspace/*`, `apps/collector`, `apps/desktop`, `packages/`, `scripts/`, `docs/`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] `only-render-visible-elements` bật (hoặc lý do kỹ thuật cụ thể từ mã Vue Flow khiến không bật được + cách thay thế đã làm).
  - [ ] Mọi chỗ đọc kích thước node có fallback khi node không mount (liệt kê file:line).
  - [ ] Test logic mới + `npm test -w @ccm/web -- --maxWorkers=6` + `npm run typecheck -w @ccm/web` pass.
  - [ ] Đo lại + nhìn: CHƯA KIỂM trong WP (main build stage + perf-bench + chụp).
- Nhìn thấy được: có → CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-2-map-report.md`

## Nhật ký
- 08:20 Đọc mã Vue Flow 1.48.2; sửa AgentMap/MainNode/SubPill/PeerEdge/useMapNode/useOffscreenCapsules; thêm lib/mapNodeSize + test; typecheck + 217 files / 1860 tests pass. Báo cáo: WP-v87-2-map-report.md
