# PLAN v96 — Ngân hà mặc định + phần v94 còn lại (chủ nhân 11/11: "làm cả 4")

1. Quỹ đạo: 1 project = 1 hệ (trước: 1 session = 1 hệ). 2–3 session = cụm mặt trời, > 3 → 3 mặt trời gấp nhất + "+N".
2. Theme mặc định: tối → `galaxy`, sáng → `orrery-light`. `galaxy.pair = orrery-light`. Migration `4.0.2-galaxy` chạy 1 lần
   (theme sáng → `orrery-light`, còn lại → `galaxy`), kể cả máy đã qua migration `4.0.2`.
3. v94 C: Giám sát › Bảng có "Nhóm theo" Trạng thái / Project / Agent (`ccm.monitor.groupBy`).
   v94 E: `WorkspaceFooter` hiện cả ở Giám sát: đếm theo mọi session (`sessionCounts`), cổng dev của mọi session,
   bấm chạy / chờ → focus thẻ trên Bảng; nhánh / context / tập trung chỉ ở Workspace.
   v94 F/G: Settings › Giao diện thẻ theme lớn có preview + chip màu agent; theme chỉ đổi màu thành hàng chip.
4. Commit: `5b8af211` (đợt v92–v95), đợt v96 commit sau khi QA.
