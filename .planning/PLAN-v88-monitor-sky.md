# PLAN v88 — Monitor: hàng chờ + quỹ đạo + panel lệnh (chủ nhân duyệt 2026-11-11)

Wireframe chốt: artifact "Monitor redesign directions", artboard **H4 · Hàng chờ + quỹ đạo + panel lệnh** (https://claude.ai/artifact/FZYwp3gfSZSyYKtPDXXHPA). Chủ nhân chọn A + B, bỏ dải giờ C, thay bằng panel lệnh đang chạy ở dưới, ẩn / hiện được (2026-11-11).

## Bố cục
0. **2 tab chế độ** (thanh dưới header, chủ nhân 2026-11-11): `Bảng` = mode A (4 cột Kanban: Cần anh / Đang chạy / Rảnh / Xong hôm nay) · `Quỹ đạo` = hàng chờ + quỹ đạo. Phím 1 / 2, lưu trong `stores/ui.ts`. Panel lệnh dùng chung cho cả 2 tab.
- Agent (Claude / Codex / DeepSeek) chỉ hiện bằng icon (`workspace/PresetIcon.vue`), không ghi chữ; `aria-label` + tooltip mang tên.
1. **Hàng chờ (trái, ~280 px)** — thay `SessionList`: nhóm Cần anh / Đang chạy / Rảnh / Xong hôm nay (`lib/sessionGroups.ts` `groupSessions`, đã có). Mỗi dòng: vòng quỹ đạo mini (mặt trời = trạng thái agent chính, chấm = có subagent đang chạy), tên project · agent, tool + thời gian. Lọc project giữ `ProjectStrip` dropdown. Cửa sổ hẹp: bottom sheet như hiện tại.
2. **Quỹ đạo (giữa)** — `AgentMap` đổi layout: mỗi project 1 hệ, agent chính = mặt trời (vầng cam khi cần anh, xanh khi chạy, xám khi rảnh), subagent = hành tinh trên quỹ đạo, peer link = đường nét đứt (`PeerEdge` có sẵn). Zoom / pan / Vừa khung giữ cụm `data-map-zoom`.
3. **Panel "Lệnh đang chạy" (dưới)** — thanh 32 px luôn hiện (số lệnh, lệnh chạy lâu nhất); bấm hoặc phím `` ` `` để mở / đóng, lưu trạng thái trong `stores/ui.ts` (`ccm.*`). Bảng: Session · Lệnh · Tool · Đã chạy · Mở pane. Dòng đang chờ quyền tô cam.

## Bất biến
- Hàng chờ luôn đặt "Cần anh" trên cùng; trạng thái = icon + chữ + màu.
- Hành tinh không quay liên tục: chỉ nhích khi có tool call mới; `prefers-reduced-motion` = đứng yên.
- Vị trí mỗi hệ cố định theo hash tên project, không nhảy giữa các lần mở.
- > ~8 session: rảnh / xong thu thành chấm mờ ở rìa.
- Không blur, không gradient, màu theo token (`.planning/COLOR-SYSTEM.md`), chuỗi qua vue-i18n (en, vi).
- Monitor vẫn không gửi gì vào agent: nút "Mở pane" chỉ chuyển sang Workspace.

## Đã chốt (2026-11-11)
0. **Ưu tiên tab Bảng (mode A) + panel lệnh.** Tab Quỹ đạo (hàng chờ quỹ đạo mini + quỹ đạo radial) để bản sau; tạm thời tab 2 = Monitor hiện tại (SessionList + AgentMap + Office) giữ nguyên. Mặc định mode `board`.
1. Cột "Lệnh" hiện **đầy đủ lệnh**: field mới trong `packages/shared` `Agent` (`currentToolSummary`, `currentToolStartedAt`), sinh ở cả Node collector lẫn Rust core (mặc định rust) + Codex rollout; cắt ngắn, che secret (`mask_secrets` / mask TS hiện có), home → `~`, trước khi rời collector. Parity Node ↔ Rust.
2. **Office (Habblaud): để yên, không động gì** — giữ đúng cách mở / chế độ hiện tại, không xoá, không đổi.

## Thứ tự làm (sau khi duyệt)
1. Protocol + collector/core: field tool summary đã sanitize + thời điểm bắt đầu (nếu chốt câu 1).
2. Khung 3 vùng + panel lệnh (ẩn / hiện, phím tắt, lưu trạng thái).
3. Hàng chờ thay `SessionList` (vòng quỹ đạo mini).
4. Quỹ đạo: layout radial trong `AgentMap`.
5. Test (Vitest web), E2E Monitor, cập nhật `.code_index/web.md`, tour `lib/tour.ts`.

## Đổi hướng tab Quỹ đạo (chủ nhân 2026-11-11, canvas Q2 + Q3)
Thay bố cục hàng chờ + bầu trời H4 bằng 2 tầng:
1. **Tổng quan (Q2 · vòng ưu tiên):** tâm = "Bạn"; 3 vòng đồng tâm: trong = Cần anh, giữa = Đang chạy, ngoài = Rảnh / xong (mờ). Mỗi project 1 góc (sector) cố định theo hash tên, nhãn project ở rìa. Mỗi session main = node tròn có icon agent, viền màu trạng thái, Cần anh có vầng cam. Subagent = chấm nhỏ cạnh node (đặc = chạy, viền = chờ, xanh lá = xong). Panel phải (~320 px): session đang chọn — trạng thái + thời gian, câu hỏi / lệnh xin quyền + "Mở pane trả lời", danh sách subagent + tool, 3–5 tool gần nhất.
2. **Tiêu điểm (Q3):** bấm đúp node / nút "Xem hệ" / Enter → 1 hệ phóng lớn: mặt trời = agent chính (tool đang chạy), hành tinh = subagent trên quỹ đạo, vệt cung trên quỹ đạo = tool call gần nhất (dài theo thời gian), cột trái = danh sách hệ (Cần anh trên cùng) để nhảy hệ, cột phải = dòng tool theo giờ. Esc / nút Quay lại → tổng quan.
- Bất biến cũ giữ: không quay liên tục, chỉ nhích khi có tool call; reduced motion đứng yên; vị trí cố định theo hash; > ~8 session rảnh / xong thu thành chấm mờ ở vòng ngoài; status = icon + chữ + màu; không blur / gradient; i18n en + vi; Monitor không gửi gì vào agent; Office để yên; panel lệnh + Alt+1/Alt+2 giữ.
- Tab Bảng: giữ như W2 (A1 / A2 chưa chọn).
