# PLAN v98 — Orrery Remote web app (PWA) cho iPhone / iPad + ghép 2 đường + push có nội dung

Chốt qua grill với chủ nhân 2026-11-12. Dùng nội bộ: không store, không ký, không Apple Developer (memory `internal-distribution`).

## Quyết định đã chốt

| # | Chủ đề | Quyết định |
|---|---|---|
| 1 | Công nghệ | **Vue 3 + Vite + Pinia + vue-i18n**, workspace mới `apps/pwa`; dùng `packages/shared` (protocol), xterm.js như desktop, token màu Orrery; UI + motion theo mockup `docs/design/v97-mobile-ui` |
| 2 | Mã hoá | Noise = **Rust `core-remote-proto` → wasm** (wasm-bindgen), cùng code với desktop + Android |
| 3 | Host | Web app nằm trên relay của chủ nhân `https://<relay>/app/`; **chấp nhận** relay chủ nhân tin được, nhắc bật 2FA Cloudflare, ghi đánh đổi vào docs |
| 4 | Relay | PC + Mac **dùng chung 1 relay** (`orrery-relay.jlst.workers.dev`) |
| 5 | LAN iPhone | Web app tải từ relay; **LAN qua CA riêng**: mỗi máy 1 CA (khoá trong keychain máy đó), Settings → Điện thoại có QR tải hồ sơ `.mobileconfig`; desktop mở `wss://` LAN bằng chứng chỉ ký từ CA; mất LAN tự về relay. Cần spike Safari |
| 6 | Chọn đường | Mỗi máy có select **"Kết nối: Tự động / Chỉ LAN / Chỉ relay"** (mặc định Tự động). IP LAN desktop tự báo qua relay mỗi lần nối, người dùng không nhập gì |
| 7 | Ghép máy | **Cả 2 đường**: QR mới chứa LAN + relay + mã 1 lần; thử LAN ~3 s, lỗi (khác mạng / chưa cài CA) thì ghép qua relay. **Android dùng luồng mới cùng đợt**, QR cũ vẫn đọc được |
| 8 | Khoá app | Face ID / Touch ID qua **WebAuthn**, mặc định 1 giờ; máy không hỗ trợ thì báo và bỏ khoá |
| 9 | Phạm vi | **Đủ tính năng 1 lượt**, ngang Android: ghép, Home nhiều máy, Monitor A/B, Workspace A/B, terminal (Ctrl/Alt dính), viewer file/diff/git/run, Cài đặt, khoá, push, đổi tên |
| 10 | Quyền | **Ngang Android** (điều khiển từ xa mặc định tắt trên desktop, thao tác nguy hiểm xác nhận trên điện thoại) |
| 11 | Cache | Lưu snapshot cuối trong IndexedDB (sau khoá), thu hồi thiết bị thì xoá sạch |
| 12 | iPad | Hỗ trợ, ≥ 768 px chia đôi như tablet Android |
| 13 | iOS native | **Xoá `apps/mobile/ios/`**; tối nay chủ nhân chỉ test Mac desktop + A21s |
| 14 | Cập nhật | Bản web đóng gói trong app desktop, "Cập nhật relay" đẩy luôn web mới; web app thấy bản mới / giao thức lệch thì hiện thanh "Có bản mới — tải lại", không tự reload |
| 15 | Push iPhone | **Web Push (VAPID)** từ desktop, khoá VAPID mỗi máy cất keychain; 404 / 410 = xoá subscription |
| 16 | Push có nội dung | **Cả iPhone + Android**: agent · tên session · project; thân = câu hỏi / quyền đang chờ, "Xong sau 12m" hoặc lỗi; lọc secret, cắt 120 ký tự. **Hiện đủ trên màn khoá** |
| 17 | Android FCM | **Mã hoá bằng khoá thiết bị**, gửi data message; app giải mã rồi tự hiện thông báo; giải mã lỗi thì hiện bản không nội dung. Google chỉ thấy gói mã hoá |
| 18 | Ngoại lệ | Push có nội dung = **ngoại lệ thứ 7** của quy tắc không lộ nội dung session → ghi vào `CLAUDE.md` + docs; Cài đặt → Thông báo có công tắc **"Kèm nội dung"** (mặc định bật), tắt thì về thông báo cố định |
| 19 | Bật / tắt nhanh | **Chuông trên thẻ từng máy ở Home điện thoại** (tắt push từ máy đó) + **nút chuông trên desktop** mở danh sách điện thoại, bật / tắt từng cái |
| 20 | Giữ nguyên | 3 nhóm (chờ / xong / lỗi), 1 pane 30 s, 30 lần / giờ, không push khi app đang mở, chạm mở thẳng pane, Telegram giữ nguyên |
| 21 | Test | Vitest + **Playwright WebKit** (E2E với dev app + relay thật) + chủ nhân test iPhone / iPad thật cuối đợt |
| 22 | Cách làm | Như v97: chia gói giao subagent, main kiểm + review, **commit + push dần**, nhánh `feat/v98-pwa` từ `feat/v97-mobile`, PROGRESS + HANDOFF |
| 23 | Chặn IP tự động | Relay + cổng LAN desktop: **20 lỗi / 10 phút** (bắt tay Noise sai, token sai, mã ghép sai, bị 429, khung sai) → chặn **1 giờ → 24 giờ → 7 ngày** nếu tái phạm. IP thiết bị đã ghép đang nối hợp lệ và IP trong danh sách tin cậy không bao giờ bị chặn tự động |
| 24 | Danh sách IP | Desktop Settings → Điều khiển từ xa: xem (IP, nguồn relay / LAN, lý do, số lần, lúc chặn, hết hạn), **gỡ từng / gỡ hết** (xác nhận), **chặn tay IP / CIDR** (có hạn hoặc vĩnh viễn), **danh sách tin cậy**, **nhật ký truy cập** 200 lần nối / lỗi gần nhất |
| 25 | Siết relay | Mỗi IP max 4 kết nối đồng thời, chưa bắt tay xong trong 5 s thì cắt, khung quá cỡ thì cắt + tính lỗi; web app CSP chặt (không script lạ / inline, chỉ nối relay + LAN) + HSTS |
| 26 | Ghép qua relay | Mã ghép sống 2 phút, sai 5 lần là huỷ; **desktop hiện hộp duyệt** "Ghép <tên máy> từ IP <ip>?" mới cho ghép |
| 27 | Cảnh báo | Desktop + Telegram + push tới điện thoại cũ khi: **thiết bị mới ghép**; **có IP bị chặn** (gộp, max 1 lần / giờ); **thiết bị đã ghép nối từ IP / quốc gia chưa thấy** (quốc gia từ `cf-ipcountry` của relay) |
| 28 | Dev không cần Mac | Cu em làm trên Windows + iPhone: Vite dev + Cloudflare quick tunnel (https thật trên iPhone), Eruda console chỉ bản dev + log gửi về dev server, Playwright WebKit trên Windows; không debug breakpoint Safari |

## Kiến trúc

```
iPhone / iPad Safari PWA (apps/pwa, Vue + wasm Noise)
   ├─ wss://<IP LAN>:4320  (CA riêng, khi cùng mạng)      ┐
   └─ wss://<relay>/…      (luôn có, mã hoá đầu cuối)     ├─► Desktop shell (PC / Mac)
Relay Cloudflare: /app/* (static PWA) + phòng nối + ghép qua relay ┘
Desktop ──Web Push (VAPID, nội dung mã hoá theo RFC 8291)──► Apple push ──► service worker
Desktop ──FCM data message (mã hoá bằng khoá thiết bị)──► Android app giải mã, tự hiện
```

## Gói việc (thứ tự)

| WP | Việc | Phụ thuộc |
|---|---|---|
| 50 | **Spike** (làm trước, chặn mọi thứ): (a) `core-remote-proto` → wasm, bắt tay Noise với desktop qua relay từ WebKit; (b) Safari tin CA tự cài + `wss://` LAN từ trang https (kể cả PWA đã thêm màn hình chính); (c) Web Push tới iPhone thật | — |
| 51 | Desktop: CA mỗi máy (keychain), chứng chỉ LAN cho mọi IP, listener `wss` song song `ws`, endpoint tải `.mobileconfig`, báo IP LAN qua relay | 50 |
| 52 | Ghép 2 đường: QR v2 (LAN + relay + mã 1 lần), ghép qua relay phía desktop + relay worker, hộp duyệt trên desktop; Android chuyển sang luồng mới, đọc QR cũ | 50 |
| 53 | Relay: phục vụ `/app/*` (manifest, SW, CSP, HSTS), desktop đẩy bản web khi "Cập nhật relay" | 50 |
| 54 | `apps/pwa` khung: Vite + Vue + Pinia + i18n + theme token, kết nối (wasm Noise, select đường), store snapshot / reducer từ shared, cache IndexedDB, thanh "Có bản mới" | 50, 53 |
| 55 | `apps/pwa` màn: Home nhiều máy (chuông từng máy, select kết nối), ghép (camera QR + dán mã), Monitor A/B, chi tiết session, Workspace A/B, terminal xterm.js + phím phụ (Ctrl / Alt dính), viewer file / diff / git / run, Cài đặt, khoá WebAuthn, iPad | 54 |
| 56 | Push có nội dung: nội dung lọc + cắt (desktop), Web Push VAPID, FCM data mã hoá + Android giải mã tự hiện, công tắc "Kèm nội dung", chuông desktop chọn từng điện thoại, ngoại lệ thứ 7 trong `CLAUDE.md` + docs | 51, 54 |
| 57 | Dọn: xoá `apps/mobile/ios/`, docs `orrery-remote.md` (cài web app, cài CA, đánh đổi relay), `.code_index` | — |
| 58 | QA: Vitest, Playwright WebKit E2E với dev app + relay, chủ nhân test iPhone / iPad / Mac / A21s | tất cả |
| 59 | Bảo mật: bộ đếm lỗi + chặn tăng dần ở relay (Durable Object lưu danh sách) và cổng LAN desktop; desktop báo IP lỗi Noise cho relay chặn; API quản lý danh sách qua kênh desktop ↔ relay đã xác thực; UI danh sách / gỡ / chặn tay / tin cậy / nhật ký; giới hạn kết nối + timeout bắt tay; cảnh báo | 51, 52, 53 |

## Chọn model khi giao subagent

Máy làm v98 **không có jin-workflow** → main agent tự chọn model theo bảng này, ghi model vào bảng WP trong PROGRESS (`opus` / `sonnet` / `haiku` của Agent tool).

| Việc | Model | Lý do |
|---|---|---|
| WP-50 spike | `opus` | Nhiều ẩn số, quyết định kiến trúc |
| WP-51, 52, 56, 59 | `opus` | Crypto / bảo mật / giao thức, sai là lộ khoá |
| WP-53, 54, 55, 58 | `sonnet` | Làm theo plan + mockup, khối lượng lớn |
| WP-57 docs, `.code_index` | `sonnet` | Viết tài liệu theo `GLOSSARY` của repo |
| Tra code, tìm file, tóm tắt (không sửa) | `haiku` | Rẻ, nhanh |
| Review gói `opus` | `opus`, agent khác, context mới | Review bảo mật phải độc lập |
| Review gói `sonnet` | `sonnet` | |

WP về `SỬA` 2 vòng mà vẫn trượt → nâng model 1 bậc (`haiku` → `sonnet` → `opus`), ghi vào cột Ghi chú.

Luật kèm vào **mọi** prompt giao subagent (máy không nạp sẵn jin-workflow):
- Không viết comment trong code (kể cả `///`, `/** */`, `#`, TODO); lý do / invariant ghi `.code_index/` hoặc `.planning/`.
- Không `git commit` / `push` trong subagent; main commit sau khi nghiệm thu. Không thêm trailer `Co-Authored-By` / "Generated with".
- File mới ≤ ~400 dòng, quá thì tách.
- Không chạy build release để verify; chạy test / typecheck của workspace bị đụng.
- Tool nặng (Gradle, cargo build lớn, Playwright) ≤ 80% CPU / RAM, không chạy 2 cái cùng lúc giữa các subagent; tối đa 3 subagent song song.
- Text UI qua vue-i18n `vi` + `en`, màu qua token theme, trạng thái = icon + chữ + màu.
- Báo cáo về theo mẫu: đã làm gì (file + symbol), test chạy thật (số pass / fail), việc còn dở, rủi ro.

## Luật PROGRESS + HANDOFF (máy không có skill hand-off)

Hai file, hai vai trò, không gộp. Đặt trong `.planning/`.

**PROGRESS** — `.planning/PROGRESS-<YYYY-MM-DD>-v98.md`, nhật ký chạy dần, mới nhất ở CUỐI:
- Mở ngay khi bắt đầu đợt; **append sau mỗi bước** (xong việc, giao agent, agent báo về, bị chặn), không đợi cuối.
- Mỗi dòng: `- [x] / [~] / [ ] <việc> — file, lệnh, kết quả kiểm thật (số pass / fail), commit`. Không văn xuôi.
- Đầu file giữ 2 mục **sửa tại chỗ**: `## Tóm tắt hiện tại` (≤ 8 dòng) và `## Bảng WP` (`WP | Model | Trạng thái | Vòng | Test | Review | Commit | Ghi chú`; trạng thái `GIAO` · `VỀ` · `ĐẠT` · `SỬA` · `CHỜ XEM` · `BLOCKED`).
- Đợt mới trong ngày → thêm `## Phiên <giờ> (sau HANDOFF-xxxx) — <chủ nhân chốt gì>`. File > ~150 dòng → chuyển phiên đã đóng sang `.planning/progress/PROGRESS-<ngày>.md`.

**HANDOFF** — `.planning/HANDOFF-<YYYY-MM-DD>-<HHMM>.md`, ảnh chụp trạng thái **đã kiểm**:
- Viết khi: xong 1 cụm WP / 1 lượt agent về đủ; bị chặn / cần chủ nhân quyết; ~60 phút có tiến triển; sắp hết quota / context / sắp `/clear`; kết thúc đợt. Không viết sau mỗi bước lẻ.
- Trước khi viết **chạy lệnh lấy sự thật**: `git status --short`, `git log --oneline -5`, test vừa chạy, việc nền còn sống, thiết bị. Không viết theo trí nhớ, không ghi "pass" khi chưa chạy.
- Khung cố định: `1. Đang ở đâu` (commit chưa, đang chạy gì, bị chặn gì) · `2. Đã làm` (hành vi + file / symbol) · `3. Việc tiếp` (thứ tự, lệnh copy được, đường dẫn tuyệt đối, điều kiện xong; câu hỏi treo kèm mặc định) · `4. Quyết định + ràng buộc` (chủ nhân đã chốt — đừng đề xuất lại) · `5. File cần đọc lại`. Gọn ~60–130 dòng.
- Ghi "File này thay `<handoff trước>`", chuyển bản cũ vào `.planning/archive/`, append `- [x] <giờ> Handoff mới: <file>` vào PROGRESS.
- Ngày giờ tuyệt đối; không secret / token / mật khẩu.

**Resume** (session mới): đọc HANDOFF mới nhất → đầu PROGRESS → cuối PROGRESS → plan này; kiểm lại `git status` / thiết bị trước khi làm, lệch thì tin thực tế và ghi vào PROGRESS.

## Rủi ro

- Safari từ chối `wss://` LAN dù đã cài CA (Private Network Access / chính sách ATS) → WP-50 trả lời; không được thì iPhone chỉ relay.
- wasm Noise lệch byte với Rust native → test chéo với vector từ desktop.
- Android tự hiện thông báo từ data message có thể bị Samsung chặn khi app bị kill → giữ fallback thông báo cố định của FCM nếu không giải mã kịp.
- Web app iOS: xin quyền thông báo chỉ được sau khi đã thêm vào màn hình chính và phải do người dùng bấm.
- Mất relay = iPhone khác mạng mất kết nối.
- Chặn nhầm chính mình (gõ sai mã ghép nhiều lần, 4G đổi IP) → danh sách tin cậy + gỡ từ desktop; desktop luôn vào được qua LAN để gỡ.
