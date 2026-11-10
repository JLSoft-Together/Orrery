# WP-v87-6-D7 — báo cáo (2026-11-10)

Port phần Bot API của `apps/desktop/src/telegram/` sang `core-desk/src/telegram/`. Phần MTProto (gửi file lớn, đăng nhập tài khoản user để đọc danh sách topic) vẫn ở TS, Rust gọi qua host. Module `telegram` nằm trong `DESK_NODE_DEFAULT` (mặc định node), bật bằng `CCM_DESK_TELEGRAM=rust`. Chưa build release, chưa chạy E2E.

## Đã làm

1. **Rust `core-desk/src/telegram/`** (27 file, file dài nhất 401 dòng), port 1:1 `notifier.ts` + `api.ts` + `inbound.ts` + `actions.ts` + `replyButtons.ts` + `host.ts` + `command.ts` + các sanitizer Telegram trong `packages/shared/src/telegram.ts`:
   - `base*.rs` `Tg`: 1 `Mutex<St>`; không giữ lock trong lúc gọi HTTP hoặc host. `Deps` có thể thay trong test: `Transport`, host, clock, sleep, local time.
   - `call` gửi Bot API qua `ureq`, body JSON hoặc multipart. Text trong multipart đổi LF thành CRLF như FormData của undici.
   - Lỗi 429: chặn đúng chat đó theo `retry_after` (1–3600 s), hàng đợi max 20 tin / chat (bỏ tin thường trước tin quan trọng), timer `flush_held`.
   - Limit: 20 tin / phút mỗi chat, 120 tin / giờ, 10 lệnh / phút mỗi chat, trả lời người lạ 5 / phút.
   - `topicHealth`, `broken`, `health` giống TS.
   - `settings.rs` / `routes.rs`: `set`, `setMyCommands` (hash sha256 16 ký tự), `pair` / `unpair`, `sync_poll` (dừng sau 3 lần 409 liên tiếp), route, session route, panes, `followSessions` / `dropEnded` / `sweepStale`, tin "đã ngắt topic".
   - `poll.rs`:
     - long poll 50 s, cách nhau min 1 s, backoff 2 s → 60 s;
     - lúc khởi động đọc backlog 5 trang, lệnh cũ chỉ đưa ra nút ▶️ / 🗑 chứ không tự chạy;
     - xử lý theo hàng: tin thường lần lượt trên 1 thread, nút bấm / file / screen chạy riêng, max 2 việc nặng cùng lúc;
     - gom album 1,5 s; `learn` cập nhật tên group / topic.
   - `commands.rs` / `runs.rs` / `inbound.rs` / `callback.rs`: mọi lệnh (`status` `new` `stop` `compact` `close` `focus` `last` `tail` `resume` `where` `run` `all` `screen` `help`, gửi file / ảnh / album) và mọi nút (23 loại + reply button `rb:`), quyền owner / member `read` / `full`, nút xác nhận hết hạn sau 5 phút, `by` / `group`.
   - `outbound.rs` / `episodes.rs` / `ticks.rs`: báo episode kèm nút; digest (sửa tin cũ); cảnh báo disk / limit; run state chụp màn hình sau 8 s; run failed kèm `run.log`; summary cuối ngày, pin mỗi topic, icon topic.
   - `discover.rs` / `report.rs`: `test`, `discover` (kết hợp tên topic đọc từ tài khoản user qua host), `create_topic`, `user`, `report` (nút, file ≤ 50 MB, file lớn hơn qua host).
   - `files.rs`: `project_diff` (git, bỏ file bí mật, max 45 MB), `save_inbound` (`.orrery/telegram`, thêm vào `info/exclude`), file slash command.
   - `shape*.rs`: `telegram.json` cùng format và cùng điều kiện đọc như TS. TS và Rust đổi chế độ qua lại không mất dữ liệu.
   - Token / apiHash / session chỉ seal và unseal qua host `keychain.*`, không trả về trang, không ghi log.
2. **Kênh**:
   - 15 kênh `ccm:telegram-*`: Rust tự sanitize như shared. Vỏ Tauri gọi thẳng các kênh này.
   - Kênh nội bộ cho vỏ: `desk:tg-state|set|route-for|report|handle|alert|run-state|run-failed|tick|stop|upload-progress`.
   - Event `ccm:telegram-event` (thêm vào `DESK_EVENTS`).
   - `CCM_TELEGRAM_URL_OVERRIDE` chỉ nhận loopback (dùng lại `phone::send::e2e_override`).
3. **TS**:
   - `telegram/facade.ts` `TelegramFacade` thay `TelegramNotifier` trong `main.ts`. Ở rust mode: episode, alert, run, tick (60 s), MCP `telegram_send|notify|receive` đều đi `desk:tg-*`. `TelegramNotifier` giữ ở `.node`, không làm gì (không poll), chỉ chạy khi rơi về node.
   - `telegram/deskHosts.ts` `telegramStandingHosts`.
   - `telegram/ipc.ts` thêm desk binding.
   - `mcpDesk.ts`: `TelegramApi` + `await`.
   - `desk/forward.ts`: module `telegram`.
   - `main.ts`: hàm `confirmCommand` dùng chung cho TS và host.
4. **Cargo**: windows-sys thêm feature `Win32_System_Time` (giờ local theo ms trên Windows). Không thêm crate mới.

## Vì sao MTProto để TS (PARTIAL)

- Session đang lưu (`mtSession`, `userSession`) là `StringSession` của GramJS. `grammers` dùng format khác, nên sau khi đổi sang Rust chủ nhân phải đăng nhập lại tài khoản Telegram (mã SMS + mật khẩu 2 lớp). Như vậy không đạt 1:1.
- Không có cách test `grammers` nếu không gửi request tới Telegram thật. Gói này cấm việc đó.
- Hiện Rust lo phần Bot API: tin trạng thái, % tiến độ, kết quả. Upload / đăng nhập / quét topic gọi host `telegram.bigUpload` / `telegram.user`. Session mới trả về Rust để seal.
- Việc cho 6b: port `mtproto.ts` + `userScan.ts` sang `grammers` (nhập session GramJS hoặc bắt đăng nhập lại 1 lần).

## File

- Mới Rust: `apps/core/crates/core-desk/src/telegram/{mod,base,base_io,base_more,api,buttons,callback,clock,commands,discover,episodes,files,html,inbound,jsx,message,outbound,parse,poll,report,routes,runs,sanitize,settings,shape,shape_out,text,text_act,text_more,ticks,tests}.rs`.
- Sửa Rust (sửa nhỏ): `core-desk/src/lib.rs` (`pub mod telegram`, `telegram::register`), `core-desk/Cargo.toml` (feature `Win32_System_Time`).
- Mới TS: `apps/desktop/src/telegram/{facade,deskHosts}.ts`, `apps/desktop/test/{desk-telegram.twin,telegram-facade}.test.ts`.
- Sửa TS: `apps/desktop/src/{main.ts, mcpDesk.ts, desk/forward.ts, telegram/ipc.ts}`.
- Doc: `.code_index/{core,desktop,_meta}.md`, `.planning/PLAN-v87-step6-tauri.md` §4 dòng D7.
- Không đụng: file của D5 / D6 / D9, `packages/shared`, `apps/web`, `apps/shell`.

## Số test thật

- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 791 passed, 0 failed, 6 ignored.
  - Test mới của D7: 15 (8 unit trong sanitize / buttons / api / message / shape / html / files, 7 test luồng trong `telegram/tests.rs` với transport giả và host giả).
- `cargo clippy -j 4 --all-targets -- -D warnings` cả workspace: sạch.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 163 file passed + 1 skipped; 1682 test passed, 29 skipped, 0 failed.
  - Mới: `desk-telegram.twin.test.ts` 4 test, chạy qua `orrery-core desk` debug thật với server Bot API giả trên `127.0.0.1`. So sánh output IPC, toàn bộ request gửi Telegram, `telegram.json`, request gửi trang, file slash command; `twin.logs` / `fellBack` rỗng.
  - Mới: `telegram-facade.test.ts` 2 test.
  - Với `CCM_DESK_TELEGRAM=node`, twin tự skip.
- `npm run typecheck` (mọi workspace): 0 lỗi.

## E2E cần chạy (chủ nhân, sau build)

1. Build: `node apps/desktop/build.mjs`, rồi `cd apps/core && cargo build --release -j 4 -p core-bin`.
2. Bộ có Telegram: `CCM_DESK_TELEGRAM=rust CCM_E2E_PORT=4551 CCM_E2E_DBG=9551 node apps/desktop/e2e/agent-api.cjs`. Bộ này kiểm schema `telegram_send` có `buttons`, từ chối 7 nút, từ chối `buttons-file`, từ chối `not-paired`. Chạy lại 1 lần với `CCM_DESK_TELEGRAM=node`.
3. Kiểm tay bằng bot test riêng (con không gửi gì tới Telegram thật), app chạy với `CCM_DESK_TELEGRAM=rust`:
   - Settings › Telegram: nhập token, Test, Discover, Create topic.
   - Pair bằng mã 6 số; bật receive; gửi lệnh `status` / `stop` / `compact` / `close` / `focus`; gửi ảnh vào topic.
   - `telegram_send` có nút: bấm từ tài khoản owner thì có tác dụng, bấm từ tài khoản khác thì bị từ chối.
   - File lớn hơn 50 MB khi bật Large files (đi qua MTProto TS).
   - Đăng nhập tài khoản user.
4. Qua E2E thì bỏ `telegram` khỏi `DESK_NODE_DEFAULT`.

## Callback host mới (vỏ Tauri phải trả, standing)

- `keychain.available|seal|unseal`: như D2, giờ là standing host vì poll chạy nền.
- `telegram.locale` → `en` | `vi`.
- `telegram.ask {req}` → `TelegramInboundResult`. Vỏ chạy `parseInbound`, rồi gửi orrery action của trang (`TELEGRAM_INBOUND_ACTION`), rồi `parseInboundResult`. Việc gõ / submit vào pane do trang và PTY quyết định, không đổi.
- `telegram.focus` → đưa cửa sổ lên trước.
- `telegram.screenshot {serial}` → `{ok, png (path), device}` | `{ok:false, error}`.
- `telegram.runConfigs {project}` → `[{id, name}]` (đã bỏ config cần trust).
- `telegram.runStart {project, configId}` → `{ok, sessionId}` | `{ok:false, error}`.
- `telegram.runLog {sessionId}` → `string[]`.
- `telegram.confirmCommand {kind: send|receive, file}` → bool (vỏ dựng hộp thoại i18n).
- `telegram.bigUpload {req:{chatId, threadId, file, name, size, caption, progress}, creds:{apiId, apiHash, token, session}}` → `{result, session?}`. Trong lúc upload, vỏ gọi `desk:tg-upload-progress {id, fraction}`.
- `telegram.user {op: sendCode|signIn|password|topics|logout, …}` → kết quả của scanner MTProto.

## Điều kiện an ninh đã giữ (§11an) và test chứng minh

- Chỉ tài khoản owner đã pair mới gửi được vào pane:
  - người lạ không tới trang, chỉ nhận câu trả lời cố định 5 lần / phút, `ignored` tăng;
  - nút bấm từ người lạ trả "Only the paired account…".
  - Test: Rust `strangers_and_receive_off_never_reach_a_pane`; twin test 2 (người lạ gửi `rm -rf /` thì không có request tới trang, owner gửi thì có đúng 1 request `send`); twin test 3 (người lạ bấm reply button bị từ chối).
- Pair: phải đúng mã, mã hết hạn sau 5 phút, max 5 lần thử mỗi người, max 50 lần tổng.
  - Test: `pairing_needs_the_code_and_caps_tries`; twin test 2 (pair qua poll).
- Receive tắt thì không lệnh nào tới pane: `strangers_and_receive_off_never_reach_a_pane`.
- Member `read` chỉ xem; `full` không có Allow / Deny / link / backlog / `close`.
  - Test: `read_members_only_view_and_close_is_owner_only`, bảng `may_press` port nguyên.
- Reply button của `telegram_send`:
  - chỉ owner bấm được, chỉ lượt bấm đầu có tác dụng;
  - đi tới đúng pane đã gửi (`target.kind = pane`);
  - submit hay chỉ gõ vẫn do trang quyết định theo trạng thái idle (`submitted`).
  - Test: `reply_buttons_answer_once_and_only_for_the_owner`; twin test 3.
  - Không pair thì `report` có nút trả `not-paired`.
- Lệnh `/compact` / `/close` cần bấm xác nhận, hết hạn sau 5 phút, chỉ người gọi hoặc owner bấm được; `/close` chỉ owner. Twin test 4 (`st` / `cp` / `cl`, request tới trang khớp TS).
- Token không trả về trang, không nằm dạng thường trong `telegram.json` (chỉ có bản sealed), không ghi log. Test: `settings_validate_token_and_keychain`; twin test 1.
- `CCM_TELEGRAM_URL_OVERRIDE` chỉ nhận `http://127.0.0.1|localhost`.

## Khác biệt còn lại / rủi ro

- **MTProto ở TS** (xem mục trên). Tauri chưa có `bigUpload` / `user` cho tới khi port `grammers`.
- **State trong RAM**: các thứ sau mất khi desk restart:
  - `receiving`, mã pair, các nút, reply button, hàng đợi 429, digest, pin text;
  - TS cũng mất các thứ này khi app restart;
  - sau restart receive tắt (an toàn).
- Rơi về node giữa phiên: `TelegramNotifier` TS dùng state đọc lúc boot (như các module khác).
- **Cache unseal**: Rust nhớ kết quả unseal theo chuỗi sealed trong 1 process. TS giải mã mỗi lần.
- **Seal lỗi**: TS ném lỗi IPC, Rust trả `{ok:false, error:'failed'}`.
- **Thứ tự / thời điểm**: TS chạy promise trên 1 thread, Rust chạy nhiều thread.
  - Album nhiều file: Rust tải lần lượt; TS tải song song (vẫn max 2).
  - Tin "đã ngắt topic" dựng nút sau khi `set_panes` xong; TS dựng ngay lúc ngắt.
  - `learn` sau mỗi lượt poll có thể ghi tên topic `#5` đè lên tên lấy từ tài khoản user, như TS. Hai bên làm cùng logic nhưng thời điểm khác nhau. Twin test tắt receive trước khi so.
- `localeCompare` (sắp tên topic) dùng so lowercase rồi so UTF-16, gần ICU, không giống hệt.
- Request tới trang qua `parseInbound` ở vỏ trước khi gửi (chặt hơn một chút; request hợp lệ thì giống TS).
- Proxy: `ureq` chỉ đọc biến môi trường proxy (như D0 đã ghi).
- `telegram.json` thứ tự key khác TS, nội dung giống.
- Windows: `clock.rs` (`SystemTimeToTzSpecificLocalTime`) và `CREATE_NO_WINDOW` cho git viết theo `cfg`, chưa biên dịch trên Windows. macOS đã chạy test thật.

STATUS: PARTIAL
