# WP-v88-W3 report: tab Quỹ đạo (Orbit)

Trạng thái: dừng ở điểm ổn định theo yêu cầu (chủ nhân đang xem hướng UI mới). Không làm thêm radial.

## Đã xong
- Đổi tên hiển thị Bầu trời -> Quỹ đạo (en Orbit): `i18n/ui-v88-monitor.ts`, `i18n/en.ts`, `i18n/vi.ts` (monitorMode), test ModeBar, `.code_index/web.md`. Mode nội bộ `sky` + key `ccm.monitorMode` giữ nguyên.
- Hàng chờ: `components/monitor/OrbitQueue.vue`, `OrbitRow.vue`, `OrbitMini.vue`; thay SessionList trong `MonitorView.vue` (chưa mở session tab). Cần anh luôn trên cùng. Lọc project dùng chung `ui.monitorProject` (`useMonitorSessions.ts` giờ lọc cả 2 mode).
- Layout radial: `lib/skyLayout.ts` (hash cố định theo tên project, quỹ đạo elip, thu chấm mờ > 8 session, nudge theo tool call, reduced motion đứng yên); `AgentMap.vue` prop `layout`, `SkyOrbit.vue`, `SkyDot.vue`, halo `data-sun` trong `MainNode.vue` + `style.css`.
- Office giữ nguyên. Panel lệnh, Alt+1/Alt+2 không đổi. Tour không trỏ SessionList nên không đổi.
- Test: `lib/skyLayout.test.ts`, `components/monitor/orbit.test.ts`; sửa 1 test monitor.test.ts (lọc project 2 mode).

## Còn lại / ghi chú
- Mặt trời vẫn là card MainNode có halo (chưa là hình tròn như wireframe H4); planet vẫn là SubPill.
- Hàng chờ bỏ Favorites / Jobs / nút xoá ended của SessionList trong tab này.
- Tree edge / tray ẩn ở radial; peer link dùng PeerEdge cũ (chưa kiểm hình dạng đường nét đứt trên layout radial).
- Chưa E2E, chưa xem bằng mắt.
