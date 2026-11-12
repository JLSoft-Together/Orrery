# WP-19 review (độc lập, opus)

Phạm vi: `apps/shell/src/remote/{cf_http,cf_api,relay_setup,relay_setup_tests}.rs`, phần relay của `config.rs` / `mod.rs` / `ccm.rs`, `preload.ts` + `shim.js`, `packages/shared/src/remote.ts`, `scripts/relay-bundle.mjs`, `apps/shell/resources/relay/*`. Thay đổi WP-20 (push trong `relay.rs`, `ctx.rs`) bỏ qua.

## Phát hiện

- apps/shell/src/remote/relay_setup.rs:249-262 — vừa — `relay_remove` với `deleteWorker: true` xoá Worker trên Cloudflare ngay, không có confirm native; brief yêu cầu "confirm native" và trang không phải nguồn tin cậy (lệnh từ renderer là đủ xoá) — gọi confirm native (dialog của shell, như `remote/confirm.rs` / Extensions) trong Rust trước `cf_api::delete_script`, trả `{ok:false,error:"cancelled"}` khi người dùng huỷ.
- apps/shell/src/remote/relay_setup.rs:251 — vừa — kiểm `busy` rồi nhả lock (TOCTOU): deploy có thể bắt đầu ngay sau đó; remove xoá url + token trong khi deploy đang chạy đã đọc token, cuối deploy ghi lại `url`/`account_id` ⇒ relay "sống lại" sau khi người dùng bấm Gỡ; thêm nữa lúc remove đang gọi DELETE (tới 60 s) không đặt `busy` nên deploy song song được — giữ `busy = true` (BusyGuard) suốt `relay_remove`, trả `busy` nếu đang bận; `relay_token` cũng nên từ chối khi `busy`.
- apps/shell/src/remote/cf_api.rs:82 + 178 — vừa — `subdomain` lấy từ JSON trả về không được kiểm rồi ghép thẳng vào `relay.url` (lưu config, gửi điện thoại qua QR / `relay.token`); giá trị có `/`, `@`, `.`… tạo URL tới host khác — kiểm `^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$` (lowercase) trước khi dùng, sai ⇒ `Err("subdomain")`; tương tự kiểm lại url trong `worker_url` (https + đuôi `.workers.dev`).
- apps/shell/src/remote/relay_setup.rs:101-106 — thấp — `restart_if_running` (check `running` rồi stop + start) không khoá chung với `enable()`: người dùng tắt gateway đúng lúc deploy xong có thể bị bật lại — chạy restart dưới một mutex điều khiển chung, hoặc chỉ start lại nếu `cfg.enabled` vẫn true sau stop.
- apps/shell/src/remote/relay_setup.rs:231-246 + cf_http.rs:7 — thấp — `wait_health` hạn 60 s nhưng mỗi request có timeout 60 s ⇒ tệ nhất ~120 s; DNS workers.dev mới tạo có lúc > 60 s, khi đó Worker đã deploy nhưng không lưu url/hash (người dùng phải deploy lại) — dùng timeout ngắn (vd 10 s) cho health client, và khi hết hạn vẫn lưu `account_id`/`deployed_hash` + url với trạng thái chờ, hoặc nâng hạn lên ~120 s.
- apps/shell/src/remote/cf_api.rs:119-126 — thấp — tag cũ khác tag bundle chỉ gửi `{old_tag,new_tag}` không có `steps`/`new_sqlite_classes`: khi relay sau này thêm migration (v2 thêm class) Cloudflare sẽ nhảy tag mà không chạy bước mới — bundle lưu đủ danh sách migration, gửi `{old_tag,new_tag,steps:[các bước sau old_tag]}`; tag lạ không có trong danh sách ⇒ báo lỗi thay vì ghi đè.
- apps/shell/src/remote/relay_setup.rs:274 — thấp — "gỡ token" ghi đè `-` thay vì xoá entry và bỏ qua kết quả ghi: keychain lỗi thì token thật vẫn nằm đó mà trả `ok:true` — thêm `delete` vào `KeyStore` (`keyring::Entry::delete_credential`), báo lỗi `keyring` khi không xoá được.
- apps/shell/src/remote/relay_setup.rs:255 — thấp — `deleteWorker: true` với config cũ không có `account_id` (url jinvu) bỏ qua xoá mà vẫn trả `ok:true` — trả cờ `workerDeleted:false` hoặc lỗi `no_account` để UI không báo đã xoá Worker.
- apps/shell/src/remote/relay_setup.rs:94-98 — thấp — `saved_token` trả `String` thường (bản sao không zeroize) — giữ `Zeroizing<String>` tới `UreqHttp::new`.
- apps/shell/src/remote/cf_http.rs:52 + relay_setup.rs:236 — thấp — health check tới workers.dev vẫn gửi header `authorization: Bearer ` rỗng; client vẫn theo redirect (ureq mặc định 10; header auth không theo nhờ `RedirectAuthHeaders::Never`, đã kiểm ureq 3.5.1) — bỏ header khi token rỗng, đặt `max_redirects(0)` cho cả API và health (API Cloudflare không redirect).
- apps/shell/src/remote/cf_api.rs:68 — thấp — `/accounts?per_page=50` không phân trang: token gắn > 50 account sẽ thiếu account — đọc `result_info.total_pages` hoặc chấp nhận và ghi docs.
- apps/shell/src/remote/relay_setup.rs:108-120 — thấp — sau một lần deploy/cập nhật lỗi, `state` = `error` dù relay cũ vẫn chạy với `url` hợp lệ — trả `state: ready` + `error` riêng, hoặc docs nói rõ.
- apps/shell/src/remote/relay_setup_tests.rs:380-381 — thấp — test "QR không có relay" chỉ kiểm QR tồn tại, không giải mã kiểm thiếu trường relay (code `pair.rs:123` đúng) — giải mã payload QR và assert không có `relay`.
- apps/shell/src/remote/relay_setup_tests.rs (423 dòng) — thấp — vượt ~400 dòng — tách test HTTP thật (`the_real_client_…`) sang `cf_http_tests.rs`.
- scripts/relay-bundle.mjs:12 — thấp — `JSON.parse` trên `wrangler.jsonc`: thêm comment / dấu phẩy cuối vào file đó sẽ làm script hỏng — bỏ comment/trailing comma trước khi parse (hoặc dùng `jsonc-parser` có sẵn trong wrangler).

## Đã kiểm, không có vấn đề

- Token: chỉ ở keychain (`OsKeyring`, `orrery-cf-token` — cùng keychain các khoá remote khác, không qua `vault.rs`, chấp nhận được), không vào config/status/progress/lỗi (lỗi là mã `&'static str`; lỗi ureq bị nuốt thành `offline`); `relay_token` trả về chỉ danh sách account; test `the_token_never_shows_up…` phủ.
- Đầu vào: token lọc `[A-Za-z0-9_-]{20,256}`, accountId 32 hex trước khi vào path; tên account lọc ký tự điều khiển, cắt 80.
- HTTP: TLS PlatformVerifier, timeout 60 s, body max 4 MB, base cố định `api.cloudflare.com`; không giữ mutex nào qua HTTP (`env` chỉ khoá lúc tạo client, `ctx.config` chỉ khoá lúc ghi cuối).
- Cloudflare API (docs developers.cloudflare.com): PUT `/accounts/{id}/workers/subdomain {subdomain}` (quyền Workers Scripts Write), PUT script multipart `metadata` + module `application/javascript+module`, binding `durable_object_namespace{name,class_name}`, `ratelimit{name,namespace_id,simple{limit,period}}`, `plain_text`, migrations `new_tag`+`new_sqlite_classes` lần đầu và bỏ khi tag trùng, POST `…/scripts/{name}/subdomain {enabled,previews_enabled}`, DELETE `?force=true`, fallback `/accounts` cho token account-owned. Quyền tối thiểu trong docs (Workers Scripts Edit + Account Settings Read) khớp.
- Config cũ: `url` jinvu giữ nguyên, `registered` giữ; thiếu `url` ⇒ rỗng; `start()` không chạy `relay::run` khi url rỗng; `pair.rs:123` bỏ relay khỏi QR; đổi url ⇒ `registered=false` + restart; điện thoại cũ nhận url mới qua `relay.token` khi nối LAN.
- Bundle: `worker.js` chứa mọi module `apps/relay/src` (chỉ `issueToken`/`expBytes` bị tree-shake, chỉ test dùng), route `/health` khớp `index.ts:55`; bindings/vars/migration khớp `wrangler.jsonc`; hash = sha256(worker chuẩn hoá CRLF, bỏ sourcemap + meta) ổn định.
- Không comment trong code mới; file nguồn ≤ 400 dòng (trừ file test 423).
- IPC: 4 kênh có trong `CHANNELS` + `LOCAL`; preload/shim kiểm kiểu + độ dài; progress `ccm:remote-relay-progress` khớp shim.

## Test

`cargo test --locked -j 3 remote`: 94 passed, 0 failed (gồm 19 test `relay_setup_tests`). Cảnh báo mới do WP-19: `remote/host.rs:116 pub fn dispatch` không còn ai gọi (ccm.rs chuyển sang `dispatch_with`) — thấp — xoá hàm hoặc cho `dispatch` gọi `dispatch_with(..., &|_| {})`.

## Kết luận

CẦN SỬA — 3 mục mức vừa (confirm native khi xoá Worker, TOCTOU busy ở remove, kiểm subdomain trước khi ghép url). Phần còn lại mức thấp, sửa được cùng lượt.
