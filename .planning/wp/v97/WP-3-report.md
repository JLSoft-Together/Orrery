## WP-3 — DONE

- Đã đổi:
  - `apps/core/crates/core-pty/src/hub/mod.rs`: `Session.conn` → `viewers: Vec<Viewer {conn, mode: Own|View, size}>` + `input_owner` + `size` (cỡ PTY hiện tại); `Out` derive `Clone`; `Closed` gọi `leave_all`.
  - `hub/viewers.rs` (mới): `attach_mode`, `fanout`, `join` / `leave` / `leave_all`, `claim`, `resize_from`, `fit_owner`, `rebind` (bind Gate theo owner), `size_note`.
  - `hub/spawn.rs`: `attach(conn, key, replay, mode, size)`; spawn = `own` với cỡ spawn; `own` vẫn đóng `own` cũ `4000 replaced`, `view` không đóng ai.
  - `hub/frames.rs`: binary → `claim` rồi ghi; `resize` → `resize_from`; thêm `{t:"claim"}`; `attach` đọc `mode` / `cols` / `rows`.
  - `hub/from_main.rs`: data / `exit` / `branch` fanout; `cwd` (khi `live_cwd` đổi) gửi thêm cho viewer khác; reaper = exited + không còn viewer.
  - `hub/tests.rs`: helper `Log` / `spawn_long` / `drain` thành `pub(super)`; 1 dòng đổi cách đọc `s.conn` → `s.viewers.is_empty()` (kỳ vọng giữ nguyên: conn không còn gắn session cũ).
  - `hub/tests_viewers.rs` (mới, 7 test unit), `core-bin/tests/viewers.rs` (mới, 3 test tích hợp).
  - `.code_index/core.md` mục core-pty: thêm "Hub viewers".
- Message WS mới:
  - client → core: `{t:"attach", id, mode:"view", cols?, rows?}` (không `mode` / khác `"view"` = `own` như cũ); `{t:"claim"}` (tuỳ chọn, gõ phím đã tự claim).
  - core → client: `{t:"resized", cols, rows}` — gửi cho mọi viewer khác khi cỡ PTY thật sự đổi (owner resize, đổi owner); attach `view` nhận nó ngay sau `ready` (trước replay): chuỗi `ready, resized, cwd, <replay>, branch, exit`.
  - Desktop chỉ có 1 socket `own`: không thấy message mới nào, hành vi y cũ.
- Tiêu chí: 9/9 đạt
  - [x] test cũ core-pty + `pty.rs` + `flow.rs` pass, kỳ vọng không đổi.
  - [x] own + view cùng bytes, không 4000 (unit `own_and_view_get_the_same_bytes…`, tích hợp `desktop_and_phone_see_the_same_bytes…`).
  - [x] điện thoại gõ → owner, PTY theo cỡ điện thoại (child in `size:50x20`), desktop nhận `resized`; desktop gõ lại → ngược lại.
  - [x] owner close → owner = `own` còn lại, nếu không thì viewer sớm nhất, cỡ đổi + `resized`.
  - [x] view không đọc, 16 MiB flood → chỉ nó 4001, desktop đủ mọi dòng, child chạy hết.
  - [x] attach view sau exit nhận `ready, resized, <replay>, exit`; reaper không xoá khi còn viewer (unit, đồng hồ giả).
  - [x] clippy `-p core-pty --all-targets -D warnings` sạch (cả `--target aarch64-apple-darwin`); core-bin + tests sạch, nhưng `-p core-bin -D warnings` vẫn fail vì `core-desk` (dependency, 3 lỗi: unused import `disk/space.rs`, `network_fs` dead code, `set_readonly(false)` `layout/cds.rs`) — không thuộc WP-3. CHƯA KIỂM: clippy core-bin với target mac (core-desk chặn trước).
  - [x] code_index cập nhật. [x] không comment.
- Audit: diff chỉ trong `core-pty/src/hub/*`, `core-bin/tests/viewers.rs`, `.code_index/core.md` (mục core-pty), brief. Không đụng Cargo.toml / lock / core-remote-proto / apps/web / shared.
- Review (edge case):
  - viewer close giữa chừng: `leave` gỡ meter, chuyển owner, `rebind`.
  - owner đổi khi PTY đang paused: `rebind` unbind meter cũ (unpause) rồi bind meter owner mới theo backlog của nó.
  - exit khi đang drain: `Draining` không đổi; `exit` fanout sau data; claim / resize sau exit không đụng PTY.
  - kill: từ viewer nào cũng được; viewer giữ lại tới exit nên vẫn nhận `exit`.
  - Rủi ro còn: (1) owner là điện thoại chậm thì vẫn pace PTY (người đang gõ) — desktop chờ theo; (2) giữa lúc owner bị 4001 và event `Closed` tới, pane không có binder (rất ngắn, PTY chạy không pace); (3) socket `own` đã bị thay (4000) mà còn gửi frame thì giữ hành vi cũ: input vẫn ghi, resize vẫn áp, không claim; (4) attach `view` có `cols/rows` khi chưa có owner không tự áp cỡ, chỉ áp khi nó resize / gõ.
- Test:
  - `cargo test --locked -j 3 -p core-pty` → 42 pass / 0 fail.
  - `cargo test --locked -j 3 -p core-bin --test pty --test flow --test viewers` → 16 + 8 (2 ignored) + 3 pass / 0 fail (viewers chạy 3 lần, pass).
  - `cargo clippy --locked -j 3 -p core-pty --all-targets -- -D warnings` (Windows + aarch64-apple-darwin) → sạch.
- Hot file cần thêm: không (không cần dep mới).
- Ngoài phạm vi cần main xử lý:
  - `packages/shared/src/pty.ts` (không nằm trong phạm vi file của brief): thêm `{ t: 'attach'; id; mode?: 'view'; cols?; rows? }`, `{ t: 'claim' }` vào `PtyClientMessage` và `{ t: 'resized'; cols; rows }` vào `PtyServerMessage` — làm ở WP-7 / gateway.
  - Renderer (WP-7): xử lý `resized` (khi điện thoại là owner, xterm desktop cần hiển thị theo cỡ PTY hoặc báo "đang điều khiển từ điện thoại"); desktop gõ là tự lấy lại cỡ.
  - `core-desk` clippy lỗi (chặn `-D warnings` toàn core-bin).
- Commit message đề xuất:
  `feat(core-pty): multiple viewers per pane with an input owner, view-mode attach and resized notices`

## Vòng 2 (theo WP-3-review.md, cách sửa main chốt) — DONE, 6/6 mục

- #1 (cao) `flow.rs`: `Gate` đếm hold từ nhiều `Meter` (`holds`; meter vượt `PAUSE_AT` lấy 1 hold, dưới `RESUME_AT` / đổi bind / unbind thì trả; khoá meter rồi gate, `unhold` notify khi về 0 nên không lost wakeup; `set_paused` giữ làm cờ tay). `rebind` bind meter của mọi viewer `own` + owner; `view` không phải owner thì unbind. Test unit `several_meters_pause_one_gate_until_every_one_is_under_128k`, `a_slow_desktop_pauses_the_pane_while_a_claimed_phone_reads_fast`; tích hợp `a_slow_desktop_still_paces_the_pane_while_the_claimed_phone_reads_fast` (16 MiB, phone claim đọc nhanh, desktop không đọc → pane dừng; desktop đọc lại → đủ mọi dòng, không 4001).
- #2 (vừa) `frames.rs`: bỏ claim ngầm theo binary; owner chỉ đổi bằng `{t:"claim"}`; input từ mọi viewer vẫn ghi. Chưa có owner → `own` đầu tiên như cũ. Test unit / tích hợp sửa theo (gửi `claim` trước khi gõ), thêm assert gõ phím không đổi owner.
- #3 (vừa) `mod.rs`: `send` ghi conn overflow vào `Hub.spilled`, cuối `handle` gọi `leave_all` → rời viewers ngay, không được chọn owner / binder, reaper không chờ nó.
- #4 `resize_from`: conn không còn trong `viewers` → bỏ qua.
- #5 `from_main.rs` `kill`: xoá `c.session` của mọi viewer của session (viewer vẫn nhận `exit`).
- #6 test `an_overflowed_owner_leaves_at_once_and_input_goes_back_to_the_desktop` (owner phone 4001 → owner desktop, cỡ về 80x24, claim / resize của conn đó bị bỏ, reaper xoá session sau exit dù socket chưa `Closed`).
- Contract cho WP-6 / WP-7: client gửi `{t:"claim"}` khi người dùng thật gõ phím / dán, KHÔNG gửi khi xterm tự trả lời query. Viewer `view` không phải owner vẫn ghi được input, nên client `view` không nên forward auto-reply của xterm khi không phải owner.
- Test vòng 2: `cargo test --locked -j 3 -p core-pty` → 45 pass; `-p core-bin --test pty --test flow --test viewers` → 16 + 8 (2 ignored) + 4 pass; `cargo clippy -p core-pty --all-targets -D warnings` sạch (Windows + aarch64-apple-darwin); clippy core-bin: không warning trong core-bin / core-pty (core-desk vẫn lỗi riêng).
- Đổi thêm file: `apps/core/crates/core-pty/src/flow.rs` (trong phạm vi `core-pty/src/**`).
- Commit message đề xuất (thay bản cũ): `feat(core-pty): multiple viewers per pane with explicit input claim, shared gate backpressure and view-mode attach`

## Vòng 3 — DONE
- `hub/viewers.rs` `join`: attach `own` lấy owner khi chưa có owner HOẶC owner hiện tại là viewer `view` (desktop remount sau khi `Closed` socket cũ tới trước → không còn kẹt cỡ phone); áp cỡ attach nếu có, nếu không thì `resize` kế tiếp của desktop áp + gửi `resized` cho phone. Test `desktop_remount_after_its_old_socket_closed_takes_input_and_size_back_from_the_phone`; test `own_attach_still_replaces_own_but_never_a_view` đổi kỳ vọng owner → desktop mới (theo quyết định main).
- Test: core-pty 46 pass; core-bin `--test viewers` 4 pass; clippy core-pty `-D warnings` sạch.
