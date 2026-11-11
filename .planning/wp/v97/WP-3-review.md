# WP-3 review — core-pty hub nhiều viewer (review độc lập, chỉ đọc)

Phạm vi: `git diff -- apps/core/crates/core-pty` + `hub/viewers.rs`, `hub/tests_viewers.rs`, `core-bin/tests/viewers.rs`. `cargo test --locked -j 3 -p core-pty`: 42 pass. Không chạy core-bin / clippy.

## Findings

- apps/core/crates/core-pty/src/hub/viewers.rs:119-133 — cao — `rebind` chỉ bind Meter của `input_owner`; khi điện thoại (`view`) gõ và thành owner, Meter của desktop `own` bị unbind nên desktop mất backpressure: gateway điện thoại đọc nhanh thì PTY chạy hết tốc độ, renderer desktop chậm hơn (`cat` file lớn) dồn tới `CONN_MAX` 8 MiB → desktop nhận 4001 → `ptyClient.reattach` reset màn hình, lặp tới `MAX_REATTACH` rồi đóng pane. Desktop 1 socket trước đây không bao giờ bị 4001 trong flow thường (Gate pause ở 512 KiB). — Fix: Gate cho nhiều Meter cùng pause (đếm / tập Meter đang over `PAUSE_AT`, paused = có ít nhất 1), bind mọi viewer `own` + owner; chỉ viewer `view` không phải owner để unbind (chậm thì 4001 riêng nó).
- apps/core/crates/core-pty/src/hub/frames.rs:19 — vừa — claim ngầm trên MỌI binary frame: xterm tự trả lời query của app (DA `\e[c`, CPR `\e[6n`, OSC 10/11 màu, focus `\e[I`/`\e[O` khi DECSET 1004, mouse tracking) nên viewer chỉ đang xem cũng thành owner và kéo cỡ PTY về cỡ của nó; 2 renderer cùng trả lời 1 query → PTY nhận 2 câu trả lời (ký tự rác lọt vào input Claude Code / vim) và owner lật qua lật lại mỗi lần app query. — Fix: chỉ đổi owner bằng `{t:"claim"}` client gửi khi có phím thật; ghi vào contract WP-7 là client `view` không được forward auto-reply khi không phải owner (hoặc hub bỏ input của viewer `view` không phải owner trừ khi vừa `claim`).
- apps/core/crates/core-pty/src/hub/viewers.rs:48, :82, :130 + mod.rs:223-228 — vừa — conn đã overflowed vẫn nằm trong `viewers` tới khi `Closed`, mà `Closed` có thể không bao giờ về: `Out::Close(4001)` xếp sau tới 8 MiB trong `write_loop` (ws.rs:151), peer ngừng đọc nhưng TCP còn sống thì `sink.send` treo. Trong lúc đó conn này vẫn được `leave()` chọn làm owner (fallback viewer đầu tiên), vẫn `claim` được (reader còn chạy), còn `rebind` bỏ qua nó (`overflowed`) → không Meter nào bind Gate → các viewer `own` mất backpressure; reaper cũng không xoá session đã exit vì `viewers` không rỗng. — Fix: khi `send()` phát hiện overflow, ghi conn vào danh sách chờ và cuối `handle` gọi `leave_all(conn, None)`; `claim` / `leave` / `rebind` bỏ qua conn `overflowed` khi chọn owner / binder.
- apps/core/crates/core-pty/src/hub/viewers.rs:92-101 — thấp — `resize_from` từ conn không còn trong `viewers` (socket vừa bị `4000 replaced`, `c.session` vẫn `Some(key)`) bỏ qua check owner và vẫn resize PTY + fanout `resized`, đè cỡ của owner hiện tại. — Fix: không tìm thấy viewer thì `return`.
- apps/core/crates/core-pty/src/hub/frames.rs:43-55, :82-87 + from_main.rs:92-114, :158 — thấp — `kill` chỉ xoá `c.session` của conn gửi lệnh; viewer khác của session bị kill (session bị remove ở `kill` khi đã exit, hoặc ở `Exit` vì `known=false`) giữ `c.session = Some(key)` trỏ tới session không còn → socket đó không `attach` / `spawn` lại được (guard `current.is_none()`), input / resize bị bỏ im lặng. Desktop mở socket mới mỗi pane nên ít ảnh hưởng, gateway điện thoại dùng lại socket thì kẹt. — Fix: khi remove session, set `c.session = None` cho mọi viewer của nó.
- apps/core/crates/core-pty/src/hub/viewers.rs:121 — thấp — owner là điện thoại ngừng đọc (app về nền, TCP chưa đứt) giữ Gate pause cho mọi viewer; desktop `resize` / focus không lấy lại owner, phải gõ phím. Đúng theo thiết kế nhưng desktop thấy pane đứng hình không rõ lý do. — Fix: desktop gửi `{t:"claim"}` khi pane được focus (WP-7) hoặc theo fix finding 1 thì desktop `own` vẫn bind Gate nhưng owner chậm vẫn pause → cân nhắc timeout bỏ owner `view` không đọc.

## Đã kiểm, ổn

- Desktop 1 socket `own`: `attach` không `mode` = `Own`; socket `own` cũ nhận `4000 replaced` trước outs của socket mới như cũ; thứ tự `ready, cwd, <replay>, branch, exit` giữ nguyên, `resized` chỉ gửi khi attach `view`; mọi fanout skip conn gốc nên 1 viewer không nhận thêm message nào; `resize` vẫn gọi `proc.resize` mỗi lần như cũ; `ptyClient` attach không gửi `cols/rows` nên attach không resize PTY, `TermPane` gửi `resize` ngay khi nhận `ready`.
- Meter: unbind khi `Closed` (`conns.remove` rồi `bind(None)`), khi `leave`, khi bị replace; `bind(None)` luôn `set_paused(false)` trên Gate cũ trước khi bind Meter mới theo `unsent` của nó → đổi binder không để Gate pause kẹt (không lost wakeup).
- Không output sau `exit`: `Data` bị bỏ khi `exit_code` đã có; `exit` fanout tới mọi viewer; session `known=false` remove sau fanout.
- Replay ordering: hub chạy 1 luồng, `attach` gửi xong outs (ring snapshot) trước khi xử lý `Pty` event kế tiếp → không mất / lặp byte giữa replay và live.
- Attach `view` sau exit nhận replay + `exit`; reaper chỉ xoá session đã exit và `viewers` rỗng.
- Owner hand-off khi owner close / bị replace / chuyển sang session khác; `fit_owner` không resize sau exit; claim bị bỏ sau exit.
- Viewer `view` chậm không phải binder → chỉ nó 4001, `send()` overflow `bind(None)` nên Gate không bị pause vì nó.
- Tăng trưởng mỗi viewer: binary bị chặn ở `CONN_MAX`; `viewers` không trùng conn (`retain` trước `push`); text (`resized`, `cwd`, `branch`) không droppable nhưng tần suất thấp.
- `kill` từ viewer nào cũng được như cũ; `KillAll` / `shutdown` không đổi.
- `cargo test --locked -j 3 -p core-pty`: 42 pass.

## Vòng 2

`cargo test --locked -j 3 -p core-pty`: 45 pass (chạy lại). Không chạy core-bin / clippy.

Mục cũ:
- #1 cao (desktop mất backpressure khi phone là owner) — đã sửa: `Gate.holds` đếm hold, mỗi `Meter` giữ max 1 hold (`holding`), `queued` > `PAUSE_AT` lấy hold, `sent` < `RESUME_AT` / `bind(None)` / đổi bind trả hold; `rebind` bind mọi viewer `own` + owner, bỏ conn `overflowed`.
- #2 vừa (claim ngầm theo binary) — đã sửa: owner chỉ đổi bằng `{t:"claim"}`; contract auto-reply ghi trong report cho WP-6 / WP-7.
- #3 vừa (conn overflowed kẹt trong `viewers`) — đã sửa: `send` đẩy vào `spilled`, cuối `handle` lặp `leave_all` tới khi rỗng (overflow phát sinh trong lúc `leave` → `fit_owner` cũng được xử lý); `leave` chọn owner bỏ conn overflowed.
- #4 thấp (`resize_from` từ conn không phải viewer) — đã sửa (`viewers.rs:94` `else { return }`).
- #5 thấp (`c.session` kẹt sau `kill`) — đã sửa (`from_main.rs` `kill` xoá `c.session` mọi viewer đang trỏ session đó; viewer vẫn nhận `exit`).
- #6 thấp (owner phone ngừng đọc giữ pause) — giữ theo thiết kế, desktop `own` giờ cũng bind nên không còn 4001; lấy lại owner cần `{t:"claim"}` từ desktop (WP-7).

Lỗi mới:
- apps/core/crates/core-pty/src/hub/viewers.rs:47-53, :67 — vừa — owner `own` rời (pane desktop remount / reload: `Closed` của socket cũ tới trước `attach` của socket mới) mà còn phone `view` → `leave` giao owner cho phone và `fit_owner` kéo PTY về cỡ phone; socket desktop mới attach `own` không lấy lại owner (`takes` chỉ khi owner `None`), `ptyClient` hiện chưa gửi `claim`, nên `resize` của desktop sau `ready` chỉ được lưu → PTY kẹt ở cỡ phone trong khi xterm desktop ở cỡ của nó. Vòng 1 desktop gõ phím còn lấy lại được, vòng 2 thì không tới khi WP-7 xong. Không ảnh hưởng desktop khi không có phone. — Fix: `join` mode `own` lấy owner khi owner hiện tại là viewer `view` (hoặc `leave` chỉ giao owner cho viewer `own`, không có thì để `None`); nếu giữ thiết kế thì WP-7 bắt buộc gửi `{t:"claim"}` khi pane desktop được focus / gõ.

Đã kiểm thêm, ổn:
- `holds`: lock order luôn Meter → Gate, Gate không khoá ngược; `holding` nằm trong mutex của Meter nên `sent` (luồng writer) và `queued` / `bind` (luồng hub) không đếm trùng / trả trùng; `unhold` dùng `saturating_sub` và `notify_all` khi về 0; `acquire` xét `paused || holds > 0`; `set_paused` chỉ còn test gọi → không pause kẹt, không lost wakeup.
- Overflow: `send` `bind(None)` trả hold trước khi xếp `Close(4001)`; `Closed` `bind(None)` rồi `leave_all`; session drop `gate.close()` thả reader dù Meter còn trỏ Gate cũ.
- Desktop 1 socket: chỉ 1 Meter bind, hysteresis 512 KiB / 128 KiB như cũ; input không còn đi qua claim; `kill` qua socket tạm (`ptyClient.kill`) giờ xoá `c.session` của socket pane nhưng pane vẫn nhận `exit`, không thêm / bớt message nào.
