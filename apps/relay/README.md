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
| `GET /p/<machineId>?pt=<vé>` (WS) | vé ghép 1 lần | điện thoại đang ghép (chưa có token); vé do desktop mở bằng control `pair.open {h: sha256(vé), ttl}`, relay chỉ lưu hash; đúng vé → xoá vé, `open` gửi desktop có `pairing: true`; sai / dùng lại / hết hạn → `4401` + 1 lỗi cho IP |
| `POST /revoke` | Ed25519 | `{machineId, deviceIds[], ts, sig}`: ghi thu hồi, đóng phone đang nối bằng `4401` |
| `GET /o/<machineId>/<paneKey>` | không | trang bounce vi/en, nút `orrery-remote://m/<machine>/p/<pane>`; Android thêm `intent://` |
| `GET /.well-known/assetlinks.json` | không | 404 nếu `ANDROID_CERT_SHA256` trống (nhiều fingerprint cách nhau bằng dấu phẩy) |
| `GET /.well-known/apple-app-site-association` | không | `appIDs` từ `APPLE_APP_ID` (`TEAMID.bundle`), trống thì `details: []` |
| `GET /app/*` (+ `HEAD`) | không | web app iPhone nhúng trong worker (`/app` → `/app/`, SPA fallback, CSP + HSTS, `/app/version.json`); 404 nếu bản dựng không có `apps/pwa/dist`. Chi tiết: `docs/remote-integration.md` §7.5 |
| `GET /health` | không | `200 ok` |

Chữ ký (WebCrypto Ed25519, `ts` = unix giây, lệch ≤ 120 s, UTF-8 nối thẳng không phân cách):

- register / revoke: `"orrery-register/1" ‖ machineId ‖ ts`
- desktop WS: `"orrery-machine/1" ‖ machineId ‖ ts`

Chống replay: mỗi room nhớ `ts` lớn nhất đã dùng cho register/revoke (chung 1 bộ đếm) và cho desktop WS; `ts` phải lớn hơn hẳn, nếu không `409`. Hệ quả: 2 lệnh cùng giây phải tăng `ts` lên 1; đồng hồ desktop lùi thì bị chặn tối đa 120 s.

Token phone (khớp byte với `core-remote-proto/src/relay.rs`, test bằng `vectors.json`): `base64url(deviceId ‖ exp u64 BE ‖ HMAC-SHA256(relayKey, "orrery-relay/1" ‖ machineId ‖ deviceId ‖ exp u64 BE))`, `exp` là unix giây, `now >= exp` là hết hạn. So HMAC bằng `crypto.subtle.verify` (constant-time).

## Envelope và mã đóng

- Desktop: `[u16 connId BE][noise msg]`. `connId = 0` = control JSON do DO gửi: `{"t":"open","conn":n,"ip"?,"country"?,"pairing"?}` (`ip` / `country` lấy từ `cf-connecting-ip` / `cf-ipcountry`, qua header nội bộ `x-orrery-ip` / `x-orrery-country`) / `{"t":"close","conn":n}`. Desktop gửi `connId = 0`: `{"t":"close","conn","code"}` (code `4400` / `4401` / `4426` / `4503`, hoặc `1000` cho conn ghép), `{"t":"pair.open","h","ttl"}` (`ttl` 1–300 s, relay tính hạn theo đồng hồ của mình), `{"t":"pair.close"}`, và (WP-59) `{"t":"strike","conn","why"}`, `{"t":"block.list"}`, `{"t":"block.remove","c"|"all"}`, `{"t":"block.set","list":[{c,ttl?}]}`, `{"t":"trust.set","list"}` (≤ 4096 B); relay gửi desktop `{"t":"blocked","row"}` / `{"t":"block.list","items","total"}`; còn lại bỏ qua. Desktop nối mới → vé cũ bị xoá. Chi tiết: `docs/remote-integration.md` §1.
- Điện thoại: không có envelope; DO thêm / bỏ 2 byte.
- `connId` tăng dần 1..65535 rồi quay vòng, lưu trong storage nên không đổi sau hibernation, bỏ qua id đang dùng.
- Mã đóng (thêm WP-59: `4429 blocked` IP / dải bị chặn, `4429 too many` cả khi quá 4 socket / IP, `4400 timeout` frame đầu sau 5 s): `1000` conn ghép, desktop đóng sau `pair.ok` · `4401` token sai / hết hạn / bị thu hồi, vé ghép sai / đã dùng / hết hạn · `4400` frame sai (text khác `"ping"`, phone > 65535 B, desktop < 2 B hoặc > 65537 B) · `4429` quá 64 phone / room · `4503` không có desktop · `4409` desktop bị thay (chỉ gửi cho desktop cũ).
- Desktop mất hoặc bị thay: mọi phone bị đóng `4503`, vì session Noise gắn với socket desktop. Phone vào khi chưa có desktop: đóng `4503` ngay thay vì giữ chờ, để DO không có timer và hibernate được; điện thoại tự backoff rồi nối lại.
- Keepalive: text `"ping"` -> `"pong"` bằng `setWebSocketAutoResponse`, không đánh thức DO.

## Giới hạn và lưu ý

- Rate limit theo IP ở Worker (binding `LIMITER`, 30 request / 60 s mỗi nhóm `reg`, `rev`, `m`, `p`) -> `429`. IP chỉ dùng làm key, không log. `429` của `p` còn tính 1 lỗi cho IP (Worker gọi DO `POST /strike` nội bộ, max 1 lần / phút / IP / isolate).
- Chặn IP (WP-59, `src/{guard,guard-control,cidr}.ts`, chi tiết `docs/remote-integration.md` §5.1): 20 lỗi / 10 phút / IP (IPv6 theo /64) → chặn 1 giờ → 24 giờ → 7 ngày; bản ghi chặn + bậc lưu SQLite `guard_block` (max 1024), tin cậy + chặn tay trong `guard_rule`, bộ đếm lỗi trong RAM. Tin cậy và IP có conn desktop đã báo `{t:"good",conn}` (sau IK + hello; đang mở hoặc trong 10 phút) không bị tự chặn; token hợp lệ chỉ đi qua lệnh chặn tự động khi IP đó hoặc deviceId đó (24 giờ, RAM) đã được bảo lãnh, chặn tay áp cho mọi phone. Desktop gửi `close` mã `4429` được (reason `rate`); `strike` cho conn vừa đóng vẫn tính (nhớ 64 conn / 30 s). Max 4 socket phone / IP (trừ IP tin cậy). Không áp cho `/m/`.
- Không log gì (observability tắt, code không có `console`).
- Free plan: 100k request DO / ngày, message WebSocket vào tính 20:1. Gộp frame ở gateway / điện thoại, không ở relay.
- TOFU: ai đăng ký `machineId` trước thì giữ nó; `machineId` phải là chuỗi ngẫu nhiên đủ dài.
- Chữ ký register / revoke không phủ phần thân (`relayKey`, `deviceIds`) theo đúng hợp đồng; bảo vệ dựa vào HTTPS + `ts` tăng dần.
- `wrangler deploy` ghi đè `vars` (`ANDROID_CERT_SHA256`, `APPLE_APP_ID`) bằng giá trị trong `wrangler.jsonc`; điền thật vào file đó trước khi deploy bản chính thức.
- Chưa kiểm bằng test tự động: hibernation thật (`evictDurableObject` treo trong pool test), ping không đánh thức handler, `429` thật của `LIMITER` → `/strike`. Độ bền lệnh chặn qua restart kiểm bằng `new Room(state, env)` trên cùng storage (`test/guard.test.ts`).

## Bundle cho desktop tự dựng relay

Desktop deploy relay lên Cloudflare của người dùng qua API (không wrangler). Bundle nằm ở `apps/shell/resources/relay/{worker.js,worker.json}`, commit vào git, Rust `include_str!`. Đổi code `src/` hoặc `wrangler.jsonc` (bindings, vars, migrations, compatibility_date) → chạy lại từ root: `node scripts/relay-bundle.mjs` (chạy `wrangler deploy --dry-run --outdir`, không cần mạng) rồi commit cả 2 file; hash đổi thì bản desktop hiện "Cập nhật relay". Chi tiết API: `docs/remote-integration.md` §7.3. Web app: đặt `CCM_PWA_DIST` (mặc định `apps/pwa/dist`) rồi chạy cùng lệnh, bundle nhúng cả thư mục (§7.5).
