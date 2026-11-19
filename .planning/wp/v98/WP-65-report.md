# WP-65 báo cáo: khoá Face ID sau ghép, editor báo file đổi, Hoàn tác sau xác nhận

## File + symbol (apps/pwa/src)
- Khoá: `lib/lockOffer.ts` (`lockKind`, `shouldOfferLock`), `components/LockOffer.vue`, `views/PairView.vue` (phase `lock`, `finish`), `storage/localPrefs.ts` (`lockOffered`), i18n `lockOffer.*`. Hiện khi: máy đầu tiên (`hub.machines.length` trước ghép = 0) + khoá tắt + chưa hỏi + `lock.checkSupport()`. `lockOffered` đặt ngay khi hiện bước (nhớ đã hỏi). Huỷ / lỗi: dòng `lock-offer-fail`, không chặn, vẫn bấm Bật lại hoặc Để sau.
- Editor: `lib/diskWatch.ts` (`touchesPath`, `isOwnChange` 3 s, `watchRoot`/`unwatchRoot`, `currentStamp`), `components/DiskChangedBanner.vue`, `FileEditor.vue` (props mới `root`, `subscribe`; `fs.watch` khi mở, `fs.unwatch` khi đóng; `keepMine` đọc lại mtime làm `base` để Lưu đè), `ViewerFiles.vue` / `ViewerPage.vue` truyền `subscribe`. Banner cũ `edit-conflict` giữ nguyên.
- Hoàn tác: `lib/closeUndo.ts` (`rememberClose`, `closeAnswered`, `offerUndo`, `undoOffer`), `lib/paneActions.ts` (`CloseResult.confirm`), `PaneActionsSheet.vue`, `RemoteConfirmHost.vue`, `WorkspaceView.vue`; `answerConfirm` (`remote/machineData.ts`, `remote/hub.ts`, `stores/hub.ts`) giờ trả reply.
- E2E peer (`e2e/peer/src/{main,conn,vw}.rs`): stdin `closeask` (ws.close hỏi cid `cidc`), `diskchange` phát `fs.changed`. Steps: `e2e/lib/edit-steps.mjs`, `pane-steps.mjs`, `lock-offer-step.mjs` (mới), `ctx.mjs` `finishPair` chịu được bước khoá, `run.mjs` `qaDirE`, `scenario.mjs` 43 step / browser.
- Docs: `docs/remote-integration.md` §4.6, `docs/orrery-remote.md` mục cuối, `.code_index/{_meta,web-pwa}.md`.

## Hợp đồng
Không đổi hợp đồng desktop, không đụng `apps/shell`. Lý do: `confirm.answer` ok trả phản hồi của lệnh gốc (`{ok:true,...}`), từ chối trả `cancelled:true`, lỗi trả error (reject) nên web app tự suy ra "đã đóng" mà không cần trường mới. Không chạy cargo test shell (không đụng).

## Test (lệnh thật)
- `npm run typecheck -w @ccm/pwa`: sạch.
- `npx vitest run --root apps/pwa`: 22 file, 209 pass / 0 fail (so 198: +11 trong `lockOffer.test.ts`, `diskWatch.test.ts`, `closeUndo.test.ts`; `paneActions.test.ts` cập nhật).
- `node e2e/run.mjs --no-build --no-peer-build` (sau lượt `npm run e2e -w @ccm/pwa` đã build): WebKit + Chromium 86 pass / 0 fail, 86/86 step (so 80). Lượt đầu hỏng 2 lần do lỗi test của con (RP id `127.0.0.1` không hợp lệ cho WebAuthn, đã chuyển sang `localhost` như `lock-step.mjs`) và 1 lần timeout lẻ ở step ảnh Pair (không liên quan, chạy lại sạch).
- Giới hạn WebKit: headless WebKit không có platform authenticator nên step khoá chỉ kiểm "không hỏi, vào thẳng Monitor, chưa đánh dấu đã hỏi". Chromium dùng virtual authenticator CDP: huỷ (isUserVerified=false) = báo nhẹ, Để sau = nhớ, Bật = `lockEnabled` + `credentialId`.

## Ảnh `docs/qa/v98/WP-65/`
`lock-offer-phone-{light,dark}-chromium`, `lock-offer-fail-phone-light-chromium`, `edit-disk-changed-phone-{light,dark}-{webkit,chromium}`, `close-undo-phone-light-{webkit,chromium}`. Ảnh WP-55a/55b/56a/61a/61b bị e2e ghi đè đã `git checkout` lại. Dòng `M` của `.planning/PROGRESS-2026-11-12-v98.md` và `PLAN-v98-merge-master.md` không phải của gói này.

## Bước QA iPhone cho chủ nhân
1. Xoá app / dữ liệu, ghép máy đầu tiên: hiện "Bật khoá Face ID?"; Bật = Face ID thật hiện ra; Huỷ Face ID = dòng báo nhẹ, vẫn dùng được; Để sau = vào Monitor, ghép máy thứ hai không hỏi lại; Cài đặt vẫn bật được.
2. Mở file trong Viewer, bấm Sửa, gõ vài chữ, sửa cùng file trên desktop: banner "File đã đổi trên máy" hiện ngay, bản đang gõ còn nguyên. Giữ bản của tôi rồi Lưu: lưu đè không báo conflict; Tải lại: mất bản đang sửa, hiện bản mới.
3. Workspace, đóng pane đang chạy (desktop hỏi xác nhận): bấm xác nhận trên điện thoại thì thanh Hoàn tác hiện, bấm Hoàn tác mở lại pane; bấm Giữ lại thì không có thanh Hoàn tác.

## Rủi ro / việc dở
- Face ID hay Touch ID chỉ đoán theo user agent (iPhone / iPad = Face ID); iPhone cũ có Touch ID vẫn thấy chữ Face ID. Nếu cần chính xác phải để chủ nhân chốt.
- Hoàn tác chỉ hiện nếu `WorkspaceView` của máy đó đang mở lúc xác nhận; rời tab Workspace trước khi xác nhận thì không có thanh.
- `fs.watch` tính vào giới hạn root mỗi thiết bị của desktop (8); editor mở / đóng là watch / unwatch, lỗi `limit` bị nuốt (banner không hiện).
- Không dùng cổng 4320-4322; relay e2e 8791 / 9331 đã tắt cùng run, không còn tiến trình nền của gói này.
