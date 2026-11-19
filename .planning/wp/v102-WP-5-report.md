WP-5: ĐẠT (code + unit); chưa kiểm bằng mắt / E2E
File: apps/web/src/components/next/explorer/{ExplorerDrawer.vue,useExplorerDrawer.ts}, apps/web/src/lib/next/explorerPin.ts (+ .test.ts), App.vue (import + 1 dòng mount), i18n/mist-v102.ts (khối mist.explorer en + vi)
SPEC: E1 (drawer 288 trượt từ phải bằng transform, Ctrl+Shift+E toggle dùng lại action `explorer` của workspaceKeys, scrim 10% = --ccm-drawer-scrim, Esc đóng + trả focus, trapTab), E2 (ghim = explorerPinned; hàm thuần pinVerdict / effectivePinned; viewport >= 1440 và canvas >= 720; không đủ thì về drawer + toast), E3 (dùng nguyên ExplorerPanel, 5 view, badge, trash, Chuyển sang ổ khác, kéo file)
Typecheck: pass. Test: explorerPin 4 test pass.
Ghi chú kiểm: i18n/messages.test.ts đang FAIL do placeholder Ctrl+K của gói khác có `@` trần (không phải khoá explorer).
Quyết định:
- Mount trong default slot của NextShell (sau grid views), không dùng slot `drawer`, vì wrapper App.vue ở classic là `div`, slot có tên trên div không an toàn. Root drawer position:absolute trong canvas.
- Ghim = card absolute bên phải canvas + canvas `padding-right: 296px` khi root có `data-explorer-pinned=on` (CSS trong ExplorerDrawer.vue). Cờ lưu `explorerPinned` giữ nguyên khi hết chỗ, tự ghim lại khi đủ chỗ; toast 1 lần mỗi lần rớt (id explorer-pin-fallback).
- Handler open / trash / rename viết lại ngắn trong drawer bằng store + lib dùng chung (trashedPaneKeys, workspace.requestClose record:false, renamedPanePatches), không đụng WorkspaceView. Trash không qua dust animation / auto save như closePanes.
- Phím: listener capture ở window, preventDefault nên WorkspaceView.onGlobalKey bỏ qua (nếu WorkspaceView đăng ký trước thì nó chỉ mở cột cũ đang bị ẩn).
- workspace.searchRequest: drawer tự mở + showSearch.
Việc cho main / WP4a:
- Cột cũ `aside[data-explorer-panel]` ẩn bằng CSS `:root[data-ui='next'] [data-explorer-panel]{display:none}`; vẫn mount ExplorerPanel thứ hai (ẩn). Nên `v-if="!isNext"` ở WorkspaceView để khỏi chạy đôi watcher git.
- Nút toggle Explorer ở toolbar WorkspaceView (data toggleExplorer) chỉ đổi cột cũ ẩn; ở next nên nối vào useExplorerDrawer().open.
- Chưa chụp ảnh drawer mở / ghim / viewport hẹp, chưa chạy E2E.
