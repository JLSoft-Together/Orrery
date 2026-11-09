## WP-v87-2-map — DONE (code), CHƯA NHÌN / CHƯA ĐO
### Đã đổi
- `components/AgentMap.vue`: `only-render-visible-elements` trên `<VueFlow>` (~1296); destructure `nodes: allNodes` thay `getNodes` (getNodes của Vue Flow bị lọc khi bật cờ) ở syncNodes `current`, applyPicked, flowBox/arcClearance, autoFit, `nodeIds`, tab-switch restore; `sizeOf()` + `flowBox`, `goToPeer`, `ensureVisible` dùng `nodeSize(n, ui.nodeSizes)`; `focusWhenMounted()` (retry rAF ≤30 frame) cho `moveCursor` / `goToPeer` vì node đích có thể chưa mount; `SeenNodes` provide `ccm-map-seen` (prune theo `wanted` trong syncNodes) + `ccm-map-seen-edges`.
- `components/map/useOffscreenCapsules.ts`: dep mới `sizeOf` thay `n.dimensions.width || MAIN_WIDTH`.
- `components/PeerEdge.vue`: `boxOf` dùng `nodeSize`; fade-in chuyển sang class `ccm-peer-edge-enter` chỉ khi edge mount lần đầu.
- `composables/useMapNode.ts`: `animateIn` (take từ SeenNodes), `popDelay` = 0 khi remount; `MainNode.vue` / `SubPill.vue`: `ccm-node-in` / `ccm-pill-in` chỉ khi `animateIn`.
- Mới: `lib/mapNodeSize.ts` (`nodeSize`, `defaultNodeSize` theo type main/sub/tray, `SeenNodes`) + `lib/mapNodeSize.test.ts` (6 test).
- Chỗ còn đọc `node.dimensions` trực tiếp: `AgentDetails.vue:116-117` (ngoài phạm vi; node đang mở popover thì đang hiển thị) và `AgentMap.vue ~985 zoomTo` (chờ node mới đo, node mới luôn mount).
### Kết luận mã Vue Flow 1.48.2 (`dist/vue-flow-core.mjs`)
- `getNodes` (7821): lọc bằng `getNodesInside(..., partially=true)` (3735): node chưa đo (`dimensions` null) = visible; chồng lấn > 0 với viewport = visible; node `dragging` luôn visible. Lần đầu mọi node mount để đo, sau đó mới bị cull.
- Unmount KHÔNG xoá `dimensions` / `handleBounds` (chỉ `updateNodeDimensions` ghi, 6305); node vẫn nằm trong `state.nodes`, nên `findNode`, `fitView` (5680, đọc `state.nodes`, bỏ node chưa đo), `setCenter`, `useNodesInitialized` vẫn đúng. Chỉ `useVueFlow().getNodes` bị lọc → đã đổi sang `nodes`.
- `getEdges` (7837): `isEdgeVisible` theo bbox hai đầu (edge nối node ngoài viewport nhưng cắt viewport vẫn hiện). Cảnh báo: dereference `source`/`target` không guard; AgentMap chỉ phát edge khi `nodeIds` có cả hai đầu và `removeNodes` xoá edge nối kèm nên an toàn.
### Tiêu chí 3/4
- [x] `only-render-visible-elements` bật.
- [x] Mọi chỗ đọc kích thước có fallback (liệt kê trên).
- [x] Test logic + test/typecheck pass.
- [ ] Đo + nhìn: CHƯA KIỂM / CHƯA NHÌN.
### Audit / Review / Test
- Audit: không đụng stores/*, SessionList, motion.ts, workspace, collector, desktop. Không comment mới.
- Review edge case: nodeSizes là kích thước user resize, chỉ là fallback sau dimensions (đo thật). Node từng mount có dimensions giữ lại nên fallback thực tế chỉ cho node chưa đo. Rủi ro còn lại: (1) node đang focus bàn phím bị pan ra khỏi viewport thì unmount, mất focus (moveCursor tự pan trước rồi focus nên đường phím ổn). (2) `ccm-edge-new` draw-in replay nếu remount trong ~vài trăm ms đầu. (3) AgentDetails vẫn dùng dimensions trực tiếp. (4) Edge tree/peer ngoài viewport không mount => flight/pulse bỏ qua (mong muốn); capsule offscreen vẫn chạy từ state.
- Test: `npm run typecheck -w @ccm/web` → pass; `npm test -w @ccm/web -- --maxWorkers=6` → 217 files / 1860 tests passed. Test cho useOffscreenCapsules với node không mount: không viết (cần i18n + mount), logic size đã test qua helper.
### Main cần nhìn bằng mắt
Build stage + demo-30x6: (a) đếm `.vue-flow__node` giảm; (b) pan/zoom nhanh không flash, node vào lại không chạy animation enter; (c) fit view lần đầu + sau tab switch (hiện đủ node, animation enter vẫn chạy khi mở map); (d) peer edge arc + envelope bay, capsule offscreen khi node ngoài view; (e) phím mũi tên / goToPeer focus node ngoài view; (f) resize handle, rename, context menu node; (g) edge tree không biến mất khi cạnh còn cắt viewport. `.code_index/web.md` chưa cập nhật (ngoài phạm vi).
### Commit đề xuất
⚡️ Agent Map mounts only visible nodes and edges: only-render-visible-elements, cached node sizes, no enter animation on remount

## Vòng 2
1. `AgentMap.vue focusWhenMounted`: token `focusToken` tăng mỗi lần gọi, retry rAF dừng khi token đổi hoặc `mapUnmounted` (onBeforeUnmount). Node A mount muộn không cướp focus của B.
2. `seenEdges` (provide `ccm-map-seen-edges`) prune theo id `peerEdges` đang sống (watch), edge bị xoá rồi thêm lại animate enter lại.
3. `PeerMapContext.sizeOf?` (useMapNode.ts) = `sizeOf` của AgentMap (có `ui.nodeSizes`); `PeerEdge.boxOf` dùng `ctx.sizeOf`, fallback `nodeSize` khi không có ctx (test cũ không đổi).
4. Rail `.ccm-rail-item` soát SessionCard*: `shadow-card` = `0 0 0 0.5px, 0 1px 3px` (tràn ≤ 4px); focus ring `ring-2` 2px; outline 2px + offset 2px = 4px; `.ccm-flash::after` inset -1px + border 2px = 3px; notice / header absolute nằm trong card; ContextMenu dùng Teleport to body. Max tràn = 4px = padding hiện tại, giữ nguyên, không sửa SessionList.vue.
5. Chấp nhận (không sửa): peer arc bị cull khi cả 2 đầu ngoài viewport; tab stop mất khi node đang focus bị cull.
- Test: typecheck pass; 217 files / 1860 tests pass.
