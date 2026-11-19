# WP-63 report: snapshot > 1 MiB, web LAN `wss://`, desktop relay backoff, update bar WebKit

## Lỗi 1: snapshot > 1 MiB kẹt "Chưa có dữ liệu"
- Nguyên nhân: `dashboard.rs Phone::snapshot` → `Secure::json` → `Secure::frame` → `Chunker::new` trả `TooLarge` khi payload > `MAX_MESSAGE` (1 MiB) → `frame` trả `false` → `hello` trả `false` → `Phone::start` thoát, `session::serve` kết thúc **không đóng socket**. Điện thoại vẫn nối, không ai đọc lệnh nữa (`ws.get`, `push.register` hết giờ), không có lỗi nào. Phía nhận không thấy gì vì desktop không bao giờ gửi khúc > 1 MiB. Mọi `Secure::json` khác (reply / event lớn) cũng có cùng đường chết im lặng.
- Sửa (chọn **cắt bớt**, không chia phần / nén): `apps/shell/src/remote/fit.rs` `fit_snapshot` cắt `data` về ≤ 900 000 B theo thứ tự, dừng khi vừa: activity giữ 100 mới nhất → bỏ sub-agent đã xong cũ nhất trước → bỏ hết activity → cắt chuỗi > 1000 ký tự → bỏ sub-agent còn lại. Agent `main` và terminal không bao giờ bị bỏ. Event thêm field tuỳ chọn `trimmed {activity, agents, text}` + log stderr. Không vừa được → event `limit {what:"snapshot", bytes, max}`. `link.rs Secure::json`: message nào > 1 MiB → reply `{re, ok:false, error:"limit"}` hoặc event `limit`, kèm log; `dashboard.rs forward`: batch `events` quá lớn → gửi lại snapshot đã cắt.
- Lý do: không đổi hợp đồng (app Android / web đã cài vẫn nhận snapshot thường, field lạ bị bỏ qua); điện thoại chỉ cần trạng thái hiện tại; chia phần hay nén đều cần app mới ở cả 2 phía.
- Web app: `remote/machineData.ts` `MachineView.trimmed` / `limit` (`parseTrim`, `parseLimit`, `console.warn`), `views/MonitorView.vue` banner `monitor-limit` (icon + chữ + Thử lại) và dòng `monitor-trimmed`; i18n vi / en `monitor.{limitTitle,limitBody,trimmed,trimmedText}`.
- Android: **có dính** (cùng desktop). Đã hết kẹt nhờ sửa desktop: nhận snapshot đã cắt như thường, phiên sống. Android chưa hiện dòng "đã lược bớt" / banner `limit` (bỏ qua event lạ) — không sửa `apps/mobile` trong gói này.

## Lỗi 2: web luôn thử `wss://<LAN>` trước
- Nguyên nhân: `dialer.planRoutes` thêm `lan` khi có `lastHost` (luôn có từ QR), `pairing.pairCandidates` cũng thử mọi LAN trước. Desktop chỉ nghe `ws://`, trang https không mở được `ws://` (mixed content) → mỗi lần nối / ghép ngoài LAN mất `lanBudgetMs` 3 s.
- Sửa: cờ tuỳ chọn `lanTls` — `Machine.lanTls` lấy từ hello reply `lanTls: true` (`connection.rememberRoute`) hoặc QR `tls: true` lúc ghép (`QrView.tls`, `pairing.ts`). Không có cờ → không thử LAN (`planRoutes`, `openLan`, `pairCandidates`). Không có field sẵn nào nên thêm mới; desktop hiện không gửi. WP-51 bật lại: desktop gửi `lanTls: true` trong `welcome` + thêm `tls` vào QR v2 (`core-remote-proto` + wasm).

## Lỗi 3: desktop nối lại relay ~60 s
- Nguyên nhân: `relay.rs run` backoff 1 → 60 s, gấp đôi mỗi lần lỗi; relay tắt vài phút là chạm 60 s.
- Sửa: `MAX_BACKOFF` 15 s, `next_backoff`, jitter `jittered(d, r)` = ngẫu nhiên trong `[d/2, d]` (getrandom) trong `wait`; `4409` giữ 60 s riêng (`REPLACED_BACKOFF`). Tối đa ~8 lần mở `/m/` / phút, dưới `LIMITER` 30 / 60 s / IP của Worker; `/m/` không bị chặn IP (WP-59).

## Lỗi 4: WebKit lần đầu hiện "Có bản mới"
- Nguyên nhân (tái hiện bằng Playwright WebKit + `vite preview`, log SW): lần cài đầu WebKit gán `navigator.serviceWorker.controller` (do `clients.claim()`) **trước** khi bắn `statechange` `installed`; `update/serviceWorker.ts` đọc `controller` lúc đó → gọi `onWaiting`. `version.json` không liên quan (preview trả HTML → bỏ qua).
- Sửa: chốt `hadController` trước `register`; chỉ báo khi trang lúc tải đã có controller (`offer = hadController && !suppress`), áp cả `registration.waiting`. Kiểm lại WebKit thật: thanh không còn (trước 1, sau 0).

## Test (lệnh thật)
- `cargo test -j 4 -p core-remote-proto` (apps/core): 48 pass (không đổi crate).
- `cargo test -j 4 --bin orrery-shell remote::` (apps/shell): 216 pass, 1 fail, 3 ignored (so 207: +8 `fit_tests`, +2 `relay_tests`). Fail `relay_setup_tests::the_first_deploy_creates_the_subdomain_and_sends_bindings_with_migrations` có sẵn, không do gói này: test còn chờ 2 binding `plain_text` nhưng WP-62 đã bỏ `APPLE_APP_ID` khỏi bundle relay (left 3 binding, right 4). Không sửa (ngoài phạm vi).
- Fail trước / pass sau: `fit_tests::a_snapshot_over_one_mebibyte_reaches_the_phone_trimmed_and_the_session_keeps_working` (store thật 700 sub-agent, snapshot > 1 MiB, phiên LAN thật): với `dashboard.rs` + `link.rs` bản HEAD → fail (không nhận được frame nào, socket hết giờ); bản sửa → pass. `update/serviceWorker.test.ts` ca WebKit: HEAD fail 1/3, sửa pass 3/3.
- `npm test -w @ccm/pwa`: 198 pass (19 file; so 189: +3 machineData, +3 serviceWorker, +2 routing, +1 pairing). `npm run typecheck -w @ccm/pwa`: sạch.
- `node e2e/run.mjs --no-peer-build` (apps/pwa): 80 pass / 0 fail. Ảnh `docs/qa/v98/*` đã `git checkout --`.
- Relay, mobile: không đụng nên không chạy.

## File
- Shell: `apps/shell/src/remote/fit.rs` (mới), `fit_tests.rs` (mới), `link.rs`, `dashboard.rs`, `relay.rs`, `relay_tests.rs`, `mod.rs`.
- PWA: `src/remote/{machineData,dialer,pairing,connection,types,noise}.ts` + test `machineData`, `routing`, `pairing`, `connection`; `src/update/serviceWorker.ts` + `serviceWorker.test.ts` (mới); `src/views/MonitorView.vue`; `src/i18n/{vi,en}.ts`.
- Docs `docs/remote-integration.md` §1, §4.1, §4.2, §7.5; `.code_index/{_meta,shell,web-pwa}.md`.

## Ảnh hưởng app cũ / rủi ro
- Android / web cũ: nhận snapshot đã cắt (đúng dạng), bỏ qua `trimmed` và event `limit`. Sub-agent đã bị cắt: `agent.updated` sau đó bị bỏ qua phía điện thoại (không có id) — chỉ là sub-agent đã xong / cũ.
- Machine đã lưu trên web app không có `lanTls` → từ giờ không bao giờ thử LAN cho tới WP-51 (đúng ý định, vì LAN chưa từng thành công từ trang https). Chính sách "Chỉ LAN" trên web giờ luôn `noRoute`.
- Ngưỡng 900 000 B là bản JSON của `data`; đuôi event (`t`, `seq`, `trimmed`) thêm < 200 B nên vẫn dưới 1 MiB.
- Jitter làm lần thử đầu sau rớt nhanh hơn (0,5–1 s); 4409 giờ 30–60 s thay vì đúng 60 s.
- Còn dở: Android chưa hiện thông báo cắt / `limit`; desktop chưa gửi `lanTls` (chờ WP-51); test `relay_setup_tests` fail có sẵn từ WP-62.

## Dọn
`vite preview` cổng 5191 đã tắt; script tái hiện nằm ở `D:/tmp/wp63` (ngoài repo). Không commit.
