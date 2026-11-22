# core-desk-telegram: bot Telegram

Đường vào: `apps/core/crates/core-desk/src/telegram/mod.rs` (`register`, `handle`), host trong vỏ `apps/shell/src/host_telegram.rs` + `host_extra.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-desk/src/telegram/api.rs` | gọi Bot API (URL ghi đè bằng `CCM_TELEGRAM_URL_OVERRIDE` khi test, đọc ở `telegram/mod.rs`) |
| `apps/core/crates/core-desk/src/telegram/poll.rs` | long poll nhận update |
| `apps/core/crates/core-desk/src/telegram/inbound.rs` | tin nhắn vào -> pane (chỉ tài khoản đã pair) |
| `apps/core/crates/core-desk/src/telegram/outbound.rs` | gửi tin, topic theo project |
| `apps/core/crates/core-desk/src/telegram/buttons.rs`, `callback.rs` | nút `telegram_send`, callback gõ giá trị vào pane |
| `apps/core/crates/core-desk/src/telegram/commands.rs` | lệnh `/...` |
| `apps/core/crates/core-desk/src/telegram/report.rs` | báo cáo tự động |
| `apps/core/crates/core-desk/src/telegram/pane_link.rs`, `routes.rs` | link pane với topic |
| `apps/core/crates/core-desk/src/telegram/settings.rs` | cấu hình, token niêm phong |
| `apps/core/crates/core-desk/src/telegram/sanitize.rs` | làm sạch nội dung |
| `apps/core/crates/core-desk/src/telegram/mt/` | kết nối MTProto (`CCM_TELEGRAM_MTPROTO`) |

Phía vỏ: `apps/shell/src/episodes.rs` (episode Telegram / phone), `host_telegram.rs` (`telegram.runLog`, `telegram.ask`).

## Bất biến

- Chỉ tin nhắn từ tài khoản đã pair mới được submit vào pane; nút `telegram_send` do agent gắn thì chạm vào là gõ giá trị (ngoại lệ chủ nhân duyệt, docs §11an, `docs/core-principle.md`).
- Nội dung gửi ra qua mask secret; token không về trang.

## Liên kết

- Web: `web-remote.md` (`features/telegram`). Skill: `bot-tele-receive`, `bot-tele-send-notify`. Plan: `.planning/PLAN-v82-*.md`.
