# WP-68b3 báo cáo: Home, Ghép, Face ID, Monitor, chi tiết, banner limit theo "Sương kính"

Trạng thái: code xong, typecheck + vitest pass. **E2E 2 engine CHƯA chạy được** (ổ D hết chỗ, `rustc: no space on device` khi build peer). Ảnh `docs/qa/v98/WP-68b3/` chưa có.

## Đã làm
1. **Home**: `MachineCard.vue` viết lại (tên, "Orrery <version>, <OS>", LinkChip Relay / LAN, chip 5 trạng thái icon + chữ + màu gồm lỗi và xong, hạn mức, "Cập nhật hh:mm" hoặc "Dữ liệu cũ" kèm icon đồng hồ, chọn đường, chuông; máy mất kết nối mờ 74%). `QuotaBars.vue` mới (Claude Code 5g / Tuần, Codex; thanh + %, vàng từ 65, đỏ từ 85, thiếu số không vẽ, stale mờ + nhãn). `HomeView.vue`: thanh kính sticky, logo, nút `+` Ghép máy trên thanh trên (testid `pair-fab` giữ nguyên), bỏ nút nổi, `TransitionGroup` thêm / bớt thẻ.
2. **Ghép + Face ID**: seg có icon, khung quét bo tròn (`QrScanner`), thẻ e2e xanh; `LockOffer` thẻ kính, 3 gạch tiến trình, "Đã ghép <máy>", icon Face ID / vân tay trên nút.
3. **Monitor**: seg Project / Trạng thái trên thanh trên (`mode-cards` / `mode-groups`), chip lọc có icon + số, thẻ project (chữ cái, tên, nhánh, số session) chứa `SessionRow` mới, chế độ Trạng thái dùng cùng row; banner offline; banner `limit` có nút Thử lại; dòng `trimmed`. `SessionCard.vue` xoá.
4. **Chi tiết session**: thẻ đầu (tile agent, project, nhánh, chip), facts Agent / Bắt đầu / Máy, banner "Chờ bạn" + nút "Trả lời trong terminal" (`session-ask`, `session-answer`), sub-agent, hoạt động có giờ bên trái. Không bịa Model / Context (chưa có dữ liệu).
5. **iPad**: màn dùng master / detail hiện có; nội dung không phụ thuộc rail (rail do b4).
6. **Motion**: chip, thẻ máy, thẻ project / row, banner, ask, sub-agent, hoạt động; dùng token `--ccm-dur-*` / `--ccm-ease-*` nên Giảm chuyển động tự tắt; Giảm trong suốt chỉ ảnh hưởng thanh / sheet (thẻ không blur).
7. **Desktop**: `apps/shell/src/remote/ctx.rs` `welcome()` thêm `"os": std::env::consts::OS` (1 dòng, chưa chạy cargo test); `docs/remote-integration.md` §4.2.

## File đổi
- Mới: `apps/pwa/src/lib/projectModel.ts` (+ `.test.ts`), `components/QuotaBars.vue`, `e2e/lib/mist-home-steps.mjs`.
- Viết lại: `components/{MachineCard,SessionRow,LockOffer}.vue`, `views/{HomeView,MonitorView,SessionView}.vue`. Sửa: `views/PairView.vue`, `components/QrScanner.vue`, `i18n/{en,vi}.ts`.
- **Dùng chung, sửa tối thiểu**: `components/PageBar.vue` (thêm prop `sub` + slot `below`, tương thích ngược), `e2e/lib/scenario.mjs` (import + gọi `mistHomeSteps`, `STEPS_PER_BROWSER` 47 -> 48 trên nền con số của b4, đổi selector `monitor-list > button`), `e2e/lib/{ui-steps,shots,lock-offer-step}.mjs`, `e2e/live/s2.mjs` (selector + `mode-toggle` -> `mode-groups` / `mode-cards`), `e2e/peer/src/conn.rs` (hạn mức + `os` trong fixture), `e2e/peer/src/main.rs` (lệnh `limit`). Không đụng `base.css`, `App.vue`, prefs.
- Đã đổi EOL CRLF -> LF ở các file `.vue` / i18n con viết lại (file trộn EOL).
- `.code_index/{web-pwa,_meta}.md`.

## Test (lệnh chạy thật)
- `npm run typecheck -w @ccm/pwa`: sạch.
- `npm test -w @ccm/pwa`: 26 file, 247 / 247 (gồm `projectModel.test.ts` mới: gom project, đếm trạng thái, trạng thái thẻ, OS / phiên bản, ngưỡng màu hạn mức).
- `npm run qa -- --no-e2e --no-cargo`: kế hoạch rỗng (pwa không có trong map), không có gì chạy.
- `npm run e2e -w @ccm/pwa -- --relayPort=8795 --inspectorPort=9355`: **fail ở bước build peer, ổ D 0 GB trống**. Chưa có số bước pass. Không xoá gì để lấy chỗ (không phải file của gói này).

## Việc dở
- Giải phóng ổ D rồi chạy lại lệnh E2E trên (không dùng `--no-peer-build`, vì peer đổi). Bước mới `Mist screens...` kỳ vọng: 4 thẻ project, 3 thanh hạn mức, 91% đỏ, "Orrery 2.3.0, Windows", `session-ask` ở t3, banner limit + Thử lại; ảnh vào `docs/qa/v98/WP-68b3/` (home, monitor-a, monitor-b, detail, limit, pair-scan, pair-code; phone + iPad; sáng / tối; thêm nebula, galaxy phone; lock-phone sáng / tối) rồi so với `docs/qa/v98/WP-68a/`. Sau E2E: `git checkout --` ảnh cũ bị ghi đè trong `docs/qa/v98/WP-55b|56a|61a|61b|65|68b1/`.
- Có thể có assertion e2e cũ lệch (chưa chạy): `machine-counts` giờ có thêm chip lỗi / xong, `row-t2` chỉ ở chế độ Trạng thái.
- Máy cũ chưa gửi `os` thì thẻ chỉ hiện phiên bản; Model / Context của session chưa hiện vì chưa có dữ liệu từ desktop.
- Android parse `hello`: chưa kiểm field `os` mới (giả định bỏ qua field lạ).

## Test macOS / iPhone
- [ ] Home: thẻ máy hiện tên, "Orrery x.y.z, Windows / macOS", chip Relay / LAN, số 5g / Tuần của Claude Code và Codex bằng số footer desktop; tắt bridge thì không hiện hàng hạn mức, không hiện 0%.
- [ ] Máy tắt: thẻ chuyển nhóm "Máy khác", mờ, có "Dữ liệu cũ hh:mm"; bật lại thì quay về "Đang kết nối" có motion.
- [ ] Nút `+` Ghép máy trên thanh trên, đủ vùng chạm 44 px, không còn nút nổi đè thẻ.
- [ ] Ghép: quét QR trên iPhone thật (khung bo tròn), tab Nhập mã, mã sai / hết hạn / bị từ chối; sau ghép hiện bước "Bật khoá Face ID?" với "Đã ghép <máy>", Face ID mở được và "Để sau" đi tiếp.
- [ ] Monitor: seg Project / Trạng thái nhớ qua mở lại app; thẻ project đúng nhánh; chạm session mở chi tiết; chuyển trạng thái session có animation, bật Giảm chuyển động thì đứng yên.
- [ ] Chi tiết: session đang chờ có banner vàng + nút "Trả lời trong terminal" mở đúng pane.
- [ ] Banner `limit` (máy có > 4 MB dữ liệu): hiện đỏ, Thử lại gửi resync; dòng `trimmed` khi bị cắt.
- [ ] Giảm độ trong suốt của iOS: thanh trên / tab bar đặc, thẻ không đổi; cuộn Monitor 30 session vẫn 60 fps.
- [ ] iPad: nội dung Home / Monitor / chi tiết vẫn đúng trong layout master / detail và khi b4 gắn rail.
