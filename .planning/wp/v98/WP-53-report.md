# WP-53 báo cáo: relay phục vụ `/app/*`

## Đóng gói chọn: (A) nhúng vào `worker.js`
- Giới hạn hiện hành (developers.cloudflare.com/workers/platform/limits, kiểm 2026-11-12): Free và Paid đều 64 MiB không nén, KHÔNG có giới hạn gzip. Ngưỡng an toàn đặt 80% = 51,2 MiB raw, `relay-bundle.mjs` fail nếu vượt.
- Đo: worker không web 28 848 B raw / 8 260 B gzip. Với dist mẫu 1,5 MB (ngẫu nhiên, không nén được): worker.js 2 029 925 B raw / 1 516 888 B gzip (base64 +33%, + gzip). PWA ước 1–1,5 MiB nên ~2 MiB raw, xa ngưỡng. Không đụng Rust (luồng deploy một part `worker.js` giữ nguyên). Hash đổi theo nội dung web (không web `3fe27e2c2e285714`, có web mẫu `2e322a6497059ce7`).
- `web-assets.generated.ts` commit ở dạng stub (`WEB = null`); `relay-bundle.mjs` ghi nội dung thật tạm thời, chạy wrangler dry-run, rồi luôn trả stub (finally). Nội dung web chỉ nằm trong `apps/shell/resources/relay/worker.js`. Bản hiện commit trong resources là bản KHÔNG có web (chưa có `apps/pwa/dist`).
- Nén: worker không tự nén, để edge Cloudflare nén theo `accept-encoding`.

## File
- Mới: `apps/relay/src/app.ts` (`serveApp`, `appHeaders`, `LAN_SOURCES`, `PROTO`, `MIN_PROTO`), `src/web-bundle.ts` (kiểu), `src/web-assets.generated.ts` (stub), `test/app.test.ts`, `scripts/relay-web.mjs` (`buildWebModule`, `STUB`).
- Sửa: `apps/relay/src/index.ts` (route `/app`, `/app/*` đặt trước các route khác), `scripts/relay-bundle.mjs` (`CCM_PWA_DIST`, in kích thước, kiểm ngưỡng), `apps/shell/resources/relay/worker.{js,json}` (sinh lại), `docs/remote-integration.md` (§7.5, đặt trước §7.4; §7.1/7.2 không đụng), `apps/relay/README.md`, `.code_index/_meta.md`, `WP-53.md` (nhật ký).
- Không đụng Rust, `room.ts`, `token.ts`, `apps/web`, `apps/mobile`, `apps/core`.

## Test thật
- `cd apps/relay && npm test`: 5 file, 45 pass / 0 fail (34 cũ + 11 mới trong app.test.ts). `npm run typecheck`: sạch.
- `node scripts/relay-bundle.mjs` chạy có / không dist; dist có file đuôi lạ hoặc thiếu index.html thì lỗi rõ.
- `wrangler dev --port 8799` (đã tắt) + curl -I:
  - `/app/`: 200, `text/html; charset=utf-8`, `Cache-Control: no-cache`, ETag, HSTS `max-age=31536000`, CSP đầy đủ (`connect-src 'self' wss://127.0.0.1:8799 wss://*:4320 https://*:4320`), `permissions-policy: camera=(self)`, `referrer-policy: no-referrer`, `x-content-type-options: nosniff`.
  - `/app/assets/core_bg-Ab12Cd34.wasm`: 200, `Content-Type: application/wasm`, `Cache-Control: public, max-age=31536000, immutable`, cùng CSP / HSTS.
  - `/app/sw.js`: 200, `text/javascript; charset=utf-8`, `service-worker-allowed: /app/`, cùng CSP / HSTS.
  - `/app/version.json`: `{"web":"c0b0714b32eec3d8","proto":1,"minProto":1}`, `no-store`.
  - `/app/sessions/1` (SPA fallback): 200.
- Test vitest phủ: content-type, CSP / HSTS / các header, redirect, SPA fallback, 404 khi không bundle, path traversal (`%2e%2e`, `..%2f`, `%5c`, NUL, `//`, `__proto__`, `hasOwnProperty`), ETag 304, HEAD, 405, version.json, định tuyến qua worker.

## Việc dở / rủi ro
- Chưa có `apps/pwa`; tới khi WP-54 xong, `/app/` trả 404 (đúng thiết kế). Kiểm thật với dist PWA thật khi có.
- Wildcard `wss://*:4320 https://*:4320` trong CSP: WebKit chưa biết có nhận host IP / wildcard port không (chờ iPhone). Đổi ở hằng `LAN_SOURCES`. Ngoài ra `https://*:4320` chỉ hợp khi LAN là TLS (spike WP-50bc).
- `PROTO` / `MIN_PROTO` ở `app.ts` là bản sao tay của `core-remote-proto` (hiện 1/1); phải đổi cùng nhau.
- Mỗi lần web đổi, worker.js trong resources đổi (vài MiB trong git); chấp nhận theo thiết kế.
- `%2e` đơn lẻ trong đường dẫn bị URL chuẩn hoá thành `.` nên `/app/%2e/sw.js` = `/app/sw.js` (hợp lệ, không thoát).
- Mục §7.5 nằm trước §7.4 trong docs (chèn sau §7.3); đổi số nếu muốn.
- Spike Node cổng 4320–4322 vẫn chạy (không đụng).

## Hợp đồng cho WP-54
- Base path `/app/` (Vite `base: '/app/'`), build vào `apps/pwa/dist` (hoặc `CCM_PWA_DIST`), bắt buộc có `index.html`. File hash để dưới `assets/<tên>-<hash≥8 ký tự>.<ext>` thì được cache immutable; `index.html`, `sw.js`, `manifest.webmanifest` ở gốc dist (no-cache).
- Đuôi cho phép: html js mjs css json webmanifest wasm png jpg jpeg svg ico webp woff2 woff ttf txt (map bị bỏ qua); đuôi khác làm build relay fail (thêm vào `TYPES` trong `scripts/relay-web.mjs`).
- Service worker: `/app/sw.js`, scope `/app/`. Điều hướng không có đuôi file tới `/app/...` luôn ra `index.html`.
- `GET /app/version.json` → `{web, proto, minProto}`, `no-store`: so `web` với giá trị lúc tải để hiện "Có bản mới — tải lại".
- CSP phải tuân: không script / style inline (không `<script>` nội tuyến, không `style=` / `<style>`; thư viện đẩy style động bằng JS qua CSSOM thì được, `style.cssText` / attribute `style` có thể bị chặn bởi `style-src 'self'`), không font / ảnh từ ngoài (`data:` / `blob:` cho ảnh được), wasm cần `'wasm-unsafe-eval'` (đã có), kết nối chỉ tới `'self'`, `wss://<host relay>`, `wss://*:4320`, `https://*:4320`, không form submit, không iframe. Camera chỉ `camera=(self)`.
