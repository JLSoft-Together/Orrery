WP-9: ĐẠT (restyle bằng class + CSS; chưa chụp ảnh từng view)
File: apps/web/src/next/tokens.css (khối "monitor" cuối file), components/monitor/{BoardCard,ModeBar,SessionBoard}.vue, components/usage/{UsageHero,UsageModels,UsageProjects,UsageView}.vue, components/history/HistoryView.vue.
Cách làm: class `mist-card` / `mist-card-blur` / `mist-card-flat` / `mist-lane` / `mist-section` / `mist-seg` không có CSS ở classic. Next: kính (--ccm-glass), bo 18 (--radius-card), viền glass-edge. Blur chỉ ở card lớn Usage; BoardCard và lane không blur (list dài). Lane = fill nhẹ. History: các khối border-t thành thẻ kính. ModeBar tab = segment kính. Nhãn cột 11px hoa nhẹ. Legend Quỹ đạo = glass-pop.
BoardCard ở next: dòng trạng thái dùng `Chip` (icon + chữ + màu, giữ `data-board-line`), chuỗi "Cần duyệt, 3m" dùng dấu phẩy thay " · ". Context vẫn 1 meter (bar sẵn có).
Quỹ đạo: animation sẵn chỉ transform (star-orbit rotate); thêm tắt star-spin / star-needs khi data-motion=reduced (off đã tắt chung). Sao nền không đụng.
Kiểm: không có view nào dùng window.scroll / sticky theo viewport (chỉ sticky thead trong bảng và nút ProjectTable, scroll nội bộ).
Chưa làm: InboxPanel dùng `.ccm-menu` nên đã glass theo khối WP6 (không sửa riêng); AgentMap / nodes / PlanLimits / SessionCard giữ nguyên (token bo 18 tự áp); không có khối i18n monitor vì không thêm chuỗi mới. 8 component chết không vẽ.
Typecheck: pass. Test: 36 pass / 0 fail (npm test -w @ccm/web -- src/components/monitor src/components/usage src/components/history src/stores/uiGen.test.ts).
Rủi ro: cần chụp ảnh 4 view ở các palette sáng/tối ở WP11; Chip trong BoardCard có thể cần kiểm truncate với dòng file dài.
