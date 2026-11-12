# WP-54 báo cáo: khung web app `apps/pwa`

## Đã làm
- Workspace `@ccm/pwa` (Vite 7, Vue 3, Pinia, vue-i18n 11, `base /app/`, hash trong `assets/`). Wasm: `npm run wasm -w @ccm/pwa` (`scripts/build-wasm.mjs`, cargo release + wasm-bindgen WP-50a, ra `src/gen/` gitignored; `build` / `test` / `typecheck` tự gọi khi thiếu). Chưa chạy `wasm-opt`.
- `src/remote/`: NoiseLink (`noise.ts`), WireSocket, RemoteSession, dialer + chính sách Tự động / Chỉ LAN / Chỉ relay (`dialer.ts`), MachineConnection (backoff 1..30 s, keepalive, `visibilitychange` / `pageshow` / `online` nối lại ngay, 4401 xoá sạch, 4426 dừng), PairFlow (QR v1 và v2: LAN trước rồi relay `?pt=`; token mồi `?t=` còn cho v1), `jsonPatch.ts`, `monitorReducer.ts`, `machineData.ts` (snapshot / events / ws.patch, seq lệch -> `resync`, base lệch -> `ws.get`, activity cap 500), `hub.ts`.
- `src/storage/`: IndexedDB wrapper, khoá thiết bị bọc AES-GCM bằng CryptoKey không trích xuất (`Unlocker.unlock()` là chỗ móc WebAuthn), MachineRepo, IdbSnapshotCache; forget xoá máy + snapshot, có tombstone chặn lưu muộn.
- Cập nhật: `version.json` lúc mở + mỗi lần foreground, thanh "Có bản mới"; SW (`vite/serviceWorker.ts`): precache theo build, xoá cache cũ, chỉ skip-waiting khi có message, hook `push` / `notificationclick` gọi `self.orreryPush` cho WP-56.
- UI: tab Máy / Gỡ lỗi, thẻ máy (chip đường icon + chữ + màu, đếm đang chạy / chờ / rảnh, "dữ liệu cũ", select đường), iPad >= 768 chia đôi, safe-area, theme sáng / tối theo hệ thống (token Orrery sao chép vào `styles/tokens.css`), vi + en. Gỡ lỗi: ghép tạm bằng QR text, log kết nối, seq / ws rev từng máy, xoá dữ liệu. Dev: `vite --host`, Eruda + log về dev server chỉ khi `serve`.
- E2E: `e2e/run.mjs`, `e2e/peer` (crate Rust bản sao peer spike của WP-52 + lệnh stdin `events` / `patch` / `revoke`), static server áp đúng CSP relay.
- Sửa nhỏ ngoài phạm vi: không (chỉ `package-lock.json`, `.code_index/`).

## Test thật
- `npm test -w @ccm/pwa`: 11 file, 102 pass / 0 fail (reducer, patch, dialer, đóng mã, connection với socket giả, storage fake-indexeddb, hub revoke, wasm thật, version, build dist: không inline script / style, không eruda, không eval, SW precache đủ file).
- `npm run typecheck -w @ccm/pwa` và `npm run typecheck` gốc: sạch. `npm run build -w @ccm/pwa`: ok.
- `npm run e2e -w @ccm/pwa` (một lệnh: build, cargo peer, `wrangler dev` cổng 8791, static, 2 trình duyệt, tắt sạch): WebKit 12/12 + Chromium 12/12 = 24 pass / 0 fail. Bước: khởi động + không vi phạm CSP + không cuộn ngang, SW precache, ghép bằng vé QR qua relay, IK qua relay, snapshot hiện, đổi đường (LAN-only không địa chỉ -> Ngắt, Tự động -> nối lại), events + ws.patch, ảnh, desktop mất -> dữ liệu cũ + reload đọc IndexedDB ngay, thanh bản mới + Tải lại, 4401 -> xoá UI + IndexedDB + không quay lại sau reload, không lỗi CSP/console. WebKit chạy 4 lần ổn định.
- `CCM_PWA_DIST=apps/pwa/dist node scripts/relay-bundle.mjs`: web 10 file 462 393 B; worker.js 651 121 B raw / 252 811 B gzip. File sinh ra đã khôi phục từ bản sao (md5 trùng bản trước; không dùng git checkout vì WP-52 có thể đang sửa).

## Kích thước dist
462 393 B thô / 174 292 B gzip tổng; JS 199 KB (68,9 KB gzip), wasm 237 KB (89 KB gzip), CSS 9,4 KB.

## Ảnh (chụp sau lần sửa cuối) `docs/qa/v98/WP-54/`
`machines-{phone,ipad}-{light,dark}-{webkit,chromium}.png` (390x844, 820x1180), `machines-stale-phone-light-*.png`, `revoked-phone-light-*.png`.

## Phát hiện / rủi ro
- CSP: không vi phạm trong cả 2 trình duyệt. Vue chỉ chạm CSSOM, vue-i18n 11 không dùng `new Function`. Playwright WebKit khi chụp ảnh tự chèn stylesheet nên bị CSP báo "Refused to apply a stylesheet" (mỗi ảnh 1 lần, không phải lỗi app); harness loại đúng các lần đó.
- xterm.js (WP-55): v6 chèn `<style>` bằng `document.createElement('style')` lúc chạy -> bị `style-src 'self'` chặn. Đề xuất: ưu tiên (1) bundle CSS xterm qua Vite và kiểm xem lõi chỉ dùng CSSOM / `adoptedStyleSheets` (khuyến nghị thử trước, đo trên WebKit), (2) nếu vẫn chèn `<style>` thì nới `style-src` thêm hash hoặc `'unsafe-inline'` chỉ cho style (sửa `apps/relay/src/app.ts`), hoặc (3) tự vẽ terminal bằng canvas. Chưa kiểm xterm trong WP-54.
- Baseline thanh cập nhật là hash `web` ở lần tải đầu; trang cũ do SW phát cache + server mới sẽ không báo bằng version.json, nhưng SW mới ở trạng thái waiting sẽ báo. Sau "Tải lại" không có waiting thì app gỡ SW + xoá cache rồi reload (cờ sessionStorage chặn báo giả).
- Peer E2E là bản sao `scripts/spike-v98/wasm/peer` của WP-52 (phụ thuộc `core-remote-proto` đang đổi); WP-52 đổi API proto thì `e2e/peer` phải theo. Wasm trong `src/gen` phải build lại sau khi WP-52 xong (QR v2 đã cần bản build mới).
- Chưa thử trên Safari iOS thật (LAN `wss://ip:4320` chưa kiểm; chỉ chạy với server giả), `apple-mobile-web-app-status-bar-style` đặt `default` (cần xem trên máy thật).
- `@ccm/shared` chỉ dùng trong test (đối chiếu `REMOTE_PROTO`).

## Hợp đồng cho WP-55
- Khởi động: `createRuntime()` (`src/runtime.ts`) rồi `useHubStore().start(runtime)`; giao diện chỉ đọc store `useHubStore`: `machines`, `linkOf(id)`, `viewOf(id)` (`snapshot`, `seq`, `workspace`, `wsRev`, `confirms`, `hello`, `stale`, `fromCache`, `updatedAt`), `rtt`, `notices`, `log`; hành động `pair(qr, token?, name)`, `setPolicy`, `forget`, `wipeAll`.
- Lệnh: `runtime.hub.session(id)?.request(type, fields)` (lỗi `RemoteCommandError.code`), `sendPtyInput`, `onPty` qua `MachineConnection.pty`; `ws.*` có tác dụng phụ không tự retry khi `timeout`.
- Ghép thật: thay nút gỡ lỗi bằng giao diện dùng `PairFlow` (`createPairFlow`, `pairCandidates`); nếu cần `platform` dùng `Pairing.fromQrOn` của WP-52 (thêm vào `noise.ts`).
- Khoá: thay `Unlocker` trong `createKeyStore` bằng bản WebAuthn.
- i18n: thêm khoá vào `src/i18n/{vi,en}.ts` (kiểu `Messages` ép đủ 2 ngôn ngữ). Không dùng `:style` dạng chuỗi (CSP).
