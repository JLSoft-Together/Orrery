## WP-19 — DONE
- Đã đổi:
  - `scripts/relay-bundle.mjs` (mới) — `wrangler deploy --dry-run --outdir` → `apps/shell/resources/relay/{worker.js,worker.json}` (hash ổn định qua 3 lần chạy, không lộ đường dẫn máy).
  - `apps/shell/src/remote/cf_http.rs` (mới, 94 dòng) — trait `Http`, `UreqHttp` (Bearer, token Zeroizing), `Multipart`.
  - `apps/shell/src/remote/cf_api.rs` (mới, 180) — verify / accounts / subdomain get+create / migration_tag / metadata + migrations / upload / enable workers.dev / delete script; lỗi là mã tĩnh.
  - `apps/shell/src/remote/relay_setup.rs` (mới, 281) — `relay_status|token|deploy|remove`, token trong keychain `orrery-cf-token`, health poll, restart gateway khi url đổi / gỡ.
  - `apps/shell/src/remote/relay_setup_tests.rs` (mới, 423) — 19 test.
  - `remote/config.rs` — bỏ `DEFAULT_RELAY`, `url` mặc định rỗng, thêm `accountId/accountName/deployedHash`.
  - `remote/mod.rs` — 4 kênh `ccm:remote-relay-*`, field `setup`, `call_with`, `dispatch_with`, `RELAY_PROGRESS`; `ccm.rs` — emit `ccm:remote-relay-progress`, LOCAL list.
  - `apps/desktop/src/preload.ts` (`remoteRelayStatus/Token/Deploy/Remove`, `onRemoteRelayProgress`), `apps/shell/src/shim.js` (regen bằng `node scripts/gen-desk-shim.mjs`), `packages/shared/src/remote.ts` (kiểu `RelaySetupStatus` …).
  - Docs: `docs/remote-integration.md` §7.3, `apps/relay/README.md`, `.code_index/{shell,protocol,_meta}.md`.
- Tiêu chí: 5/5 (test HTTP giả đủ các case; url rỗng = LAN + QR có thể tạo, config cũ giữ url; `cargo test --locked -j 3 remote` 94 pass; shared + desktop typecheck sạch; không gọi Cloudflare thật; không comment, file ≤ ~400 dòng — test file 423).
- Audit: diff chỉ trong phạm vi; không đụng `push*.rs`/`fcm*.rs`/`apps/web`/`apps/mobile`/`apps/relay/src`. `relay.rs`, `ctx.rs` là sửa dở của WP-20, không đụng.
- Review (edge case, đều có test): token sai (401) → `token_invalid`, không lưu; thiếu quyền → `token_scope`; token account-owned (verify user 400) → qua `GET /accounts`; nhiều account + id rác bị lọc; không có subdomain → tạo `orrery-<6hex>`; script đã có tag v1 → bỏ `migrations`, tag khác → `{old_tag,new_tag}`; deploy 2 lần liền idempotent (1 lần PUT subdomain), đang deploy → `busy`; DNS chưa lên → `health`, url cũ giữ nguyên; mất mạng giữa upload → `offline`, chạy lại được; token không xuất hiện trong status / lỗi / config.json / progress / header health; gỡ khi đang nối → gateway restart, `relayUrl` rỗng, vẫn chạy LAN; `deleteWorker` lỗi → giữ cấu hình.
- Test (lệnh → kết quả):
  - `cargo test --locked -j 3 relay_setup` → 19 passed
  - `cargo test --locked -j 3 remote` → 94 passed
  - `cargo test --locked -j 3 shim` → 7 passed (gồm `the_local_channels_exist_in_the_generated_shim`)
  - `npm run typecheck -w @ccm/shared` và `-w @ccm/desktop` → sạch
  - CHƯA KIỂM: `cargo test` toàn bộ (chỉ lọc `remote`/`shim`); chạy thật với Cloudflare (cấm); `UreqHttp` qua HTTPS thật (chỉ test http local).
- Docs Cloudflare đã đối chiếu: developers.cloudflare.com/api/resources/workers/subresources/scripts/methods/update/ (multipart, bindings durable_object_namespace / ratelimit / plain_text, migrations new_tag / old_tag / new_sqlite_classes), .../scripts/methods/list/ (`migration_tag`), .../scripts/subresources/subdomain/methods/create/ (`enabled`, `previews_enabled`), .../workers/subresources/subdomains/methods/get/, .../user/subresources/tokens/methods/verify/. Docs không nêu rõ lỗi "chưa có subdomain" nên code coi 404 / mã 10007 là chưa có. PUT subdomain và DELETE script (`?force=true`) dựa trên hiểu biết API, không fetch được trang riêng: CHƯA KIỂM bằng docs.
- Quyền token: Account · Workers Scripts · Edit + Account · Account Settings · Read, chọn đúng account. Link: https://dash.cloudflare.com/profile/api-tokens (Create Custom Token; Cloudflare không cho điền sẵn quyền qua URL ổn định nên chỉ link trang, hướng dẫn quyền ở UI WP-21).
- Hot file cần thêm / Ngoài phạm vi:
  - `KeyStore` không có delete: gỡ token = ghi `-` (coi như không có). Muốn xoá thật cần sửa `remote/keys.rs`.
  - `preload.ts` / `shim.js` / `shared/remote.ts` / `config.rs` có thể WP-20 cũng sửa: merge phải giữ cả hai (shim sinh lại bằng gen-desk-shim sau khi gộp preload).
  - `apps/shell/resources/` chưa trackd; commit kèm. Đổi relay code → chạy lại `node scripts/relay-bundle.mjs`.
  - Restart gateway khi url đổi sẽ ngắt điện thoại đang nối (tự nối lại).
- Commit message đề xuất:
  `feat(remote): let the desktop deploy the relay to the user's own Cloudflare account through the API, with a bundled worker, keychain token and LAN-only default`

## Vòng 2 (sửa theo WP-19-review.md)
- Medium 1: xoá Worker cần confirm native phía shell — `Remote::set_delete_confirm` (relay_setup.rs) nối `hosts::confirm` trong `host.rs install` (vi / en); chưa có confirm hoặc người dùng huỷ → `{ok:false,error:"cancelled"}`, không gọi DELETE. Test `a_page_request_alone_never_deletes_the_worker`.
- Medium 2: `relay_remove` giữ `busy` suốt lúc gọi API; gỡ khi đang deploy đặt cờ `cancel`, deploy kiểm sau mỗi bước (và trong vòng health) rồi dừng `cancelled`, không ghi url; `relay_token` / remove thứ hai khi bận → `busy`. Test thread thật `a_removal_during_a_deploy_cancels_it_and_the_url_never_comes_back` + `a_cancel_flag_raised_between_steps…` + `…while_busy_is_refused`.
- Medium 3: `valid_subdomain` (`[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?`) áp lên subdomain từ API và trong `worker_url`; sai → `subdomain`. Test `a_hostile_subdomain_from_the_api_never_reaches_the_url`.
- Low: restart gateway chỉ start lại nếu `enabled` còn true (test `a_gateway_the_user_turned_off…`); health hết hạn → lưu url + hash, `state ready` + `warning:"health"` (chọn phương án này, đã ghi docs §7.3), health client timeout 10 s; migrations theo `steps` với danh sách trong `worker.json.migrations`, tag lạ → lỗi `migration` (test `migration_steps_follow_the_docs_forms`); `KeyStore::delete` (OsKeyring `delete_credential`), lỗi → `keyring`, không còn nuốt; health không gửi Authorization + `max_redirects(0)` cho mọi client (test 3 trong `cf_http.rs`); accounts phân trang theo `result_info.total_pages` (max 10, test `accounts_are_read_across_pages`); deploy lỗi khi đã có url → `state ready` + `error`; `deleteWorker` với relay cũ không account → `workerDeleted:false`; `saved_token` là `Zeroizing`; test QR giải mã `QrPayload` có / không `relay`; tách test: `cf_fake.rs`, `relay_setup_tests.rs` 174, `relay_setup_tests2.rs` 267, real-client test vào `cf_http.rs` (tất cả < 400); `relay-bundle.mjs` strip jsonc (comment + trailing comma, bundle hash mới `fdcd78f9214349dd` do `worker.json` đổi shape); xoá `host.rs dispatch` chết.
- File chạm thêm ngoài danh sách (tối thiểu): `remote/keys.rs` (thêm `KeyStore::delete` mặc định + override OsKeyring), `packages/shared/src/remote.ts` (mã lỗi `cancelled` `migration`, `warning`, `workerDeleted`), docs + `.code_index/shell.md`. `mod.rs` / `ccm.rs` / `preload.ts` / `shim.js` KHÔNG sửa thêm.
- Test: `cargo test --locked -j 3 relay_setup` 29 pass (tính cả cf_http: 3 pass riêng); `shim` 7 pass; `remote` toàn bộ 124 pass / 4 fail, 4 fail đều do WP-20 (`push.config` chen vào `session_tests` ×3 và `relay_tests::the_relay_socket…` index out of bounds) — không thuộc WP-19; typecheck shared sạch. Có chạy 1 lần `cargo build` debug để soi warning (không phải build verify release).
