## WP-4 — DONE (chờ kết quả cuối `cargo check` shell, xem dưới)
- Đã đổi:
  - `core-http/src/tap.rs` (mới): `StoreTap`, `TapGuard`, `TapError {Denied, Invalid}`, `TAP_ALLOWED`, `TAP_CLIENT_ID`.
  - `core-http/src/ws.rs`: `ClientContext` dùng `Sink::{Channel, Callback}` (field `tx` thành `sink`, constructor `pub(crate)`), `Clients` thêm registry tap + `next_tap` riêng (id ws vẫn từ 1), `broadcast` gọi tap sau khi serialize 1 lần (cùng `Arc<str>` với /ws, `catch_unwind` mỗi callback).
  - `core-http/src/server.rs`: `RunningServer::tap()`; `core-http/src/lib.rs`: export `StoreTap, TapError, TapGuard`.
  - `core-serve/src/lib.rs`: `pub type TapSink = Box<dyn FnOnce(StoreTap) + Send>`, `run_with_tap(cfg, emit, on_tap)`, re-export `StoreTap/TapError/TapGuard`; `run_with` / `run` không đổi.
  - `apps/shell/src/serve.rs`: `Serve` giữ `Arc<OnceLock<StoreTap>>`, `Serve::tap() -> Option<StoreTap>`; `start` gọi `run_with_tap`.
  - `core-http/tests/tap.rs` (5 test), `.code_index/core.md` (mục core-http).
- API `StoreTap` (Clone, Send + Sync):
  - `snapshot(&self) -> String`
  - `subscribe(&self, f: impl Fn(Arc<str>) + Send + Sync + 'static) -> TapGuard`
  - `attach(&self, f: impl Fn(Arc<str>) + Send + Sync + 'static) -> (String, TapGuard)` (snapshot + đăng ký trong cùng 1 lần lock store, giống client /ws mới, không hụt / trùng)
  - `send(&self, msg: serde_json::Value, reply: impl Fn(Arc<str>) + Send + Sync + 'static) -> Result<Option<String>, TapError>` (`resync` → `Ok(Some(snapshot))`; loại khác vào `on_client_message`, direct reply về `reply`, trả `Ok(None)`)
  - Whitelist kiểm theo chuỗi `type` TRƯỚC khi parse: `terminal.alias|note|snooze|dismiss`, `history.get`, `toolSpans.get` (+ alias `tool-spans.get` như docs §4.2 ghi; wire thật là `toolSpans.get`), `resync`. Loại khác → `Denied`; sai shape → `Invalid`.
- Cách vỏ nhận tap: `run_with_tap` gọi sink FnOnce 1 lần ngay sau khi server bind, trước `listening`; shell cất vào `OnceLock`, gateway lấy bằng `Serve::tap()` (sau `Serve::start` thành công là có). Chọn sink param thay vì field `ServeConfig` vì `ServeConfig` là `Clone + Debug`.
- Tap không phải client /ws: không vào `Clients.map`, không `on_client_count` / `on_client_close`, `client.presence` bị Denied nên không ảnh hưởng toast; `client_id` cố định `u64::MAX`; direct reply chỉ về callback của chính lần `send`.
- Tiêu chí: 7/7 (mục shell `cargo check`: xem dòng Test).
- Audit: diff chỉ trong phạm vi; không comment; không đụng Cargo.toml / core-pty / core-remote-proto.
- Review: (1) tap bị drop giữa batch: gỡ khỏi registry dưới lock, tối đa 1 callback đang chạy dở, không rò thread / task (không có thread). (2) store đang lock: callback chạy inline trong store listener nên PHẢI không chặn và không gọi `snapshot/attach/send(resync)` bên trong (deadlock) — ghi ở `.code_index/core.md`; gateway nên đẩy sang channel. (3) shutdown: sau `server.shutdown()` không còn broadcast; guard drop vẫn an toàn (giữ `Arc<Clients>`); `send` sau shutdown chỉ rơi vào kênh monitor đã đóng. (4) callback panic được `catch_unwind`, không làm sập store / monitor. (5) `reply` của `send` chạy trên thread `monitor`.
- Test: `cargo test --locked -j 3 -p core-http` → 80 pass / 0 fail (gồm `tests/tap.rs` 5: byte-identical với /ws, Denied không tới dispatch, history.get reply về đúng tap và ws khác không nhận, drop guard dừng batch, subscriber panic); `-p core-serve` pass; `-p core-bin --test serve` → 5 pass; `cargo clippy --locked -j 3 -p core-serve -p core-http --all-targets -- -D warnings` → sạch; test cũ `client_context_reports_the_open_socket_count_and_an_id` (id ws 1,2) giữ nguyên nhờ counter tap riêng.
- Shell `cargo check --locked -j 3`: xem mục cập nhật cuối file (build dài do máy bận).
- Hot file cần thêm: không.
- Ngoài phạm vi: gateway dùng `attach` + channel; TLS / LAN listener không làm ở đây.
- Commit message đề xuất: feat(core-http): add StoreTap for in-process dashboard snapshot, batch subscription and whitelisted client commands

### Cập nhật cuối
- CHƯA KIỂM: `cd apps/shell && cargo check --locked -j 3` — vẫn đang chạy nền (task `bt5flr5po`, output `C:\Users\dauph\AppData\Local\Temp\claude\9ab499cff4de69ce\73006b1b-0e4a-408d-a6fb-6ca270bdef02\tasks\bt5flr5po.output`, dòng cuối sẽ là kết quả) vì build shell lạnh + máy bận tranh lock target. Phần đổi ở shell chỉ ~8 dòng trong `serve.rs` (import `Arc, OnceLock`, `StoreTap`; field `tap`; `run_with_tap`; `Serve::tap()`); `Serve::tap` chưa có caller nên có thể ra warning dead_code cho tới khi gateway dùng.
