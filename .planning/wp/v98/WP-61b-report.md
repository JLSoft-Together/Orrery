# WP-61b báo cáo

## Việc đã làm
- Sửa file: `FileEditor.vue` (textarea mono 16 px, Lưu gửi `fs.write {path,text,base=mtimeMs}`, xung đột `conflict` hiện banner + Tải lại, thoát khi chưa lưu hỏi qua `lib/editGuard.ts`: nút back, đổi segment, đóng viewer). Chế độ chỉ xem (`hub.isViewOnly`) ẩn nút Sửa.
- Git (`ViewerChanges.vue`, `ChangeRow.vue`, `CommitBox.vue`): 2 nhóm Đã stage / Thay đổi, stage / unstage từng file và tất cả, discard hỏi trên điện thoại rồi `confirm.ask` của desktop, commit message + `git.commit`. Cập nhật qua `git.changed` và 800 ms sau khi `confirms` về 0.
- Run (`ViewerRun.vue`): `run.configs`, `run.devices`, Run / Reload (hot) / Restart / Stop, log theo `run.event`.
- Lỗi `denied` -> chế độ chỉ xem; lỗi khác `err.<code>`; pending = "chờ xác nhận trên máy tính" (`NoticeLine`, `lib/useAction.ts`). i18n en + vi, motion cho danh sách (TransitionGroup).
- Tách `WorkspaceView.vue` 496 -> 313 dòng (`PaneChips`, `PaneGrid`, `NewPaneSheet`), không đổi hành vi.
- Logic: `lib/viewerActions.ts`, `viewerModel.ts` (mtimeMs, staged / unstaged, run configs / devices).
- E2E: peer `e2e/peer/src/vw.rs` (trạng thái giả + `diskchange`, `vwreset`), `e2e/lib/edit-steps.mjs` (6 bước / engine). `ws-steps.mjs` đổi kỳ vọng footer từ "chỉ đọc" sang "dòng". `.code_index/web-pwa.md` đã cập nhật.

## Test chạy thật
- `npm run typecheck -w @ccm/pwa`: sạch.
- `npm test -w @ccm/pwa`: 189 pass / 0 fail (trước 175, thêm `viewerEdit.test.ts` 14).
- `npm run e2e -w @ccm/pwa` (cả 2 engine): 80/80 pass (40 mỗi engine, trước 34); 0 vi phạm CSP (bước boot kiểm).
- `npm run build -w @ccm/pwa`: ok. `cargo build -j 4` peer e2e: ok.
- Đã `git checkout --` ảnh WP-55b, WP-56a, WP-61a sau e2e. Không comment trong code; file mới đều dưới 260 dòng.

## Ảnh
`docs/qa/v98/WP-61b/`: editor, changes (commit box), run: phone sáng / tối + iPad sáng / tối (webkit), phone sáng chromium; `edit-conflict-phone-light-*`.

## QA tay iPhone
1. Viewer > Tệp > mở file > Sửa > gõ > Lưu; sửa file đó trên máy tính rồi Lưu lại: phải hiện xung đột + Tải lại. Bấm back khi chưa lưu: hỏi bỏ.
2. Diff: stage / unstage từng file và tất cả, nhập message + Commit; nút hoàn tác (discard) hỏi trên điện thoại, rồi hộp xác nhận từ desktop.
3. Chạy: chọn config + thiết bị, Run, xem log, Reload / Restart / Stop (config lạ sẽ hỏi trust).

## Rủi ro / việc dở
- Không theo dõi `fs.changed` khi đang sửa: chỉ phát hiện file đổi lúc Lưu (xung đột), chưa có banner báo sớm như Android.
- Hợp đồng desktop không đổi, không thấy thiếu gì. Sau commit hết file thì AsyncState hiện "Không có thay đổi", ô commit ẩn theo.
- Lần chạy e2e đầu bị `waitForCargo` chờ cargo của project khác (đã dừng, chạy lại `--no-peer-build` sau khi build peer thủ công).
- Không tiến trình nền còn chạy; chưa commit.
