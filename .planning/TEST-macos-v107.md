# Test macOS: v104 đến v107 (+ v108 nếu đã có)

Cách dùng: làm lần lượt, `[x]` khi đúng, lỗi thì ghi ngay dưới dòng và điền bảng ở mục 12. Mục "(nếu đã có trong bản build)" bỏ qua được khi chưa thấy tính năng. Danh sách test Mac cũ hơn: `.planning/QA-mac-checklist.md`.

## 0. Chuẩn bị
Nguồn: `package.json` scripts, `scripts/dist-tauri.mjs`, `README.md:118`, `apps/shell/tauri.release.json:34`.
- [ ] `git pull` nhánh `feat/v98-pwa` -> `git log -1` thấy commit `73b8f2ea` hoặc mới hơn.
- [ ] `npm install` -> không lỗi.
- [ ] Chạy thử nhanh: `npm run shell:dev` (build web, rồi `cargo run`). Cần Rust + Xcode CLT. -> cửa sổ Orrery mở.
- [ ] Bản cài: `npm run dist:mac` (cần Rust target `aarch64-apple-darwin` + `x86_64-apple-darwin`, và `apps/layout-daemon/kit.json` đã pin). -> có `apps/shell/release/Orrery-<ver>-arm64.dmg` (và `-x64.dmg`).
- [ ] Mở `.dmg`, kéo vào Applications, mở. App ký ad-hoc, chưa notarize -> bị chặn: System Settings > Privacy & Security > Open Anyway, hoặc `xattr -dr com.apple.quarantine "/Applications/Orrery.app"`.
- [ ] Mở lần đầu -> không crash, tray hiện, 3 nút đèn đỏ / vàng / xanh nằm trong dải trên, không đè rail.
- [ ] Data dir: `ls ~/Library/Application\ Support/ccm` -> có file cấu hình, `shell.log`.
- [ ] (tuỳ chọn) fixture: `node scripts/demo-fixture.mjs --sessions=5 --subagents=3`, chạy app với `CLAUDE_CONFIG_DIR=<root in ra> CCM_VERIFY_PROCESSES=0`.
- [ ] (tuỳ chọn) E2E: `npm run e2e -w @ccm/desktop` cần build shell debug trước (`node scripts/gen-desk-shim.mjs`, `cd apps/shell && cargo build -j 4`). Chưa kiểm là chạy được trên Mac (xem Chưa xác minh).

## 1. Giao diện desktop gọn v107: rail
Nguồn: `.planning/DECISION-v107-declutter.md`.
- [ ] Rail: không còn logo O, không số session cạnh tên project, không nút ngắm khi hover. -> đúng.
- [ ] Hover hàng session -> hiện nút đóng; branch, vòng context mini chỉ hiện khi hover / focus.
- [ ] Chip Telegram và chip điện thoại ở chân rail luôn hiện; "Lịch sử", "Cài đặt", menu "..." còn.
- [ ] Dòng "Công cụ N" gập / mở được, chấm file chưa lưu hiện ở đó.
- [ ] Hàng session chờ: chip "Chờ bạn", chi tiết + thời gian gộp trên cùng hàng, không xuống dòng thừa.
- [ ] Thu rail bằng Cmd+Shift+B, cả khi đang focus terminal -> thu / mở, tắt mở app vẫn nhớ.
- [ ] Cửa sổ hẹp -> rail thu icon / overlay vẫn đúng.

## 2. Tab, toolbar, Inbox, màn trống
- [ ] Dải tab: không chip project đầu dải, không chuông việc project khác, không chữ "2 pane". -> đúng.
- [ ] Tab hiện ô chữ cái + chấm trạng thái; Lỗi và "chờ" là chấm, không chip chữ.
- [ ] U11: tab đặt tên theo pane; chip project trên tab chỉ hiện khi có nhiều project.
- [ ] Nút "Bố cục" chỉ hiện khi hover dải tab.
- [ ] Inbox: Cmd+Shift+A mở / đóng; Ctrl+Option+Q nhảy mục kế, panel không mở. Phím J/K, E, S dùng được, dải phím ẩn.
- [ ] Inbox dòng 2 chỉ có thời gian ("2m"); nhóm Xin quyền / Câu hỏi / Lỗi đúng thứ tự.
- [ ] Phím S hoãn 15 phút -> toast Hoàn tác.
- [ ] U12: đóng hết pane -> màn trống 1 dòng + danh sách khôi phục, không tiêu đề dài.

## 3. Đầu pane, footer quota, Ctrl+K (Cmd+K)
- [ ] Đầu pane: không số token context; chip diff, điện thoại đang xem, ghim đính kèm, zoom, chia pane chỉ hiện khi hover.
- [ ] Context chỉ hiện từ 70%. Vòng subagent phẳng. Chip chạy: "Bash 1m" hiện.
- [ ] Footer quota: không dòng "Chưa có hạn mức gói"; thanh 5 giờ / 7 ngày ẩn khi mức ok; "Sắp hết" hiện; bấm mở popover chi tiết.
- [ ] Cmd+K: ô tìm + 5 tab phạm vi; không phím Esc, không số đếm cạnh nhóm, không dải prefix; "x phút trước" gộp vào dòng phụ.
- [ ] Cmd+K "Ghép điện thoại" mở QR. (Ctrl+K có là Cmd+K trên Mac không: ghi lại.)

## 4. Giám sát và Settings
- [ ] Giám sát (Bảng): menu "Hiển thị"; cột 0 ẩn; card click mở Workspace; không rail session khi ở Giám sát.
- [ ] U13: History hôm nay có hoa văn, "làm N%", gộp session idle.
- [ ] Settings còn 7 trang (không còn thanh 14 trang): Thông báo + Telegram + Remote gộp; GitHub + Tích hợp + DeepSeek gộp; Dữ liệu + Ổ đĩa + Quyền gộp; Editor + Workspace gộp. Office không hiện ở Chung.
- [ ] Ô tìm setting tìm ra mục nằm trong trang gộp, bấm đi đúng chỗ.

## 5. Bảng MCP và plugin (header session)
- [ ] Mở bảng ở header session -> checklist, lưới nhiều cột, lọc, bật / tắt cả nhóm.
- [ ] Tick vài mục -> chỉ 1 nút "Áp dụng"; bấm -> 1 lần ghi, không chờ 1.5 s mỗi mục.
- [ ] Mỗi mục có số lần gọi theo session; mục không dùng có gợi ý "tắt".
- [ ] Mở lại bảng -> trạng thái khớp lệnh `claude mcp list` / `claude plugin list`.

## 6. Remote: bật, QR cài app, relay
Nguồn: `apps/shell/src/remote/relay_wrangler.rs:66-82`, `relay_setup.rs:74`.
- [ ] Settings > Remote: bật. -> cổng LAN 4320 lắng nghe, chip điện thoại ở chân rail đổi trạng thái.
- [ ] Thẻ cài app bằng QR: Android hiện QR APK qua Wi-Fi (token 10 phút), iPhone hiện link `/app/` của relay riêng. Không có relay thì iPhone báo cần cài relay trước.
- [ ] Android: quét QR -> tải đủ ~40 MB trên Wi-Fi; hết 10 phút link không tải được.
- [ ] Relay 1 chạm bằng wrangler: bấm -> máy có Node + npx thì tự chạy `wrangler@4.160.1`, mở trình duyệt login Cloudflare lần đầu, deploy xong hiện URL + "Jin" (tên account).
- [ ] App mở từ Finder (PATH ngắn): wrangler vẫn tìm được npx ở `/opt/homebrew/bin`, `/usr/local/bin`, `~/.volta/bin`, `~/.local/bin`, `~/.nvm/versions/node/<bản mới nhất>/bin`, `~/.asdf/shims`. Không có Node nào -> phải rơi về đường token, không treo. Ghi lại thông báo thấy.
- [ ] Đường token: bấm -> mở trang Cloudflare tạo token với quyền điền sẵn; tạo token, copy; quay lại Orrery -> tự đọc clipboard và nhận token (không dán tay). macOS có hỏi quyền đọc clipboard không: ghi lại.
- [ ] Relay lỗi (token sai / copy sai thứ khác) -> báo lỗi rõ, không lưu.
- [ ] Gỡ relay (kèm worker) chạy được, token trong Keychain bị xoá.
- [ ] Ghép điện thoại bằng QR / mã. Từ chối trên hộp native -> bảng QR hiện "đã từ chối" + nút tạo mã mới.
- [ ] Tắt relay rồi bật lại -> desktop nối lại trong khoảng 15 s.

## 7. Chat trên điện thoại (iPhone web app + Android)
Nguồn: `.planning/PLAN-v107-phone-chat.md`, `SPEC-v107-chat-protocol.md`.
iPhone: Safari mở `<relay>/app/` > Share > Add to Home Screen > mở từ icon.
- [ ] Hộp thư: mỗi session 1 dòng (tên, câu cuối, trạng thái, thời gian), "Chờ bạn" lên đầu.
- [ ] Bấm session Claude / Codex -> mở chat (bong bóng câu hỏi, agent không bong bóng); pane shell mở thẳng terminal. Nút "Xem terminal" có.
- [ ] Mở chat lần 2 nhanh (cache trước, dưới 1 giây).
- [ ] Gửi tin khi agent đang chạy -> vào session ngay, trên Mac thấy chữ xuất hiện trong pane đang làm, không đợi rảnh.
- [ ] Nút Dừng -> agent dừng (Esc) trong pane Mac.
- [ ] Xin quyền / menu: bong bóng có nút (Có / Không, từng lựa chọn). Bấm -> đếm 3 s Hoàn tác; hết 3 s mới gõ phím vào pane Mac; Hoàn tác kịp thì không gõ.
- [ ] Mục menu "Other" chỉ có nút mở terminal.
- [ ] Thẻ file đã sửa: +/- ước lượng; bấm xem diff hiện tại của file.
- [ ] "Shell mới" mở pane shell trên Mac.
- [ ] Tắt Wi-Fi điện thoại, gõ phím: dải "Chưa gửi" + Gửi / Bỏ; bật lại mạng, bấm Gửi mới gửi, Bỏ thì xoá. Không tự gửi.
- [ ] Push khi app ở nền: thông báo có agent, session, câu hỏi (đã che secret). Bấm -> mở thẳng chat của session đó.
- [ ] Telegram: nhắn bot trong topic của project (đã `/bot-tele-receive on`) khi agent đang làm -> tin vào pane ngay, không đợi rảnh.
- [ ] Tool call hiện 1 dòng gộp ("Đã sửa 3 file, chạy 2 lệnh"), đường dẫn tương đối; không thinking, không kết quả tool thô.

## 8. Giao diện Mac riêng
- [ ] Retina: chữ, icon rail, border 1 px sắc nét, không mờ.
- [ ] Font hệ thống là SF; tiếng Việt có dấu đúng trong rail, tab, Settings.
- [ ] Trackpad: cuộn 2 ngón mượt trong terminal, Explorer, Giám sát; kéo mép Explorer; không cuộn ngang cả trang.
- [ ] Phím: Cmd+Shift+E (Explorer), Cmd+Shift+B (thu rail), Cmd+Shift+A (Inbox), Ctrl+Option+Q, Ctrl+Option+P (chuyển dự án). Ghi phím nào không chạy.
- [ ] Cmd+W không đóng nhầm app khi đang ở compact; Esc thoát compact.
- [ ] Traffic lights: không đè rail / logo; double click dải trên = phóng to / thu nhỏ; kéo cửa sổ bằng dải trên.
- [ ] Full screen (nút xanh / Ctrl+Cmd+F): dải trên, rail, footer không đè nhau; thoát lại bình thường.
- [ ] 2 màn hình: kéo cửa sổ sang màn khác -> co giãn đúng, không vỡ chữ; mở lại app nhớ vị trí.
- [ ] Keychain: lần đầu có thể hỏi quyền (token relay, tài khoản GitHub, VAPID, `Orrery Safe Storage`). Chọn Always Allow -> lần sau không hỏi lại.
- [ ] Thông báo: app ký ad-hoc nên toast OS có thể không hiện. Ghi lại có / không (dock badge / bounce theo `CCM_TOAST_BACKEND`, Tauri có thể chưa có, đừng coi là lỗi).
- [ ] Sleep / wake: gập máy 1 phút, mở lại -> remote tự nối lại trong khoảng 15 s, phone thấy lại pane, không phải ghép lại.
- [ ] Đóng cửa sổ -> ẩn vào tray; mở lại từ tray và từ Dock đều được; thoát hẳn từ menu tray.

## 9. Workspace cơ bản (kiểm không hỏng)
- [ ] Mở Claude, Codex, Shell; resize; `cd` đổi nhánh; đóng pane; Hoàn tác đóng pane (`--resume`).
- [ ] Kéo thả file từ Finder vào pane. Deep link `agent-monitor://open?agent=shell&cwd=<thư mục>` mở đúng pane.
- [ ] Click menu v100 (hover, click, double click) trên menu của Claude / Codex.
- [ ] Git Log, Changes (lọc theo session), Explorer icon Atom Material.

## 10. Bảng màu v108 (nếu đã có trong bản build)
Nguồn: `.planning/DECISION-v108-colors.md`. Lúc viết, `apps/web/src/lib/themes.ts` chưa có `mono`: chưa vào code.
- [ ] Settings > Giao diện có "Đen trắng"; UI next còn: Sương kính, Than chì, Ngân hà, Orrery, Tự chọn.
- [ ] Máy đang dùng Tinh vân / Cực quang / Bình minh sương / Console / theme classic cũ -> lần đầu mở tự chuyển sang bảng gần nhất, chỉ 1 lần.
- [ ] Ngẫu nhiên: bật -> mỗi lần mở app ra 1 bảng màu mới, chữ đọc rõ (AA), không trùng bộ có sẵn.
- [ ] "Trộn lại" ra bảng khác ngay; "Giữ màu này" -> mở lại app vẫn đúng màu đó, danh sách "đã giữ" có mục mới.
- [ ] Lúc mở app không nháy sai màu (không flash trắng / màu cũ rồi mới đổi).
- [ ] Sáng / tối / theo hệ thống: đổi Dark Mode macOS -> app theo.

## 11. Rà chữ (nếu đã có trong bản build)
- [ ] Lướt rail, Settings, Inbox, Ctrl+K, bảng MCP, màn chat điện thoại: không còn dấu chấm giữa làm ngăn cách, không em dash. Ghi chỗ còn sót (chụp ảnh).
- [ ] Không còn chữ "trần" chỉ giới hạn trên (phải là "max").

## 12. Báo lỗi
Chụp và gửi:
- Ảnh màn hình Mac (Cmd+Shift+4) và ảnh điện thoại, có thấy dòng trạng thái.
- Log: `~/Library/Application Support/ccm/shell.log` (và `shell.old.log`), `pty.log` cùng thư mục. Crash: Console.app > Crash Reports > Orrery. Nếu app có nút gửi báo lỗi (bug report) thì dùng nó, nó tự gom log đã che secret.
- Phiên bản: Orrery về app, `git log -1 --oneline`, macOS + chip, iPhone / Android + trình duyệt.

| Bước | Kết quả | Ghi chú |
|---|---|---|
| ví dụ 6.5 | relay không tìm thấy npx | Node cài kiểu khác (fnm, n), thông báo: ... |
|  |  |  |
|  |  |  |

## Chưa xác minh khi viết
- E2E Tauri chạy trên Mac được hay không (script có nhánh darwin, chưa thấy tài liệu chạy).
- Tên thật của nút Telegram / tên mục menu trong i18n từng mục (các nhãn ở trên theo DECISION, có thể lệch chữ).
- Cmd có thay Ctrl cho Ctrl+K trên Mac không (README chỉ nói `Ctrl+Alt+P` thành `⌃⌥P`).
- macOS có hỏi quyền clipboard khi `arboard` đọc không; đường dẫn `pty.log`.
- Themes v108 và rà em dash / chấm giữa: chưa có trong code lúc viết.
