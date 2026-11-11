# orrery-relay

Cloudflare Worker + Durable Object `Room` (SQLite-backed, Free plan). Relay chỉ chuyển byte giữa 1 desktop và N điện thoại theo `machineId`, không đọc payload (Noise đầu cuối). Hợp đồng: `docs/remote-integration.md` §1, §6.

Package riêng, cài cục bộ: `npm install --workspaces=false --include-workspace-root=false` (root `package.json` có glob `apps/*`; không cài kiểu này npm sẽ hoist vào root).

## Lệnh

- `npm test` — vitest + `@cloudflare/vitest-plugin` (bản kế nhiệm `@cloudflare/vitest-pool-workers`, package cũ đã deprecated). Chạy bằng `npm test`, không dùng `npx vitest` (npx có thể lấy vitest của root).
- `npm run typecheck`
- `npm run deploy` (`wrangler deploy`, cần `wrangler login`)

## Route

| Route | Auth | Việc |
|---|---|---|
| `POST /register` | Ed25519 | TOFU lần đầu lưu `pubkey` + `relayKey`; lần sau phải ký bằng key đã lưu, pubkey không đổi được |
| `GET /m/<machineId>?ts&sig` (WS) | Ed25519 | desktop; 1 desktop / room, desktop mới đóng desktop cũ bằng `4409` |
| `GET /p/<machineId>?t=<token>` (WS) | HMAC token | điện thoại; DO gán `connId` u16 |
| `POST /revoke` | Ed25519 | `{machineId, deviceIds[], ts, sig}`: ghi thu hồi, đóng phone đang nối bằng `4401` |
| `GET /o/<machineId>/<paneKey>` | không | trang bounce vi/en, nút `orrery-remote://m/<machine>/p/<pane>`; Android thêm `intent://` |
| `GET /.well-known/assetlinks.json` | không | 404 nếu `ANDROID_CERT_SHA256` trống (nhiều fingerprint cách nhau bằng dấu phẩy) |
| `GET /.well-known/apple-app-site-association` | không | `appIDs` từ `APPLE_APP_ID` (`TEAMID.bundle`), trống thì `details: []` |
| `GET /health` | không | `200 ok` |

Chữ ký (WebCrypto Ed25519, `ts` = unix giây, lệch ≤ 120 s, UTF-8 nối thẳng không phân cách):

- register / revoke: `"orrery-register/1" ‖ machineId ‖ ts`
- desktop WS: `"orrery-machine/1" ‖ machineId ‖ ts`

Chống replay: mỗi room nhớ `ts` lớn nhất đã dùng cho register/revoke (chung 1 bộ đếm) và cho desktop WS; `ts` phải lớn hơn hẳn, nếu không `409`. Hệ quả: 2 lệnh cùng giây phải tăng `ts` lên 1; đồng hồ desktop lùi thì bị chặn tối đa 120 s.

Token phone (khớp byte với `core-remote-proto/src/relay.rs`, test bằng `vectors.json`): `base64url(deviceId ‖ exp u64 BE ‖ HMAC-SHA256(relayKey, "orrery-relay/1" ‖ machineId ‖ deviceId ‖ exp u64 BE))`, `exp` là unix giây, `now >= exp` là hết hạn. So HMAC bằng `crypto.subtle.verify` (constant-time).

## Envelope và mã đóng

- Desktop: `[u16 connId BE][noise msg]`. `connId = 0` = control JSON do DO gửi: `{"t":"open","conn":n}` / `{"t":"close","conn":n}`. Desktop gửi `connId = 0` thì bị bỏ qua.
- Điện thoại: không có envelope; DO thêm / bỏ 2 byte.
- `connId` tăng dần 1..65535 rồi quay vòng, lưu trong storage nên không đổi sau hibernation, bỏ qua id đang dùng.
- Mã đóng: `4401` token sai / hết hạn / bị thu hồi · `4400` frame sai (text khác `"ping"`, phone > 65535 B, desktop < 2 B hoặc > 65537 B) · `4429` quá 64 phone / room · `4503` không có desktop · `4409` desktop bị thay (chỉ gửi cho desktop cũ).
- Desktop mất hoặc bị thay: mọi phone bị đóng `4503`, vì session Noise gắn với socket desktop. Phone vào khi chưa có desktop: đóng `4503` ngay thay vì giữ chờ, để DO không có timer và hibernate được; điện thoại tự backoff rồi nối lại.
- Keepalive: text `"ping"` -> `"pong"` bằng `setWebSocketAutoResponse`, không đánh thức DO.

## Giới hạn và lưu ý

- Rate limit theo IP ở Worker (binding `LIMITER`, 30 request / 60 s mỗi nhóm `reg`, `rev`, `m`, `p`) -> `429`. IP chỉ dùng làm key, không log.
- Không log gì (observability tắt, code không có `console`).
- Free plan: 100k request DO / ngày, message WebSocket vào tính 20:1. Gộp frame ở gateway / điện thoại, không ở relay.
- TOFU: ai đăng ký `machineId` trước thì giữ nó; `machineId` phải là chuỗi ngẫu nhiên đủ dài.
- Chữ ký register / revoke không phủ phần thân (`relayKey`, `deviceIds`) theo đúng hợp đồng; bảo vệ dựa vào HTTPS + `ts` tăng dần.
- `wrangler deploy` ghi đè `vars` (`ANDROID_CERT_SHA256`, `APPLE_APP_ID`) bằng giá trị trong `wrangler.jsonc`; điền thật vào file đó trước khi deploy bản chính thức.
- Chưa kiểm bằng test tự động: hibernation thật (`evictDurableObject` treo trong pool test), ping không đánh thức handler.
