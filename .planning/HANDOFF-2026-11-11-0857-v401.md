# HANDOFF 2026-11-11 08:57 — Orrery 4.0.1 (Windows "Mất kết nối")

Đọc file này là làm tiếp được. Trả lời tiếng Việt, gọi "chủ nhân", xưng "con". File này thay `HANDOFF-2026-11-11-0230-v4.md` (bản 4.0.0).

## 1. Đang ở đâu
- Lỗi: máy Windows 11 của chủ nhân update 3.4.0 → 4.0.0 (Tauri), header popover "Bộ theo dõi" hiện **Mất kết nối** suốt (`connection.state === 'disconnected'` = WebSocket chưa nhận `snapshot` lần nào).
- Số liệu chủ nhân đo trên máy đó (lúc lỗi): `curl.exe -s http://127.0.0.1:4317/health` → `{"ok":true,"seq":5}`; `netstat` → `127.0.0.1:4317 LISTENING` (pid 8676) + ~13 `TIME_WAIT` phía `127.0.0.1:4317` = server sống, tự đóng các kết nối `/ws`.
- Giả thuyết (chưa đo header thật, release không có devtools): `/ws` bị từ chối ở `core-http` `keyed()` vì cookie chỉ được nhận khi có `Sec-Fetch-Site` `same-origin`/`none`; WebView2 (Chromium) không gửi fetch metadata trên handshake WebSocket, WebKit mac thì gửi → mac chạy, Windows kẹt.
- Commit `67e87fb2` (đã push `master` + tag `v4.0.1`):
  - `apps/core/crates/core-http/src/cookie.rs` `authorized_ws` + `server.rs` `ws_keyed` (dùng trong `upgrade()`): thiếu `Sec-Fetch-Site` mà có Origin (đã qua `local()` = đúng `http://127.0.0.1:<port>` của app) → coi như `same-origin`; cookie vẫn phải đúng key. HTTP thường giữ kiểm cũ.
  - `apps/shell/src/log.rs` `log::install` (gọi trong `main.rs` sau tạo data dir): release đổi stderr → `<data>/shell.log` (Windows `%LOCALAPPDATA%\ccm\shell.log`), giữ console/tty nếu có, `CCM_SHELL_LOG=0|1`, > 2 MB → `shell.old.log`, dòng `[shell] start …` + `[shell] panic at <unix>`.
  - Bump 4.0.1: `apps/desktop/package.json`, `package-lock.json`, `apps/web/src/lib/changelog.ts`, `.planning/RELEASE-4.0.1.md`.
- Test đã chạy trên mac: `cargo test -p core-http` 75 pass; `cargo test` trong `apps/shell` 104 pass; web `src/lib/changelog` 8 pass. Nhánh `#[cfg(windows)]` của `log.rs` **chưa biên dịch lần nào** (máy dev là mac).
- CI `desktop` run `42941493186` (tag `v4.0.1`) lúc 08:57 **vẫn đang chạy** cả 2 job. Release v4.0.1 **chưa đăng**: chủ nhân dừng việc đăng.
- Chủ nhân báo "test window bị failed" (08:5x), chưa có chi tiết (CI hay cài thử) → chủ nhân tự test local trên máy Windows kia.
- Chưa commit: `.planning/RELEASE-4.0.0.md` = chủ nhân đang sửa dở, không đụng, không commit.

## 2. Việc tiếp (theo thứ tự)
1. Chủ nhân test local trên máy Windows: `git pull` rồi `npm ci` + `npm run dist:win` (ra `apps/shell/release/Orrery-Setup-4.0.1.exe`), hoặc `npm run shell:dev` (cổng 4419, data tạm, không đụng bản cài).
2. Lấy lỗi: output build nếu build fail; nếu chạy được mà vẫn "Mất kết nối" → gửi `%LOCALAPPDATA%\ccm\shell.log`. Với `shell:dev` (debug, có console) xem thẳng stderr; devtools mở được (F12) → tab Network xem request `/ws` có `Sec-Fetch-Site`, `Origin`, `Cookie` không, status.
3. Xem CI: `gh run view 42941493186 --log-failed | tail -80` (job Windows nếu fail).
4. Sửa xong → nếu đổi code sau tag: xoá / đặt lại tag `v4.0.1` chỉ khi chủ nhân cho (`git tag -d v4.0.1 && git push origin :refs/tags/v4.0.1`), hoặc lên 4.0.2.
5. Đăng release khi chủ nhân bảo: `gh run download <run> -D <scratch>` rồi `gh release create v4.0.1 --title "Orrery v4.0.1" --notes-file .planning/RELEASE-4.0.1.md <Orrery-Setup-4.0.1.exe> <Orrery-4.0.1-arm64.dmg> <Orrery-4.0.1-x64.dmg>` (asset tên khớp `pick_asset` của updater, giống release v4.0.0).

Còn treo từ 4.0.0: E2E `--shell=tauri` trên Windows (T3); OSC 633 trên Tauri; bash shim `--init-file`; Office khoá trên Tauri; W4 nút Office (chủ nhân chưa trả lời).

## 3. Quyết định và ràng buộc
- Commit / push / tag / release: chỉ khi chủ nhân nói rõ, mỗi lần tính riêng. Không thêm trailer attribution.
- Không tự build verify; báo lệnh build rồi dừng.
- Không comment trong code (rule máy này).
- Ở bản release, `/health` không cần key; mọi request khác cần header `x-ccm-app` hoặc cookie `ccm_app` (+ fetch metadata, hoặc Origin cho `/ws`).
- Quirk: rtk rewrite `cargo` → cần `export PATH="$HOME/.cargo/bin:$PATH"` trước `cargo test`.

## 4. File cần đọc lại
`.code_index/shell.md` (Khởi động, mục `log.rs`), `.code_index/_meta.md` (2 dòng 4.0.1 cuối file), `apps/core/crates/core-http/src/{cookie,server,origin}.rs`, `apps/web/src/stores/connection.ts`, `.planning/RELEASE-4.0.1.md`.
