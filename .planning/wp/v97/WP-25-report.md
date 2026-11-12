## WP-25 — DONE

### Đã đổi
- Box-drawing: nguyên nhân là box glyph rơi vào font fallback hệ thống (metrics khác, line height 1.2). Bundle `assets/fonts/CascadiaMono-Terminal.ttf` (subset OFL, Latin + Việt + box + block + braille, 100 KB) làm font chính `OrreryMono`, `height = 2380/2048` (`terminal_fonts.dart`, pubspec, LICENSES, `test/support/fonts.dart`).
- Gộp output pty: `terminal_pane.dart` ghi 1 lần mỗi frame (`_pendingOut` + `scheduleFrameCallback`), flush trước `pty.ready` / `pty.exit`, huỷ khi đổi session.
- Tên máy khi ghép: `lib/remote/device_name.dart` (`device_info_plus ^13.3.0`, Android `Manufacturer Model`, iOS `utsname` -> tên thân thiện, max 40), `bootstrap.dart` dùng `loadDeviceName()`. Gateway đã giới hạn (`clean_name`/MAX_NAME).
- Log đổi tên session: shell `dashboard.rs` (`logged`) hỏi page `remote.logged` sau `terminal.alias` thành công; `packages/shared/src/orreryApi.ts` spec `remote.logged`; `useRemotePhone.ts` `logged`; i18n `remoteV97.log.alias/aliasClear` en + vi. (~35 dòng.)
- Index `apps/mobile/.code_index/mobile.md`.

### Tiêu chí
- Test tái hiện: box nối liền (pixel scan) pass; metrics cùng cell cho box/block/Việt pass; mất dòng: test 3000 dòng xen cuộn pass (xem Review: KHÔNG tái hiện được fail trước). Analyze sạch, flutter test 203 pass, cargo test remote 139 pass, Vitest useRemotePhone 15 pass, vue-tsc sạch.
- Ảnh A21s: `docs/qa/v97/WP-25/b1-box-vietnamese.png`, `b2-scroll-flow.png`.

### Audit
Box lệch do fallback font hệ thống A21s (Symbols subset không phủ U+2500-257F). Mất dòng: không thấy đường drop client trong `pty_stream`/`terminal_pane`; kênh duy nhất làm mất dữ liệu là gateway `pty.overflow` (backlog > 4 MiB hoặc send timeout) -> `_reattach` -> `pty.ready` xoá màn + scrollback; xảy ra khi UI thread kẹt lúc cuộn.

### Review
- Edge: xoay máy (fit lại qua LayoutBuilder, không đổi); font scale hệ thống (cell đo bằng TextPainter cùng style, không đổi); alt screen vim/htop (chỉ đổi font và thời điểm write, sequence giữ nguyên thứ tự); CJK/emoji vẫn fallback, width xterm tự tính.
- Mất dòng: chưa tái hiện được ở test (fail trước không có); sửa gộp frame chỉ giảm tải UI, chưa chứng minh nguyên nhân. Trên A21s bản debug mới: 6 vòng vuốt lúc `1..2000` chảy, n1918–n2000 liền mạch (kể cả vùng 1962–1981 từng thiếu). Chưa chạy lại đủ để khẳng định.
- Giật 47.5 fps: chưa đo lại (không build/chạy profile); chỉ giảm số lần write.
- Ngoài danh sách phạm vi nhưng cần thiết: `lib/app/bootstrap.dart`, `packages/shared/src/orreryApi.ts`, `apps/web/src/composables/useRemotePhone{,.test}.ts`, `apps/web/src/i18n/remote-v97.ts`.

### Test
`flutter test test/terminal_glyphs_test.dart test/device_name_test.dart test/terminal_pane_test.dart`; full `flutter test --concurrency=4`; `cargo test --locked -j 3 remote` (apps/shell); `npm test -w @ccm/web -- src/composables/useRemotePhone.test.ts`; `npm run typecheck -w @ccm/web`. Không có Rust test mới cho `logged` (chỉ Vitest nhận `remote.logged`).

### Nhìn
Đã mở `b1-box-vietnamese.png` (khung góc nối, `┃` liền giữa các dòng, "ạếữ" đúng) và `b2-scroll-flow.png` + 2 ảnh cuộn (số liền). Tên máy ghép hiện "Samsung SM-A217F" ở desktop. Fixture khung của con tự lệch độ rộng 1 dòng (lỗi file mẫu, không phải render).

### Dọn / máy
Dev Tauri của con (port 4441/4341/4566) đã /quit; A21s về Home; quyền camera chọn "Chỉ lần này"; máy ghép "DESKTOP-SMCL7FD" tạm còn trên điện thoại. `apps/mobile/shots/*.png` đổi do test screenshot chạy lại với font mới (chủ nhân có thể bỏ). Có tạm một APK debug arm64 đã cài lên A21s.

### Hot file
terminal_fonts.dart, terminal_pane.dart, device_name.dart, dashboard.rs, orreryApi.ts, useRemotePhone.ts, pubspec.yaml

### Commit message
fix(mobile): bundle Cascadia Mono as the terminal font so box drawing joins, write pty output once per frame, send the real phone model when pairing, and log phone session renames on the desktop
