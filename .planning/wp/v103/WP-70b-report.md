# WP-70b report

## Nguyên nhân gốc

- **#2 menu "…" không tắt khi click ngoài**: `components/next/ui/GlassMenu.vue` không có xử lý outside click / mất focus (chỉ có Esc khi focus nằm trong menu). Menu "…" (rail foot, `NextShell.vue` `moreItems`) là chỗ duy nhất dùng GlassMenu.
- **#4 vào chế độ gọn không về được**: `NextShell.vue` `railHidden = ui.compact` ẩn cả rail (nơi có menu "…" chứa mục "Đầy đủ"), `AppHeader` classic không có trong next, `CompactView` không có nút nào. Chỉ còn Ctrl+K (không ai biết). Compact trong cửa sổ chính chỉ là `ui.compact` (URL `?mode=compact`), không đổi kích thước cửa sổ, nên không phải lỗi IPC / kích thước.

## Đã làm

- **#2** `composables/useDismiss.ts` (mới): 1 cơ chế dùng chung (pointerdown capture ngoài vùng, `window` blur, `visibilitychange` hidden, Esc tuỳ chọn, `ignore` selector). Dùng cho `GlassMenu` (Esc + blur + outside), và thay các listener tự viết của `LayoutMenu`, `ColorPicker`, `RailCollapsed`, `InboxPanel`, `HeaderStatusPopover` (nay cũng tắt khi mất focus cửa sổ). Nút mở menu bấm lại thì đóng, không mở lại (guard `moreClosedAt` có sẵn).
- **#4** `CompactView.vue`: nút "Đầy đủ" `[data-compact-exit]` + phím Esc (window capture, bỏ qua khi đang gõ / có dialog). `stores/ui.ts` `leaveCompact()`: compact trong cửa sổ chính thì `compact=false`; cửa sổ mở bằng `?mode=compact` (tray) thì `window.open(origin + '/')`. Electron `main.ts` `openAppWindow` và Tauri `shell/src/window.rs` `on_new_window`: URL gốc không có `mode=compact` thì đưa cửa sổ chính lên (`showMain` / `show_main`), `mode=compact` thì mở / focus cửa sổ gọn như cũ. Palette, menu "…", `AppHeader` dùng `leaveCompact`. Tray đã có "Open" từ trước.
- **#12 Bỏ Mission Control**: xoá `MissionControl.vue`, `workspace/mission/*`, `lib/rail/mission.ts`, `missionMenu.ts` (+ test), `i18n/workspace-v50-mission.ts`, `workspace-v52-mission.ts`, css `.mist-mission-*`, phím Ctrl+Shift+O (`workspaceKeys.ts`, `shortcuts.ts`), mục "Mở Mission Control" trong menu project (`railMenu.ts`, `LiveRail.vue`, `WorkspaceSidebar.vue`, `workspaceBridge.ts`), i18n `rail.mission.*`. `WorkspaceView.vue`: bỏ state / dialog, nút `[data-strip-project]` thành nhãn không bấm (vẫn hiện tên project + số cần xử lý). `workspace/motionTokens.ts` (mới) giữ `dur` / `ease` cho `AttachmentLook`. Trả lời menu v100 vẫn qua thanh dưới pane + chip "Chờ trả lời" (Ctrl+Alt+Q) + toast. `CLAUDE.md`, `docs/claude-code-integration.md`, `.code_index/*` đã cập nhật.
- **UI next mặc định**: `lib/uiGeneration.ts` `startUi` + `UI_MIGRATION_KEY='ccm.uiMigration'` / `'4.1-next'`; `stores/uiGen.ts` `initialGeneration` (cài mới và máy đang classic không có khoá → next, ghi khoá; sau đó classic do chủ nhân chọn được giữ). `index.html` `pickUi` khớp, không nháy UI cũ. `parseUiGeneration` mặc định next. Appearance vẫn chọn lại được classic, ghi chú "sẽ bỏ ở bản sau" (`mist-v102.ts`). `NextInvite` tự tắt vì khoá `ccm.uiGeneration` đã là next.
- E2E: `e2e/cdp.cjs` + `ui-gen.cjs` ép classic khi `launch({ ui: 'classic' })` và luôn ghi `ccm.uiMigration`; gỡ phần Mission Control khỏi `rail.cjs`, `v50-groups.cjs`, `v52-polish.cjs`; `v100-menu-queue.cjs` trả lời qua thanh dưới pane thay vì card.

## Test (lệnh thật)

- `npx vitest run` (apps/web, toàn bộ): 311 file, **2687 pass, 1 fail** (`composables/useRemotePhone.test.ts` "rename and move change state only", lần chạy riêng lại fail 1-2 ca, ca còn lại là timeout hook khi máy tải; file và `stores/workspace.ts` không do WP này đổi, nghi lỗi sẵn có sau merge master, cần main xác nhận).
- Mới: `components/next/ui/GlassMenu.test.ts` 3, `stores/ui.test.ts` +2 (leaveCompact), `stores/uiGen.test.ts` +2 (migration), `next/palettes.test.ts` +2 (mirror `pickUi`), `lib/workspaceKeys.test.ts` (Ctrl+Shift+O hết map).
- `npm run typecheck -w @ccm/web`: 1 lỗi duy nhất `lib/explorer/treeWidth.test.ts(15,63)` của WP-71, phần WP-70b sạch. `typecheck -w @ccm/desktop` sạch. `cargo check -j 4` (apps/shell) OK.
- E2E (cổng riêng `CCM_E2E_PORT=4472 CCM_E2E_DBG=9472`, `vite build` rồi chạy): `e2e/v103-polish.cjs` (mới) **16 PASS 0 FAIL** (menu click ngoài / bấm nút mở / Esc / blur / pointerdown rail, compact có nút về + Esc + Ctrl+K, bỏ Mission Control, migration + classic giữ sau reload). `e2e/v103-rail.cjs` **39 PASS** (WP-70a vẫn nguyên).
- Ảnh: `docs/qa/v103/WP-70b/` (`01-more-open`, `02-compact`, `03-classic-after-choice`, `04-next`).

## QA tay Windows

1. Next: bấm "…" ở đáy rail, click vùng canvas / Esc / alt-tab qua cửa sổ khác: menu tắt. Bấm lại "…" khi đang mở: đóng.
2. "…" > Chế độ gọn: thấy nút "Đầy đủ" góc phải; bấm, Esc, hoặc Ctrl+K "Full view" đều về giao diện đầy đủ. Tray > Cửa sổ gọn > "Đầy đủ": cửa sổ chính hiện lên, cửa sổ gọn giữ nguyên.
3. Ctrl+Shift+O không làm gì; Ctrl+K không còn Mission Control; chuột phải project không có mục Mission Control; nhãn project trên strip không bấm được.
4. Máy đang classic: mở lần đầu bản mới vào next; Appearance chọn classic, restart vẫn classic.
5. Menu chọn Claude / Codex: chip "Chờ trả lời" + thanh dưới pane + toast còn trả lời được.

## Test macOS (chép vào `.planning/QA-mac-checklist.md`)

- Tray macOS > "Compact window" rồi "Full view" trong cửa sổ gọn: cửa sổ chính (Electron `showMain`, Tauri `show_main`) hiện lên, không sinh cửa sổ thứ hai.
- Phím Cmd+K trong compact mở palette; Esc thoát compact (không đụng Cmd+W).
- Menu "…" tắt khi Cmd+Tab sang app khác (sự kiện blur của WKWebView) và khi bấm sang cửa sổ khác của Orrery.
- Máy macOS đang classic: migration sang next 1 lần, index.html không nháy UI cũ.
- Cửa sổ có title bar overlay: chế độ gọn không còn rail, nút "Đầy đủ" không bị che bởi vùng kéo cửa sổ.

## Việc dở

- E2E cũ `v51-compact.cjs` và `v25.cjs` đọc output terminal qua tile Mission Control (`[data-mission-output]`): sẽ fail cho tới khi có cách đọc buffer khác (không thêm hook lộ nội dung phiên). Chưa sửa.
- Chưa chạy E2E trên Tauri shell (cần exe build sẵn); chỉ `cargo check`.
- Các popover classic khác (`AppSelect`, `PlanLimits`, `WorktreeMenu`, `SplitMenu`...) vẫn dùng listener riêng (đã tắt khi click ngoài, chưa tắt khi mất focus cửa sổ); `useDismiss` sẵn để gom dần, không đụng vì thuộc phạm vi classic / WP khác.
- `i18n` còn key `mist.rail.needs.*` (WP-70a để lại) và `workspace.rail.needs` còn dùng.
- Không có tiến trình nền của WP này còn chạy.
