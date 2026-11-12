# HANDOFF 2026-11-12 (trưa) — v97 Orrery Remote: tự dựng relay + push, đổi tên, QA thật trên A21s

## Đang ở đâu
- Branch `feat/v97-mobile` đã push (commit cuối 0b1ce358). Bản trước: `.planning/archive/HANDOFF-2026-11-12-v97-done.md`.
- Thiết kế đổi (chủ nhân 2026-11-12, `DECISIONS.md` 4 dòng cuối): mỗi người tự dựng relay (dán Cloudflare API token trong Settings, Orrery deploy bundle `apps/shell/resources/relay/worker.js` qua Cloudflare API) và push (thả `google-services.json` + service account; desktop gửi FCM HTTP v1 thẳng, điện thoại init Firebase lúc chạy từ `push.config`). Chưa dựng relay ⇒ chỉ LAN. Relay jinvu là relay riêng của chủ nhân (deploy lại 2026-11-12, `account_id` ghim trong `wrangler.jsonc`, secret Firebase đã xoá khỏi relay).

## Gói đợt này (brief / report / review trong `.planning/wp/v97/`)
| WP | Việc | Commit |
|---|---|---|
| 14 | App: push FCM init runtime, register / tap / quyền / máy khác Firebase | cee32555, 1d6606d2 |
| 19 | Desktop tự deploy relay qua CF API (review opus: sửa 3 lỗi vừa) | cee32555 |
| 20 | Desktop gửi FCM thẳng, trigger từ StoreTap working→waiting/idle (review opus: sửa lỗi cao) | cee32555 |
| 21 | UI Settings dựng relay + thả file Firebase | 1433e232 |
| 22 | QA thật A21s: 7/8 ĐẠT (`docs/qa/v97/WP-22/RESULT.md`) | f347a0e2 |
| 23 | Đổi tên điện thoại (desktop), máy tính + session + tab (điện thoại) | 7df2c92c |
| 24 | SA Firebase niêm phong ra file (Credential Manager max 2560 byte); push thử hiện trên A21s | b4897a25 |
| 25 | Font terminal Cascadia Mono (box nối), ghi PTY 1 lần / frame, tên model khi ghép, log đổi tên session | 0b1ce358 |

## Còn mở
- Mất dòng khi vuốt lúc output chảy: chưa tái hiện lại; nghi `pty.overflow` → reattach xoá scrollback (WP-25 report). Cần đo bản profile / release trên A21s.
- Push `done` chưa thấy trên máy thật (cần pane Claude chạy thật). Push thử đã ĐẠT.
- Luồng tự dựng relay chưa chạy với Cloudflare thật (cần API token thật của chủ nhân — Settings → Điều khiển từ xa).
- Relay jinvu giữ vài machine id dev do QA tự đăng ký (vô hại, chưa có API gỡ).
- iOS: build trên MacBook khi chủ nhân báo (chưa có `scripts/ios.command`; Firebase iOS cần `GoogleService-Info.plist` → hiện app chỉ nhận options Android; APNs key).
- Việc tồn nhỏ cũ: browser args WebView2 chọn 1 lần lúc mở app; clippy 8 lỗi sẵn ngoài v97; `apps/shell/src/remote/mod.rs` ~425 dòng.

## Thiết bị
- A21s `192.168.1.217:<port Wireless debugging>` (port đổi sau mỗi lần bật lại). Đang cài APK debug arm64 có font mới; ô "Nhập mã ghép" chỉ có ở debug. Script QA: `apps/shell/e2e/v97-device.cjs`.
