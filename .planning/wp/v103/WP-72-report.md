# WP-72 report: Git Log xem diff nhanh + xem ảnh trong commit

Trạng thái: xong, chưa commit (theo luật chung).

## #9 Ảnh trong thay đổi của commit

Bấm `png / jpg / jpeg / gif / webp / svg / bmp / ico` trong danh sách file của 1 commit giờ hiện ảnh trước / sau (Cạnh nhau, Thanh trượt, Onion skin như Changes), không còn "Tệp nhị phân". Ảnh thêm chỉ có bên sau, ảnh xoá chỉ có bên trước, ảnh đổi tên đọc bên trước theo tên cũ. SVG hiện bằng `<img>` từ `data:` URL, không chèn markup vào DOM. Max 20 MB mỗi bên.

- Desktop Node: `apps/desktop/src/preview/gitImage.ts` `commitImage(root, hash, rel, side, origPath?)` + `blobImage` dùng chung với `headImage`; thêm `ico`. `log.ts` export `resolveCommit`. IPC mới `ccm:log-image` trong `preview/ipc.ts` (gate `logRepo`, thêm vào `CHANNELS`), preload `logImage`.
- Rust (desk, Tauri và Electron chế độ rust): `core-desk/src/preview/gitmisc.rs` `commit_image` + `blob_image`, `ico`; đăng ký `ccm:log-image` trong `ipc_git.rs`.
- Shim Tauri sinh lại: `apps/shell/src/shim.js` (`node scripts/gen-desk-shim.mjs`, chỉ thêm `logImage`).
- Web: `lib/preview.ts` `logImage` / `hasLogImage`; `ImageCompare.vue` thêm prop `commit` + `labels` (`<hash>^` / `<hash>`), lỗi riêng `imageCompare.commitError.*`; toolbar ImageCompare co theo bề rộng (ẩn kích thước khi hẹp, trước đây chữ đè lên thanh trượt; áp dụng cả Changes). `lib/imageCompare.ts` nhận `.ico`.
- Changes pane đã có ảnh HEAD | working copy từ v70, giữ nguyên.

## #8 Xem diff nhanh từ Git Log

So với IntelliJ / VS Code / Fork, các điểm khó dùng đã sửa:

| Trước | Bây giờ |
|---|---|
| Chọn commit chỉ thấy danh sách file, phải double-click mới thấy diff | Diff preview ngay dưới bảng commit: chọn commit là thấy diff file đầu (hoặc file đang xem) |
| Mở diff thì bảng commit **và** danh sách file biến mất, chỉ đổi file bằng ← → | Chế độ toàn khung (Enter / double-click / nút phóng to) vẫn giữ danh sách file + chi tiết commit bên phải, bấm file nào xem file đó |
| Đổi commit là mất file đang xem | Nhớ file đang xem khi đổi commit, đi theo cả đổi tên (`origPath`); commit không có file đó thì mở file đầu |
| Không đổi commit được khi đang xem diff | Alt+↑ / Alt+↓ (và nút ˄ ˅ ở toolbar toàn khung) sang commit mới hơn / cũ hơn, giữ file |
| Esc quay lại thì bảng commit cuộn về đầu (bị unmount) | Bảng commit luôn mounted (`v-show`), giữ vị trí cuộn và lựa chọn |
| Không có phím vim | j / k chuyển commit trong bảng; trong diff j / k (và F7) sang thay đổi kế, hết thay đổi thì sang file |
| Đổi gộp chung / cạnh nhau phải vào Changes | Nút Gộp chung / Cạnh nhau ngay trên toolbar diff (dùng chung tuỳ chọn với Changes) |
| | Vị trí file "2 / 3", nút tắt preview (X) và nút bật lại ở header pane, kéo đổi chiều cao preview (lưu trong `ccm.workspace.logCols`) |

Code: `apps/web/src/components/workspace/log/LogDiffPanel.vue` (mới, 150 dòng: tải diff 1 file, `DiffView` hoặc `ImageCompare`, toolbar, `stepBlock`), `LogPane.vue` (`syncDetail`, `openFile`, `pickFile`, `closeFull`, `togglePreview`, `stepFile`, `stepBlock`, `stepCommit`, `onKey`, `cols.preview` / `previewSplit`, `diffFull`; file còn 536 dòng, ngắn hơn trước 559), `LogTable.vue` (j / k, bỏ qua khi có Alt/Ctrl/Meta). i18n `workspace-log.ts` (`diff.toolbar|position|newerCommit|olderCommit|layout|expand|preview|showPreview|hidePreview`, `keys` mới), `workspace-commit.ts` (`imageCompare.commitError`). Copy hash đã có (`LogCommitDetails`, Ctrl/⌘+C trong bảng, menu chuột phải).

UI next: panel diff đặc (`bg-well`), không blur; preview hiện bằng `ccm-fade`, đổi file / commit crossfade.

Docs: `docs/claude-code-integration.md` §11s thêm dòng "Image in a commit". `.code_index/web.md`, `desktop.md`, `_meta.md` cập nhật.

## Test (lệnh chạy thật)

- `npm run typecheck -w @ccm/web` (JIN_RAW): pass lúc làm xong phần code. Chạy lại cuối phiên còn lỗi ở `src/lib/explorer/treeWidth.test.ts`, `src/lib/next/explorerPin.test.ts`, `src/lib/rail/rail.test.ts` (file của WP-70a / WP-71 đang làm dở), không có lỗi trong file của WP-72.
- `npm run typecheck -w @ccm/desktop`: pass.
- `npm test -w @ccm/desktop -- test/git-image.test.ts test/preview-ipc.test.ts`: 18/18 pass (thêm describe `commitImage`: 2 test, ico).
- `npm test -w @ccm/desktop -- test/desk-preview-git.twin.test.ts --testTimeout=90000`: 5/5 pass (thêm 11 ca `ccm:log-image` Node = Rust). Với timeout mặc định 5 s cả 3 test git timeout vì máy đang tải nặng (cargo test của gói khác), kể cả test không đụng tới.
- Web `npx vitest related --run` cho các file đổi: 887/888, 1 fail `DevicesView.test.ts` (QR popover) chạy riêng 25/25 pass, chập chờn khi tải.
- `npx vitest run src/lib/imageCompare.test.ts`: 3/3.
- `cargo test -j 4 -p core-desk preview`: 54 pass. `cargo test -j 4 shim` (apps/shell): 8 pass (shim có `ccm:log-image`).
- E2E (Electron dev, `CCM_CORE_BIN` = `orrery-core` debug mới build):
  - mới `apps/desktop/e2e/v103-log-diff.cjs` (UI next): **18/18 PASS** (svg / png 2 bên, slider, thêm 1 bên, xoá 1 bên, đổi tên, ↓ giữ file, k, Enter toàn khung giữ danh sách file, Alt+↑, ←, Esc về preview, ẩn / hiện preview, không cuộn ngang, file chữ vẫn diff chữ).
  - `e2e/log.cjs` (sửa 2 check cho chế độ preview): 27/27 PASS.
  - `e2e/split-diff.cjs` (sửa chờ đúng hash ở chế độ toàn khung): 28/29, fail `imp.ts has 2 changes when comparing all` (Changes pane, "Change 1 of 3" do tuỳ chọn Highlight "Split changes" từ bước trước còn lưu; WP-72 không đụng Changes / DiffView). 2 check Log trong file này pass.
- Ảnh: `docs/qa/v103/WP-72/` `v103-wp72-01-svg-preview-light.png`, `02-png-slider.png`, `03-full-renamed.png`, `04-preview-dark.png`, `05-text-dark.png`, `results-v103-log-diff.txt`. Đã tự xem.

Build: `vite build` web + `npm run build -w @ccm/desktop` với `CCM_SKIP_CORE_BUILD=1` để chạy E2E (lần đầu build.mjs tự gọi `cargo build --release -j 6`, con đã dừng để giữ luật `-j 4`; `cargo build -j 4 -p core-bin` debug). Không build release / dist.

## QA tay

Windows (Tauri bản dev cần build lại shell vì `shim.js` nhúng lúc compile, và `orrery-core` mới):
1. Mở Git Log một repo có commit đổi ảnh png / webp / svg. Chọn commit: diff preview hiện ngay dưới bảng.
2. Bấm file ảnh: thấy 2 bên, thử Thanh trượt / Onion skin. Commit thêm ảnh: 1 bên "Added"; xoá: 1 bên "Deleted".
3. ↑ ↓ (hoặc j k) trong bảng: preview đi theo, giữ file đang xem.
4. Enter: diff toàn khung, danh sách file vẫn bên phải; Alt+↑ ↓ đổi commit; ← → đổi file; j k / F7 nhảy thay đổi; Esc về bảng, vị trí cuộn còn nguyên.
5. Nút X trên toolbar preview tắt preview; nút ở header pane bật lại; kéo thanh ngang đổi chiều cao, mở lại pane vẫn nhớ.
macOS: như trên, Alt = ⌥.

## Cần chủ nhân chốt

1. Diff preview dưới bảng **bật mặc định** (máy đang dùng cũng bật vì key `preview` mới). Muốn mặc định tắt (giống cũ, phải Enter mới thấy diff) không?
2. Phím: Alt+↑ / Alt+↓ đổi commit trong diff, j / k trong bảng = commit, trong diff = thay đổi (giống Changes). Có muốn j / k trong diff cũng đổi commit không?
3. **Wrap dòng** trong diff chưa làm: `DiffView` / `SplitDiff` vẽ theo cửa sổ hàng cao cố định (virtual rows), wrap cần đo chiều cao từng hàng, là việc riêng. Có cần làm không?
4. **So 2 commit bất kỳ** (chọn 2 commit, diff giữa chúng) chưa làm: cần IPC mới `diff A B`. Hiện có "Compare with Current" cho nhánh. Có muốn làm gói riêng không?
5. Chế độ preview khi pane hẹp: preview nằm dưới bảng. Muốn thêm lựa chọn đặt preview bên phải (như IntelliJ "Diff preview: right") không?

## Việc dở

- Không có trong phạm vi. Fail `split-diff.cjs` `imp.ts` là của Changes pane, nên chạy riêng xác nhận ở nhánh trước WP-72 nếu cần.
