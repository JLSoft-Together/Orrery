# WP-v87-6-D7b — báo cáo (2026-11-11)

Port nốt phần MTProto của Telegram sang Rust: gửi file lớn (thay host `telegram.bigUpload`) và tài khoản user để đọc danh sách topic (thay host `telegram.user`). Code nằm ở `apps/core/crates/core-desk/src/telegram/mt/`. Khi module `telegram` chạy rust (`CCM_DESK_TELEGRAM=rust` ở Electron, Tauri luôn rust), core-desk tự nói MTProto, không cần Node. Chưa build release, chưa chạy E2E, không kết nối Telegram thật.

## Đã làm

1. **Crate**: dùng họ `grammers` 0.12 ở tầng thấp: `grammers-mtsender` (kết nối, `SenderPool`), `grammers-session` (`MemorySession`, không bật `sqlite-storage` để không kéo libsql), `grammers-tl-types` (TL layer), `grammers-crypto` (SRP cho mật khẩu 2 lớp), `grammers-mtproto` (lỗi transport). **Không** dùng `grammers-client`:
   - `request_login_code`, `bot_sign_in`, `check_password` của client có `panic!` / `unwrap()` ở các nhánh hiếm (`SentCode::Success`, `PaymentRequired`, mật khẩu không có `current_algo`). Release build của `orrery-core` là `panic = "abort"`, panic sẽ tắt cả core.
   - Vì vậy các bước đăng nhập tự viết bằng raw TL (`auth.importBotAuthorization`, `auth.sendCode`, `auth.signIn`, `account.getPassword` + `calculate_2fa` + `auth.checkPassword`), có xử lý `*_MIGRATE_X` (đổi home DC).
   - Ít crate hơn (không `jiff`, `mime_guess`, `md5`, `html5ever`).
   - tokio thêm feature `rt-multi-thread`; 1 runtime 1 worker thread (`orrery-mtproto`), tạo lúc dùng lần đầu. Code sync gọi qua `Handle::block_on`, mỗi request có timeout 60 s.
2. **Trait để test**: `mt::Connector` / `mt::Link` (`mt/mod.rs:171`, `:186`). Bản thật là `mt::grammers::Grammers` (`mt/grammers.rs:37`). Test dùng `Mock` / `MockConn`. `Deps.mt: Option<Arc<dyn Connector>>` (`base.rs:48`):
   - `Some`: đi đường Rust.
   - `None`: đi host GramJS như D7.
   - `deps_from` lấy từ `mt_connector()` (`mod.rs:68`). `CCM_TELEGRAM_MTPROTO=node` cho `None`: đường GramJS fallback ở Electron, cũng là cách twin test cũ giữ nguyên.
3. **Gửi file lớn** (`mt/upload.rs`, `mt/service.rs`):
   - Chia part 512 KB, max 4000 part (2000 MB), `upload.saveBigFilePart`, 4 worker cùng lúc.
   - Mỗi part thử max 5 lần:
     - `FLOOD_WAIT_X` ≤ 60 s thì chờ đúng X giây;
     - lỗi mạng / 5xx thì backoff 1–8 s;
     - lỗi khác thì dừng.
   - Lỗi mạng sau khi hết lượt thử: kết nối lại bằng cùng session (cùng auth key) và **chỉ gửi các part còn thiếu**, cùng `file_id` (max 2 lần kết nối lại).
   - `messages.sendMedia`:
     - `InputFileBig`, `force_file`, tên file, mime theo đuôi file;
     - caption HTML chuyển thành entity (`mt/html.rs`, offset UTF-16);
     - `reply_to` + `top_msg_id` = topic;
     - `FILE_PART_X_MISSING` thì gửi lại đúng part X;
     - gửi lại dùng cùng `random_id`, `RANDOM_ID_DUPLICATE` coi là đã gửi.
   - Bot đăng nhập bằng token (`auth.importBotAuthorization`). Session bot lưu sealed, gắn với fingerprint sha256(apiId:apiHash:token) 16 ký tự hex. Đổi token / hash / id thì session cũ tự bỏ.
   - Session bị từ chối (`SESSION_REVOKED`, `AUTH_KEY_*`, 401) thì xoá và đăng nhập bot lại 1 lần, như TS.
   - Upload chạy lần lượt (1 hàng đợi). Kết nối bot giữ 120 s sau lần gửi cuối rồi đóng.
   - Kết quả trả cho `report.rs` cùng dạng `{result:{ok|error}}` như host. Lỗi map như `classifyMtprotoError`: `auth` / `chat` / `network` / `failed`.
4. **Tài khoản user** (`mt/login.rs`, `mt/account.rs`):
   - Luồng phone → code → 2FA password giữ 1 kết nối, hết hạn sau 10 phút (`expired`). Sai code vẫn nhập lại được. `SESSION_PASSWORD_NEEDED` trả `next: 'password'`. `SignUpRequired` trả `phone`.
   - Kết quả và lỗi (`flood` / `phone` / `code` / `password` / `auth` / `network` / `failed`) khớp `TelegramUserResult`. UI `TelegramAccountScan.vue` dùng lại được, không sửa.
   - Quét topic như `userScan.ts`: `messages.getDialogs` max 200 dialog; chỉ lấy forum megagroup; `messages.getForumTopics` trang 100 với offset (topic, top message, date); bỏ topic General (1); max 300 topic / group, max 50 group.
   - Quét gặp `auth` thì bỏ session Rust.
   - Đăng xuất: `auth.logOut` (lỗi bỏ qua), xoá session Rust và cả session GramJS trong `telegram.json`, như TS.
5. **Lưu session** (`mt/store.rs`):
   - File `telegram-mtproto.json` cạnh `telegram.json` trong data dir, mode 0600, ghi atomic: `{v, bot:{key, session}, user:{apiId, session}, noticed}`.
   - Session ở đây đã qua `keychain.seal`. Dạng gốc trước khi seal là JSON riêng: home DC + auth key từng DC (`mt/blob.rs`).
   - Không có phone / code / password / token / apiHash. File rỗng thì bị xoá.
   - `mtSession` / `userSession` của GramJS trong `telegram.json` **giữ nguyên** để chuyển về node không mất gì.
6. **Đăng nhập lại 1 lần**:
   - Ở đường Rust, `state().userAccount` lấy từ session Rust (`base_io.rs:156`, `:192`). Có session GramJS mà chưa có session Rust thì `userAccount = null`: Settings hiện lại nút Sign in.
   - Trong trường hợp đó, lần đầu gọi `state()`, core-desk gọi host `telegram.notice {kind:'relogin'}` đúng 1 lần (`mt/account.rs:27`). Cờ `noticed` lưu trong file nên lần mở sau không báo lại.
   - Electron: `deskHosts.ts:105` hiện Notification của hệ điều hành (en / vi theo locale), hoặc gọi `deps.notice` nếu có.
   - Bot không cần đăng nhập lại tay: lần upload đầu tự đăng nhập bằng token.
7. **Nối vào code D7** (sửa nhỏ):
   - `report.rs:191`: `big_upload` gọi `mt_upload`.
   - `discover.rs:207`: quét topic.
   - `discover.rs:237`: `user()`.
   - `user_keys` / `drop_user` thành `pub(crate)`.
   - `shape_out::write_private` thành `pub(crate)`.
   - `Tg.mt` (`base.rs:190`).

## File

- Mới Rust: `apps/core/crates/core-desk/src/telegram/mt/{mod,upload,login,account,service,store,blob,html,grammers,grammers_tl,tests,tests_tg}.rs`. File dài nhất là `tests.rs`, 347 dòng.
- Sửa Rust: `telegram/{mod,base,base_io,discover,report,shape_out,tests}.rs`, `core-desk/Cargo.toml` (+6 dep: `grammers-{tl-types,crypto,mtproto,session,mtsender}` 0.12, tokio `rt-multi-thread`), `apps/core/Cargo.lock`.
- Sửa TS: `apps/desktop/src/telegram/deskHosts.ts` (host `telegram.notice`, `TelegramHostDeps.notice?`), `apps/desktop/test/desk-telegram.twin.test.ts`:
  - twin cũ chạy với `CCM_TELEGRAM_MTPROTO=node`;
  - thêm describe mới chạy đường Rust, không cần mạng.
- Doc: `.code_index/_meta.md`, `.planning/PLAN-v87-step6-tauri.md` dòng D7.
- Không đụng: `apps/shell/*`, `core-pty`, `main.ts`, `packages/shared`, `apps/web`.

## Test thật

- `cargo test -j 4 --no-fail-fast` cả `apps/core`: 873 passed, 0 failed, 6 ignored.
  - Mới: 18 test `telegram::mt`. 12 test trong `mt/tests.rs`:
    - chia chunk và biên 4000 part;
    - mỗi part đúng byte, đúng 1 lần;
    - `FLOOD_WAIT` chờ rồi thử lại;
    - flood dài / lỗi mạng mãi thì bỏ;
    - `FILE_PART_MISSING` + `RANDOM_ID_DUPLICATE`;
    - bảng map lỗi upload / user;
    - HTML → entity UTF-16;
    - blob session round-trip;
    - chat id → peer, tên hiển thị;
    - luồng phone / code / password / expired / sign-up;
    - phân trang topic;
    - đếm part qua worker.
  - 6 test trong `mt/tests_tg.rs`, Tg thật với connector giả:
    - **resume**: kết nối 1 lỗi mạng từ part 3, kết nối 2 dùng lại session, chỉ gửi part thiếu, cùng `file_id`;
    - session bot bị revoke thì đăng nhập lại 1 lần; file 0600, không chứa token / hash;
    - token sai → `auth`, chat đóng → `chat`;
    - đăng nhập / đăng xuất tài khoản qua `user()`;
    - báo đăng nhập lại đúng 1 lần, kể cả sau khi mở lại;
    - quét gặp `auth` thì bỏ session.
- `cargo clippy -j 4 --all-targets -- -D warnings`:
  - `telegram/` sạch;
  - workspace **còn 3 lỗi ở `core-desk/src/agent_data/{service,tree}.rs`**. Thư mục này untracked, của gói khác đang chạy song song, con không sửa.
- `npm test -w @ccm/desktop -- --maxWorkers=4`: 168 file passed + 1 skipped; 1708 passed, 29 skipped, 0 failed. Có twin mới `telegram: orrery-core desk speaks MTProto itself`, chạy `orrery-core` debug thật:
  - `userAccount` null;
  - `relogin` báo 1 lần;
  - file `{v:1, noticed:true}` mode 0600;
  - `userSession` GramJS còn nguyên;
  - `signin` / `password` khi chưa gửi code trả `expired`.
- `npm run typecheck`: exit 0.

## E2E / test tay cần chạy (chủ nhân)

1. Build: `node apps/desktop/build.mjs`, rồi `cd apps/core && cargo build --release -j 4 -p core-bin`.
2. Bộ Telegram như D7: `CCM_DESK_TELEGRAM=rust CCM_E2E_PORT=4551 CCM_E2E_DBG=9551 node apps/desktop/e2e/agent-api.cjs`.
3. Test tay bằng bot test riêng + tài khoản Telegram thật, app chạy với `CCM_DESK_TELEGRAM=rust`:
   - Mở app khi đã có tài khoản GramJS: thấy 1 thông báo "Telegram: cần đăng nhập lại"; mở lại app thì không thấy nữa.
   - Settings › Telegram › tài khoản: Sign in → phone → code (trong app Telegram) → mật khẩu 2 lớp (nếu có) → hiện "Signed in: …"; Scan liệt kê đủ topic (cả topic chưa có tin); Sign out → Telegram › Devices không còn phiên.
   - Large files bật, gửi file 51 MB và khoảng 300 MB vào topic: thấy % tiến độ, file đến đúng topic, caption đậm tên project. Thử ngắt mạng giữa chừng 10–20 s: upload chạy tiếp hoặc báo `network`.
   - Đổi token bot: lần gửi file lớn sau tự đăng nhập bot lại.
   - Đặt `CCM_TELEGRAM_MTPROTO=node`: đường GramJS cũ vẫn chạy (session GramJS chưa bị đụng).
   - Windows: lặp lại 2 mục đầu (chưa biên dịch trên Windows; code không có `cfg` riêng cho Windows).
4. Qua E2E thì có thể bỏ `telegram` khỏi `DESK_NODE_DEFAULT`; sau đó bỏ `mtproto.ts` / `userScan.ts` / `gramjs.ts` và gói `telegram` (GramJS) khỏi Electron khi bỏ Electron.

## Nối vỏ Tauri (con không sửa `hosts.rs`, việc của T2 / D8)

- `apps/shell/src/hosts.rs` cần thêm standing host `telegram.notice {kind:'relogin'}`:
  - hiện notification hệ điều hành;
  - title en "Telegram: sign in again" / vi "Telegram: cần đăng nhập lại";
  - body xem `deskHosts.ts` `RELOGIN`;
  - trả `true`.
- Không có host này thì core-desk vẫn đánh dấu đã báo và không báo lại; UI vẫn hiện nút Sign in.
- Tauri **không cần** `telegram.bigUpload` / `telegram.user` nữa. Không đặt `CCM_TELEGRAM_MTPROTO` ở Tauri.

## Size `orrery-core` (ước tính, chưa build release)

- Debug: 81,7 MB → 94,0 MB. Con số này có lẫn thay đổi của gói khác chạy song song.
- Release (lto thin, strip): ước tính **+3 đến +5 MB**. Phần lớn là bộ (de)serializer TL layer của `grammers-tl-types` mà `messages.sendMedia → Updates` kéo theo, cộng tokio multi-thread, `num-bigint` / `crypto-bigint` (SRP, DH), `os_info`.
- Đo thật sau bước build 1: `ls -l apps/core/target/release/orrery-core`.

## Khác TS / rủi ro

- Không nhập được session GramJS (format khác). Chủ nhân đã chấp nhận đăng nhập lại.
- Còn rủi ro panic bên trong `grammers-mtsender` / `grammers-session` (con không thấy `unwrap` ở đường dùng, nhưng chưa audit hết). Release là `panic = abort`.
- `ConnectionParams::default()` đọc `os_info` / locale hệ thống. Thiết bị hiện trong Telegram › Devices là "Mac OS 64-bit" / "Windows 64-bit", app version là version core-desk.
- Không proxy (feature `proxy` của grammers không bật). GramJS hiện cũng không dùng proxy.
- Chat cá nhân (chat id dương) không gửi file lớn được qua Rust (trả `chat`). Route thực tế là group / forum (`-100…`, `-…`).
- Upload Rust song song 4 part trên 1 kết nối. GramJS `workers: 4` tương tự.

STATUS: DONE (chưa E2E)
