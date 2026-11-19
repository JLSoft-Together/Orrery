# WP-70a report

## Đã làm (#1 #3 #6 #7 #10)

- **#1 Nút Explorer**: `components/workspace/WorkspaceView.vue` `[data-explorer-toggle]`, chế độ next đổi icon `PanelRight` sang `FolderTree`, thêm chữ "Explorer" nhìn thấy, dạng `ccm-btn` (nút có nền), tooltip + aria-label "Explorer (Ctrl+Shift+E)" (`mist.shell.explorerTip`), trạng thái bật tô accent (`NextShell.vue` style `[data-explorer-toggle][aria-pressed='true']`). Lý do: trước đó icon `PanelRight` + nút plain giống hệt các nút icon của thanh pane; chữ + nền khác nút Layout bên cạnh. Classic giữ nguyên.
- **#3 Hộp việc cần**: là `rail/RailNeeds.vue` ("Cần bạn xử lý", `data-rail-needs`). Đã xoá component và phần nối trong `rail/LiveRail.vue` (needs, needsFrozen, snoozeRow, openNext, toasts). Thông tin chờ trả lời vẫn ở glyph + `detail` trong `RailRowItem`, chip "Chờ trả lời" ở thanh pane, badge rail icon (`RailCollapsed`). Giữ nguyên `lib/next/railLive.ts` / `railSnooze.ts` (còn dùng cho `collapsedProjects`). i18n `mist.rail.needs.*` còn lại chưa dùng.
- **#7 Indent**: `LiveRail.vue` danh sách session của project có `data-rail-children` (`ml-[17px]`, `border-l`, `pl-1`), chế độ nhóm theo status không đổi, rail icon không đổi.
- **#10 Collapse**: `lib/next/railCollapse.ts` (+test), `rail/RailCollapseButton.vue`, `NextShell.vue`: `baseMode` (theo viewport) + `collapsed` -> `mode` computed; thu về dải icon 56px, nhớ qua `localStorage` `ccm.next.railCollapsed`, motion `grid-template-columns` (tắt khi kéo size hoặc motion off), phím Ctrl+Shift+B / Cmd+Shift+B (capture, chạy cả khi focus terminal). Chỉ khi viewport ở chế độ full; icon-by-width và overlay không có nút. Kéo size cũ giữ nguyên (chỉ ở full). Hạn chế: chưa có kiểu "ẩn hẳn".
- **#6 Hạn mức**: `lib/next/quotaBar.ts` (+test), `next/NextQuotaBar.vue` (+test) là footer 1 dòng span cả cửa sổ (`.ccm-next-footer`, grid row 2). Mỗi agent có số thật 1 cụm: "tên, 5h/Tuần, thanh nhỏ, %, reset lúc HH:mm hoặc thứ + giờ". Claude Code và Codex cùng hiện (trước chỉ 1). Không có số thì không hiện cụm; chưa có agent nào thì hiện ghi chú. Báo cáo cũ hơn 30 phút thì mờ + tooltip. Bỏ khối quota cũ trong rail; nút trạng thái (`HeaderStatusPopover`) chuyển vào hàng nút dưới rail. i18n en + vi trong `i18n/mist-v102.ts` (`mist.shell.*`).
- `.code_index/_meta.md` đã ghi.

## Test

- `npm test -w @ccm/web -- src/lib/next src/components/next src/i18n`: 18 file, 105 test pass (mới: railCollapse 3, quotaBar 6, NextQuotaBar 3, RailCollapseButton 1).
- `npm run typecheck -w @ccm/web`: 0 lỗi TS (lần đầu bị lỗi cú pháp `stores/diskMap.test.ts` của WP-73 đang sửa dở, lần sau sạch).
- E2E `apps/desktop/e2e/v103-rail.cjs` (mới, cổng riêng `CCM_E2E_PORT=4471 CCM_E2E_DBG=9471`, `launch({ ui: 'next' })`): 21 PASS, 0 FAIL (nút Explorer, không còn hộp việc, indent, footer ở đáy, 2 cụm hạn mức, collapse / expand / nhớ qua reload / phím tắt, icon mode không nút, footer ở 1100px). Dùng `vite build` vào `apps/web/dist` (không qua `vue-tsc`) trước khi chạy.
- Chưa chạy lại `v102-next.cjs` và `rail.cjs` (cũ): có thể tham chiếu `data-rail-needs` / `data-next-quota` trong rail, grep không thấy trong `apps/desktop/e2e`.

## Ảnh

`docs/qa/v103/WP-70a/`: `01-light-full.png`, `02-light-collapsed.png`, `03-dark-full.png`, `04-dark-collapsed.png`.

## QA tay (Windows + macOS)

1. Next, workspace có pane: nút "Explorer" có chữ + icon cây thư mục, hover thấy chord; bật thì tô accent.
2. Rail không còn hộp "Cần bạn xử lý"; session chờ trả lời vẫn thấy ở dòng session và chip pane.
3. Session thụt vào dưới project, đường kẻ dọc nhẹ.
4. Nhấn nút thu gọn đầu rail và Ctrl+Shift+B (macOS Cmd+Shift+B), kiểm lúc đang focus terminal; tắt / mở app vẫn nhớ; kéo size vẫn chạy.
5. Footer đáy: số Claude Code / Codex khớp header status; macOS kiểm giờ 12h/24h theo locale.
6. Cửa sổ hẹp 1100 (icon) và <1024 (overlay): footer vẫn full width, không nút thu gọn.

## Việc dở

- Không có kiểu thu gọn "ẩn hẳn" (chỉ dải icon 56px).
- Không có Mac để kiểm Cmd+Shift+B và trường hợp title bar overlay.
- WP-70b đụng NextShell: `baseMode` / `collapsed` / `NextQuotaBar` đã tách riêng để dễ merge.

## Sửa sau review chính (cuộn trang + đáy pane bị cắt)

- Nguyên nhân: footer thêm một hàng grid nhưng `--ccm-header-h` (next) vẫn 24px, nên `WorkspaceView` / `MonitorView` (`100dvh - --ccm-header-h`) cao hơn vùng canvas, trang cuộn và đáy pane bị cắt.
- Sửa: `NextShell.vue` hàng footer cố định 24px, `row-gap` 8px, `padding` 12px 12px 6px, rail overlay `bottom: 38px`; `NextQuotaBar.vue` `h-6`; `App.vue` `--ccm-header-h` next = `50px` (có title bar overlay / mac: `titlebar + 38px`), tức padding trên + dưới + gap + footer. Canvas, rail, pane đều kết thúc cách đáy footer 8px, giữ bo góc như v102.
- E2E `v103-rail.cjs` thêm `fit()` cho full, collapsed, dark full, dark collapsed, icon (1100), overlay (900): `scrollingElement.scrollHeight <= innerHeight` và `scrollWidth <= innerWidth`, đáy pane cuối và đáy rail cách đỉnh footer >= 6px. Kết quả: 39 PASS, 0 FAIL (đo thực: pane bottom 862, footer top 870).
- Ảnh `01..04` chụp lại sau sửa. Title bar overlay / mac chưa chạy thật trên máy này, chỉ cùng công thức `titlebar + 38px` (cần QA tay trên macOS).
