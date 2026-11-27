## WP-113j — DONE (code + test xong; chờ main thêm hot file để chạy thật)

### Đã đổi
Rust (apps/shell/src/remote):
- `clip.rs` (mới, 141 dòng): `ClipSink` trait (+ `SystemClip` dùng `arboard` đã có trong Cargo.toml, `FakeClip` trong test), `ClipHub` (broadcast event `clip` + sink), `receive` (invalid / limit > 64 KB / secret khi `mask_text` đổi nội dung / internal), `send` (noSession / empty / limit / secret), `Phone::{clip_attach, clip_push, on_clip}`, `Remote::clip_send()`.
- `clip_tests.rs` (mới): 4 test (unit hub, e2e 2 phone one old app no cap, view-only denied).
- `phone_ws.rs`: `#[path="clip.rs"] pub(super) mod clip`, field `Extra.clip`, dispatch `"clip.push"`.
- `workspace.rs`: `Board.clip: ClipHub`.
- `dashboard.rs` (ngoài list cấm): 1 arm select `next_clip` + 1 dòng `self.clip_attach()` sau hello.
Web: `features/remote/lib/remoteClip.ts` (+ test), `composables/useClipToPhone.ts` (+ test), `index.ts` export, `palette/components/CommandPalette.vue` thêm lệnh `c:phone-clip` (nhóm actions, dưới `remote.supported`).
Mobile: `lib/remote/clip_sync.dart` (logic thuần), `lib/ui/clip/clip_ui.dart` (`ClipTile`, `ClipListener`), `connect_route.dart` `helloCaps = ['chat','clip']`, `settings_body.dart` (tile, ẩn khi desktop thiếu cap `clip`), `workspace_page.dart` (bọc `ClipListener`), `app_en.arb` + `app_vi.arb` (9 key `clip*`) + `flutter gen-l10n` (4 file l10n sinh), `test/clip_test.dart`, `test/connection_test.dart:599` caps `['chat','clip']`.

### Tiêu chí
- [x] cargo: `clip.push` > 64 KB `limit`, secret `secret`, text thường ghi qua trait fake; event `clip` chỉ tới phiên đang mở có cap `clip`; không phiên `noSession`.
- [x] Vitest: palette gọi wrapper, toast theo lỗi (`noSession`, `secret`...) - test dùng key i18n (chuỗi thật nằm ở hot file i18n, xem dưới).
- [x] Dart: gửi clip, nhận event chép + snackbar; desktop thiếu cap `clip` ẩn nút.
- [x] Không comment; file < 400 dòng; i18n en + vi (mobile xong, web chờ main); test cũ xanh.

### Audit
Diff chỉ gồm file trong phạm vi + `dashboard.rs` (2 điểm) + `workspace.rs` + `phone_ws.rs` + `connection_test.dart` (1 dòng) + `CommandPalette.vue` (3 dòng). Không đụng file cấm.

### Review
- Clipboard rỗng / không phải text: desktop `empty` (arboard `get_text` lỗi khi là ảnh); phone `empty` không gọi desktop; `text` không phải string `invalid`.
- Nhiều phone cùng mở: event broadcast tới mọi phiên có cap `clip` (sent = số phiên). Phone cũ không cap: không subscribe, nên nếu chỉ có phone cũ thì `noSession`.
- Phiên đứt giữa chừng: receiver drop theo Phone; `sec.json` false thì vòng lặp thoát. `send` đọc clipboard SAU khi kiểm có listener (không đọc clipboard vô ích).
- Secret: chuỗi liền >= 32 ký tự chữ-số bị coi là secret (cùng luật `mask_text` của push), đúng chủ ý "chặn khi mask đổi".
- Không log nội dung clipboard (không có log nào trong code mới). Không qua push.
- `clip.push` cần `host.control()` (view-only thì `denied`), chịu rate chung `MAX_COMMANDS_PER_MINUTE`.
- Phone chép thẳng + snackbar khi nhận (theo brief). Chỉ nhận khi `WorkspacePage` đang mở (máy đang chọn).
- Giới hạn hạ tầng test: frame test không chia khúc nên không test e2e 64 KB+ (unit test `limit` có).
- `tool/gen_arb.mjs` đã LỖI THỜI so với ARB (chạy sẽ xoá hơn 600 dòng key), nên ARB thêm tay; đã khôi phục ARB về HEAD trước khi thêm, không để lại diff lạ.

### Test
- `cd apps/shell; cargo test --locked -j 4 clip` → 8 passed (4 mới + 4 trùng tên khác).
- `cargo test --locked -j 4 remote` → 341 passed, 0 failed, 3 ignored.
- `npm test -w @ccm/web -- src/features/remote src/features/palette` → 234/235; 1 flaky `useRemotePhone ... rename and move` (timeout khi máy tải), chạy riêng file đó 17/17 pass. `npm run typecheck` → sạch.
- `flutter analyze` (cả apps/mobile) → No issues. `flutter test --concurrency=2 test/clip_test.dart connection_test arb_no_dots hello_info ui_actions ui_screens narrow_layout kit_test` → toàn bộ pass (clip_test 11).

### Nhìn: CHƯA NHÌN
Chưa chạy app thật: palette command "Gửi clipboard sang điện thoại" (nhóm Actions) và tile "Gửi clipboard lên máy tính" ở tab Cài đặt của phone, snackbar khi nhận. Tray: chưa làm (chỉ palette, theo brief cho phép).

### Hot file cần thêm
1. `apps/shell/src/remote/mod.rs`: thêm `"ccm:remote-clip-send",` vào `CHANNELS`; trong `call_with`: `"ccm:remote-clip-send" => Ok(self.clip_send()),`.
2. `apps/shell/src/remote/ctx.rs`: `DESKTOP_CAPS` thêm `"clip"` (đổi kiểu mảng `[&str; N]`, N+1; nếu WP khác cũng thêm cap thì cộng dồn). Cần cho nút phone hiện.
3. `preload.ts` / shim / `core/lib/desktop.ts`: `remoteClipSend: () => ipcRenderer.invoke('ccm:remote-clip-send')`; trong interface desktop.ts: `remoteClipSend?: () => Promise<unknown>`. (Nếu shim sinh từ danh sách kênh thì chỉ cần thêm kênh.)
4. i18n web en + vi, nhóm `remoteClip`:
   - en: `palette` "Send clipboard to phone", `paletteSub` "Only when you run it, never automatic", `sent` "Sent to your phone", `noSession` "Open the app on your phone, then send again", `empty` "The clipboard has no text", `limit` "The text is over 64 KB", `secret` "Looks like a secret, blocked", `internal` "Could not send the clipboard", `off` "Orrery Remote is off", `unavailable` "Not available in this build".
   - vi: "Gửi clipboard sang điện thoại", "Chỉ khi bạn bấm, không tự đồng bộ", "Đã gửi sang điện thoại", "Mở app trên điện thoại rồi gửi lại", "Clipboard không có chữ", "Chữ vượt 64 KB", "Có vẻ là secret, đã chặn", "Không gửi được clipboard", "Orrery Remote đang tắt", "Bản này chưa hỗ trợ". Cập nhật snapshot i18n.
5. `.code_index`: lookup `clip` -> `apps/shell/src/remote/clip.rs::ClipHub`, `features/remote/lib/remoteClip.ts::sendClipboardToPhone`, `apps/mobile/lib/remote/clip_sync.dart::pushClip`, `ui/clip/clip_ui.dart::ClipTile/ClipListener`; mobile.md: helloCaps có `clip`, gen_arb.mjs lỗi thời (ARB sửa tay).
6. `docs/remote-integration.md`: op `clip.push {text}` (reply ok | invalid | limit | secret | denied | internal), event `clip {text}`, cap `clip` hai chiều.
7. `docs/core-principle.md`: ghi `clip.push` ghi clipboard OS của desktop khi điều khiển từ xa bật (thuộc ngoại lệ 7), không tự đồng bộ, chặn secret, không log nội dung.

### Ngoài phạm vi
Tray menu (Rust tray nằm ngoài); ảnh clipboard (spec để sau); `tool/gen_arb.mjs` cần dọn lại (lỗi thời).

### Commit message đề xuất
feat(remote): share clipboard between phone and desktop on tap, with secret and 64 KB guards
