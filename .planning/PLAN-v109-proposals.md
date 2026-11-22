# PLAN v109: 12 đề xuất UI/UX + chức năng

Chủ nhân chọn 2026-11-22 từ `.planning/wireframes/v109-proposals.html`: **Làm cả 12**. Chi tiết vấn đề / file chạm: `.planning/RESEARCH-v109-proposals.md`. Wireframe Trước / Sau trong trang đề xuất là bản chủ nhân đã duyệt; gói nào đổi bố cục khác wireframe thì làm wireframe mới trong `.planning/wireframes/` cho chủ nhân xem trước.

## Ràng buộc
- Chỉ làm cho Tauri `apps/shell` + Rust `apps/core` + UI `apps/web` (classic và UI next). KHÔNG thêm code vào Electron `apps/desktop` main / `apps/collector`: sắp bỏ theo `PLAN-v110-modules.md`.
- Code mới đặt theo hướng v110: file mới của 1 tính năng gom vào `apps/web/src/features/<feature>/` khi tạo mới hoàn toàn; sửa file cũ thì sửa tại chỗ.
- Test tự động có assert (unit + E2E), thêm vào `E2E_MAP` của `scripts/qa-fast.mjs`; chỉ chụp màn khi FAIL.
- Không comment trong code; i18n en + vi; token màu; không "—" / "·" trong chữ UI.

## Gói
| WP | Model | Mục | Vùng chính | Chờ |
|---|---|---|---|---|
| F1 | sonnet | A1 menu project gọn | `lib/rail/railMenu.ts`, `LiveRail.vue` | WP-D (rail) |
| F2 | sonnet | A2 gộp thông báo + A3 badge taskbar / Dock | `useNotifications.ts`, `apps/shell` window | - |
| F3 | sonnet | A4 Ctrl+Tab MRU + B4 đổi phím tắt | `workspaceKeys.ts`, shortcut registry, Settings | - |
| F4 | sonnet | B1 quota dự báo giờ hết | `quotaBar.ts` | - |
| F5 | opus | B2 góp ý trên diff, điền sẵn cho agent | ChangesPane / diff, prefill pane | - |
| F6 | opus | B3 shell báo lệnh dài xong (OSC 133) | `core-pty`, TermPane | - |
| F7 | opus | C1 chặn lộ secret trước commit / push | `CommitBox.vue`, `mask.rs`, core-desk git | - |
| F8 | opus | C2 xem dạng chat trên desktop | `core-transcript` chat, pane mới | - |
| F9 | sonnet | C3 chế độ chia sẻ màn hình | lớp che path / email / mã ghép | - |
| F10 | opus | C4 tìm trong mọi terminal | `TermFind`, Ctrl+K | - |
| F11 | sonnet | Footer % CPU + % RAM máy, khác kiểu quota (chủ nhân 2026-11-22), wireframe `.planning/wireframes/v109-footer-sys.html` | `NextQuotaBar.vue` / footer classic, sampler Rust | chủ nhân chọn kiểu A / B / C |

Thứ tự lượt (max 6 subagent cùng lúc, cùng working tree): lượt 1 = F2, F3, F4, F7 khi 5 gói hiện tại (R1, R2, T, D, L) xong bớt; lượt 2 = F1, F5, F6, F8; lượt 3 = F9, F10. Sau v109 mới chạy P1 / P3 của v110 (dời file).

## Remote (chủ nhân chốt 2026-11-22)
- Điện thoại KHÔNG BAO GIỜ đổi cỡ PTY của PC (kể cả khi gõ); `pty.resize` từ điện thoại bị bỏ qua. 1 PTY chỉ có 1 cỡ, Claude vẽ theo cỡ đó.
- Terminal trên điện thoại: thu nhỏ vừa bề ngang, không cuộn ngang, pinch zoom, double tap về vừa ngang. Đọc / trả lời thoải mái dùng chat (mặc định cho Claude / Codex).
- Bỏ nút "Vừa màn" và "Desktop / Đọc" ở cả web app và Android.
- Relay iPhone cần bundle mới (`apps/shell/resources/relay/`), chủ nhân tự bấm cập nhật relay trong Settings.

## Tiến độ
- [x] F2 (gộp thông báo + badge, E2E 13/13), F11 (chip CPU/RAM kiểu C, E2E 30/30), theme mở lại (E2E 37/37), kéo sắp xếp project (E2E 30/30), Android remote lượt 1.
- [x] R1 (relay bundle `49e0f97bdb57be75`, gateway không resize, E2E remote 26/26), R2 (378 test), F1 (E2E 7/7), F3 (E2E pass), F4 (unit 22/22, E2E chưa chạy), F5 (E2E 25/25), F7 (E2E 22/22).
- [x] F6 (OSC 633 + B3, E2E 16/16), F8 (chat desktop, E2E 32/32), F9 (share mode, E2E 32/32), F10 (tìm mọi terminal, Ctrl+Alt+F, E2E pass).
- [ ] Đang chạy: P0.5 parity Rust, G0 sửa test lẻ.
- Gate cuối: build lại `apps/core` release (`orrery-core.exe` release đang cũ 15/11, Tauri debug mặc định dùng nó) + shell + web, rồi E2E map của v109 + `scripts/qa-remote.mjs` + `npm run qa`. `v73.cjs` bước shell marks còn SKIP trên Tauri, bật lại.
- [ ] Gate cuối cần soát: E2E PWA chờ `palette-nebula` (nebula sang legacy), `packages/shared/src/remote.ts` CRLF, `useProjectRoots.test.ts` timeout, `tips.test.ts`, `core-desk` telegram `episodes_go_to_the_topic_with_buttons_only_when_receiving`, E2E F4 `v103-rail.cjs`, `npm run qa` full + build app desktop Tauri + `scripts/qa-remote.mjs`.
- [ ] Lượt 1
- [ ] Lượt 2
- [ ] Lượt 3
- [ ] Gate full: `npm run qa`, E2E map, build app desktop (Tauri)
