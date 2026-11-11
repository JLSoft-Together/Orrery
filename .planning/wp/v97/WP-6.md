## WP-6: Gateway — PTY cho điện thoại, lệnh Workspace, xác nhận trên điện thoại, keep awake, chống throttle
- Mục tiêu: điện thoại xem / gõ được terminal của pane desktop và điều khiển Workspace qua gateway: `pty.attach/detach/resize` + kind 1 / kind 2 (+ flag `claim`) nối vào hub PTY bằng viewer `mode:"view"`; `ws.*` → renderer qua `Shell::ask("ccm:orrery", source 'phone')`; `ws.snapshot` / `ws.patch` từ `ccm:remote-publish`; `confirm.ask` / `confirm.answer` cho lệnh từ điện thoại; keep awake khi bật remote; giữ renderer chạy đúng nhịp khi cửa sổ ẩn xuống tray; sửa 5 lỗi thấp còn lại của review WP-5.
- Spec item: PLAN-v97-mobile.md T6, T7, H1, H4, H6, H7, H8; review #2, #10, #11, #12, #24; `docs/remote-integration.md` §3, §4.3, §4.4, §4.6; `.planning/wp/v97/WP-5-review.md` mục "Vòng 2" (5 lỗi thấp mới); `.planning/wp/v97/WP-7-report.md` (hợp đồng renderer: kết quả `{ok}` / `{needsConfirm}`, `confirmed` chỉ gateway đặt).
- Phạm vi file: `apps/shell/src/remote/**` (thêm `pty_bridge.rs`, `workspace.rs`, `confirm.rs`, `awake.rs`, test), `apps/shell/src/ccm.rs` / `main.rs` / `state.rs` chỉ nối dây, `apps/shell/src/pty.rs` (thêm origin của gateway vào `init.origins` + hàm lấy endpoint cho gateway), `apps/shell/src/hosts.rs` (KHÔNG `show_main` khi confirm đến từ lệnh điện thoại — chỉ nếu cần), `apps/web/src/**` CHỈ cho chống throttle (giữ 1 Web Lock pending khi `remote.connected > 0`, trong `useRemoteSummary` hoặc composable mới nhỏ) — ghi rõ, `.code_index/shell.md`. | Cấm đụng: `apps/core/**` (hub đã hỗ trợ viewer / claim / resized — WP-3), `apps/mobile/**` (WP-11 đang làm), `apps/relay/**`.
- Phụ thuộc: WP-3, WP-5, WP-7 (xong). | Song song với: WP-11 | Worktree: không.
- Model: opus (concurrency, auth đường lệnh).

### Thiết kế
- PTY: gateway mở 1 WS client tới `ws://127.0.0.1:<p>/pty` (`Shell.pty.endpoint()`, subprotocol token, Origin riêng `orrery-remote://gateway` thêm vào `init.origins`) **cho mỗi attach của điện thoại**, gửi `{t:"attach", id: sessionId, mode:"view", cols, rows}`; output binary → kind 1 trên `stream` của điện thoại (qua `Chunker`, gộp ≤ 30 ms); kind 2 từ điện thoại → binary vào WS, flag `claim` → gửi `{t:"claim"}` trước; `pty.resize` → `{t:"resize"}`; `resized` → event `pty.resized`; `exit` → `pty.exit`; close 4001 → `pty.overflow`. Max 4 attach / thiết bị. Phiên điện thoại đóng → đóng mọi WS PTY của nó. `sessionId` phải thuộc pane có trong Workspace summary mới nhất (không cho attach id tuỳ ý).
- Workspace: `ws.get` → summary mới nhất (`ws.snapshot`); mỗi `remotePublish` mới → diff JSON Patch với bản trước (crate nhỏ hoặc tự viết, test) → `ws.patch {rev}` cho mọi phiên; phiên mới / lệch rev → `ws.snapshot`. `ws.*` khác → `Shell::ask` chạy trong `spawn_blocking`, id do gateway sinh, timeout 5 s → `{re, ok:false, error:"timeout"}`; renderer chưa lên → `offline`. Rate limit `ws.*` 30 / phút / thiết bị.
- Confirm: renderer trả `{needsConfirm}` → gateway tạo `cid` 1 lần, event `confirm.ask` CHỈ tới thiết bị gửi lệnh; `confirm.answer {cid, ok:true}` trong 60 s → gửi lại lệnh với `confirmed: true` (chỉ gateway đặt; `confirmed` do điện thoại gửi bị bỏ); quá hạn / phiên đóng → huỷ.
- Keep awake: remote bật + `keepAwake` → gọi module power của core-desk (`core-desk/src/power/awake.rs`, qua `desk::call_inside` kênh power có sẵn hoặc API trực tiếp); tắt khi remote tắt / app thoát.
- Chống throttle (H1 / H6): renderer giữ 1 `navigator.locks.request('orrery-remote-awake', …)` pending khi có thiết bị kết nối, nhả khi không còn; thêm vào WebView2 `additional_browser_args` `--disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows` chỉ khi remote đang bật lúc tạo cửa sổ (đổi cần khởi động lại cửa sổ — chấp nhận, ghi chú trong Settings bởi WP sau nếu cần). Gửi `\r` của `ws.send submit` vẫn do renderer (TermPane) làm; nếu đo thấy trễ thì để việc tồn.
- 5 lỗi thấp WP-5: `register` relay chạy nền (không giữ lock `running`, không chặn bật / thoát); macOS giữ `SO_REUSEADDR` (chỉ Windows dùng `SO_EXCLUSIVEADDRUSE`); `is_hidden` lọc thêm U+2028 / U+2029 + mọi ký tự General Category `Cf`; giới hạn thử ghép qua relay tính theo IP do relay báo (nếu envelope có) hoặc toàn cục 20 / phút cho đường relay; rebind cập nhật `error` trong status.

### Tiêu chí nghiệm thu
- [ ] `cargo test --locked -j 3 remote` + toàn shell pass; test mới: điện thoại attach pane qua hub thật (spawn `orrery-core` pty như test hiện có của shell nếu có, hoặc hub giả) nhận replay + output; desktop socket `own` không bị đóng khi điện thoại attach; kind 2 + claim → hub nhận claim; attach `sessionId` không có trong summary → `notFound`; `ws.*` timeout 5 s; confirm 1 lần, quá 60 s huỷ, `confirmed` từ điện thoại bị bỏ; JSON Patch diff đúng (snapshot + patch == snapshot mới) với ≥ 5 trường hợp.
- [ ] Renderer: Web Lock chỉ giữ khi có thiết bị (Vitest); typecheck web sạch.
- [ ] 5 lỗi thấp WP-5 đã sửa + test cho `register` không chặn và `is_hidden`.
- [ ] `cargo clippy` phần `remote/` sạch; `gen-desk-shim --check` sạch.
- [ ] Không comment trong code; file ≤ ~400 dòng; `.code_index/shell.md` cập nhật.

### Lệnh test
`cd apps/shell && cargo test --locked -j 3 remote && cargo test --locked -j 3` và `npm run typecheck -w @ccm/web && npm test -w @ccm/web -- --maxWorkers=2 <file test mới>`

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-6-report.md`

### Nhật ký
- 2026-11-12 WP-6: đọc brief + WP-3/5/7 report, review WP-5, gateway remote/**, pty.rs, hub frames; thiết kế: Board (summary + broadcast ws.patch có base), patch.rs, confirm.rs, pty_bridge.rs (WS client view tới hub, claim bit gỡ ở gateway vì proto crate từ chối flag ≠ more), awake.rs (browser args), Web Lock.
- 2026-11-12 WP-6: code xong (Board/patch/confirm/phone_ws/pty_bridge/phone_pty/awake + window/guest/pty.rs nối dây + 5 lỗi WP-5 + useRemoteAwake); cargo test remote 42 pass ×3, toàn shell 236 pass, clippy remote/ sạch, shim --check sạch, typecheck web sạch, Vitest 6 pass.
- 2026-11-12 WP-6 vòng 2: deadline 2 s + backlog 4 MiB cho bridge, idle 45 s LAN, ASK_WAIT 4 s + rid dedupe renderer, gen trên FromHub, Slot trong task; test remote 43, shell 237, Vitest 25 pass.
