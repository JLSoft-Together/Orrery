## WP-5: Gateway desktop — listener LAN, mDNS, ghép QR, thiết bị, phiên, dashboard
- Mục tiêu: trong vỏ Tauri có module `apps/shell/src/remote/` chạy được phần lõi: bật / tắt, nghe `ws://<IP LAN private>:4320/remote`, quảng bá mDNS, tạo QR ghép, ghép (XXpsk3 + xác nhận native), lưu thiết bị, phiên IK, rate limit / chống DoS, gửi `welcome` + `snapshot` + event dashboard qua `StoreTap`, nhận lệnh dashboard whitelist; nối relay (desktop `/m/<id>`) với envelope connId. Workspace / PTY / viewer / confirm để WP-6, WP-8.
- Spec item: PLAN-v97-mobile.md T1, T2, T4, T5, T10, T11, bất biến 1–4, 8; H9, H10, H13, H14; review #5, #8, #12, #16, #17; `docs/remote-integration.md` §1, §2, §4.1, §4.2, §5, §6, §7.
- Phạm vi file: tạo `apps/shell/src/remote/**` (chia file ≤ ~400 dòng: `mod.rs`, `config.rs`, `lan.rs`, `mdns.rs`, `relay.rs`, `pair.rs`, `devices.rs`, `session.rs`, `limits.rs`, `dashboard.rs`, test `*_tests.rs`), nối dây tối thiểu trong `apps/shell/src/main.rs`, `state.rs`, `ccm.rs` (kênh `ccm:remote-status`, `ccm:remote-enable`, `ccm:remote-pair-start`, `ccm:remote-pair-cancel`, `ccm:remote-revoke`, `ccm:remote-revoke-all`, `ccm:remote-keep-awake`; event `ccm:remote-changed`), `apps/shell/Cargo.toml` + `apps/shell/Cargo.lock` (WP này là chủ), `apps/shell/src/shim.js` (sinh lại bằng `node scripts/gen-desk-shim.mjs` nếu kênh mới cần — kiểm `--check`), `.code_index/shell.md` (mục mới). | Cấm đụng: `apps/core/**` (dùng crate proto + `StoreTap` có sẵn; cần đổi → "Ngoài phạm vi"), `apps/web/**`, `apps/mobile/**`, `apps/relay/**`.
- Phụ thuộc: WP-2 (proto), WP-4 (StoreTap: `core_serve::run_with_tap`, `Serve::tap()`), WP-9 (relay deploy). | Song song được với: (lượt 3) WP-7 phần contract | Worktree: không.
- Model: opus (auth, crypto, mạng).

### Thiết kế
- Runtime tokio multi-thread riêng 2 worker, thread `remote`; tắt mặc định; trạng thái lưu `<data>/remote/config.json` `{enabled, port, keepAwake: true, relay: {url, registered}}`.
- Key: private Noise của máy + `relayKey` + key Ed25519 relay trong `vault.rs` (keyring OS, xem cách `vault.rs` đang dùng); `<data>/remote/devices.json` chỉ `{deviceId, name, platform, pubkey, pairedAt, lastSeen}`. `machineId` = 16 B ngẫu nhiên base64url, sinh 1 lần.
- LAN: bind từng IP IPv4 private (10/8, 172.16/12, 192.168/16) của interface đang lên, KHÔNG bind 0.0.0.0, không bind loopback (port 4317 của core không đổi). Handshake WS thủ công giống `core-http/src/ws.rs` hoặc `tokio-tungstenite` `accept_async` (feature đã có); path `/remote` khác → 404. Max 8 kết nối chưa xác thực / IP, handshake Noise max 5 s, lỗi giải mã → đóng.
- mDNS: crate `mdns-sd`, `_orrery._tcp`, TXT `{v=1, m=<machineId>, n=<tên máy>}`; chỉ khi bật.
- Ghép: `ccm:remote-pair-start` → mã 16 B mới, QR payload (`QrPayload` của crate, `host` = IP LAN đầu, `port`, `relay` URL), trả `{qr: string, exp}` (renderer vẽ QR; crate `qrcode` không cần nếu renderer tự vẽ — renderer có thư viện QR không? kiểm, không có thì trả cả PNG data URL bằng `core-desk/src/devices/qr.rs`). Mã chỉ hợp lệ 5 phút, 1 lần, max 5 lần thử sai rồi huỷ. Sau msg 3: hỏi native `hosts::confirm` "Cho phép «<tên>» điều khiển Orrery?" (vi/en như `desk_text.rs`); đồng ý → lưu thiết bị, `pair.ok {machine, name, relay?, relayToken?}` (token theo `relay::issue_token`, hạn 30 ngày, gia hạn khi dùng).
- Phiên: `session::accept` + `authorize` = deviceId có trong `devices.json` + key khớp + chưa hết hạn 30 ngày không dùng (H14); cập nhật `lastSeen`; thông báo desktop (notification) khi thiết bị lạ / lần đầu trong ngày.
- Dashboard: `hello` → `welcome {machine, name, version, minProto, theme, locale}` (theme / locale: đọc `shell-appearance.json` + locale tray đang có; palette đầy đủ do WP-7 bổ sung qua kênh renderer — tạm gửi `mode`), rồi `StoreTap::attach` → `snapshot` + `events`; lệnh whitelist §4.2 → `StoreTap::send`. Mỗi phiên 1 task; gửi theo `Chunker`, nhận theo `Reassembler`. Gộp frame ≤ 30 ms.
- Relay: khi bật + có internet: `POST /register` (TOFU) lần đầu, rồi giữ WS `/m/<id>?ts&sig` (ts tăng dần), envelope connId theo `relay::decode`; mỗi `connId` = 1 phiên Noise như LAN; mất kết nối → backoff max 60 s. Thu hồi thiết bị → `POST /revoke`.
- Đóng: `remote.off` / `machine.restarting` (khi app thoát / update) gửi trước khi đóng; mã đóng theo §6.
- Keep awake: khi bật + `keepAwake` → dùng `core-desk/src/power/awake.rs` (qua `desk::call_inside` kênh power có sẵn hoặc gọi thẳng), tắt khi remote tắt.

### Tiêu chí nghiệm thu
- [ ] `cargo test -j 3` trong `apps/shell` pass, có test: ghép đủ vòng qua WS localhost (client giả dùng crate proto) + confirm qua `CCM_E2E_CONFIRM=ok`; mã sai 5 lần → huỷ; QR hết hạn → từ chối; thiết bị lạ IK → 4401; proto cũ → 4426; `hello` nhận `welcome` + snapshot giống `/ws`; lệnh cấm (`client.presence`) → `denied`; DoS: > 8 kết nối chưa xác thực bị từ chối; relay envelope 2 connId tách phiên đúng (relay giả trong test).
- [ ] Không bind 0.0.0.0 / loopback (test liệt kê địa chỉ bind).
- [ ] Key riêng không nằm trong file JSON nào (test đọc `devices.json` / `config.json`).
- [ ] `cargo clippy -j 3 --all-targets -- -D warnings` (apps/shell) sạch; `node scripts/gen-desk-shim.mjs --check` sạch.
- [ ] Không comment trong code; `.code_index/shell.md` mục `remote/` (≤ 40 dòng).

### Lệnh test
`cd apps/shell && cargo test --locked -j 3 remote && cargo clippy --locked -j 3 --all-targets -- -D warnings` (lần đầu thêm dep thì bỏ `--locked` một lần).

- Nhìn thấy được: không (UI Settings ở WP-7).
- Báo cáo: `.planning/wp/v97/WP-5-report.md`

### Nhật ký
- WP-5: viết xong apps/shell/src/remote/* (config, keys, devices, limits, link, pair, session, dashboard, lan, mdns, relay, ctx, host, mod) + test; nối main.rs / ccm.rs; Cargo.toml thêm dep. Bước kế: cargo test / clippy.
- WP-5: thêm theo main: preload 9 method + shim sinh lại, ccm:remote-publish, status khớp RemoteStatus WP-7; cargo test remote 24/24 pass. Bước kế: chạy lại nhiều lần + clippy + báo cáo.
- WP-5 vòng 2: sửa replay IK (admit sau hello), TOCTOU, keyring NoEntry, budget pre-auth, relay generation, rate ghép / IP, mDNS `k`, relay.token, SO_EXCLUSIVEADDRUSE, rebind 30 s, tên thiết bị; đang chạy test.
- WP-5 vòng 2 xong: 31/31 remote, 225/225 shell; report mục Vòng 2.
