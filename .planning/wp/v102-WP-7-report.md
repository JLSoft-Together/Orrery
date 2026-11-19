WP-7: ĐẠT (cần xem bằng mắt)
File: components/workspace/mission/MissionCard.vue, components/next/explorer/ExplorerDrawer.vue, next/tokens.css (khối cuối "mist-mission-*").
Mission next: thẻ `mist-card mist-card-flat` bo 18, header 34 + Chip trạng thái (icon + chữ + màu) + model (usePaneInfo.modelOf) + ContextGaugeMini; body/tile bo 12; ask box dùng TermMenuBar (kiểu click menu next có sẵn ở tokens.css: nền vàng nhạt, nút số, nút chính accent). Hover chỉ transform, tắt khi data-motion off/reduced và prefers-reduced-motion. Classic giữ nguyên class cũ. Không đụng MissionControl.vue, phím, kéo, snapshot, dust.
Explorer: watcher searchRequest trong ExplorerDrawer mở drawer rồi gọi panel.showSearch (chuyển view Search + focus ô tìm), bỏ focusFirst chen trước.
Không thêm key i18n (dùng lại chuỗi có sẵn).
Typecheck: pass. Test: 159 pass / 0 fail (src/components/workspace/mission src/lib/rail).
Rủi ro: chưa xem ảnh thật 8 session; nếu Explorer không phải repo thì showSearch bỏ qua (hành vi cũ).
