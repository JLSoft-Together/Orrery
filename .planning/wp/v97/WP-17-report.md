# WP-17 report

Trạng thái: DONE

## Đã đổi
- `core-remote-proto`: `FLAG_CLAIM`, `Frame.claim` (chỉ kind 2, encode / decode kiểm), `Chunker::claiming`, `Message.claim` (Reassembler giữ theo stream), `relay::encode_close(conn, code)`. Bytes cũ giữ nguyên (vector pass). Test mới frame + relay.
- Shell `remote/`: bỏ `strip_claim` / `CLAIM_FLAG` / `claims`; `on_vevent` chỉ gom (`batch` + `flush_at`), nhánh `flush_due` trong select gửi (không sleep); `merge` cắt `fs.changed` ≤ 64 KB + `more:true`; `Board::has_session_fresh` chờ ≤ 2 s sau connect (`mark_connect`); `scope::owner`; `Hub::ask/answer` + `VEvent::Ask` + `Phone::on_ask` + `confirm_answer`; `relay.rs` gửi control close cho 4400 / 4401 / 4426. `host_run.rs`: `run.origin`, `run.confirm` có `origin` → hỏi điện thoại.
- `core-desk run/sessions.rs`: `Inner::origin`, `confirm_from`, `SessionHooks.origin` (reinstall từ runner thread vẫn hỏi đúng điện thoại).
- `mobile/rust`: `seal` dùng `Chunker::claiming`; fake gateway test đọc `Message.claim`.
- Relay `room.ts` `desktopControl` (+ `close.ts`, `envelope.ts`): desktop gửi `connId=0 {t:"close",conn,code}` (code 4400/4401/4426/4503) → đóng đúng 1 phone. KHÔNG deploy.
- Docs `docs/remote-integration.md`, `.code_index/{core,shell,_meta}.md`.

## Tiêu chí 5/5
1. reinstall từ phone → confirm.ask: test shell `a_run_confirm_from_a_phone_command_goes_to_that_phone...` (Hub + WS thật); từ desktop (không origin) vẫn `hosts::confirm`. CHƯA KIỂM: đường `host_run::run_confirm` với AppHandle thật và test core-desk cho `Inner::origin` (không có fixture Inner / app).
2. proto 40 pass, shell 259, mobile rust 14.
3. relay test close 1 phone (4401 / 4426, phone khác không ảnh hưởng, code / conn sai bị bỏ) + shell `a_failed_handshake_over_the_relay...`; `npm test` 34 pass, tsc sạch.
4. `on_vevent` không sleep; test `one_merged_event_stays_under_64_kb...`.
5. Không comment; docs + index xong.

## Audit / Review
- Ask chặn runner thread tối đa 60 s, thoát sớm khi phiên đóng (poll 250 ms). Confirms max 4: ask bị đẩy ra sẽ timeout (coi huỷ).
- `confirm.ask.re` = `"run.reinstall"` (không trùng id lệnh): WP-12 cần chấp nhận re lạ.
- Stale summary: mỗi connect đặt stale 10 s; pane không tồn tại chờ tối đa 2 s mới `notFound`.
- Mục thấp WP-6 "browser args OnceLock" chưa làm.

## Test
cargo -p core-remote-proto 40 · core-desk run 89 · shell 259 · mobile/rust 14 · relay vitest 34 + tsc.

## Ngoài phạm vi
Web renderer publish ngay khi connected 0 sang >0; `awake.rs` OnceLock; mobile Dart xử lý `confirm.ask` có `re` lạ.

## Commit message đề xuất
feat(remote): claim bit in the proto Frame, non-sleeping vevent batching with a 64 KB cap, wait for a fresh summary after reconnect, ask the phone for run reinstall confirms, relay control close for a single phone
