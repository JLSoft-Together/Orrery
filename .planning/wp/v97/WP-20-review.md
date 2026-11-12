## WP-20 review (độc lập, opus)

Phạm vi: `apps/shell/src/remote/{fcm,fcm_jwt,fcm_files,push,push_setup}.rs` + tests, chỗ nối ctx / dashboard / devices / config / mod / pane_link / phone_ws / ccm.rs, `preload.ts` `remotePush*`, `packages/shared/src/remote.ts`. Bỏ qua relay setup (WP-19).

Test: `cd apps/shell && cargo test --locked -j 3 remote` -> 128 passed (3 warning unused import ở `relay_setup_tests.rs`, của WP-19).

### Phát hiện

- `apps/shell/src/remote/pane_link.rs:58` — CAO — `attention()` chỉ được gọi khi `link_for()` trả URL, mà `build()` (dòng 23-29) đòi relay URL `https://`. WP-19 bỏ relay mặc định (`RelayConfig.url` mặc định rỗng) nên máy chỉ dùng LAN không bao giờ push, trái với báo cáo ("LAN-only vẫn push được"). — Sửa: gọi `r.attention(terminal, reason)` không phụ thuộc `url` (vẫn trả `{url}` như cũ); thêm test LAN-only (relay url rỗng, có token) -> `push_attention` xếp job.
- `apps/shell/src/remote/pane_link.rs:58` + `apps/core telegram/pane_link.rs` — TRUNG BÌNH — đường gọi `remote.link` chỉ chạy từ Telegram pane_link; nếu apps/core chưa truyền `reason` và chưa gọi khi Telegram tắt (báo cáo ghi ở "Hot file cần thêm") thì push không bao giờ chạy khi Telegram tắt, và kind luôn `attention`. — Sửa: main gộp patch apps/core trước khi nghiệm thu push; ghi rõ vào WP-21 / QA.
- `apps/shell/src/remote/fcm.rs:103-127` — TRUNG BÌNH — `access_token` giữ mutex `token` suốt lần POST OAuth (timeout 20 s). `set_account` (gọi từ IPC `push-setup` / `push-remove`) và `send` 401 (dòng 160) phải chờ tới 20 s; IPC chạy trên `blocking()` nên không treo runtime, nhưng Settings đứng tới 20 s. Cố ý thì là để tránh 2 lần OAuth song song. — Sửa (tuỳ chọn): tách mutex "đang lấy token" khỏi slot cache, hoặc ghi rõ đánh đổi này vào `.code_index/shell.md`.
- `apps/shell/src/remote/push.rs:196-207` + `push.rs:165` — THẤP — job đã xếp hàng được gửi sau khi chờ (mỗi job tối đa ~66 s: 3 × 20 s + backoff 2 + 4 s); `deliver` không kiểm lại `is_live`, nên điện thoại vừa mở lại phiên vẫn nhận push cũ. `stop` cũng chỉ được nhận sau khi job hiện tại xong. — Sửa: trong `run`, bỏ job khi `ctx.is_live(device)` (thêm `device` vào `Job`) hoặc khi đã `stop`.
- `apps/shell/src/remote/fcm.rs:168` — THẤP — 404 bị coi là token chết mà không đọc body (đúng brief; docs Firebase: `UNREGISTERED` = 404). Rủi ro chỉ còn khi project bị xoá / FCM API tắt -> xoá hết token. Chấp nhận được; có thể chỉ xoá khi `error_code == "UNREGISTERED"` hoặc `status == NOT_FOUND`.
- `apps/shell/src/remote/fcm.rs:169-171` — THẤP — 400 `INVALID_ARGUMENT` không xoá token (docs nói được xoá nếu payload hợp lệ). Payload cố định nên an toàn hơn là cứ để; chỉ ghi nhận.
- `apps/shell/src/remote/push_setup.rs:114` — THẤP — gỡ cấu hình ghi `"-"` đè vào keychain thay vì xoá mục; `KeyStore::delete` đã có (keys.rs). Khoá cũ không còn trong keychain nên không lộ. — Sửa: dùng `self.secrets.delete(SECRET)`, `saved_account()` vẫn coi `NotFound` là chưa cấu hình.
- `apps/shell/src/remote/fcm_files.rs:59` — THẤP — file SA không phải JSON trả `bad_json` (trùng mã với google-services hỏng), trang không biết ô nào sai. — Sửa: trong `service_account()` trả `bad_service_account` khi JSON lỗi.

### Đã kiểm, đạt
- SA: chỉ trong keychain (`orrery-fcm-sa`, JSON `{client_email, private_key}`); `config.json` chỉ có projectId / apiKey / appId / senderId / client_email (email không phải bí mật, không gửi đi đâu). `ServiceAccount` có `Debug` tự viết `ServiceAccount(..)`, khoá trong `Zeroizing`, DER và JWT cũng zeroize; `Fcm` không derive Debug; lỗi là mã tĩnh; không có log / panic nào mang khoá; `push_status` không trả SA hay token.
- JWT: header `{"alg":"RS256","typ":"JWT"}` (theo docs Google, `kid` không bắt buộc), claims iss / scope `firebase.messaging` / aud `https://oauth2.googleapis.com/token` / iat / exp = iat + 3600; ký `RSA_PKCS1_SHA256` bằng ring, PKCS#8 từ PEM `BEGIN PRIVATE KEY`; `grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer`; `token_uri` trong file bị bỏ qua. FCM v1 `https://fcm.googleapis.com/v1/projects/{id}/messages:send` (projectId đã lọc `[a-z0-9-]{4,64}`). Cache 55 phút, retry 429 / 5xx / mất mạng 3 lần, 401 lấy token lại 1 lần, timeout 20 s, body tối đa 256 KB, TLS platform verifier.
- Payload: `data` chỉ `{m, k}` + `p` (pane key đã kiểm), title / body chuỗi cố định en / vi; `android.priority=high`, `notification.channel_id="attention"` khớp `MainActivity.ATTENTION_CHANNEL` + manifest.
- Token điện thoại: lưu theo thiết bị, kiểm charset 16–512; token trùng chuyển sang thiết bị mới; thu hồi / hết hạn xoá cả bản ghi; Dead -> `clear_push_token`; đổi project xoá mọi token; `device_json` (status cho trang) không có token; không gửi token tới thiết bị khác. `push.register` sau `hello` và trong rate `MAX_COMMANDS_PER_MINUTE`.
- Tần suất: 1 / pane / 30 s, 30 / giờ / thiết bị, kiểm `is_live` trước khi lấy lượt; queue max 32; FCM gọi trong `spawn_blocking`, không giữ lock `push` / `devices` / `config` qua mạng.
- `push.config`: chỉ `apiKey, appId, senderId, projectId` hoặc rỗng; gửi sau `welcome` và qua `watch` khi đổi.
- File: max 100 KB ở preload và Rust; mã lỗi `bad_json` / `bad_service_account` / `mismatch_project` / `no_android_client` / `keyring`, khớp `PushSetupError`.
- Không comment trong file mới; file dài nhất 311 dòng (`fcm_tests.rs`).

### Kết luận: CẦN SỬA
Sửa bắt buộc: lỗi CAO ở `pane_link.rs` (LAN-only không push) + test; gộp patch apps/core (`reason`, gọi khi Telegram tắt). Các mục TRUNG BÌNH / THẤP còn lại sửa luôn nếu rẻ.
