# PLAN v113: kết nối điện thoại, thiết bị, 20 tính năng (new_feat_2611)

Nguồn đã duyệt: wireframe https://orrery.jinvu.vn/new_feat_2611/ (chủ nhân duyệt 26/11/2026, cả phần Bổ sung), ý tưởng `.planning/IDEAS-2026-11-26.md`. Spec chi tiết từng phase: `.planning/SPEC-v113-phone-devices.md`. Nhánh: `feat/new_feat_2611`.

## 0. Quyết định đã chốt

| # | Câu hỏi | Chốt (theo đề xuất, chủ nhân duyệt 26/11) |
|---|---|---|
| Q1 | APK lấy từ đâu | Đóng vào bản cài desktop (resource `apk/orrery-remote.apk`), bản arm64 release |
| Q2 | Mã ghép đôi qua cáp USB | Có: deep link `orrery-remote://pair?q=…` mở qua adb, mã 1 lần, vẫn bắt "Cho phép" trên máy tính |
| Q3 | P5 xem dev server | Làm ở đợt 3, ghi ngoại lệ 9 vào `docs/core-principle.md` trước khi code |
| Q4 | P4 giọng nói | Ưu tiên nhận trên máy (`onDevice`), không tự gửi |
| Q5 | P1 đưa nhãn lựa chọn menu vào push + bấm trên thông báo | Làm P1. Sửa câu chữ ngoại lệ 6 và 8 trong `docs/core-principle.md` (+ dòng tóm tắt trong `CLAUDE.md`) ở đầu F5, trước khi code P1: push được chở nhãn lựa chọn đã mask (≤ 24 ký tự, max 3); nút trên thông báo chỉ mở app (qua khoá app) rồi gõ khi chủ nhân bấm (chủ nhân chốt 26/11) |
| Q6 | Máy đang cài app ký debug key (A33) | Đồng ý gỡ, cài bản ký keystore mới, ghép đôi lại 1 lần. Con làm qua adb ngay khi xong F0, chủ nhân chỉ bấm "Cho phép" (chủ nhân chốt 26/11) |
| Q7 | Keystore | Tạo khoá mới `orrery-remote.jks` ngoài repo (`D:\keys\orrery\`), `key.properties` trỏ tới (đã gitignore); chủ nhân tự chép sang Mac + 1 nơi sao lưu (chủ nhân chốt 26/11) |

## 1. Lệch so với wireframe (do code thật, đã sửa trong spec)

| Wireframe | Code thật | Spec chọn |
|---|---|---|
| Mã ghép đôi hết hạn 5 phút, có mã 6 số | `PAIR_TTL=120` (`apps/shell/src/remote/pair.rs:15`); mã nhập tay chỉ có ở debug (`pair_page.dart:177`), PSK 16 byte | Giữ 2 phút, bỏ mã 6 số (mã ngắn yếu hơn PSK 16 byte) |
| Thanh % khi cài qua USB | `adb install` không trả tiến độ | Thanh chạy vô định + 3 bước rõ ràng |
| E1 cài APK "không cần cùng Wi-Fi" | Kênh Noise max 1 MiB / message, chưa có truyền file lớn | Bản đầu: tải qua LAN (route install sẵn có). Ngoài Wi-Fi: báo rõ, để phase sau |
| Gộp dòng thiết bị theo "id máy băm" | Android không cho app đọc serial; ANDROID_ID khác nhau giữa app và adb | Gộp chắc chắn khi ghép đôi qua USB (desktop biết serial); còn lại chỉ gợi ý gộp theo model, người dùng xác nhận |
| Nút "Copy link" tải APK | Link LAN mang ticket bí mật | Bỏ; giữ "Lưu vào Downloads" |
| Mã ghép đôi qua cáp nằm trong deep link | Link trong URI lộ ở logcat, trang web giả được | Mã đi bằng intent extra; thêm `adb reverse` để ghép được cả khi khác Wi-Fi |
| Emulator có nút "Cài Remote" | APK chỉ arm64 | Ẩn nút với emulator x86_64, ghi lý do |
| Gửi clipboard khi không có phiên | Ngoài phạm vi ngoại lệ 8 | Chỉ qua phiên đang mở |

## 2. Phase và thứ tự

Đợt 1 (kết nối, giá trị cao nhất) → Đợt 2 (điện thoại) → Đợt 3 (lớn). Mỗi phase xong: test xanh, `npm run qa`, cập nhật `.code_index`, ghi `.planning/QA-mac-checklist.md`, commit + push theo lệ v87 (chủ nhân cho phép từng đợt).

| Phase | Nội dung | Tính năng | Phụ thuộc | Công sức |
|---|---|---|---|---|
| **F0** | Nền: app version + model trong hello, `caps` 2 chiều, record Device mới, keystore cố định + migration 1 lần | nền mọi phase | - | S-M |
| **F1** | APK đóng vào bản cài, Lưu vào Downloads, ticket nhiều slot | R3 | F0 (keystore) | M |
| **F2** | Cài qua USB, mã ghép đôi qua intent extra, `adb reverse` | R2, nửa R4 | F1 | M |
| **F3** | Wizard 3 bước, gom "Nâng cao", trang tải LAN có "Mở và ghép đôi" (khoá theo IP, 1 lần), kiểm kết nối | R1, R4, R6 | F1, F2 | L |
| **F4** | Trang Settings › Thiết bị hợp nhất, thu hồi có hoàn tác ở core | R5 | F0, F2 | M |
| **F5** | Push nâng cao: sửa ngoại lệ 6, 8 trước; thông báo đang chạy, mở pane trên điện thoại, trả lời menu trên thông báo; quota riêng | P3, P6, P1 | F0 | M |
| **F6** | Widget màn hình chính + ô Quick Settings | P2, E4 | F5 (field `waiting`) | M |
| **F7** | Gửi APK build xong lên điện thoại, Run log + crash trên điện thoại | E1, E2 | F1 (ticket nhiều slot), F0 | L |
| **F8** | Vân tay cho lệnh nguy hiểm | E5 | F0 (caps) | S-M |
| **F9** | Nói / chụp ảnh gửi vào pane (op `fs.put` theo chunk) | P4 | F0 (caps) | M-L |
| **F10** | Clipboard chung (chỉ qua phiên đang mở) | D3 | F0 (caps) | S-M |
| **F11** | Checklist trước merge trên điện thoại (chạy kiểm qua web) | E6 | F0 (caps) | M-L |
| **F12** | Chạy trên nhiều thiết bị + lưới ảnh chụp | E3 | - | M |
| **F13** | Mirror màn thiết bị Android trong pane | D2 | spike trước | L |
| **F14** | Xem dev server trên điện thoại (ngoại lệ 9) | P5 | spike trước, ghi ngoại lệ | L |

Đợt 1 = F0-F4. Đợt 2 = F5-F8. Đợt 3 = F9-F14 (F13, F14 bắt đầu bằng spike 1 ngày, chủ nhân xem kết quả spike rồi mới làm tiếp).

## 3. Gói việc (chế độ manager)

Song song ≤ `JIN_MAX_SUBAGENTS`, mỗi gói 1 worktree `wp/*`. Gói không đụng cùng file mới chạy song song.

| WP | Phase | Phạm vi file chính | Model | Chạy song song với |
|---|---|---|---|---|
| WP-113a | F0 | `apps/mobile/lib/remote/connect_route.dart`, `apps/mobile/android/app/build.gradle.kts`, `apps/shell/src/remote/{devices,status,dashboard,ctx}.rs`, `packages/shared/src/types.ts` | sonnet | - |
| WP-113b | F1 | `scripts/dist-tauri.mjs`, `apps/shell/tauri.release.json`, `remote/install*.rs`, `host.rs`, `ctx.rs` | sonnet | sau WP-113a (cùng sửa `ctx.rs`) |
| WP-113c | F2 desktop | `core-desk/src/devices/install.rs` (mới), `devices/ipc.rs`, `run/parse.rs` (dùng lại `install_failure`) | sonnet | WP-113d |
| WP-113d | F2 phone | `apps/mobile/lib/app/{pane_link,deep_link_host}.dart`, `MainActivity.kt`, manifest | sonnet | WP-113c |
| WP-113e | F3 | `features/remote/**`, `features/settings/lib/settingsFlash.ts`, `remote/install.rs` (trang HTML), `remote/push_setup.rs` (check) | opus | - |
| WP-113f | F4 | `features/devices/**` (SettingsDevices), `core/lib/uiTypes.ts`, `features/settings/components/index.ts` | sonnet | sau WP-113e |
| WP-113g..n | F5-F12 | xem SPEC | sonnet / opus (F5, F7) | theo bảng phụ thuộc |
| WP-113o, p | F13, F14 | spike | opus | - |

## 4. Quy tắc chung mọi phase

- Không comment trong code. Chuỗi UI qua vue-i18n (`en`, `vi`) / ARB (`apps/mobile/lib/l10n`); không dùng `·` và em dash trong chuỗi UI.
- Màu qua theme token; trạng thái = icon + chữ + màu. Motion theo bảng trong wireframe (220ms vào, 160ms ra, chỉ transform / opacity, tôn trọng reduced-motion).
- Không thêm prompt tự gửi, không gọi LLM. Mọi thao tác gõ vào pane chỉ khi chủ nhân bấm (ngoại lệ 6, 7).
- Secret không rời máy: mọi text đi ra điện thoại (log, crash, clipboard, push) qua `core-transcript/src/mask.rs::mask_secrets` / `mask_log`.
- Giao thức: chỉ thêm field / op JSON (bên nhận bỏ qua field lạ); không đổi `PROTO` trừ F13 (kind frame mới). Mỗi op mới ghi vào `docs/remote-integration.md` §4.
- File ≤ ~400 dòng; logic thuần tách khỏi component để test (`lib/*.ts`, `*.rs` thuần, Dart thuần).
- Không làm Workspace chậm: không poll; tính năng chạy nền (mirror, run nhiều máy) chỉ chạy khi pane hiện.
- Test bắt buộc mỗi phase: unit (Vitest / cargo / flutter test) + E2E desktop liên quan + chạy thật trên Galaxy A33 (`RFCTB05YJWJ`) cho phần điện thoại. Mac: ghi bước vào `.planning/QA-mac-checklist.md`.

## 5. Tiêu chí xong toàn bộ

1. Android có cáp: từ Settings tới điện thoại dùng được ≤ 3 thao tác, không chọn file, không quét QR (đo bằng E2E `v113-connect.cjs` với adb giả + thử tay trên A33).
2. Không cáp: 1 QR, không quét lần 2.
3. Mọi E2E cũ của remote / devices xanh (selector giữ hoặc đã cập nhật cùng commit).
4. `npm test`, `npm run typecheck`, `cargo test` (core, shell), `flutter test` xanh; `node scripts/index-check.mjs` sạch.
5. `docs/remote-integration.md`, `docs/core-principle.md` (ngoại lệ 9 nếu làm F14), `.code_index/features.md` cập nhật.

## 6. Review (26/11/2026)

Agent review độc lập (opus) đối chiếu 28 trích dẫn với code, tìm 24 điểm (3 blocker, 14 major, 7 minor). Đã sửa hết vào SPEC:

- Blocker: F9 `fs.write` chỉ nhận text, ảnh vượt 1 MiB → op `fs.put` theo chunk, chỉ ghi `.orrery/inbox/`. F11 Rust không có runner kiểm → chạy qua web `runCheck`. F3 `/pair` lộ PSK, gọi liên tục chặn được ghép đôi → khoá theo IP đã tải, 1 lần, chỉ khi wizard đang ở bước QR, không thay slot đang chạy.
- Bảo mật: mã USB qua intent extra (không lộ logcat, web không giả được); link pair từ trình duyệt kiểm cùng subnet; thu hồi treo thiết bị ngay ở core; offer APK lấy path từ run session, không nhận path từ web; clipboard không đi qua push; luật vân tay ghi rõ mô hình đe doạ và giới hạn.
- Tương thích: `caps` 2 chiều trong `welcome` / `hello`; chỉ đổi `hello`, không đổi pair message; migration chữ ký 1 lần; APK arm64 (ẩn cài trên emulator x86_64); Mac stage APK 1 lần qua `ORRERY_APK`.
- Khác: đo ms bằng frame Ping có sẵn; `adb reverse` cho đường USB; quota push riêng; widget không để số cũ; khôi phục biến thể adb sau crash; spike mirror đo macOS cũ; trích dẫn lệch dòng đã sửa (`install.rs:92`, `config.rs:74-77`, `run/parse.rs:229-255`, `desk/apk/mod.rs:106`).
- Chủ nhân đã chốt Q5, Q6, Q7 ngày 26/11. Không còn câu hỏi treo; bắt đầu F0 khi chủ nhân bảo làm.