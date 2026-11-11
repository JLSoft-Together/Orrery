## WP-16 — DONE (4/4 tiêu chí)

### Đã đổi
- `docs/claude-code-integration.md`: thêm mục §11as cuối phần §11 (sau §11ar): ngoại lệ nội dung session ra điện thoại + lệnh từ điện thoại, chủ nhân duyệt 2026-11-12, 6 điều kiện (ghép, Noise đầu-cuối, tắt mặc định, confirm trên điện thoại, hiện trên máy, limit), chỉ pane idle mới tự submit, không env / secret / app key / token PTY, danh sách "chưa có", file code chính, lệnh test.
- `docs/orrery-remote.md` (mới): tình trạng, bật + ghép, LAN / relay, firewall Windows / macOS, thu hồi, giữ máy thức, link Telegram / ntfy, bảng giới hạn, lỗi thường gặp, cài APK / iOS, dữ liệu.
- `.code_index/features.md`: thêm mục `## v97` 1 dòng tính năng. `.code_index/_meta.md`: 1 dòng.

### Tiêu chí 4/4
- [x] §11as nêu ngoại lệ, ngày duyệt, điều kiện, file code chính.
- [x] `orrery-remote.md` có port 4320, QR 5 phút, 30 ngày, max 16 phiên, max 4 attach, 60 s xác nhận.
- [x] `glossary.mjs check --all` sạch cho `docs/orrery-remote.md` và `docs/claude-code-integration.md`. (`features.md` còn 4 hit cũ ở dòng có sẵn, không phải dòng mới.)
- [x] Không bịa: push APNs / FCM = chưa có, iOS = chưa build, relay close-1-phone chưa deploy, assetlinks chưa kiểm máy thật, WP-1 / WP-15 chưa xong đều ghi rõ.

### Lưu ý cho main
- Trạng thái chốt theo report WP-1..WP-17 tới lúc viết; WP-15 chưa có report nên mục QA ghi "chưa".
- Số lệnh: hợp đồng ghi `ws.*` 30 / phút, code `limits.rs` đếm chung max 240 lệnh / phút; doc ghi cả hai. Main kiểm lại nếu muốn chỉ 1 số.
- Settings chưa báo "bật remote giữa phiên cần khởi động lại" (ghi trong mục 6).

### Đoạn đề xuất thêm vào CLAUDE.md (mục "What this is", cuối danh sách ngoại lệ)
Sixth exception (owner, 2026-11-12): since v97 the Flutter app Orrery Remote (`apps/mobile`, Android first) is a second frontend: after a paired, end-to-end encrypted (Noise) connection over LAN port 4320 or the Cloudflare relay (`apps/relay`) it receives session contents and terminal bytes and may open / close panes, type, edit files and run git / run commands; a pane is submitted only while idle; env, secrets, app key and PTY tokens never leave the machine; remote control is off by default and dangerous actions confirm on the phone (docs §11as, `docs/orrery-remote.md`, `docs/remote-integration.md`, `.planning/PLAN-v97-mobile.md`). Env: `CCM_REMOTE_PORT` (4320).

### Commit message đề xuất
docs(remote): v97 Orrery Remote exception in section 11as, Vietnamese user guide, code index entries
