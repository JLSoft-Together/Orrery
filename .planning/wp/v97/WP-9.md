## WP-9: Relay Cloudflare Worker + Durable Object (`apps/relay`)
- Mục tiêu: relay chỉ chuyển byte giữa 1 desktop và N điện thoại theo `machineId`, đúng `docs/remote-integration.md` §1, §6 và PLAN T11; có trang bounce link `https://…/o/<machine>/<pane>` (H2, H15); test đầy đủ bằng `@cloudflare/vitest-pool-workers`; deploy lên `*.workers.dev` của chủ nhân (đã `wrangler login`).
- Spec item: PLAN-v97-mobile.md T5 (envelope relay), T11, H2, H15, review #4, #5, #15, #18.
- Phạm vi file: tạo `apps/relay/**` (package riêng, KHÔNG thêm vào npm workspaces root — cài dep trong `apps/relay` bằng `npm install` cục bộ; ghi lý do nếu cần đổi), báo cáo. | Cấm đụng: root `package.json`, `apps/core/**`, `apps/shell/**`, `apps/web/**`, `apps/mobile/**`.
- Phụ thuộc: WP-2 cho format token (đọc `apps/core/crates/core-remote-proto` khi có; trước đó theo brief WP-2 `.planning/wp/v97/WP-2.md` mục Thiết kế "relay"). | Song song được với: WP-4, WP-1b | Worktree: không.
- Model: sonnet.

### Thiết kế
- Worker `orrery-relay` (TS, Wrangler 4, `compatibility_date` mới nhất). Route:
  - `POST /register` `{machineId, pubkey (Ed25519 base64url), relayKey (32 B base64url, mã hoá? không — chỉ HTTPS), sig}` → DO lưu lần đầu (TOFU), lần sau phải ký bằng key đã lưu (đổi `relayKey` / thêm thu hồi). Ký: Ed25519 trên `"orrery-register/1" ‖ machineId ‖ ts` (WebCrypto `Ed25519`), `ts` lệch ≤ 120 s.
  - `GET /m/<machineId>` (WS upgrade): desktop; auth = query `ts`, `sig` (Ed25519 trên `"orrery-machine/1" ‖ machineId ‖ ts`). Chỉ 1 desktop / DO: desktop mới hợp lệ thay desktop cũ (đóng cũ 4409).
  - `GET /p/<machineId>?t=<token>` (WS upgrade): điện thoại; token theo WP-2 (`deviceId ‖ exp ‖ HMAC(relayKey, "orrery-relay/1" ‖ machineId ‖ deviceId ‖ exp)`), kiểm hạn + danh sách thu hồi; DO gán `connId` (u16, 1..65535, không dùng lại ngay), báo desktop `{t:"open",conn}` / `{t:"close",conn}` qua `connId = 0`.
  - `POST /revoke` (ký như register) `{deviceIds[]}` → đóng phone đó (4401), lưu thu hồi.
  - `GET /o/<machineId>/<paneKey>`: trang HTML nhỏ (vi/en theo `Accept-Language`) → nút mở `orrery-remote://m/<machineId>/p/<paneKey>`; Android thêm `intent://…#Intent;scheme=orrery-remote;package=dev.ccm.orrery.remote;end`; phục vụ `/.well-known/assetlinks.json` (SHA-256 cert ký đặt bằng env var, trống thì 404) và `/.well-known/apple-app-site-association`.
  - `GET /health`.
- DO `Room` (SQLite-backed, Free plan): WebSocket Hibernation API (`ctx.acceptWebSocket`, tags `m` / `p:<connId>`, `serializeAttachment` để nhớ connId qua hibernation), `setWebSocketAutoResponse("ping" → "pong")`. Forward: desktop → phone: đọc 2 byte connId, gửi phần còn lại cho phone đó; phone → desktop: thêm 2 byte connId. **Không** timer, không gộp, không đọc payload. Desktop vắng → phone nhận đóng 4503 hoặc giữ chờ max 30 s (chọn 1, ghi lý do).
- Rate limit theo IP ở Worker trước DO cho upgrade / register (Rate Limiting binding nếu Free plan có; không thì đếm trong DO theo IP, max 30 upgrade / phút).
- Log: không log token / payload / IP đầy đủ.

### Tiêu chí nghiệm thu
- [ ] `npm test` (vitest-pool-workers) pass: register TOFU + lần 2 sai chữ ký bị từ chối; desktop sai chữ ký / `ts` cũ bị từ chối; phone token sai / hết hạn / bị thu hồi bị từ chối; 2 phone + 1 desktop forward đúng connId hai chiều, byte không đổi; phone đóng → desktop nhận `close`; desktop mới thay desktop cũ; ping auto-response không đánh thức handler (kiểm bằng spy nếu được, không thì CHƯA KIỂM); trang `/o/…` trả HTML đúng link.
- [ ] `npx tsc --noEmit` sạch.
- [ ] Deploy `npx wrangler deploy` lên `orrery-relay.<subdomain>.workers.dev`; `GET /health` trả 200 (ghi URL vào báo cáo). Nếu tài khoản chưa có subdomain workers.dev → `wrangler` hỏi tạo: chọn tên `orrery` hoặc gần nhất; không mua gì, không đổi plan.
- [ ] Không comment trong code; README ngắn `apps/relay/README.md` (lệnh test / deploy, route, giới hạn).

### Lệnh test
`cd apps/relay && npm install && npm test && npx tsc --noEmit`

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-9-report.md`

### Nhật ký
- 01:51 WP-9: viết xong src/ (Worker + Room DO + token/sign/envelope/bounce); chuẩn bị config + test
- 01:54 WP-9: viết config + test (token/auth/forward/bounce), chạy test
- 01:58 WP-9: tsc sạch, npm test 33/33 pass (evictDurableObject treo → bỏ test hibernation, sẽ smoke test trên bản deploy)
- 02:01 WP-9: DONE. Deploy https://orrery-relay.jinvu.workers.dev, smoke test live OK, báo cáo WP-9-report.md
