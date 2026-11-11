## WP-7 — DONE (7/7 tiêu chí; ảnh trang Settings CHƯA NHÌN)

### Đã đổi
- `packages/shared/src/remote.ts`: `RemotePhoneCall {device, cmd, confirmed?}`, `RemoteNeedsConfirm`, `RemotePairStart {qr, exp}`, `RemoteKeyPress`, parser `parseRemoteCommand` / `parseRemotePhoneCall` / `parseRemoteKeys` (text ≤ `REMOTE_TEXT_MAX`, không NUL, preset claude|codex|shell, resume phải là UUID, `confirmed` nhận ở cấp call hoặc trong cmd, chỉ `true`).
- `packages/shared/src/orreryApi.ts`: source `'phone'`; `OPEN_POLICY.phone = {submit:false, limit:true, dock:false, askSubmit:true}` (prompt chỉ submit khi lệnh `submit:true`); action `runs:'page'`, source `['phone']`: `remote.summary.get`, `ws.open|close|rename|move|reopen|compact|send|keys`.
- Mới: `lib/remoteSummary.ts`, `lib/remote.ts`, `stores/remote.ts`, `composables/useRemoteSummary.ts`, `composables/useRemotePhone.ts`, `components/settings/SettingsRemote.vue` + `settings/remote/{RemotePairPanel,RemoteDeviceRow,RemoteLogList}.vue`, `components/workspace/RemotePhoneChip.vue`, `i18n/remote-v97.ts`.
- Sửa: `lib/desktop.ts` (bridge types + `hasRemote`), `lib/ptyClient.ts` (`claim()`, `claimIfNeeded()`), `TermPane.vue` (claim khi keydown / paste, nhận `resized`, chip), `App.vue` (start store, install 2 composable, mount Workspace ẩn khi `remote.connected > 0`), `components/settings/index.ts` (+ `SettingsMenu.vue`, `useSettingsEnv.ts`: trang `remote`, `SettingsEnv.remote`), `i18n/ui-v79-settings.ts` (`pages.remote`), tests `settings/index.test.ts` + `lib/paletteSearch.test.ts` (env mới), `.code_index/{web,protocol,_meta}.md`.
- `SettingsMenu.vue` ngoài danh sách file của brief nhưng trong `apps/web/src/**`; bắt buộc để hiện trang mới.

### Kênh native thực sự gọi (qua `window.ccmDesktop`, thiếu thì `hasRemote` false → trang ẩn)
- `remoteStatus(): RemoteStatus|null` · `onRemoteChanged(h): unsubscribe` · `remoteEnable(on): RemoteStatus|null` · `remoteKeepAwake(on): RemoteStatus|null` · `remotePairStart(): {qr, exp}|null` (exp giây hoặc ms đều được) · `remotePairCancel(): boolean` · `remoteRevoke(deviceId): boolean` · `remoteRevokeAll(): boolean` · `remotePublish(snapshot): boolean`. Tên method preload do WP-5 map sang `ccm:remote-*`.
- Orrery (gateway → renderer, `onOrrery`): `remote.summary.get` args `{}` → `RemoteWorkspaceSnapshot`; `ws.*` args `{device: string, cmd: RemoteCommand, confirmed?: true}`, caller `{source:'phone'}`. Trả: `{ok:true, paneKey|paneKeys|submitted?}` | `{ok:false, error: RemoteErrorCode}` | `{needsConfirm:{title, detail?, ok, cancel}}` (chuỗi đã dịch theo locale desktop).
- `ws.keys.keys` ∈ `approve|deny|stop|esc|1..9` (số = chọn option); `since` = `statusSince` ms, lệch → `busy`.

### Quyết định ghi lại
- `resized` từ hub: KHÔNG `term.resize` (giữ cỡ desktop). Lý do: mirror cỡ điện thoại làm pane desktop co giật và `onResize` sẽ gửi cỡ điện thoại làm cỡ lưu của desktop trong hub, nên lần claim sau áp sai cỡ. `claimIfNeeded()` chỉ gửi `claim` sau khi nhận `resized`; hub tự áp lại cỡ desktop và gửi `resized` cho điện thoại. Desktop 1 socket không điện thoại không bao giờ gửi `claim`.
- Claim bắt ở `onKey` (keydown, bỏ phím modifier) và `paste()`; câu trả lời query của xterm (`onData`) không claim.
- Chip: icon `Smartphone` (không emoji, theo ui-ux-pro-max), hiện khi có điện thoại kết nối và pane vừa nhận lệnh (60 s) hoặc đang giữ cỡ (`resized`). `RemoteStatus` không có thông tin "đang xem pane nào" nên không chính xác hơn được.
- Log: `stores/remote.ts` ring 50 dòng (hiện ở Settings) + note vào inbox của session với chữ "📱 thiết bị: việc".
- `ws.open`/`ws.reopen`/`ws.move` trả focus desktop về pane cũ sau khi gọi store (store tự `activate`).

### Tiêu chí: 7/7
- [x] typecheck web + shared (+ desktop) sạch; Vitest mới: `remoteSummary` (không rò field cấm, hash ổn định), `useRemotePhone` (open không đổi focus / view, close bận → needsConfirm, không set `pendingClose`), parser phone (text > 2000 bị từ chối, `lib/remoteParse.test.ts` vì shared không có test runner), `ptyClient.claim`.
- [x] Không gửi summary khi không có thiết bị (`useRemoteSummary.test.ts`).
- [x] Settings → Điều khiển từ xa: bật/tắt, listener + IP + port, relay (icon + chữ + màu), thêm điện thoại (QR + đếm ngược 5 phút), list thiết bị (thu hồi hỏi trên dòng), ngắt mọi điện thoại, giữ máy thức (mặc định bật), dòng firewall, log lệnh; en + vi; token màu; không blur; `ccm-list` / `ccm-fade` khi thêm / xoá. CHƯA NHÌN: không chạy app.
- [x] Desktop không điện thoại: toàn bộ Vitest web pass (TermPane không đổi hành vi).
- [x] (CHƯA NHÌN) ảnh Settings: không chạy dev Electron.
- [x] Không comment trong code (comment cũ giữ nguyên).
- [x] `.code_index/web.md` + `protocol.md` + `_meta.md`.

### Audit / Review / Test / Nhìn
- Audit: diff chỉ trong `apps/web/src/**`, `packages/shared/src/{remote,orreryApi}.ts`, `.code_index/`.
- Review edge: không có thiết bị → không publish, `remote.summary.get` vẫn trả; renderer reload → `remote.start()` đọc lại status, `rev` khởi từ giây epoch; pane đóng giữa lệnh → `notFound`; pane không tồn tại → `notFound`; desktop đang ở Monitor → Workspace mount ẩn, `ui.view` giữ nguyên (test); sheet đóng của desktop đang mở chứa pane đó → gỡ key khỏi `pendingClose`; file dirty → hỏi trên điện thoại, lưu trước khi đóng, lưu lỗi → `busy`.
- Test: `npm run typecheck -w @ccm/web` sạch; `-w @ccm/shared` sạch; `-w @ccm/desktop` sạch; `npm test -w @ccm/web -- --maxWorkers=3` → 245 file / 2090 pass / 0 fail.
- Nhìn: CHƯA NHÌN (không chạy app, không chụp Settings).

### Hot file cần thêm
- `apps/web/src/i18n/en.ts`: `import remoteV97 from './remote-v97'` và `...remoteV97.en,` trong object export. `apps/web/src/i18n/vi.ts`: tương tự `...remoteV97.vi,`. Đến khi thêm, chữ trên trang Settings / chip hiện key thô; test không bị ảnh hưởng (test settings tự gộp `remoteV97`).

### Ngoài phạm vi (WP-5 / WP-6)
- WP-5: map preload (9 method ở trên); parse payload gửi vào `onOrrery` đúng `{device, cmd, confirmed?}` source `phone`; gateway nên bỏ `confirmed` do điện thoại tự gửi và chỉ tự đặt sau `confirm.answer {ok:true}`; chuyển `needsConfirm` thành event `confirm.ask` (cid do gateway sinh); gọi `remote.summary.get` khi `hello` / `ws.get` và nhận `remotePublish` để diff thành `ws.patch`; đẩy `ccm:remote-changed` khi thiết bị đổi.
- WP-6 (mobile): từ vựng `ws.keys.keys` ở trên.

### Commit message đề xuất
feat(web): Orrery Remote renderer — phone source, ws.* commands, workspace summary, PTY claim and Settings page
