## WP-4: `StoreTap` — cho gateway đọc dữ liệu dashboard trong process
- Mục tiêu: gateway (sẽ ở `apps/shell/src/remote/`) lấy được snapshot + event batch của dashboard store và gửi được `ClientMessage` (đã whitelist) vào monitor, giống 1 client `/ws`, mà không mở thêm socket và không đổi hành vi `/ws` hiện có.
- Spec item: PLAN-v97-mobile.md §3 dòng core-serve, H13; `docs/remote-integration.md` §4.2; khảo sát `.planning/research/v97-rust-side.md` §1, §3.
- Phạm vi file: `apps/core/crates/core-serve/src/**`, `apps/core/crates/core-http/src/**`, test trong 2 crate đó, `apps/shell/src/serve.rs` (chỉ để nhận + cất `StoreTap` vào state, không logic khác), `.code_index/core.md` (chỉ mục core-serve / core-http). | Cấm đụng: `apps/core/crates/core-pty/**`, `core-remote-proto/**`, `Cargo.toml` / `Cargo.lock` (cần dep → "Hot file cần thêm"), `apps/web/**`, `apps/mobile/**`.
- Phụ thuộc: — | Song song được với: WP-1b, WP-9 | Worktree: không.
- Model: sonnet.

### Thiết kế
- Type công khai `StoreTap` (Clone, Send + Sync) trả ra từ `core_serve::run_with` qua callback / channel cấu hình (vd `ServeConfig.on_tap: Option<Box<dyn FnOnce(StoreTap) + Send>>`) — chọn cách ít đụng nhất, ghi lý do.
- API: `snapshot() -> String` (JSON `snapshot` message y như client mới của `/ws` nhận, kèm `seq`); `subscribe(f: impl Fn(Arc<str>) + Send + Sync) -> TapGuard` (mỗi batch `ServerMessage` đã serialize; drop guard = huỷ); `send(msg: serde_json::Value) -> Result<Option<String>, TapError>` (parse bằng `parse_client_message` có sẵn, chỉ nhận loại trong whitelist `terminal.alias|note|snooze|dismiss`, `history.get`, `tool-spans.get`, `resync`; reply direct trả về qua `Option<String>` hoặc callback; loại khác → `TapError::Denied`).
- Client tap **không** được tính presence / attentive (không làm tắt toast desktop), không nhận direct reply của client khác.
- Không đổi thứ tự / format message của `/ws`.

### Tiêu chí nghiệm thu
- [ ] Test: tap nhận snapshot rồi các batch giống hệt (byte) 1 client `/ws` trong cùng kịch bản.
- [ ] Test: `send` loại cấm (`client.presence`, `terminal.open`, `terminal.focusWindow`, `folder.pick`, `favorite.open`) → `Denied`, không tới dispatch.
- [ ] Test: `history.get` qua tap trả reply về đúng tap.
- [ ] Drop guard → không còn nhận batch; không rò thread / task.
- [ ] Test cũ core-serve / core-http / `core-bin/tests/serve.rs` pass không đổi kỳ vọng.
- [ ] `cargo clippy -p core-serve -p core-http --all-targets -- -D warnings` sạch; shell `cargo check` (trong `apps/shell`, `-j 3`) sạch.
- [ ] Không comment trong code; `.code_index/core.md` cập nhật.

### Lệnh test
`cd apps/core && cargo test --locked -j 3 -p core-serve -p core-http && cargo test --locked -j 3 -p core-bin --test serve && cargo clippy --locked -j 3 -p core-serve -p core-http --all-targets -- -D warnings` và `cd apps/shell && cargo check --locked -j 3`

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-4-report.md`

### Nhật ký
- 2026-11-12: core-http `tap.rs` (StoreTap/TapGuard/TapError) + `Clients` tap registry + `ClientContext` sink callback; core-serve `run_with_tap`; shell `Serve::tap()`. Test `tests/tap.rs` 5 pass, core-http 80 pass, core-bin serve 5 pass, clippy sạch. Shell cargo check chạy tiếp.
