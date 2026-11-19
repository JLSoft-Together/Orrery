WP-2 (v102 shell Sương kính): ĐẠT

File:
- Mới: `apps/web/src/lib/next/railMode.ts` (+ `railMode.test.ts`), `lib/next/shellContext.ts`, `components/next/rail/LiveRail.vue` (placeholder).
- Viết lại: `components/next/NextShell.vue` (shell thật; style shell nằm trong khối `<style>` cuối file, dưới `:root[data-ui='next']`).
- Sửa nhỏ: `App.vue` (next: không AppHeader / footer; views bọc trong `<component :is="NextShell | div.contents">`; `--ccm-header-h` = 24px ở next, 0px khi compact), `i18n/mist-v102.ts` (khối `mist.shell`), `next/tokens.css` (`data-paused` dừng animation).

SPEC ID đã làm: S1 (rail head logo + segmented; foot: hạn mức 5h/tuần, Inbox badge, Usage, History, Settings, "..."), S2 (lớp `--ccm-bg-image` nằm trên `html` từ WP1, Ngân hà giữ sao), S3 (>=1440: 264; 1200-1439: 248; 1024-1199: 56; <1024 overlay 248 + nút mở + scrim + Esc; hysteresis 24px; kéo rộng 224-360, khoá `ccm.next.railWidth`, dblclick reset), M1 (từ WP1), M6 (root `data-paused` theo `document.visibilityState`, CSS `animation-play-state: paused`).
Chưa làm: M3, R1-R8 (WP3), G1 (WP10).

Lối vào đã giữ (đối chiếu AppHeader): Workspace / Monitor (segmented, icon ở rail 56), Usage, History, Inbox + InboxPanel, Settings (SettingsMenu mount ẩn trong `<div hidden>` để giữ dialog + Ctrl+,), Ctrl+K (ô tìm trong rail), HeaderStatusPopover (limits, keep-awake, disk, resources, connection; ép icon-only bằng CSS), TelegramToggle + chip nhận, nút update, menu "..." (GlassMenu): Extensions, Devices, Disk, Resources, Compact, Dim inactive (tab split), bật / tắt thông báo (thay chuông cũ), Shortcuts, Tour (Workspace), "Về giao diện Orrery cũ". Office nằm trong Monitor như cũ.

Typecheck: pass (`npm run typecheck -w @ccm/web`).
Test: 30 pass / 0 fail (`npx vitest run --root apps/web src/lib/next src/components/next src/i18n src/stores/uiGen.test.ts`; railMode 11 test mới).

API cho gói sau:
- WP3 (rail): thay NỘI DUNG `components/next/rail/LiveRail.vue`, giữ props `{ collapsed?: boolean }` (true = rail 56). NextShell gắn nó giữa rail head và rail foot (`flex-1 min-h-0`). Placeholder liệt kê term pane của workspace, bấm = `workspace.activate` + `setView('workspace')` + `shell.closeRail()`. Đóng overlay sau khi chọn: `inject(NEXT_SHELL_KEY)`.
- WP4 (canvas): default slot của NextShell = canvas (`.ccm-next-canvas`, flex column, `overflow-y: auto`). Context `NEXT_SHELL_KEY` (`lib/next/shellContext.ts`): `railMode` (full | icon | overlay), `railWidth`, `railOpen`, `openRail`, `closeRail`, `toggleRail`. Chiều cao view: `100dvh - var(--ccm-header-h)` = 100dvh - 24px (padding shell 12px).
- WP5 (Explorer drawer): named slot `drawer` của NextShell, render sau default slot trong canvas (`position: relative`). App.vue chưa đổ gì vào slot này, WP5 tự thêm vào App.vue hoặc mount trong WorkspaceView.

Rủi ro / việc cho gói sau:
- Sidebar cũ trong `WorkspaceView` ẩn bằng CSS `[data-tour='ws-sidebar'] { display: none }` ở next (khối style của NextShell). Nút toggle sidebar trong WorkspaceView vẫn còn, WP4 nên bỏ khi `isNext`.
- Đổi classic <-> next làm cây view mount lại (wrapper khác). Classic dùng `div.contents` nên layout không đổi, chỉ thêm 1 node bọc.
- `GlassTooltip` chỉ có side `top | bottom` nên tooltip rail 56 hiện phía dưới nút; cần `right` thì thêm vào file WP1.
- Canvas có scroll riêng thay vì scroll cả trang: view dùng `window.scroll*` / sticky theo viewport cần kiểm khi restyle (Usage, History, Disk...).
- Hạn mức rail đọc `extras.limits` (Claude), rơi về `codexLimits` khi Claude chưa có; thiếu số liệu thì ghi "Chưa có hạn mức gói", không bịa số.
- Selector E2E cũ (`[data-status-button]`, `[data-inbox-button]`, `[data-more-views]`, `[data-tour=*]`) vẫn có ở next; chưa chạy E2E.

## Sửa remount
- App.vue: bỏ `<component :is="isNext ? NextShell : 'div'">`. Giờ `<NextShell v-if="isNext"/>` / `<div id="ccm-classic-host" class="contents"/>` và một `<Teleport defer>` cố định chứa grid views + ExplorerDrawer (v-if isNext); `to` = `[data-next-canvas]` hoặc `#ccm-classic-host`. Vue chỉ di chuyển DOM, không unmount WorkspaceView / TermPane / webview.
- Vì Teleport resolve target đồng bộ lúc update (target mới chưa mount), `to` đi theo `viewsInNext` cập nhật bằng `watch(..., { flush: 'post' })`: NextShell / host mount trước, rồi Teleport mới chuyển.
- Classic DOM: div.contents bọc grid như cũ, không thêm wrapper nào khác.
- Test: `components/next/shellSwap.test.ts` (child mount 1 lần qua classic -> next -> classic, cùng node DOM). Lưu ý: test mô phỏng cơ chế, chưa chạy E2E trên app thật.
