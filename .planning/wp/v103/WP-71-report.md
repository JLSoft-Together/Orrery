# WP-71 report

## Đã làm
- Ghim: bỏ điều kiện viewport 1440. Chỉ còn ràng buộc canvas >= 560px (`PIN_MIN_CANVAS`): width Explorer tự co về tối thiểu 220 khi chật (`fitPinnedWidth`, width đã lưu giữ nguyên), vẫn không đủ thì tự về drawer nổi, nút ghim có `data-pin-blocked` + tooltip lý do và bấm vào hiện toast. Cột ghim đặc bên phải (nền `--ccm-material-pop`, không blur), pane co lại. Bỏ ghim thì Explorer ở lại dạng drawer đang mở.
- Kéo width: thanh `[data-explorer-grip]` (role separator) ở mép trái card, kéo bằng chuột, double click về 288, phím ArrowLeft/Right (±16, Shift ±48), Home/End. Min 220, max 640; khi ghim còn bị chặn để canvas >= 560px; drawer nổi chặn theo cạnh canvas. Lưu `ccm.next.explorerWidth`, biến `--ccm-explorer-w` đẩy `padding-right` canvas.
- Drawer nổi và cột ghim dùng CHUNG một width: kéo ở đâu cũng nhớ, đổi chế độ không nhảy size.
- Cuộn ngang: `ExplorerTree.vue` đo độ rộng nội dung (`lib/explorer/treeWidth.ts`, đo chữ bằng canvas, cache) và đặt `--ccm-tree-min-w` lên khối trong; chỉ dưới `:root[data-ui='next']` thì `[data-explorer-tree]` `overflow-x: auto`, dòng chọn / hover phủ hết chiều rộng nội dung, tên không bị "…", tooltip giữ. Dòng header của cây (tên project + nút tạo file / thư mục / ẩn / refresh / thu gọn) cố định: `position: sticky; left: 0` với width = clientWidth (`--ccm-tree-cw`), không trôi khi cuộn ngang. Classic không đổi.
- Motion: canvas `padding-right` transition khi ghim / bỏ ghim / đổi width (tắt khi đang kéo và khi motion off), grip sáng accent khi hover / focus / kéo, drawer vẫn trượt vào.
- i18n en + vi: `mist.explorer.resize`, `mist.explorer.resizeTip`.

## File
- Mới: `apps/web/src/lib/explorer/treeWidth.ts` (+test), `apps/desktop/e2e/v103-explorer.cjs`.
- Sửa: `apps/web/src/components/next/explorer/ExplorerDrawer.vue`, `apps/web/src/components/workspace/ExplorerTree.vue`, `apps/web/src/lib/next/explorerPin.ts` (+test), `apps/web/src/lib/uiGeneration.ts` (`NEXT_KEYS.explorerWidth`), `apps/web/src/stores/uiGen.ts` (`explorerWidth`, `setExplorerWidth`), `apps/web/src/i18n/mist-v102.ts`, `.code_index/_meta.md`.
- NextShell.vue: KHÔNG sửa (Explorer gắn qua `ExplorerDrawer` trong App.vue).

## Test
- `npm test -w @ccm/web -- src/lib/next src/components/next src/lib/explorer src/stores src/i18n`: 61 file, 586 test pass (explorerPin width + fit, treeWidth).
- `npm run typecheck -w @ccm/web`: 0 lỗi.
- E2E `v103-explorer.cjs` (cổng 4472 / 9472, dữ liệu riêng): 38 PASS, 0 FAIL, gồm: ghim ở cửa sổ 1366 và 1280 (canvas >= 560, trang không cuộn, pane trái cột), cột co khi chật mà width lưu vẫn 640, cửa sổ 700 tự về drawer + tooltip chặn, header cây đứng yên khi cuộn ngang, kéo, min / max, double click, phím, reload nhớ.
- E2E `v103-rail.cjs` (WP-70a): 39 PASS, 0 FAIL.
- `npm run qa -- --no-e2e` đã chạy: 3/6 bước pass, fail do timeout tải nặng ở `desk-preview-git.twin.test.ts` (WP-72) và `useProjectRoots.test.ts`, không thuộc WP-71.

## Ảnh
`docs/qa/v103/WP-71/`: `01-light-drawer-scrolled.png`, `02-light-pinned.png`, `03-dark-pinned.png`, `04-dark-drawer.png`, `05-light-pinned-1366.png`, `05-light-pinned-1280.png`.

## QA tay Windows
1. Next, workspace có pane, mở Explorer, kéo mép trái, nhả, mở lại app vẫn đúng width; double click mép về 288.
2. Bấm ghim: pane co lại, cột đặc, không có scrim; kéo mép khi ghim, pane đổi theo mượt; kéo hết cỡ thì canvas không nhỏ hơn 560px. Thử cả cửa sổ 1366 và 1280.
3. Mở cây sâu: có thanh cuộn ngang trong panel, trang không cuộn ngang, tên file đầy đủ khi cuộn hết.
4. Bỏ ghim: về drawer nổi (glass), Ctrl+Shift+E bật / tắt.

## Test macOS
- Ctrl+Shift+E là Cmd+Shift+E, vẫn bật / tắt khi đang ghim (chuyển focus vào cây).
- Kéo mép bằng trackpad (pointer capture), thanh cuộn ngang của cây (overlay scrollbar macOS ẩn cho tới khi cuộn, cuộn ngang hai ngón tay).
- Đo chữ canvas bằng font Manrope trên Mac: độ rộng cây đủ cho tên dài (không còn "…").
- Title bar overlay: cột ghim vẫn kết thúc cách footer 8px, trang không cuộn.

## Việc dở
- Độ rộng nội dung không tính badge touch của agent (nhãn "Chỉnh sửa" của file đang bị agent sửa) và cột size thư mục; cuộn ngang vẫn thấy được chúng ở dòng đó nhưng có thể cần cuộn thêm.
- Chưa chạy trên macOS.
