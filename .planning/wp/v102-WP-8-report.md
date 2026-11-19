WP-8: ĐẠT (code, chưa chụp ảnh)
File: components/workspace/{FilePane,ChangesPane,RunPane,BrowserPane,BrowserToolbar}.vue, workspace/log/LogPane.vue, next/tokens.css (khối viewer cuối file)
SPEC ID đã làm: T3 viewer. Ở isNext, header toolbar gốc của File / Changes / Log / Run / Browser (BrowserToolbar) đổi thành `PaneHeader` 34 (component :is, giống TermPane), class `ccm-next-head ccm-next-viewer-head`. Classic giữ nguyên class cũ. Section viewer thêm `ccm-next-viewer` (bỏ shadow, card nằm ở wrapper `data-next-pane-card`). Toolbar con (`.ccm-material` trong viewer) nền kính nhẹ; address bar Browser 28px.
Giữ nguyên: ViewPaneControls vẫn nằm trong slot `end` (WorkspaceView không sửa), `<webview>` không remount (không đổi cấu trúc BrowserPane), dirty marker, save, find bar, XML / Flutter preview vẫn trong FilePane.
Chưa làm: LayoutPreview / FlutterPreviewPanel toolbar chỉ nhận CSS kính chung (không đổi DOM); GlassMenu cho menu "..." (PaneMore giữ nguyên); Chip cho trạng thái Run (thanh state Run giữ nguyên); không thêm chuỗi i18n nên không đụng `mist-v102.ts`.
Typecheck: pass (npm run typecheck -w @ccm/web). Test: 30 pass / 0 fail (vitest run --root apps/web src/components/workspace src/components/next).
Rủi ro: PaneHeader gom mọi nút vào slot mặc định (không tách left/right); Run header ở next mất wrap `@max-[480px]`, cần xem ảnh pane hẹp. Chưa E2E.
