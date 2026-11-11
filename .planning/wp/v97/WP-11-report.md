## WP-11 — PARTIAL (UI + test xong; claim bit, chạy với gateway thật, camera, iOS: CHƯA KIỂM)

### Đã đổi
- App (`apps/mobile`): màn Máy (HomePage mới), Workspace (project), Tab (PageView / chia cây ≥ 700 dp), Pane terminal (xterm), Xin quyền, Mission Control, Inbox, Mở pane, Thao tác (đổi tên tab, chuyển, mở lại, Compact, đóng), hộp xác nhận `confirm.ask` (đếm 60 s), banner offline, màn revoked / protoTooOld. Theme + ngôn ngữ theo `hello.theme` / `hello.locale` của máy đang chọn. Motion: Appear, AnimatedSwitcher / AnimatedSize cho status, badge, banner, dot; tôn trọng `disableAnimations`.
- Code mới: `lib/ui/**` (model, theme, terminal, workspace, widgets, machine_api, ui_providers, commands), `lib/l10n/` (ARB en + vi, sinh bằng `tool/gen_arb.mjs` + `flutter gen-l10n`, cấu hình `l10n.yaml`), sửa `lib/app/home_page.dart`, `pair_page.dart`, `lib/main.dart`.
- Sửa nhỏ `lib/remote/store.dart`: `ws.patch {base, rev}` áp khi `base` == rev hiện tại, lệch → `ws.get` (theo thông báo WP-6); vẫn nhận patch cũ không có `base`.
- Package thêm (pubspec): `flutter_localizations` (sdk) + `intl: any` — cần cho ARB / gen-l10n.
- Test mới: `workspace_model_test` (7), `ui_screens_test` (23), `terminal_pane_test` (17), thêm 1 test store (`ws.patch` có base), `screenshots_test` (19, chỉ chạy khi `WP11_SHOTS=1`).
- `.code_index/mobile.md` đã cập nhật.

### Tiêu chí: 5/6 đạt
1. `flutter analyze` sạch; `flutter test --concurrency=4` pass — ĐẠT (132 pass, 19 skip là test chụp ảnh).
2. Chạy với fake gateway PC (mở tab, xem pane, gõ, echo, xác nhận, offline) — CHƯA ĐẠT đủ: UI chạy với `FakeMachineApi` ở tầng widget test (attach, echo, gõ, phím phụ, xin quyền, confirm, offline giữa lúc gõ, pane đóng khi đang xem, tab biến mất, ≥ 700 dp, max 4 attach). KHÔNG chạy qua `fake_gateway.rs` thật (Rust ngoài phạm vi; `integration_test` cũ của WP-10 vẫn pass).
3. Ảnh từng màn chụp SAU lần sửa cuối — ĐẠT, nguồn: `flutter test` headless (font Roboto + MaterialIcons thật từ Flutter SDK, Consolas thật). Không có ảnh từ `flutter run -d windows` (flutter config `no-enable-windows-desktop`, không thấy Visual Studio, không đổi setting máy) và không dùng A21s theo lệnh main. Ảnh ở `docs/qa/v97/WP-11/` (19 file). Tofu (ô vuông) ở dòng `✔` trong terminal chỉ do font test không có glyph, máy thật có font fallback.
4. vi + en đủ chữ, không chuỗi cứng — ĐẠT (ARB 2 ngôn ngữ; chữ spike giữ nguyên vì là màn đo của WP-1).
5. File ≤ ~400 dòng, không comment — ĐẠT với code; `test/ui_screens_test.dart` 452 dòng (hơi vượt).
6. `.code_index/mobile.md` — ĐẠT.

### Audit / Review / Test / Nhìn
- Audit: sửa trong `apps/mobile/lib/**` (+ `lib/remote/store.dart` nhỏ), `test/**`, `pubspec.yaml`, `l10n.yaml`, `tool/gen_arb.mjs`, `.code_index`, `docs/qa/v97/WP-11`. Không đụng `lib/spike`, `rust/**`, ngoài `apps/mobile`.
- Review (đã xử lý): `ref` dùng trong `dispose` (lỗi thật, đã lưu api trong `initState`); xterm không tự xoá khi `pty.ready` bằng ESC c → dùng `ESC[3J ESC[2J ESC[H`; ô chia quá hẹp tràn Row → placeholder khi < 260x220; vuốt ngang chuyển pane xung đột với cuộn ngang terminal (chỉ xem giữ cỡ desktop) → `PageView` không vuốt trực tiếp, chuyển pane bằng vuốt ở header / thanh chấm hoặc chạm chấm; ẩn phím phụ khi màn thấp < 520 dp và chưa gõ; `pty.exit` `code: null` = "Mất terminal trên máy tính"; dialog đổi tên dùng StatefulWidget (controller dispose đúng lúc).
- Test: `cd apps/mobile && flutter analyze && flutter test --concurrency=4` → No issues; 132 pass / 0 fail.
- Nhìn: đã mở xem 01, 03, 03b, 04, 05, 05b, 06, 06b, 07, 09, 10, 11, 12, 14 (không tràn, không màn đen). 01b, 02, 03c, 08, 13 chỉ chụp, chưa mở xem từng cái.
- CHƯA KIỂM: tên token trong `hello.theme.palette` (app đọc `accent`, `bg`, `surface`, `ink` và bỏ tiền tố `--ccm-`; ngoài ra dùng palette Orrery mặc định); camera / mDNS / máy thật (WP-10 vẫn chờ); iOS; hành vi bàn phím IME tiếng Việt trên máy thật (chỉ có test nhập chuỗi có dấu qua `enterText`); "Có luôn" gửi `ws.keys "2"` (giả định option 2 của hộp xin quyền Claude).

### Ngoài phạm vi cần main xử lý
1. **Claim bit (flags bit 1 của kind 2)**: `SecureChannel.seal(kind, stream, payload)` chưa nhận cờ, Rust FRB (`apps/mobile/rust`) bị cấm với WP-11. Cần việc nhỏ cho WP-10 / Rust: thêm tham số `claim` vào `seal` (đặt `byte[1] |= 0b10` sau `Frame::encode`, trước mã hoá), rồi `SecureLink.seal`, `RemoteSession.sendPtyInput(..., {claim})`, `PtyStreams.input`, `MachineApi.input`. UI đã chia sẵn nguồn: mọi `onOutput` của xterm đi qua một chỗ (`_TerminalPaneState._onOutput`); hiện chưa phân biệt phím người dùng với phản hồi query của terminal.
2. Chip "người gõ" chỉ dựa vào trạng thái cục bộ (Bạn đang gõ / Chỉ xem). Muốn hiện "Desktop đang gõ" cần gateway gửi event cho biết ai giữ quyền gõ (chưa có trong hợp đồng).
3. `fake_gateway.rs` chưa có `ws.patch {base}`, `confirm.ask` có `re`, `pty.exit {code:null}`, `pty.attach` reply `{stream}`; (Rust, WP-10) cần cập nhật để chạy e2e cả UI.
4. Mất hub: gateway gửi `pty.exit {code:null}` — UI đã xử lý. Lệnh có xác nhận trả `{confirm: cid}` rồi `{cancelled:true}`: UI không cần gì thêm (bottom sheet sống theo event `confirm.ask`, tự đóng khi hết hạn).
5. Chọn project mở pane lấy `favorites` trong snapshot dashboard + `tab.project`; nếu gateway lọc bỏ `favorites` khỏi `snapshot` thì danh sách chỉ còn project đang có trong Workspace.
6. Ảnh chụp từ A21s / Windows desktop thật nếu main muốn đủ nguồn (cần bật `flutter config --enable-windows-desktop` + Visual Studio, hoặc máy A21s rảnh).

### Commit message đề xuất
feat(mobile): add Orrery Remote UI (machines, workspace projects, tab pager and split tree, xterm pane with line input and extra keys, permission bar, mission control, inbox, open pane, confirm sheet, offline and revoked states) with vi/en ARB and widget tests
