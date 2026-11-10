# WP-v88-W4 report: tab Quỹ đạo Q2 + Q3

## Đã xong
- Q2 tổng quan vòng ưu tiên: `components/monitor/OrbitRings.vue` + `lib/orbitRings.ts` (`computeRings`). 3 vòng theo trạng thái, 1 sector / project theo hash, > 8 session thì rảnh / xong thành chấm mờ ở vòng ngoài, subagent là chấm cạnh node, nudge khi có tool call (đứng yên khi reduced motion).
- Panel phải `OrbitDetail.vue` (bottom sheet khi `ui.narrow`). Chọn bằng click / mũi tên; Enter, bấm đúp hoặc nút "Xem hệ" vào Q3.
- Q3 tiêu điểm: `OrbitFocus.vue` + `lib/orbitFocus.ts` (`computeFocus`, `feedRows`): rail hệ trái, vệt cung tool trên quỹ đạo, dòng tool phải. Esc hoặc nút Quay lại về Q2, focus về node.
- `OrbitView.vue` giữ state chọn / tiêu điểm, menu session, nút Map / Office (chuyển từ AgentMap sang góc phải khi Office bật).
- `MonitorView.vue`: `!ui.sessionTab && ui.mapView === 'map'` thì OrbitView; còn lại AgentMap cũ (session tab = tree, Office giữ nguyên). Panel Sessions trái chỉ còn khi mở session tab.
- Gỡ code chết của W3: OrbitQueue / OrbitRow / OrbitMini / SkyOrbit / SkyDot / skyLayout (+ test), hoàn nguyên `AgentMap.vue`, `MainNode.vue`, phần CSS `.ccm-sky`. Key i18n `monitorV88.queue` bỏ, thêm `monitorV88.orbit.*` (en + vi).
- Dữ liệu tool: chỉ tên tool + giờ + thời lượng (ToolSpan) và `currentToolSummary` đã sanitize; không hiện raw args.

## Lưu ý
- Office mode không còn panel Sessions bên trái; nút "Sessions" trong thanh AgentMap khi ở Office sẽ mở panel rỗng (chỉ có ý nghĩa với session tab). Cần xem lại nếu chủ nhân muốn.
- Va chạm hash giữa 2 project vẫn đẩy 1 project sang slot kề (probing theo thứ tự hash), nên góc chỉ cố định khi không va chạm.
- Chưa E2E, chưa xem bằng mắt, chưa chạy app.

## Test
- Mới: `lib/orbitRings.test.ts` (8), `lib/orbitFocus.test.ts` (7), `components/monitor/orbit.test.ts` (8).
- `npm test -w @ccm/web -- --maxWorkers=4`: 223 file, 1907 test pass (1 lần chạy trước có 1 test QR devices fail rồi pass lại, không liên quan). `npm run typecheck`: sạch.
