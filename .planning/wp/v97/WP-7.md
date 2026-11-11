## WP-7: Renderer — Workspace cho điện thoại, lệnh `'phone'`, Settings → Điều khiển từ xa, PTY claim / resized
- Mục tiêu: renderer desktop (Vue) làm đủ phần của mình cho điện thoại: tạo + gửi Workspace summary, thực thi lệnh `ws.*` từ điện thoại bằng action có sẵn (source `'phone'`), đường đóng pane riêng cho điện thoại (hỏi trên điện thoại, không sheet trên desktop), mount Workspace ẩn khi có điện thoại, xterm desktop gửi `claim` khi người dùng gõ / dán và xử lý `resized`, trang Settings "Điều khiển từ xa" + chip 📱 + log lệnh từ điện thoại.
- Spec item: PLAN-v97-mobile.md T7, bất biến 5–7; H5, H12; review #1, #3, #11, #16, #20, #26; `docs/remote-integration.md` §4.3, §4.6; khảo sát `.planning/research/v97-renderer-side.md` (đọc hết).
- Phạm vi file: `apps/web/src/**` (composable mới `composables/useRemoteSummary.ts`, `composables/useRemotePhone.ts` (thực thi lệnh), `lib/remoteSummary.ts` (pure, test được), `components/settings/SettingsRemote.vue` (+ con nhỏ nếu > 400 dòng), sửa `components/settings/index.ts` (trang mới id `remote`), `App.vue` (mount Workspace ẩn khi `remote.connected > 0`, không đổi `ui.view`), `lib/ptyClient.ts` + `components/workspace/TermPane.vue` (gửi `{t:'claim'}` khi người dùng gõ phím / dán — KHÔNG khi xterm tự trả lời query; nhận `resized` → `term.resize` hoặc giữ cỡ, chọn cách ít giật, ghi lý do), header pane chip 📱 khi điện thoại đang xem / vừa gửi lệnh), `lib/desktop.ts` (bọc kênh `ccm:remote-*`), i18n file mới `apps/web/src/i18n/remote-v97.ts`; `packages/shared/src/orreryApi.ts` + `packages/shared/src/remote.ts` (WP này được giao sở hữu 2 file này trong lượt). | Cấm đụng: `apps/web/src/i18n/{en,vi}.ts` (hot file: ghi dòng spread cần thêm vào "Hot file cần thêm"), `apps/shell/**` (WP-5 đang làm), `apps/core/**`, `apps/mobile/**`.
- Phụ thuộc: WP-0, WP-3 (message `claim` / `resized` đã có trong `packages/shared/src/pty.ts`), WP-5 (đang làm — kênh native dưới đây là hợp đồng, dùng mock trong test). | Song song với: WP-5, WP-10 | Worktree: không.
- Skill bắt buộc: `frontend-design:frontend-design` và `ui-ux-pro-max:ui-ux-pro-max` (CLAUDE.md project bắt buộc trước khi làm frontend); theo `.planning/COLOR-SYSTEM.md`, Apple HIG, token `ccm-*`, không blur, mọi chữ qua vue-i18n (en + vi), status = icon + chữ + màu, có motion khi thêm / xoá / đổi (memory chủ nhân: animate mọi thay đổi).
- Model: sonnet.

### Kênh native (hợp đồng với WP-5 / WP-6 — gọi qua `window.ccmDesktop`, thiếu thì UI hiện "chưa hỗ trợ")
- `ccm:remote-status` → `RemoteStatus` (`packages/shared/src/remote.ts`); event `ccm:remote-changed` (payload `RemoteStatus`).
- `ccm:remote-enable(on: boolean)`, `ccm:remote-keep-awake(on)`, `ccm:remote-pair-start` → `{qr: string, exp: number}` (renderer vẽ QR bằng thư viện đã có trong `apps/web/package.json`, xem `components/devices/DeviceQrPopover.vue`), `ccm:remote-pair-cancel`, `ccm:remote-revoke(deviceId)`, `ccm:remote-revoke-all`.
- Renderer → gateway: `ccm:remote-publish(summary: RemoteWorkspaceSnapshot)` (debounce `REMOTE_SUMMARY_DEBOUNCE_MS`, chỉ khi có thiết bị đang kết nối, so hash trước khi gửi); gateway hỏi lại bằng lệnh orrery `remote.summary.get` (thêm vào `ORRERY_ACTIONS`).
- Gateway → renderer: lệnh `ws.*` đi qua `ccm:orrery` có sẵn (`useOrreryApi.ts` `onOrrery`) với source `'phone'`, payload `{device, cmd: RemoteCommand}`; confirm cho điện thoại: renderer trả `{needsConfirm: {title, detail, ok, cancel}}` rồi gateway hỏi điện thoại và gửi lại `ws.close {paneKey, confirmed: true}`.

### Việc
1. `packages/shared`: thêm `'phone'` vào `OrrerySource`, hàng `OPEN_POLICY` (`submit` theo lệnh, `limit` true, `dock` vào tab project, không đổi focus desktop), action `remote.summary.get` + `ws.*` trong `ORRERY_ACTIONS` với parser kiểm shape (text ≤ `REMOTE_TEXT_MAX`).
2. `lib/remoteSummary.ts`: hàm thuần `buildSummary(tabs, panes, terminals, …) → RemoteWorkspaceSnapshot` theo khảo sát §1 (dựng từ `missionGrid` hoặc tabs + `leaves`), không đưa url / path / prefill / env; test Vitest.
3. `useRemoteSummary`: watcher riêng (không dùng chung timer `PERSIST_MS`), watch thêm status trong `terminals` store; chỉ chạy khi `remote.connected > 0`; tăng `rev`.
4. `useRemotePhone`: thực thi `ws.open` (qua `workspace.openSession` source `'phone'`, không `activate`), `ws.close` (tính busy / dirty → `needsConfirm` hoặc `closePanes` không qua `pendingClose`, không hiện sheet desktop), `ws.rename`, `ws.move`, `ws.reopen`, `ws.compact`, `ws.send` / `ws.keys` (dùng lại đường `answer` / `typeKeys` + `since` của Telegram). Mọi lệnh ghi 1 dòng activity log "📱 <thiết bị>: <việc>".
5. `App.vue`: có điện thoại kết nối → `workspaceOpened = true` (mount ẩn), không đổi `ui.view`.
6. PTY: `ptyClient` thêm `claim()`; `TermPane` gọi khi `onKey` / paste của người dùng (không phải `onData` từ query trả lời); nhận `resized`.
7. `SettingsRemote.vue` (trang "Điều khiển từ xa" — đặt tên khác trang ntfy "Điện thoại"): bật / tắt, trạng thái listener + IP + port, relay (online / lỗi), "Thêm điện thoại" → QR + đếm ngược 5 phút, list thiết bị (tên, nền tảng, lần cuối, LAN / relay, thu hồi có confirm), "Ngắt mọi điện thoại", switch "Giữ máy thức khi bật điều khiển từ xa" (mặc định bật), dòng hướng dẫn firewall.
8. Header pane: chip 📱 khi có thiết bị đang xem pane đó (từ `RemoteStatus` hoặc event) — chỉ khi có thiết bị.

### Tiêu chí nghiệm thu
- [ ] `npm run typecheck` (web + shared) sạch; Vitest mới pass: `remoteSummary` (không rò field cấm, hash ổn định), `useRemotePhone` (open không đổi focus / view, close bận → needsConfirm, không set `pendingClose`), parser `'phone'` (text > 2000 bị từ chối), `ptyClient.claim`.
- [ ] Không gửi summary khi không có thiết bị (test).
- [ ] Settings → Điều khiển từ xa hiện đủ mục trên, chữ en + vi, token màu, không blur, có motion khi thêm / xoá thiết bị.
- [ ] Desktop 1 socket không có điện thoại: hành vi TermPane không đổi (test hiện có pass).
- [ ] Ảnh trang Settings (dev Electron/Tauri không bắt buộc — nếu không chạy được app, chụp bằng Vitest + snapshot DOM không thay được; ghi CHƯA NHÌN).
- [ ] Không comment trong code.
- [ ] `.code_index/web.md` + `protocol.md` cập nhật mục mới.

### Lệnh test
`npm run typecheck -w @ccm/web && npm run typecheck -w @ccm/shared && npm test -w @ccm/web -- src/lib/remoteSummary.test.ts src/composables/useRemotePhone.test.ts src/lib/ptyClient.test.ts` (thêm file test mới vào lệnh).

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/v97/WP-7-report.md`

### Nhật ký
- 2026-11-12 WP-7: shared (source phone, ws.*, parser), lib/remoteSummary, stores/remote, useRemoteSummary, useRemotePhone, App.vue mount an, ptyClient.claim + TermPane + RemotePhoneChip, SettingsRemote + i18n remote-v97 xong; typecheck web/shared/desktop sach, vitest web 245 file / 2090 pass; CHUA NHIN (khong chay app)
