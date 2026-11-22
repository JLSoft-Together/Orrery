# core-toast-bridge: toast, alert, statusline / hook bridge

Đường vào: `apps/core/crates/core-toast/src/toaster/mod.rs` (`Toaster`), `apps/core/crates/core-bridge/src/main.rs` (bin `orrery-bridge`).

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-toast/src/toaster/observe.rs` | `observe`: waiting, reminder, done, stuck, context >= 95 %, jobs blocked, limit hit, forecast 5 giờ |
| `apps/core/crates/core-toast/src/toaster/extra.rs` | error loops, sensitive, disk, note agent, idle |
| `apps/core/crates/core-toast/src/toaster/shell.rs` | toast lệnh shell xong / lỗi (OSC 633), gộp 8 s |
| `apps/core/crates/core-toast/src/message.rs` | `ToastContent`, `PhoneEpisode`, `TelegramEpisode` |
| `apps/core/crates/core-toast/src/text.rs` | chuỗi en / vi |
| `apps/core/crates/core-bridge/src/statusline.rs` | statusline Claude Code -> ghi file cho monitor |
| `apps/core/crates/core-bridge/src/hook.rs` | hook Claude Code -> ghi file sự kiện |
| `apps/shell/src/toast.rs`, `menu_toast.rs`, `notice.rs` | hiện toast OS, toast có nút trả lời menu |
| `apps/shell/src/attention.rs`, `badge.rs`, `mac_note.rs` | Dock bounce / badge, thông báo macOS |

## Bất biến

- Toast bị bỏ khi `CCM_TOAST=0`, cửa sổ đang attentive hoặc quiet hours; throttle waiting 30 s.
- `orrery-bridge` chỉ ghi file vào `CCM_DATA_DIR`, không gọi mạng, không in nội dung session.
- Toast menu chỉ gõ phím sau thao tác của chủ nhân (ngoại lệ docs §11as).

## Liên kết

- Web: `web-small.md` (`click-menu`, `notify`). Cài bridge: `apps/core/crates/core-desk/src/bridge/install.rs`. Plan: `.planning/PLAN-v87-step5-*.md`.
