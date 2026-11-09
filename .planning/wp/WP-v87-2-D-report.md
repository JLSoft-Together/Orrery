## WP-v87-2-D — DONE (Windows chạy thật; macOS mới type-check + clippy, CHƯA CHẠY)

- Đã đổi (chỉ `apps/core/**`, `.code_index/core.md`; không thêm crate, `Cargo.lock` không đổi):
  - `core-pty/src/ring.rs` (mới): ring byte cố định 256 KiB, replay bỏ ≤ 3 byte continuation đầu khi đã từng mất byte (bắt đầu đúng biên UTF-8).
  - `flow.rs` (mới): `Gate` theo session (reader thread chặn khi socket backlog > 512 KiB cho tới < 128 KiB, hoặc inflight trong channel `Event` > 256 KiB), `Credit` trả khi hub xử lý xong, `Meter` theo conn (hub cộng, writer trừ sau khi `sink.send` xong = analog `bufferedAmount`), binary vượt 8 MiB chưa gửi thì bỏ + log 1 lần. Stdin control có credit 4 MiB, WS inbound semaphore 2 MiB.
  - `owners.rs` (mới): Toolhelp32 (Windows) / `/bin/ps` (unix) chạy trong `spawn_blocking`, cache 1.5 s, mỗi đợt quét 1 lần, `owners_of` y `owners.ts` (sâu 32). Chọn tự viết thay `sysinfo`: không thêm crate, dữ liệu giống Node, `read_pid_table()` là chỗ thay bằng `core-procs` sau.
  - `hub.rs`: replay trong attach theo thứ tự `ready, cwd, replay, branch, exit`; chuyển `Meter` khi replace / socket đóng (bỏ pause); `MAX_LIVE` 16 → `error limit` + `removed`; reaper 60 s / 30 phút, clock giả được (`set_clock`); `Session` drop → đóng gate. `ws.rs` writer trừ meter, `server.rs` tick reaper + credit stdin. `session.rs` + `pty_win.rs` + `pty_unix.rs`: `PtyEvent::Draining` gửi ngay khi child thoát (đọc nốt dù đang pause).
  - Test: `core-bin/tests/flow.rs` (mới, 8 test + 2 test đo `#[ignore]`), unit trong ring / flow / owners / hub.
- Tiêu chí 5/5: ring wrap / UTF-8 / tail > 256 KiB / attach sau exit / 4000 (unit + `flow.rs`); backpressure 8 MiB; census (có sẵn, parity FULL xanh), owners 3 cấp, reaper bằng clock giả, MAX_LIVE 16; đo xong; test + clippy sạch, `core.md` cập nhật.

| Số đo (Windows 11, debug build) | Kết quả |
|---|---|
| Child xả 8 MiB, client không đọc | child dừng ở 656 KB / 8 MiB; đọc tiếp thì chạy tới hết |
| RSS core trong lúc đó | base 7.6 MB → peak 8.6 MB (+1.1 MB); đủ 69906 / 69906 dòng, đúng thứ tự |
| RSS `orrery-core`, 8 pane `cmd` | idle 6.0 MB → 9.5 MB (cổng ≤ 20 MB) |
| Echo 1 byte qua WS, n = 200 | p50 10.28 ms, p90 10.89 ms, max 26.5 ms |

- Echo p50 10.3 ms vượt mục tiêu 5 ms của E2, nhưng số đo này gồm cả vòng ConPTY / conhost + child node, không chỉ phía host. Đường đi của core không có timer nào (flush leading-edge, nodelay). CHƯA so với Node: script đo node-pty của con bị treo nên đã bỏ. E2 cần đo phía host / so cùng điều kiện.
- Parity FULL (`CCM_PARITY_FULL=1 CCM_CORE_BIN=…\target\debug\orrery-core.exe`, `--maxWorkers=2`): 34 pass / 1 skip × 2 lần (skip = placeholder "không có CCM_CORE_BIN"). 5 kịch bản `needs D` giờ xanh. Không sửa harness.
- Audit: không comment / debug print (chỉ `eprintln!` log theo quy ước + dòng in số đo trong test); không đụng ngoài phạm vi; port 0, thư mục tạm; chỉ dọn process tự tạo (script baseline đã dừng bằng task, kiểm không còn process sót).
- Review edge case: attach giữa lúc backpressure → bỏ bind meter cũ, gate mở, meter mới tính backlog replay (unit `unbinding_…`, test 4000); socket đóng khi đang pause → `Closed` bỏ bind, child chạy tiếp vào ring (`socket_closed_while_paused…`); child thoát / bị kill khi đang pause → `Draining` đọc nốt, `exit` tới sau backlog (`kill_while_paused…`); ring wrap giữa ký tự → bỏ continuation (unit 12 vị trí cắt + integration 600 KB đa byte). Rủi ro còn lại: socket cũ bị replace mà client không bao giờ đọc thì backlog ≤ ~0.8 MB của nó ở lại tới khi TCP đóng (Node cũng vậy). Control `Exit` / timer / Open / Closed không có credit (mỗi session / conn 1 cái).
- Test: `cargo test --locked -j 4` → 63 pass (unit 34, flow 8 + 2 ignored, pty 16, spike 1, stub 4). `cargo clippy --locked -j 4 --all-targets -- -D warnings` sạch trên Windows và `--target aarch64-apple-darwin`. Test đo: `cargo test -p core-bin --test flow -- --ignored --nocapture`.
- Ngoài phạm vi: CHƯA CHẠY trên macOS (owners `ps`, Draining unix, flood qua openpty). Đo RSS / echo bằng release build và so với Node để dành cho E2. Khác Node có chủ ý: ring tính theo byte (256 KiB) thay vì 256 K ký tự UTF-16 cắt ở 1.5×, nên độ dài replay khác nhưng phần đuôi giống; binary bị bỏ khi conn có > 8 MiB chưa gửi (Node để tăng không giới hạn).
- Commit message đề xuất: `🦀 core-pty: 256 KiB byte ring + UTF-8-safe replay, byte-credited backpressure (512/128 KiB socket pause, bounded event/inbound/control queues), owners via Toolhelp/ps with 1.5 s cache, MAX_LIVE 16, 30 min reaper; flow + parity tests`

## Vòng 2 — sửa theo review (XONG)

- #1 `hub.rs` `send`: lần đầu một binary làm socket vượt 8 MiB chưa gửi → đánh dấu `overflowed`, gỡ meter (pane không bị kẹt pause), gửi close **4001 `overflow`**, từ đó không gửi gì nữa cho socket này (không còn lủng giữa luồng byte; client attach lại thì nhận replay từ ring). `Draining` cố ý KHÔNG chặn khi unsent ≥ CONN_MAX: nếu chặn ở đó, khi child thoát thì đọc nốt sẽ quá 200 ms, `exit` đi trước và phần đuôi bị bỏ; close 4001 đã che trường hợp này. Đã ghi vào `core.md`.
- #2 `owners.rs`: `/bin/ps` chạy qua `run_with_deadline` (15 s, vòng `try_wait`, quá hạn thì kill và trả `None`; stdout đọc ở thread riêng); `scanned()` vẫn được gọi với `None` → `owners: null`.
- #3 `hub.rs` `attach` (dùng cho cả spawn lẫn attach): session khác còn trỏ conn này thì đặt `conn = None`; output / `exit` muộn của pane cũ (vd đã kill) không lọt vào socket của pane mới. Khác Node có chủ ý (Node để lọt), ghi `core.md`.
- Test mới: `backlog_over_conn_max_closes_4001_stops_output_and_reattach_replays`, `reused_conn_stops_getting_output_and_exit_of_its_previous_session` (hub, process thật + event bơm tay), `table_command_is_killed_at_its_deadline` (owners; Windows dùng powershell Start-Sleep, unix /bin/sleep).
- Test: `cargo test --locked -j 4` → 66 pass (unit 37, flow 8 + 2 ignored, pty 16, spike 1, stub 4). Clippy sạch Windows + `aarch64-apple-darwin`. Parity FULL 34 pass / 1 skip (placeholder).
- Ngoài phạm vi (main nối ở E2/F): renderer chưa xử lý close **4001** → cần attach lại theo id (như khi reload) để lấy replay.
