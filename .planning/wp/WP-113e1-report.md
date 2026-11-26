## WP-113e1 — PARTIAL

Lõi Rust + wrapper web xong, test xanh. PARTIAL vì phải có hot file `apps/shell/shim/preload.ts` (ngoài phạm vi) thì web mới gọi được kênh mới, và đường USB còn cần listener `127.0.0.1` (xem Ngoài phạm vi).

### Đã đổi
Rust `apps/shell/src/remote/`:
- Mới: `pair_meta.rs` (Via usb/qr/lan, PairMeta, parse opts từ web, `Devices::set_paired`), `pair_ops.rs` (`Ctx::start_pair`, `Remote::pair_start_with`, `pair_cancel` chuyển từ mod.rs), `install_tickets.rs` (Tickets: version, IP đã tải, đã dùng, arm có TTL 600s, `claim_pair`, `fetched_json`), `install_http.rs` (I/O HTTP chuyển từ install.rs + header HTML/302), `install_page.rs` (HTML vi/en + chữ, `pct`, `pair_url`), `install_pair.rs` (hook mức Ctx: sự kiện fetched, `/pair` → mã gắn IP → 302), `revoke.rs` (Hold + revoke có grace/undo/flush, các hàm revoke/rename chuyển từ mod.rs), `check.rs` (Probes RTT + rate push 30s, `remote_check`, cờ `revoking` trong status), `dashboard_ping.rs` (keepalive/pong/probe với hàng đợi ping), `dashboard_msg.rs` (hàm thuần tách từ dashboard.rs), `devices_push.rs` (setter push tách từ devices.rs).
- Sửa: `install.rs` (Route Page/Pair/ApkFile, Reply Html/Pair/Redirect, `File.first`), `install_ops.rs` (`start` trả thêm `page`, op `arm`/`disarm`), `pair.rs` (slot mang PairMeta, từ chối handshake khác IP gắn mã, lưu `paired_via`/`adb_serial_hash`, bỏ treo khi ghép lại), `ctx.rs` (PairAsk.source, Control::Held/Probe, Admit::Held, field hold/probes, `probe()`, `Host::event`, status `installFetched` + `revoking`, revoke xoá treo, DESKTOP_CAPS thêm `"checklist"` theo yêu cầu main), `session.rs` (chặn thiết bị đang treo bằng CLOSE_OFF "held", gán `phone.via`), `dashboard.rs` (Probe không ngắt vòng lặp, Held đóng không gửi event), `devices.rs` (field `paired_via`, `adb_serial_hash`), `status.rs` (`pairedVia`, `adbSerialHash`), `text.rs` (tiêu đề hộp Cho phép theo nguồn), `host.rs` (`event` → emit_app), `lan.rs` (gọi `install_pair::serve`), `mod.rs` (mod mới, kênh `ccm:remote-check`, `ccm:remote-revoke-undo`, shutdown flush revoke, giảm 491 → 463 dòng), `test_util.rs` (TestHost.events).
- Test: mới `install_page_tests.rs`, `pair_via_tests.rs`, `revoke_tests.rs`; sửa `install_tests.rs`, `pair_tests.rs`, `pair_relay_tests.rs` (field `source`), `device_info_tests.rs` (caps).

Web:
- `apps/web/src/core/lib/desktop.ts`: `RemotePairOpts`, `RemoteRevokeArg`, `remotePairStart(opts?)`, `remoteRevoke(string|{id,grace})`, `remoteRevokeUndo`, `remoteCheck`, `onRemoteInstallFetched`.
- `features/remote/lib/remote.ts`: `remotePairStart(opts?)`, `remoteRevoke(id, grace?)` (max 30), `remoteRevokeUndo`, `remoteCheck`, `parseFetched`, `onInstallFetched(handler, since)` (dùng event riêng nếu shim có, không thì đọc `status.installFetched` qua `onRemoteChanged`).
- `features/remote/lib/remoteInstall.ts`: `InstallTicket.page?`, `installArm(on)`. Test: mới `remote.test.ts`, sửa `remoteInstall.test.ts`.

### API (cho e2 / f)
- `ccm:remote-pair-start` arg `{via:"usb"|"qr", extraLan?:["127.0.0.1"], adbSerialHash?}` → `{qr, exp, relay, lan, via}`. `via:"lan"` từ web bị bỏ qua (chỉ đường `/pair` tạo được). extraLan chỉ nhận IPv4 loopback, max 2, chỉ khi usb. Hash 16 hex, lưu chữ thường.
- `ccm:remote-install` op `arm`/`disarm` → `{ok, armed}`; arm tự hết sau 600s, `stop` / đổi APK cũng tắt arm.
- Sự kiện `ccm:remote-install-fetched` `{ip}` + field `installFetched: {ip, at}` trong status (`at` giây unix).
- `ccm:remote-check` arg `deviceId` hoặc `{deviceId}` → `{lan:{ok, ms?}, relay:{state, ms?}, push:{ok, error?}}`; push error `rate` (lần 2 trong 30s), `none` (chưa cấu hình / máy chưa có token), còn lại giữ mã gốc. Chờ ping max 3s. id lỗi → `null`.
- `ccm:remote-revoke` arg `id` (như cũ) hoặc `{id, grace}` (giây, max 30) → bool; `ccm:remote-revoke-undo` arg `id` hoặc `{id}` → bool. Device trong status có `revoking: true` khi đang treo.
- Device json thêm `pairedVia` (`usb`/`qr`/`lan`), `adbSerialHash`.
- Khớp `connectApi.ts` của e2: tên + kiểu khớp. Hai điểm lệch: (1) `onRemoteInstallFetched` cần shim; nên đổi sang `onInstallFetched` trong `remote.ts` (chạy được cả khi chưa có shim, truyền `since` = lúc mở ticket). (2) e2 đang tự cắt `orrery.apk` để ra trang, giờ có sẵn `ticket.page`.

### Tiêu chí
- [x] HTML trang tải: header đúng (no-store, no-referrer, CSP `default-src 'none'; style-src 'unsafe-inline'` + frame-ancestors/base-uri/form-action, nosniff, DENY), vi/en theo Accept-Language, không chứa ticket nào (link tương đối `orrery.apk`, `pair`), không JS.
- [x] `/pair`: IP khác → 403 (`install_ip`), lần 2 → 403 (`install_used`), chưa arm → 403 (`install_unarmed`), chưa tải → 403; không thay slot đang chạy (QR cũ vẫn ghép được sau các lần 403); handshake từ IP khác IP gắn mã → 4401 + log `pair_ip`.
- [x] pair qua usb lưu `adb_serial_hash`, `paired_via` đúng (usb/qr/lan), ghi file devices.
- [x] remote-check: RTT đo từ frame Ping 2 chiều (LAN cũng đo được nhờ probe), push rate 30s.
- [x] revoke grace: treo đóng link (CLOSE_OFF, không gửi `device.revoked` để app không tự xoá ghép đôi), chặn handshake; shutdown trong grace vẫn thu hồi; undo khôi phục và huỷ timer; undo quá hạn → false.
- [~] Test cũ remote xanh; vitest remote lib xanh; `npm run typecheck` còn 2 lỗi ở file của e2 (xem Test).
- [x] Không comment. File mới ≤ 192 dòng; `ctx.rs` 398, `mod.rs` 463 (vi phạm cũ, đã giảm 28 dòng).

### Audit
- Diff chỉ trong phạm vi: `apps/shell/src/remote/**`, `desktop.ts`, `remote*.ts`. Không đụng viewer*, phone_ws, danger*, Vue, i18n, mobile, core-desk, core-remote-proto.
- Tách để giữ file nhỏ: devices.rs 408 → 298, dashboard.rs 419 → 346.

### Review (edge case)
- Token hết hạn: `hit`/`claim_pair` retain theo exp → 404/403. Token so hằng thời gian, quét hết slot.
- Dùng lại: `used` one-shot; lỗi claim không đốt ticket (máy thật bấm lại được sau khi arm).
- IP khác: so IP đã GET APK lần đầu (chuẩn hoá `::ffff:`), HEAD không tính là tải.
- Huỷ giữa chừng: `disarm` / `stop` / đổi APK → 403; `pair_cancel` xoá slot.
- Revoke rồi undo quá hạn: `Hold::undo` so deadline → false, timer vẫn thu hồi; ghép lại trong lúc treo thì bỏ treo.
- Mất mạng: check không có link → trả ngay `lan.ok=false`; link chết giữa chừng → max 3s. Ping chưa trả lời quá 10s coi là mất; hàng đợi max 3 ping để không có vòng ping qua lại vô hạn với app cũ (app echo khi không có ping chờ).
- Keepalive relay giữ hành vi cũ (test liveness xanh).
- Không log secret: log chỉ IP + mã lý do; mã ghép đôi chỉ nằm trong Location 302 trả cho đúng IP đã tải.
- Đánh đổi đã chấp nhận theo spec: mã đi qua HTTP thường trong LAN (1 lần, gắn IP, 120s, vẫn bắt Cho phép).

### Test
- `cd apps/shell; cargo test --locked -j 4 remote` → `300 passed; 0 failed; 3 ignored` (chạy lại sau khi thêm caps checklist: vẫn 300/0). 17 test mới trong 3 file mới đều ok. Cảnh báo còn lại đều ở file cũ (`relay_setup_tests.rs`, `config.rs`, `core-desk`).
- `npm test -w @ccm/web -- src/features/remote/lib` → 10 file, 104 test pass.
- `npm run typecheck` → 2 lỗi, đều ở `src/features/remote/components/connect/PhoneConnectWizard.test.ts` (31, 177) của e2: mock `InstallState` thiếu `source`, `version` (field có từ WP-113b, không do e1). Lỗi `page` thiếu trong mock ticket đã tránh bằng cách để `page?` optional.
- CHƯA KIỂM: gọi kênh mới thật trong app (cần shim), luồng USB trên máy thật (cần listener loopback), E2E (theo brief không chạy).

### Nhìn: không áp dụng

### Hot file cần thêm
1. `apps/shell/shim/preload.ts` (rồi `node scripts/gen-desk-shim.mjs`), thay / thêm cạnh các dòng remote hiện có:
   ```ts
   remotePairStart: (opts?: unknown): Promise<unknown> =>
     typeof opts === 'object' && opts !== null && JSON.stringify(opts).length <= 512 ? ipcRenderer.invoke('ccm:remote-pair-start', opts) : ipcRenderer.invoke('ccm:remote-pair-start'),
   remoteRevoke: (arg: unknown): Promise<unknown> => {
     const id = typeof arg === 'string' ? arg : typeof arg === 'object' && arg !== null ? (arg as { id?: unknown }).id : undefined
     const grace = typeof arg === 'object' && arg !== null ? (arg as { grace?: unknown }).grace : undefined
     if (typeof id !== 'string' || !id || id.length > 128) return Promise.resolve(false)
     return ipcRenderer.invoke('ccm:remote-revoke', typeof grace === 'number' && grace > 0 ? { id, grace: Math.min(30, Math.round(grace)) } : id)
   },
   remoteRevokeUndo: (deviceId: string): Promise<unknown> =>
     typeof deviceId === 'string' && deviceId.length > 0 && deviceId.length <= 128 ? ipcRenderer.invoke('ccm:remote-revoke-undo', deviceId) : Promise.resolve(false),
   remoteCheck: (deviceId: string): Promise<unknown> =>
     typeof deviceId === 'string' && deviceId.length > 0 && deviceId.length <= 128 ? ipcRenderer.invoke('ccm:remote-check', deviceId) : Promise.resolve(null),
   onRemoteInstallFetched: (handler: (event: unknown) => void): (() => void) => subscribe('ccm:remote-install-fetched', handler),
   ```
   Tuỳ chọn: thêm `"ccm:remote-check"`, `"ccm:remote-revoke-undo"` vào `LOCAL` (cfg test) trong `apps/shell/src/ccm.rs`.
2. `packages/shared/src/remote.ts`: `RemoteDevice` thêm `pairedVia?: 'usb' | 'qr' | 'lan'`, `adbSerialHash?: string`, `revoking?: boolean`; `RemotePairStart` thêm `via?: 'usb' | 'qr' | 'lan'`; `RemoteStatus` thêm `installFetched?: { ip: string; at: number } | null`.
3. i18n `remoteV98.guard.why.*` (en, vi) cho mã log mới: `pair_ip`, `held`, `install_pair`, `install_unarmed`, `install_missing`, `install_fetch`, `install_ip`, `install_used` (hiện fallback hiện mã thô).
4. `docs/remote-integration.md`: trang tải LAN `/install/<t>/`, `/pair` (arm, IP, 1 lần, 302 `orrery-remote://pair?q=`), mã đi qua HTTP LAN (lý do chấp nhận), `remote-check`, revoke grace (đóng bằng 4503 "held", không gửi `device.revoked`).
5. `.code_index`: module remote thêm các file mới ở trên; `issues.md`: `mod.rs` 463 dòng, `ctx.rs` 398 dòng, cần tách tiếp (CHANNELS + dispatch → file riêng).

### Ngoài phạm vi
- Đường USB: `adb reverse tcp:P tcp:P` nối tới `127.0.0.1:P` trên desktop, nhưng listener Remote chỉ bind IP LAN private (`lan::lan_addrs` bỏ loopback). Cần quyết định bind thêm `127.0.0.1` (ví dụ chỉ khi có slot ghép đôi `via:"usb"`) trong `rebind.rs`/`lan.rs`; chưa làm vì đổi bề mặt tấn công, cần main/chủ nhân chốt. Trang tải `/install` vẫn từ chối loopback.
- Lỗi typecheck ở `PhoneConnectWizard.test.ts` (e2).

### Commit message đề xuất
```
feat(remote): LAN install page with one-shot IP-bound pairing, pair source and adb hash, connection check and revoke with undo

- GET /install/<t>/ serves a static vi/en page (no JS, strict CSP, no-store, no-referrer)
- /install/<t>/pair mints a pairing code bound to the IP that downloaded the APK, once, only while the wizard is armed
- install.fetched event and status field when the APK is first downloaded
- pair_start takes via usb/qr, extra loopback LAN address and adb serial hash; devices keep pairedVia and adbSerialHash
- ccm:remote-check measures LAN/relay RTT with ping frames and rate limits the test push to once per 30s per device
- ccm:remote-revoke with grace holds the device at once and revokes after the grace or on shutdown; ccm:remote-revoke-undo restores it
- advertise the checklist desktop cap
```
