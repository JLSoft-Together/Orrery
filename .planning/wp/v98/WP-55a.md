## WP-55a: Web app — Home nhiều máy, ghép (camera QR + dán mã), Monitor A/B, chi tiết session, khoá WebAuthn, Cài đặt

- Mục tiêu: các màn "đọc + ghép + khoá" của web app iPhone / iPad ngang app Android, trên khung WP-54. Workspace / terminal / viewer là WP-55b (làm sau, cùng `apps/pwa`).
- Spec: `.planning/PLAN-v98-ios-pwa.md` #8, #9, #10, #12, #19 (chuông từng máy — chỉ UI + lưu cục bộ, gửi lệnh tắt push cho desktop là WP-56), WP-55. Mockup đã duyệt: `docs/design/v97-mobile-ui/` (ảnh `png/home-a-*`, `pair-qr-*`, `pair-code-*`, `mon-a-*`, `mon-b-*`, `detail-*`, `lock-*`, `settings-*`, `empty-*`, `offline-*`, `error-*`, `section-tokens.png`, `section-components.png`, `section-motion.png`), lựa chọn chủ nhân: `.planning/PLAN-v97-mobile-ui.md` mục "Chủ nhân duyệt mockup" (Home A, Monitor A mặc định + nút sang B nhớ lựa chọn, radius 8 px, motion theo bảng spec).
- Hợp đồng khung: `.planning/wp/v98/WP-54-report.md` mục "Hợp đồng cho WP-55" (`createRuntime`, store `hub`: `machines`, `linkOf`, `viewOf`, `pair`, `setPolicy`, `forget`, `wipeAll`; `rt.hub.session(id)?.request`; `createPairFlow` / `pairCandidates`; `Unlocker` trong `createKeyStore`; kiểu `Messages` ép đủ vi + en; không `:style` dạng chuỗi), `.code_index/web-pwa.md`. Giao thức: `docs/remote-integration.md` §2 (QR v1 / v2, ghép), §4 (JSON Monitor / phiên), §6. Tham khảo hành vi Android (chỉ đọc): `apps/mobile/lib/app/{home_page,pair_page,pair_route_row}.dart`, `apps/mobile/lib/ui/monitor/**`, `apps/mobile/lib/security/**`, `apps/mobile/lib/ui/workspace/appearance_rows.dart`.
- Phạm vi file được đụng: `apps/pwa/**` (trừ `e2e/peer/**` chỉ được thêm dữ liệu giả nếu cần), `apps/core/crates/core-remote-wasm/**` chỉ THÊM API, `.code_index/web-pwa.md` + `_meta.md`, `docs/qa/v98/WP-55a/**`, báo cáo. Cấm đụng: `apps/relay/**`, `apps/shell/**`, `apps/mobile/**`, `apps/web/**`, `packages/shared/**` (WP-59 đang sửa relay + shell).
- Model: sonnet. Song song với WP-59 (opus, relay + shell).

### Việc
0. Đọc `CLAUDE.md` Hard rules; nạp skill `frontend-design:frontend-design` + `ui-ux-pro-max:ui-ux-pro-max` trước khi làm UI.
1. Điều hướng: router nhẹ (vue-router hoặc tự viết, chọn cái nhỏ), deep link `/app/m/<machineId>/p/<paneKey>` (để dành cho push / bounce), back iOS vuốt cạnh không vỡ (`history` thật), thanh dưới theo mockup (có Cài đặt).
2. Home A: danh sách máy (gộp bản trùng + "Dọn bản cũ"), thẻ máy: tên, trạng thái đường (icon + chữ + màu), số session đang chạy / chờ, **chuông** bật / tắt push từ máy đó (lưu IndexedDB, phát sự kiện để WP-56 nối), select "Kết nối: Tự động / Chỉ LAN / Chỉ relay"; empty / offline / error theo mockup.
3. Ghép: quét QR bằng camera (`getUserMedia` + giải mã QR bằng thư viện JS nhỏ, chạy được dưới CSP hiện tại — không `eval`, không worker từ blob nếu CSP chặn; `worker-src 'self'` có sẵn), dán mã (chuỗi base64url QR); hiện đang thử đường nào (LAN → relay) như Android; đặt tên thiết bị; xử lý `pair.denied`, hết hạn, sai mã. Gửi `platform: "web"` nếu hợp đồng cho phép — không thì `"ios"` và ghi rủi ro; muốn thêm `platform` mới phải đổi shell → KHÔNG làm, ghi việc dở cho main.
4. Monitor A (thẻ) + B (nhóm theo trạng thái, dòng gọn), nút đổi mode nhớ lựa chọn; chi tiết session (trạng thái, tool đang chạy, thời gian, hoạt động gần đây 12 dòng như Android, nút sang Workspace pane — điều hướng tới route WP-55b, tạm hiện "sắp có").
5. Khoá app: WebAuthn platform authenticator (Face ID / Touch ID) — đăng ký 1 credential (`userVerification: required`, `residentKey: preferred`), mở khoá = `navigator.credentials.get` thành công; mặc định khoá sau 1 giờ ở nền (cấu hình trong Cài đặt); máy không hỗ trợ → báo rõ và cho tắt khoá. Khoá phải chặn hiện snapshot cache (IndexedDB) trước khi mở. Ghi rõ trong báo cáo: WebAuthn chỉ là cổng UI, khoá giải mã dữ liệu vẫn là `CryptoKey` không trích xuất được — đánh đổi này để reviewer xem; nếu dùng được PRF extension để dẫn khoá thì dùng, không thì ghi.
6. Cài đặt: ngôn ngữ vi / en, chủ đề sáng / tối / hệ thống, khoá app (bật / tắt / thời gian), thông báo (3 nhóm chờ / xong / lỗi + công tắc "Kèm nội dung" — chỉ lưu cục bộ, WP-56 nối), danh sách máy (đổi tên hiển thị, quên máy), xoá toàn bộ dữ liệu, phiên bản web + proto.
7. iPad ≥ 768 px: danh sách trái + chi tiết phải. Safe-area, Dynamic Type cơ bản (rem), chạm ≥ 44 px, motion theo bảng spec (tôn trọng `prefers-reduced-motion`).
8. Test: Vitest cho logic mới (gộp máy trùng, lọc / nhóm Monitor B, hẹn giờ khoá, parse QR / dán mã). E2E (`npm run e2e -w @ccm/pwa`, mở rộng kịch bản có sẵn): ghép bằng dán mã, Home → máy → Monitor A ↔ B (nhớ sau reload), chi tiết session, khoá bật → reload → màn khoá (WebAuthn giả lập: Playwright Chromium có virtual authenticator qua CDP; WebKit thì bỏ qua bước WebAuthn, ghi rõ), Cài đặt đổi ngôn ngữ / chủ đề. Không vi phạm CSP. Ảnh 390×844 + 820×1180, sáng / tối, các màn chính → `docs/qa/v98/WP-55a/`, chụp SAU lần sửa cuối.

### Tiêu chí nghiệm thu
- [ ] `npm run typecheck -w @ccm/pwa`, `npm test -w @ccm/pwa` (số), `npm run build -w @ccm/pwa` sạch (không inline script / style); E2E WebKit + Chromium pass (số bước).
- [ ] Ảnh các màn sáng / tối, phone + iPad, khớp mockup (main sẽ so).
- [ ] Không file mới > ~400 dòng; text đủ vi + en.

### Luật (bắt buộc)
- Không viết comment trong code (kể cả `/** */`, `//`, TODO); lý do / invariant ghi báo cáo hoặc `.code_index/`.
- Không `git commit` / `push`. Không build release Tauri / APK.
- Text UI qua vue-i18n `vi` + `en`; màu qua token; trạng thái = icon + chữ + màu; không blur, không gradient; font hệ thống; không tải gì từ mạng.
- Tool nặng ≤ 80% CPU / RAM; kiểm `tasklist | grep -i cargo` trước khi build cargo, có agent khác build thì chờ. Không dùng cổng 4320–4322 (server spike đang chạy).
- Bash: `rtk` có thể rewrite `grep` → dùng tool Grep hoặc `rtk proxy grep`.
- Tắt mọi tiến trình nền của mình khi xong.

### Báo cáo
`.planning/wp/v98/WP-55a-report.md`: màn đã làm (file + symbol), test chạy thật (số pass / fail, lệnh), ảnh, khác mockup ở đâu + lý do, việc dở (gồm việc cần shell / desktop làm), rủi ro, hợp đồng cho WP-55b (router, layout, chỗ cắm Workspace).
