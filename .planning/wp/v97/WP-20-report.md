## WP-20 — DONE

### Đã đổi
- Shell mới (`apps/shell/src/remote/`): `fcm_files.rs` (parse + kiểm 2 file, mã lỗi tĩnh), `fcm_jwt.rs` (JWT RS256 bằng ring), `fcm.rs` (OAuth + FCM HTTP v1 qua trait `FcmHttp`/ureq, cache token 55 phút, retry, phân loại lỗi), `push_setup.rs` (4 hàm IPC), tests `fcm_tests.rs`, `push_setup_tests.rs`.
- Viết lại `push.rs` (bỏ frame / `dead_token` / relay control; giữ token theo thiết bị, 1 pane / 30 s, 30 / giờ / thiết bị; thêm `Job`, `message`, `deliver`, task `push::run`, `config_event`), `push_tests.rs`.
- Dọn WP-14: `relay.rs` về đúng HEAD (không còn push / push.dead); `ctx.rs` (`fcm`, `push_cfg` watch), `dashboard.rs` (gửi `push.config` sau `welcome` và khi cấu hình đổi), `devices.rs` (`clear_all_push`), `config.rs` (`FirebaseConfig`, `Config.firebase`), `mod.rs` (4 kênh `ccm:remote-push-{status,setup,test,remove}`, spawn `push::run`, nạp SA từ keychain trong `ctx()`), `ccm.rs` LOCAL (+4), `apps/desktop/src/preload.ts` (`remotePush*`), `shim.js` (sinh lại bằng gen-desk-shim), `packages/shared/src/remote.ts` (`PushStatus`, `PushSetupResult`, `PushTestResult`, mã lỗi).
- Sửa test cũ vì có thêm event `push.config` ngay sau `welcome`: `session_tests.rs` (3 chỗ), `relay_tests.rs` (đếm 4 -> 6 message).
- Docs: `docs/remote-integration.md` §7.2 + §7.4, `.code_index/{shell,protocol,_meta}.md`.

### Tiêu chí 3/3
- [x] Test: parse đúng / lệch project / thiếu client package / SA hỏng / sai chỗ; JWT verify bằng public key của khoá test (và khoá khác không verify); OAuth + FCM giả; UNREGISTERED / 404 xoá token; giới hạn tần suất; không push khi có phiên; SA / token không có trong status / config.json / push.config.
- [x] `cargo test --locked -j 3 remote` pass.
- [x] Không comment; file mới ≤ 311 dòng; docs + index cập nhật.

### Audit
Diff WP-14 shell đọc hết; phần relay control bỏ. Không đụng `cf_api*`, `cf_http`, `relay_setup*.rs` (relay_setup_tests đang do agent khác tách file, không sửa), `apps/web`, `apps/mobile`, `apps/relay`.

### Review (edge, đều có test trừ ghi chú)
- File sai chỗ: google-services ở ô SA -> `bad_service_account`; SA ở ô google-services -> `bad_json`.
- Chống tiêm: `projectId` `[a-z0-9-]{4,64}`, `project_number` chỉ số, `appId`/`apiKey`/`client_email` kiểm charset; `token_uri` trong file SA bị bỏ qua (luôn oauth2.googleapis.com).
- Đổi SA cùng project: giữ token; đổi project: xoá mọi token; gỡ cấu hình: xoá options, khoá (keychain ghi `-`), token, gửi `push.config` rỗng; khởi động lại: nạp lại SA từ keychain.
- OAuth 401/400 -> `auth` không retry; 429/5xx/mất mạng retry 3 lần rồi giữ token, `last_error` quota/unavailable/offline; FCM 401 lấy token lại 1 lần; 403 `denied`; 400 khác `rejected`.
- Thiết bị thu hồi: token mất cùng bản ghi (test có sẵn). Nhiều thiết bị: mỗi thiết bị 1 Job với locale riêng; token trùng chuyển sang thiết bị mới.
- Không còn đòi relay online để push: LAN-only vẫn push được. Ghi log: không có log nào trong code mới; lỗi là mã tĩnh.
- Rủi ro đã biết: 404 từ FCM coi là token chết (đúng brief) nhưng 404 do projectId sai cũng xoá token; thông báo thử dùng `data.k=test`, không có `p`.

### Test (lệnh -> kết quả)
- `cd apps/shell && cargo test --locked -j 3 remote` -> 128 passed (gồm 13 fcm_tests, 10 push_tests, 8 push_setup_tests)
- `cargo test --locked -j 3 shim` -> 7 passed
- `npm run typecheck -w @ccm/shared -w @ccm/desktop` -> sạch
- CHƯA KIỂM: Google OAuth / FCM thật (cấm), `UreqFcm` qua HTTPS thật, chạy toàn bộ `cargo test`, `apps/mobile/rust` flow test sau khi thêm `push.config` mặc định, trang Settings (WP khác) chưa gọi IPC.

### Hot file cần thêm / Ngoài phạm vi
- `apps/core` `telegram/pane_link.rs`: truyền `reason` vào `remote.link` và gọi cả khi Telegram tắt (từ WP-14); chưa có thì `attention()` nhận reason rỗng -> kind `attention`.
- `ccm:remote-push-*` chưa có UI (WP-21). `relay_setup_tests.rs` có 3 warning unused import (agent khác đang tách file).
- `apps/mobile/rust/tests` fake gateway có thể cần chấp nhận `push.config {}` ngay sau welcome (đã gửi thật từ desktop).

### Commit message đề xuất
feat(remote): send push notifications straight from the desktop to FCM HTTP v1 with a keychain-held service account, set up from google-services.json and a service account file

## Vòng 2 (sửa theo WP-20-review.md)
- 1 + 2 (CAO / TRUNG BÌNH) đã sửa: push do chính gateway kích hoạt. Mới `remote/push_watch.rs`: task chạy trong runtime remote, giữ 1 subscription `StoreTap` (kể cả khi không có điện thoại nào nối), theo dõi `terminal.status`; `working → waiting` = `attention`, `working → idle` = `done` (`waitingFor` là text tự do nên không tách permission / question); terminal → pane bằng `pane_of` trên workspace summary, không có pane thì bỏ. `pane_link.rs` không còn gọi push (xoá `Remote::attention`) nên 1 sự kiện = 1 push. Không đụng apps/core; không còn cần patch `reason` / Telegram.
- 3: `Fcm::access_token` không giữ mutex `token` qua HTTP (lock -> kiểm cache -> nhả -> OAuth -> lock lưu; chỉ lưu nếu SA chưa bị đổi). Không single-flight cứng: 2 lần OAuth song song chấp nhận được.
- 4: `Job.device` + `push::run` bỏ job khi `is_live(device)` hoặc đã stop; `push_remove` dùng `KeyStore::delete`; SA không phải JSON -> `bad_service_account`. 404 giữ nguyên.
- IPC không đổi shape; không sửa preload.ts / shim.js ở vòng này.
- Test mới `push_watch_tests.rs` (5): bảng transition, Watch theo dõi snapshot / created / updated / removed + batch, LAN-only (relay url rỗng, không Telegram) vẫn xếp job đúng kind `attention` / `done`, trùng transition bị chặn 30 s, chạy thật qua store (không phone nào nối) -> FCM giả nhận `k:done` pane đúng, job bị bỏ khi phone đã nối.
- `cd apps/shell && cargo test --locked -j 3 remote` -> 133 passed. `cd apps/mobile/rust && cargo test -j 3` -> 15 passed (flow, gồm push.config sau hello); không phải sửa fake_gateway.
- Docs: remote-integration §7.4, `.code_index/shell.md` cập nhật. CHƯA KIỂM: status thật từ collector có đúng chuỗi `working/waiting/idle` ở mọi provider (đọc theo `TerminalStatus` trong shared); transition `idle` ngay sau `working` ở Codex.
