## WP-9 — DONE (kèm 3 điểm cần main quyết, mục "Ngoài phạm vi")

### Đã đổi / URL deploy / route
- Tạo `apps/relay/**`: `package.json`, `wrangler.jsonc`, `vitest.config.ts`, `tsconfig.json`, `README.md`, `.gitignore`, `src/{index,room,bounce,token,sign,envelope,bytes,ids,close,env,global.d}.ts`, `test/{helpers,token,auth,forward,bounce}.test.ts`. Không đụng file nào ngoài `apps/relay/**`, Nhật ký brief và báo cáo này.
- **URL deploy: `https://orrery-relay.jinvu.workers.dev`** (Version ID `95a69a95-55c2-4e06-99dd-5f9ca33f0f7c`). `GET /health` -> 200 `ok`. Account dùng: `Doanvu20122000@gmail.com's Account` (`80786e06fd9be997d29bc5bc4de835e6`; wrangler login có 3 account, con chọn account trùng git user, truyền qua env `CLOUDFLARE_ACCOUNT_ID`, KHÔNG ghi vào `wrangler.jsonc`). Subdomain `jinvu` đã có sẵn, không tạo gì mới, không đổi plan.
- Route: `POST /register`, `POST /revoke`, `GET /m/<machineId>?ts&sig` (WS), `GET /p/<machineId>?t=` (WS), `GET /o/<machineId>/<paneKey>`, `/.well-known/assetlinks.json` (404 nếu `ANDROID_CERT_SHA256` trống), `/.well-known/apple-app-site-association` (`APPLE_APP_ID`), `GET /health`. Chi tiết chữ ký / mã đóng / giới hạn: `apps/relay/README.md`.

### Tiêu chí: 4/4 đạt (có 2 mục con CHƯA KIỂM bằng test tự động, xem dưới)
- `npm test` pass: register TOFU + lần 2 sai chữ ký (401) + đổi pubkey (403) + stale/replay; desktop sai chữ ký / `ts` cũ / replay URL bị từ chối; phone token sai / sai machine / sai key / hết hạn / bị thu hồi -> đóng 4401; 2 phone + 1 desktop forward đúng connId hai chiều, byte không đổi (kể cả 65535 B); phone đóng -> desktop nhận `close`; desktop mới thay cũ (4409) và phone bị 4503; trang `/o/…` đúng link + intent Android + vi/en; rate limit 429 sau 30 request / phút / IP; token và envelope khớp từng byte với `vectors.json` của Rust (token phát ra == vector, verify vector OK).
- `npx tsc --noEmit` sạch.
- Deploy xong, `/health` 200.
- Không comment trong code (đã quét bằng rg); README có.
- CHƯA KIỂM bằng test tự động: (a) hibernation thật — `evictDurableObject` treo trong pool test nên bỏ test đó; (b) ping không đánh thức handler (không có spy). Bù bằng smoke test trên bản deploy: register -> desktop + phone qua wss thật -> forward hai chiều -> idle 20 s -> forward tiếp + ping/pong + `close` control vẫn đúng.

### Audit / Review / Test
- Audit: chỉ `apps/relay/**`. Root `package.json` có workspace glob `apps/*` nên `apps/relay` tự lọt workspace: cài bằng `npm install --workspaces=false --include-workspace-root=false` (không đụng root, không sinh thay đổi ở root lockfile; có `apps/relay/package-lock.json` riêng). Dùng `@cloudflare/vitest-plugin` ^1.8.0 thay `@cloudflare/vitest-pool-workers` (package cũ đã deprecated thành stub; cần vitest 4.x, `npm test` dùng vitest 4.1.12 của relay, `npx vitest` có thể lấy vitest 5 của root nên README dặn dùng `npm test`).
- Review bảo mật: chữ ký Ed25519 (WebCrypto) cho register / revoke / desktop WS, domain riêng `orrery-register/1` / `orrery-machine/1`; HMAC token kiểm bằng `crypto.subtle.verify` (constant-time), kiểm HMAC trước hạn như Rust, base64url canonical (re-encode so khớp), token ≤ 512 ký tự; replay: `ts` lệch ≤ 120 s và phải tăng hẳn so với `ts` đã dùng (register/revoke chung bộ đếm, desktop WS bộ đếm riêng) -> 409; thu hồi lưu SQLite trong DO, kiểm khi connect và đóng ngay phone đang nối; rate limit theo IP bằng Rate Limiting binding (30 / 60 s / nhóm) trước khi chạm DO; không có `console`, observability tắt, không log token / payload / IP; relay không đọc payload; DO không lộ ra ngoài (chỉ Worker gọi vào), body ≤ 8 KB, id chỉ ký tự an toàn (chặn XSS / injection vào trang bounce, thêm CSP `default-src 'none'`).
- Rủi ro còn lại: (1) theo hợp đồng chữ ký register / revoke KHÔNG phủ `relayKey` / `deviceIds`; kẻ chặn được 1 request đã ký có thể đổi thân trong cửa sổ 120 s nếu `ts` chưa bị dùng (cần phá HTTPS; khuyến nghị proto 2 ký kèm hash thân). (2) TOFU: ai đăng ký `machineId` trước giữ nó, `machineId` phải ngẫu nhiên dài. (3) `ts` tăng hẳn nghĩa là 2 lệnh cùng giây phải +1 và đồng hồ desktop lùi thì bị chặn tối đa 120 s. (4) undici `WebSocket` (Node global) không thấy sự kiện close 4401 khi phone bị đóng ngay sau upgrade (client `ws` thì nhận đúng `4401 bad token`); app Dart nên kiểm trên máy thật, nếu client không nhận được close code thì có thể đổi sang trả HTTP 401 trước upgrade.
- Test: `cd apps/relay && npm install --workspaces=false --include-workspace-root=false && npm test && npx tsc --noEmit` -> **33 pass / 0 fail** (4 file), tsc 0 lỗi. Smoke test live OK. Smoke để lại 1-2 DO `smoke-*` rỗng trên Cloudflare (không xoá được, không tốn gì đáng kể).

### Ngoài phạm vi cần main xử lý
1. **Root `package.json`**: glob `apps/*` đang nuốt `apps/relay` vào workspaces (root `npm test` / `typecheck` sẽ chạy cả relay, cần `@cloudflare/vitest-plugin` và có thể lỗi trên CI). Nên thêm `"!apps/relay"` vào `workspaces` (con không được sửa root).
2. **Chủ nhân xác nhận account**: con deploy lên account `Doanvu20122000@gmail.com`; nếu muốn account khác thì chạy lại `CLOUDFLARE_ACCOUNT_ID=<id> npx wrangler deploy` (Worker cũ trên account kia xoá bằng `wrangler delete`).
3. **Gateway WP-5/WP-6 cần biết**:
   - Body register / revoke có thêm trường `ts` (unix giây): `{machineId, pubkey, relayKey, ts, sig}` / `{machineId, deviceIds[≤64], ts, sig}`; `pubkey`, `relayKey`, `sig` đều base64url không padding (pubkey raw 32 B, sig 64 B). Chuỗi ký = UTF-8 nối thẳng, `ts` ở dạng số thập phân. Revoke ký cùng `"orrery-register/1" ‖ machineId ‖ ts` như brief ("ký như register").
   - Desktop WS: `wss://orrery-relay.jinvu.workers.dev/m/<machineId>?ts=<giây>&sig=<b64url>`, ký `"orrery-machine/1" ‖ machineId ‖ ts`; mỗi lần reconnect phải dùng `ts` mới lớn hơn lần trước (409 nếu không).
   - Desktop nhận control `[00 00]{"t":"open"|"close","conn":n}`; `conn` 1..65535 tăng dần. Mã đóng mới ngoài `docs/remote-integration.md` §6: **`4409`** (desktop bị desktop khác thay) — nên thêm vào §6; relay còn dùng `4400` (frame sai) và `4429` (quá 64 phone / room).
   - Khi desktop mất kết nối relay thì mọi phone bị đóng `4503` (session Noise gắn với socket desktop), desktop nối lại thì phone nối lại từ đầu. Desktop gửi frame có `connId = 0` bị bỏ qua; connId không có phone thì frame bị bỏ im lặng.
   - Phone: gửi binary thẳng (không envelope), text chỉ được là `"ping"`, binary ≤ 65535 B.
   - `ANDROID_CERT_SHA256` / `APPLE_APP_ID` đang trống trong `wrangler.jsonc` (assetlinks 404, AASA rỗng); điền SHA-256 cert ký thật và Team ID rồi `wrangler deploy` lại. Package Android giả định `dev.ccm.orrery.remote`, scheme `orrery-remote`.

### Commit message đề xuất
```
feat(relay): add Orrery Remote relay Worker + Durable Object

Cloudflare Worker with SQLite-backed Room DO forwarding Noise bytes between one
desktop and many phones by machineId: Ed25519 register/revoke/desktop auth with
replay protection, HMAC relay tokens matching core-remote-proto vectors, connId
envelope, hibernation-friendly WebSocket handling, per-IP rate limit, deep-link
bounce page and assetlinks/AASA endpoints. Includes vitest suite and README.
```
