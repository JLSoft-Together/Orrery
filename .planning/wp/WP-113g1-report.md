## WP-113g1 — PARTIAL

Phần desktop (Rust + wrapper web) xong và xanh. Chưa nối vào UI/kênh vì cần sửa hot file (mod.rs, preload.ts, desktop.ts, TermPane.vue, i18n): xem "Hot file cần thêm". `stale` cho `ws.menu` chưa đổi (file ngoài phạm vi).

### Đã đổi

Rust (`apps/shell/src/remote/`):
- `push.rs`: `kind_of`/`title_body` thêm `running`, `open`, `clear`; `Job` thêm `started`, `extra`; `message()` thêm `w`, `t0`, `android.notification.tag` (push có pane), channel `running`; `clear` là data-only (không `notification`, priority normal); `queue_pane` (gốc của `push_pane`); `deliver` = `settled` -> `enrich` -> gửi -> `note_delivered`; `run` không bỏ push `open` khi phone đang live; `push_register` nhận `pauseUntil`, `push_unregister` quên pause/told.
- `push_extra.rs` (mới, nạp bằng `#[path]` trong push.rs nên KHÔNG cần sửa mod.rs): `PER_HOUR_INFO=20`, `admit_info`/`admit_kind` (quota info riêng; `running`/`open`/`clear` không dính pane gap; `done` vẫn dính), `ongoing` (push kết thúc lượt luôn thay được notification `running` dù dính gap/mute), `set_pause`/`paused` (RAM, max 24h), `set_waiting`/`told`, `push_running`, `push_open`, `push_clear`, `enrich` (w, wl, menu, pause, clear bị thay), `menu_of`, `waiting_labels`, `Remote::send_pane`.
- `push_content.rs`: `label_of` (nhãn <=24), `LABEL_MAX`, body cho `running`/`open`.
- `push_seal.rs`: plaintext seal thêm `wl`, `menu`; data ngoài seal thêm `w`, `t0`.
- `push_web.rs`: payload Web Push thêm `w`, `t0`, `wl`, `menu`; `pauseUntil` ở `push.register` web; thời gian chạy trong body `running`.
- `push_watch.rs`: `waiting_ids`, `due_running` (>= 120s, 1 lần / lượt), tick 15s, gọi `set_waiting` / `push_clear` / `push_running`.
- Test: `push_extra_tests.rs` (mới, 15 test), sửa kỳ vọng 4 test cũ vì mọi push giờ có `w` (`push_tests.rs`, `push_watch_tests.rs`, `push_seal_tests.rs`).

Web:
- `features/workspace/lib/paneHeadActions.ts` + test: `HeadActionCtx.phone?`, `DemotedKey` thêm `phone` (chỉ hiện trong menu của pane agent khi `phone` true).
- `features/remote/lib/remotePush.ts` (mới): `sendPaneToPhone` (bridge `remoteSendPane`, thiếu bridge / lỗi -> `{ok:false,error:'unavailable'}`), `parseSendPane`, `hasPushPhone`. Test `remotePushSend.test.ts` (mới; tên `remotePush.test.ts` đã dùng cho test shared).

Tài liệu: `.planning/wp/WP-113g-contract.md` (hợp đồng cho WP-113g2).

### Tiêu chí

- [x] Contract có đủ kind + ví dụ, khớp parser 113k (`w` ngoài seal, `wl` trong seal, `k:"clear"`, `pauseUntil` epoch ms).
- [x] cargo: menu trong payload (mask, cắt 24, max 3, bỏ `other`/`submit`/menu nhiều ô, content tắt thì không có), w/wl/clear, pauseUntil (cả qua session thật), quota info tách riêng, running/done max 2 / lượt, open (link `m`+`p`), kết thúc lượt thay notification `running`.
- [~] `stale`: CHƯA. `ws.menu` sig lệch hiện trả `busy` (`apps/web/src/features/remote/lib/remoteMenu.ts:40`, `pickForPhone`); `stale` chưa có trong `RemoteErrorCode` (`packages/shared/src/remote.ts:17`). Cả 2 file ngoài phạm vi (remote*.ts thuộc e1). Cần: thêm `'stale'` vào `RemoteErrorCode`, `pickForPhone` trả `stale`, sửa test `useRemotePhone.test.ts:253` (đang kỳ vọng `busy`).
- [x] Vitest paneHeadActions key `phone` ẩn/hiện; wrapper.
- [x] Không comment, file <= ~400 dòng (push.rs 347, push_extra.rs 304, push_extra_tests.rs 421 là file test).

### Audit

- Quyết định ngoài brief: `done` chuyển sang quota info (theo spec §F5 "running/done/open/clear"), giữ pane gap 30s cho `done`.
- `pauseUntil` lưu RAM trong `PushState` (không sửa `Device`/`devices*.rs` vì cấm); phone gửi lại `push.register` mỗi lần kết nối nên đủ; desktop khởi động lại thì mất tới lần phone kết nối kế. Muốn bền: thêm `push_pause_until` vào `Device` (e1).
- `clear` chỉ gửi FCM (Web Push không có thông báo im lặng), chỉ tới phone đã nhận `w>0`, bị bỏ nếu lúc gửi số chờ lại > 0.
- Mốc `t0` của `running` = lúc Orrery thấy session bắt đầu `working` (session chạy sẵn lúc khởi động: mốc = lúc thấy).
- `open` gửi cả khi phone đang kết nối, bỏ qua pause và mute (hành động chủ nhân chọn); `security`/`test`/`clear` cũng không bị pause.

### Review

- Content tắt: không `wl`, không `menu` (test). Seal tắt + content bật (FCM): `message()` thuần chỉ có `w`, không lộ nhãn (test: chuỗi JSON không chứa nhãn/"menu"). Seal bật: nhãn nằm trong `e`.
- Phone cũ (không `pauseUntil`, không hiểu `w`): field thừa bị bỏ qua; `k:"running"/"open"/"clear"` không nằm trong `pushKinds` của phone cũ nên `pushKind()` đổi thành `attention` (hiện như thông báo thường; `clear` cũ có thể hiện thông báo trống: phone 113k đã có `isClearPush`, app cũ hơn thì chấp nhận).
- Quota đầy: info đầy không chặn push hỏi (test); `open` trả `limit`.
- Session kết thúc < 2 phút: `due_running` không bắn (test). Kết thúc sau khi đã `running`: push kết thúc đi qua `ongoing`, không dính gap/mute; bị chặn nếu phone vừa tạm dừng giữa lúc xếp hàng và gửi: phone nên đặt `timeoutAfter` ~6h cho notification `running` (ghi trong contract).
- Pause hết hạn: `paused` so `until > now` (test biên), hết mốc tự gửi lại; mốc > 24h hoặc quá khứ bị bỏ.
- Terminal bị xoá khi đang `running` (không có push kết thúc): chỉ dựa `timeoutAfter` phía phone.
- Log: không log nội dung push (không thêm log mới).

### Test

- `cargo test --locked -j 4 push` (PowerShell, apps/shell) -> 87 passed, 0 failed, 3 ignored.
- `cargo test --locked -j 4 remote` -> 316 passed, 0 failed, 3 ignored.
- `npm test -w @ccm/web -- src/features/workspace/lib/paneHeadActions src/features/remote/lib/remotePush` -> 3 files, 20 passed.
- `npm test -w @ccm/web -- src/features/workspace src/features/remote` -> 712 passed, 4 failed, đều timeout hook / đếm publish ở `useRemotePhone.test.ts`, `useRemoteSummary.test.ts`, `workspaceLinksTurn.test.ts`: không đụng file của WP này, nghi do thay đổi đang dở của e1 (`features/remote/index.ts`, settings). CHƯA KIỂM nguyên nhân.
- `npm run typecheck` -> 1 lỗi duy nhất `src/features/settings/components/rows.test.ts(54,7)` thiếu `devices` trong `SettingsEnv` (thuộc F4/e1, không phải file của WP này). Không có lỗi ở file của WP.

### Nhìn

CHƯA NHÌN mục menu `phone` trong pane head (chưa gắn vào `TermPane.vue`, hot). Không E2E, không build (theo luật).

### Hot file cần thêm

1. `apps/shell/src/remote/mod.rs`: trong mảng kênh (cạnh `"ccm:remote-revoke-undo",`) thêm `"ccm:remote-send-pane",`; trong `match` (cạnh `"ccm:remote-install"`) thêm `"ccm:remote-send-pane" => Ok(self.send_pane(arg)),`. `push_extra*.rs` đã nạp qua `#[path]` trong `push.rs`, không cần `mod`.
2. `apps/shell/shim/preload.ts` (cạnh `remotePushDevice`): `remoteSendPane: (paneKey: string): Promise<unknown> => typeof paneKey === 'string' && paneKey.length > 0 && paneKey.length <= 128 ? ipcRenderer.invoke('ccm:remote-send-pane', paneKey) : Promise.resolve({ ok: false, error: 'invalid' }),`. Có thể cần cập nhật `apps/shell/src/shim.js` bằng `node scripts/gen-desk-shim.mjs`.
3. `apps/web/src/core/lib/desktop.ts` (cạnh `remotePushDevice`): `remoteSendPane?: (paneKey: string) => Promise<unknown>`. Khi đó `remotePush.ts` có thể bỏ cast `PushBridge`.
4. `apps/web/src/features/remote/index.ts`: export `sendPaneToPhone`, `hasPushPhone`, `parseSendPane` và type `SendPaneResult`.
5. `apps/web/src/features/workspace/components/TermPane.vue`: `headCtx` thêm `phone: hasPushPhone(status.devices)`; thêm mục menu key `phone` gọi `sendPaneToPhone(pane.key)` và toast theo lỗi (`none`, `limit`, `off`, `notFound`, `unavailable`). `demotedKeys(headCtx)` đã trả `phone`.
6. i18n en/vi: `workspace.head.phone` = "Open on phone" / "Mở trên điện thoại"; toast: sent N (đã gửi tới N điện thoại), none ("Chưa có điện thoại nào bật push"), limit ("Đã gửi quá nhiều, thử lại sau"), unavailable.
7. `docs/remote-integration.md` §7.6/§7.7: thêm kind `running`/`open`/`clear`, field `w`/`wl`/`t0`/`menu`, `pauseUntil`, quota `PER_HOUR_INFO=20`, kênh `ccm:remote-send-pane` (copy từ `WP-113g-contract.md`).
8. `.code_index`: thêm `push_extra.rs::queue_pane/enrich/push_open/push_clear/push_running`, `remotePush.ts::sendPaneToPhone`; 1 dòng `_meta.md`.
9. `packages/shared/src/remote.ts` + `remoteMenu.ts` (+ test `useRemotePhone.test.ts`): lỗi `stale` cho `ws.menu` (xem Tiêu chí).

### Ngoài phạm vi

- `stale` cho `ws.menu` (file e1 / shared).
- Phone / PWA: nhận `running`, `open`, action menu, `wl`/`menu` trong seal (WP-113g2).
- 4 vitest timeout và lỗi typecheck `rows.test.ts` ở trên.

### Commit message đề xuất

```
feat(remote): running, open and clear push kinds with waiting count, menu answers and pause until

Add desktop push for sessions running over two minutes, send-to-phone for a pane,
and clear when the waiting count drops to zero. Every group push carries the waiting
count; labels and up to three menu choices ride inside the seal only with content on.
Info pushes get their own hourly quota so questions are never crowded out, and a phone
pause mutes the groups until its mark.
```
