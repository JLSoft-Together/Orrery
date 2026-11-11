# HANDOFF 2026-11-12 — v97 Orrery Remote, xong lượt 1

## Đang ở đâu
- Branch `feat/v97-mobile` (đã push). Plan `PLAN-v97-mobile.md` (+ §6 chỗ hổng đã rà), spec `SPEC-v97-remote-control.md`, quyết định `DECISIONS.md` mục v97, hợp đồng `docs/remote-integration.md`, tiến độ `PROGRESS-2026-11-12-v97.md`, brief + report `.planning/wp/v97/`, pipeline `.planning/pipeline.json` (wave1 → wave5).
- Chủ nhân đi ngủ, cho chạy liên tục: xong lượt → commit + push branch + handoff + Telegram, tự làm lượt tiếp; cổng cảm quan con tự chọn theo số đo; chỉ dừng ở Firebase (push) và iOS trên MacBook.

## Bảng WP
| WP | Trạng thái | Commit | Ghi chú |
|---|---|---|---|
| WP-0 hợp đồng | ĐẠT | 0cf1b3ee | docs + `packages/shared/src/remote*.ts` |
| WP-1 spike Flutter | CHỜ XEM | 8bf21bc0 | `apps/mobile`: FRB 2.13, xterm, cleartext, allowBackup off; APK cài lên A21s nhưng máy khoá PIN → chưa đo FPS / RAM / Telex, chưa ảnh. Tạm chọn xterm.dart (PC: 256 KiB 55 ms) |
| WP-2 core-remote-proto | ĐẠT | 0ea61082 | XXpsk3 + IK + frame + token; msg1 có tiền tố proto; 37 test |
| WP-3 core-pty nhiều viewer | ĐẠT (3 vòng) | 8adb3847 | `attach {mode:"view"}`, `claim`, `resized`; Gate nhiều Meter; owner chỉ đổi bằng `claim`; 46 + 28 test, opus review ĐẠT |
| WP-9 relay | ĐẠT | 1dbf3971 | `https://orrery-relay.jinvu.workers.dev`, 33 test + smoke live; root `package.json` loại `!apps/relay` khỏi workspaces |
| WP-4 StoreTap | GIAO (đang chạy) | — | `core-serve` `run_with_tap`, `StoreTap::{snapshot, subscribe, attach, send}`; `.code_index/core.md` đang chứa cả phần index của WP-3 chưa commit → commit cùng WP-4 |

## Bước tiếp (lượt 2 → 3)
1. Nghiệm thu WP-4 → commit (kèm `.code_index/core.md`, `apps/shell/src/serve.rs`).
2. Giao WP-1b (spike throttle, brief sẵn), WP-10 (mobile conn layer — viết brief: FRB gọi `core-remote-proto`, vector `tests/vectors.json`, LAN mDNS + QR + IK, relay `/p/<id>?t=`, reconnect, seq resume, cache offline, nhiều máy).
3. Lượt 3: WP-5 (gateway LAN, opus) → WP-6 ‖ WP-7 ‖ WP-8 (brief theo bảng PLAN §4 + §6 H1–H16).

## Treo cho chủ nhân (sáng)
- Mở khoá A21s (PIN) để con đo xterm.dart + chụp ảnh WP-1.
- wrangler có 3 account; relay deploy lên account Doanvu20122000@gmail.com — đúng account chưa?
- Clippy toàn `-p core-bin` fail vì lỗi sẵn có trong `core-desk` (không do v97).
