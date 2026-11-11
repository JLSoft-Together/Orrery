## WP-3: core-pty hub — nhiều viewer cùng xem 1 pane, bên đang gõ giữ cỡ
- Mục tiêu: 1 session PTY cho phép nhiều socket attach cùng lúc (desktop + gateway điện thoại), output tới mọi socket, không ai bị đóng `4000 replaced` khi attach kiểu viewer; ai gõ thì thành "input owner" và cỡ của người đó áp vào PTY; owner rời thì cỡ trả về viewer kế. Hành vi hiện có của desktop (1 socket / pane) giữ nguyên từng byte.
- Spec item: PLAN-v97-mobile.md T6, WP-3 (bảng §4), review #3; khảo sát `.planning/research/v97-renderer-side.md` §4.
- Phạm vi file: `apps/core/crates/core-pty/src/**`, `apps/core/crates/core-bin/tests/pty.rs` (+ file test mới trong `core-bin/tests/`), `packages/shared/src/ptyControl.ts` CHỈ nếu message WS mới phải có trong contract TS (ghi rõ), `apps/desktop/test/fixtures/pty-control.json` chỉ nếu control channel đổi (không nên đổi). | Cấm đụng: `apps/core/Cargo.toml`, `apps/core/Cargo.lock` (WP-2 sở hữu; cần dep mới → ghi "Hot file cần thêm"), `apps/core/crates/core-remote-proto/**`, `apps/web/**` (renderer đổi ở WP-7), `apps/shell/**`.
- Phụ thuộc: — | Song song với: WP-1, WP-2 | Worktree: không.
- Model: opus (concurrency, flow control).

### Hiện trạng (đã khảo sát, kiểm lại trước khi sửa)
- `Session.conn: Option<u64>` (`hub/mod.rs:63`), `attach` đóng socket cũ `4000 replaced` (`hub/spawn.rs:88-136`, :129), output chỉ tới `s.conn` (`hub/from_main.rs:136`), resize last-writer-wins (`hub/frames.rs:76`), `Meter` bind 1 Gate (`flow.rs:104`, bind `spawn.rs:133-135`), `Closed` (`hub/mod.rs:229-238`), reaper `s.conn.is_none()` (`hub/from_main.rs:117`). WS message hiện có xem `ws.rs` + `hub/frames.rs`; client desktop `apps/web/src/lib/ptyClient.ts`.

### Thiết kế
- `Session`: `viewers: Vec<ViewerConn>` (conn id, mode `own|view`, cỡ cuối `cols×rows`, attach order) thay `conn`; `input_owner: Option<u64>`.
- `attach {id}` không tham số mới = hành vi cũ (mode `own`: socket cũ cùng mode `own` vẫn bị `4000 replaced` như trước, để desktop giữ đúng contract); `attach {id, mode:"view", cols?, rows?}` = thêm viewer, không đóng ai, nhận cùng chuỗi `ready, cwd, <replay>, branch, exit` như attach thường.
- Output PTY: ghi ring 1 lần, gửi tới mọi viewer. `exit`, `branch`, `cwd` cũng fanout.
- Input (binary) từ bất kỳ viewer nào đều được ghi vào PTY; viewer gõ mà chưa là owner → thành owner (claim ngầm), nếu cỡ cuối của nó khác cỡ hiện tại → resize PTY theo cỡ nó + gửi `{t:"resized",cols,rows}` tới mọi viewer khác. `{t:"resize"}` từ viewer: lưu cỡ cuối của viewer đó; chỉ áp vào PTY nếu viewer là owner (hoặc chưa có owner). Owner close / detach → owner = viewer `own` còn lại (nếu không có thì viewer attach sớm nhất), áp cỡ cuối của nó, gửi `resized`.
- Flow control: mỗi viewer có `Meter` riêng; chỉ Meter của owner (hoặc viewer `own` nếu chưa có owner) được pause Gate; viewer `view` chậm quá `CONN_MAX` → chỉ socket đó bị close 4001 `overflow`, PTY không bị pause vì nó. Giữ nguyên: ring 256 KiB, `MAX_LIVE` 16, credit / `Draining`, không output sau `exit`.
- `Closed` bỏ viewer khỏi list; reaper: session đã exit **và không còn viewer** quá `EXITED_TTL`.
- `kill` từ viewer nào cũng được như cũ.

### Tiêu chí nghiệm thu
- [ ] Toàn bộ test core-pty + `core-bin/tests/pty.rs` + `tests/flow.rs` hiện có pass không sửa kỳ vọng (hành vi desktop không đổi).
- [ ] Test mới: desktop `own` + điện thoại `view` cùng nhận output giống nhau từng byte, desktop không nhận 4000.
- [ ] Test: điện thoại gõ → thành owner, PTY resize theo cỡ điện thoại, desktop nhận `resized`; desktop gõ lại → owner về desktop, cỡ về cỡ desktop, điện thoại nhận `resized`.
- [ ] Test: owner close → owner chuyển, cỡ đổi đúng.
- [ ] Test: viewer `view` không đọc (8 MiB output) → chỉ nó bị 4001, desktop vẫn nhận đủ, PTY không bị treo.
- [ ] Test: attach `view` sau khi exit nhận replay + `exit`; reaper không xoá session còn viewer.
- [ ] `cargo clippy -p core-pty -p core-bin --all-targets -- -D warnings` sạch (cả `--target aarch64-apple-darwin` nếu target có sẵn; không có thì ghi lại).
- [ ] `.code_index/core.md` mục core-pty cập nhật (hub nhiều viewer, owner, message mới).
- [ ] Không comment trong code.

### Lệnh test (giới hạn tài nguyên: song song WP-1 Flutter + WP-2 cargo)
`cd apps/core && cargo test --locked -j 3 -p core-pty && cargo test --locked -j 3 -p core-bin --test pty --test flow && cargo clippy --locked -j 3 -p core-pty -p core-bin --all-targets -- -D warnings`
(PowerShell trên Windows như `.code_index/core.md` ghi; test integration cần `node` trong PATH.)

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-3-report.md`

### Nhật ký
- 2026-11-12 WP-3: hub nhiều viewer + input owner + `resized` xong (`hub/viewers.rs`), test core-pty 42 pass, core-bin pty 16 / flow 8 / viewers 3 pass, clippy core-pty + core-bin sạch (core-desk lỗi riêng), code_index cập nhật, báo cáo WP-3-report.md.
- 2026-11-12 WP-3 vòng 2: 6 mục review xong (gate nhiều meter, claim chỉ qua message, overflow rời viewers ngay, resize non-viewer bỏ, kill dọn c.session), core-pty 45 pass, core-bin pty 16 / flow 8 / viewers 4 pass, clippy core-pty sạch.
- 2026-11-12 WP-3 vòng 3: attach own lấy owner từ owner view (desktop remount), core-pty 46 pass, viewers 4 pass, clippy core-pty sạch.
