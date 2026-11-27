## WP-113r1 — DONE

### Bảng finding
| Mức | File:line | SỬA / BỎ | Test |
|---|---|---|---|
| V1 | `ctx.rs` probe (dòng 240) | SỬA: probe chừa 1 chỗ cuối kênh `ctl` | `unit_tests::probes_never_take_the_last_slot_so_a_revoke_or_hold_always_reaches_the_session` |
| V2 | docs ngoại lệ 8 | BỎ: main làm | không |
| V3 | `viewer_map.rs` (`Spec::charges_write`), `viewer.rs:97`, `phone_ws.rs:279` | SỬA: `fs.put` seq 0 tốn quota ghi | `fs_put_tests::a_new_upload_counts_against_the_write_quota_but_its_later_chunks_do_not` |
| T1 | `pair.rs:328` | SỬA: `forget` sau `added` | `pair_tests::a_failed_save_while_pairing_keeps_the_pending_revoke_of_that_device` |
| T2 | `ctx.rs:200`, `push.rs:235` | SỬA: `is_held` dưới lock `live`; `still_wanted` loại held | `push_tests::a_device_waiting_for_its_revoke_gets_no_push_and_cannot_join_late` |
| T3 | `danger.rs:41` | SỬA: làm sạch ký tự điều khiển trước khi so | `danger_tests::control_characters_cannot_hide_a_dangerous_command` |
| T4 | `install_http.rs` | SỬA một phần: từ chối symlink; BỎ so sha lúc phục vụ (ticket không giữ sha, hash tới 1 GB mỗi request; phone vẫn kiểm sha) | `install_tests::a_symlink_in_place_of_the_apk_is_never_served` (bỏ qua nhánh symlink nếu Windows không cho tạo) |
| T5 | `install_pair.rs:36`, `pair.rs` (`holds_non_lan`), `install_tickets.rs` (`release_pair`) | SỬA: `busy` khi slot USB / QR, ticket không bị đốt | `install_page_tests::the_pair_route_never_replaces_a_running_usb_or_qr_pairing` |
| T6 | `run_log.rs` | SỬA: cắt theo byte (700 KB), `more` | `run_log_tests::multibyte_logs_are_cut_by_bytes_so_a_window_never_exceeds_the_reply_limit` |
| T7 | `mod.rs` `run_event` | SỬA một phần: chỉ ghi khi Remote chạy; BỎ max tổng byte store (đã có max 24 x 2000 x 1000 ký tự) | `unit_tests::run_logs_are_recorded_only_while_remote_runs` |
| T8 | `fs_put.rs` | SỬA: max tổng 32 MiB | `fs_put_tests::open_uploads_together_never_hold_more_than_the_total_byte_max` |
| T9 | `dashboard_ping.rs:49` | BỎ: cần seq trong Ping (đổi wire core-remote-proto + mobile), hậu quả 1 RTT sai | không |

### Audit
- Không thêm comment. `ctx.rs` 398 và `mod.rs` 470 giữ nguyên số dòng (sửa tại chỗ). File mới/đổi đều < 400 dòng.
- Kiểm fail-first: tạm bỏ fix V1 và T3, 2 test fail (`probes_never...`, `control_characters...`), khôi phục thì pass. Các test còn lại viết cùng lúc với fix, chưa chạy bản fail riêng (CHƯA KIỂM fail-first cho V3, T1, T2, T5, T6, T7, T8).
- Lock order `admit_live`: devices, live, hold (mutex riêng); `revoke_later`: hold rồi kick(live), không vòng.
- Không build, không E2E.

### Test
`cd apps/shell; cargo test --locked -j 4 remote` → `test result: ok. 366 passed; 0 failed; 3 ignored`

### Hot file cần thêm
- `.code_index` module remote: `Spec::charges_write`, `Pairing::holds_non_lan`, `Tickets::release_pair`, `install_http::servable`, `run_log::within_bytes`, `fs_put::MAX_TOTAL_BYTES`; ghi 1 dòng `_meta.md`.
- `docs/remote-integration.md`: `fs.put` seq 0 tính quota ghi 20/phút; `/install/<t>/pair` trả 403 khi đang ghép USB / QR; `run.log` cắt theo 700 KB; luật nguy hiểm so trên text đã bỏ ký tự điều khiển; thiết bị đang chờ thu hồi không nhận push.
- Ghi T9 vào `.code_index/issues.md` (Ping không có seq).

### Commit message đề xuất
fix(remote): keep revoke commands from being dropped, charge fs.put to the write quota, harden install pairing, danger rules, apk serving and log/upload limits
