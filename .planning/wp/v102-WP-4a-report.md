WP-4a: ĐẠT (cần xem bằng mắt)

File:
- Sửa: `components/workspace/WorkspaceView.vue`, `TermPane.vue`, `MenuQueueChip.vue`, `lib/dust.ts` (+ `dust.test.ts`), `next/tokens.css` (khối cuối file: canvas).
- Không thêm key i18n (dùng lại chuỗi sẵn có); không tạo component mới.

SPEC ID đã làm: T1 (tab strip 36 glass, pill 28 tròn, monogram tone project + chấm trạng thái, chip ưu tiên: Chờ / Lỗi / dirty; phần phụ mail, telegram, đếm pane, zoom hiện ở popover khi hover/focus, `+N` pill), T2 (chip Chờ trả lời + Cần bạn kiểu pill, kbd Ctrl Alt Q), T3 (pane card bo 18 cho term + viewer, header term = `PaneHeader` 34, active ring 2px accent ở card + tint 6%, group frame kính viền mảnh), T4 (thanh click menu nền vàng nhạt, nút có số phím, nút chính đặc accent), T5 (well đặc; `clearTerm` -> `data-term-clear` dùng `--ccm-well-clear` + xterm allowTransparency + nền xterm trong suốt; font JetBrains Mono khi codeFont = system; theme xterm theo lại khi đổi palette), M4 (dust next 340ms + stagger 20ms, max 640ms; classic giữ 1000ms), M7 (pill chỉ transition transform, popover hover opacity + transform), bỏ nút toggle sidebar cũ ở next. Theo yêu cầu main: aside Explorer cũ `!isNext`, nút toggle Explorer ở next bật `useExplorerDrawer().open`; `showSearch` / `showExplorer` ở next mở drawer.
Chưa làm: viewer header dùng `PaneHeader` (WP8); D1-D11 (WP4b). Hạn chế: `explorerPanel.showSearch` ở next là null (panel nằm trong drawer của WP5), tìm kiếm từ `workspace.searchRequest` chỉ mở drawer, WP5 cần nối focus tab Search.

Typecheck: pass. Test: 138 pass / 0 fail (`vitest run --root apps/web src/lib/dust.test.ts src/components/workspace src/components/next src/stores/workspace.test.ts`).

Điểm móc:
- WP4b: drop preview `[data-drop-preview]` + overlay `dragSource` + divider (`group/div`) + ghost `[data-drag-ghost]` còn nguyên trong `WorkspaceView.vue` (cuối template canvas). Style canvas nằm khối cuối `next/tokens.css`. Tab strip: `data-next-tabs`, `.ccm-next-pill`, caret `[data-tab-caret]`.
- WP8: viewer wrapper có `data-next-pane-card` (card visual, KHÔNG overflow hidden để khỏi cắt popover). Thay `<ViewPaneControls>` + toolbar `h-9 ccm-material` bằng `PaneHeader` trong từng viewer; TermPane dùng `<component :is="PaneHeader">` + class `ccm-next-head` làm mẫu. Ở next term header không còn teleport lên tab row (`inBar` = false).
- Rủi ro: popover hover tab dùng `:has()`; `inBar` tắt ở next nên `paneBar` slot rỗng; chưa chạy E2E (selector `data-tab-status`, `data-tab-dirty` còn, dirty ở next thêm `data-tab-dirty-next`).
