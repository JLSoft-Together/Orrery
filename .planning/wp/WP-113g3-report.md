## WP-113g3 — DONE

### Đã đổi
- `apps/web/src/features/workspace/composables/usePanePhone.ts` (mới): `available` = `hasPushPhone(remote.devices)`, `send()` gọi `sendPaneToPhone(paneKey)` rồi toast (id `pane-phone`, 4s; info khi sent > 0, warning còn lại); `phoneToastKey`. Test `usePanePhone.test.ts` (mới).
- `TermPane.vue`: import `Smartphone` + composable, `headCtx.phone`, mục menu `phone` (icon Smartphone), `pickMore` gọi `send()`. Không thêm logic dài vào file (composable riêng).
- `features/remote/index.ts`: thêm 2 dòng export (`sendPaneToPhone`, `hasPushPhone`, `parseSendPane`, type `SendPaneResult`).
- `features/workspace/i18n/workspace-v52-pane.ts`: `paneHead.phone.{label,sent,none,limit,off,notFound,unavailable}` en + vi.
- `packages/shared/src/remote.ts`: `'stale'` vào `RemoteErrorCode`. `remoteMenu.ts`: sig lệch trả `stale`. Tests `useRemotePhone.test.ts:253`, `remoteMenu.test.ts:65` đổi kỳ vọng sang `stale`.
- Snapshot `apps/web/src/i18n/__snapshots__/keys.{en,vi}.txt` cập nhật bằng `-u` (+7 dòng mỗi file, đều là key phone của WP này).

### Tiêu chí
- [x] Mục `phone` hiện khi có phone bật push (ẩn khi không, theo `demotedKeys`), bấm gọi `sendPaneToPhone(pane.key)`, toast theo lỗi (test `phoneToastKey`); `pickForPhone` sig lệch -> `stale`.
- [x] typecheck sạch; i18n en + vi; không comment.
- [~] TermPane.vue vẫn 1944 dòng (đã sẵn > 400 từ trước; thêm 5 dòng, logic ở composable).

### Audit
Chỉ các file trên. Không đụng Rust, connectApi, connect/*, devices, settings, i18n chung.

### Review
- `invalid` từ bridge gộp vào toast `unavailable`; `sent: 0` -> toast `none`.
- Menu `phone` chỉ cho pane agent (`demotedKeys` đã chặn shell).
- Chưa có test mount TermPane (quá nặng); hiện/ẩn mục dựa test `paneHeadActions` + `hasPushPhone` có sẵn.
- `stale` thêm vào union: nơi nào switch toàn phần trên `RemoteErrorCode` đã qua typecheck. Mobile/PWA nên xử lý `stale` như `busy` (WP-113g2 biết).

### Test
- `npm test -w @ccm/web -- src/features/workspace src/features/remote` -> 716 passed, 1 failed: `useRemotePhone.test.ts > rename and move change state only` (flaky; chạy riêng `-t "rename and move"` -> pass; g1 cũng ghi timeout/flaky ở file này).
- `npm test -w @ccm/web -- src/i18n -u` -> 6 passed, 2 snapshots updated.
- `npm run typecheck` -> sạch (habblaud, shared, pwa, web).

### Nhìn
CHƯA NHÌN: không chạy app/E2E theo luật.

### Hot file cần thêm
Không.

### Ngoài phạm vi
`docs/remote-integration.md`, `.code_index` (mục 7, 8 của g1) chưa làm.

### Commit message đề xuất
```
feat(workspace): open pane on phone menu item and stale error for outdated phone menu answers

Add the Open on phone pane menu entry that sends the pane to push phones with a toast per result, and return stale instead of busy when a phone answers a menu whose signature changed.
```
