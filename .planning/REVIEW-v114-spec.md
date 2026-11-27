# REVIEW v114 spec (G1-G5)

Đối tượng: `.planning/PLAN-v114-ui.md`, `.planning/SPEC-v114-ui.md`. Review chỉ đọc code, không chạy build / test. Ngày 27/11.

Tổng: **CAO 5, VỪA 16, THẤP 6**.

---

## CAO

### C1. G1: op `ws.snooze` + file `remote/snooze.rs` là thừa, đường snooze phone đã có
- Spec: "Phone ... không gửi được snooze"; thêm op `ws.snooze`, nhánh mới trong `phone_ws.rs`, file `remote/snooze.rs`, WP tự chọn đường.
- Code: shell đã forward `terminal.snooze` từ phone vào core qua tap: `apps/shell/src/remote/dashboard_msg.rs:8` (`DASHBOARD` có `terminal.snooze`), `dashboard.rs:216` (`k if DASHBOARD.contains(&k) => self.tap(...)`), `dashboard.rs:293-309`; core cho qua tap `apps/core/crates/core-http/src/tap.rs:11`, xử lý `core-serve/src/dispatch.rs:130-139` (lưu `snooze.json`, phát `snooze.updated`, ẩn toast). Phone chỉ là chưa gọi (`apps/mobile/lib` không có `terminal.snooze`).
- Sửa: bỏ op mới nếu chọn snooze của core; phone gửi `{"t":"terminal.snooze","terminalId","minutes"}`. Nếu cần whitelist `minutes ∈ {15,60,null}` thì thêm kiểm tại `dashboard.rs::tap` (core hiện nhận mọi số dương, `dispatch.rs:132`). Nhưng xem C2 trước: đường này có thể là sai hệ snooze.

### C2. G1: có 2 hệ snooze tách rời, spec không chọn; UI next + phone inbox dùng hệ KHÔNG nằm ở core
- Spec: "Desktop có `monitor/stores/snooze.ts` (`SNOOZE_CHOICES = [15, 60]`, snooze qua `connection.snoozeTerminal` → core)"; E2E "snooze store desktop đổi".
- Code:
  - Hệ 1 (core, monitor classic): `features/monitor/stores/snooze.ts:6-35`, key `terminalId`, có `since = statusSince` nên câu hỏi mới tự bỏ hoãn, lưu `snooze.json`.
  - Hệ 2 (UI next): `features/next/lib/railSnooze.ts:5-24`, **chỉ trong RAM** (mất khi reload webview), key `paneKey`, mức `[5,15,60]` (`railLive.ts:8`), lỗi tối đa 15 phút (`railLive.ts:111-114`), không phân biệt câu hỏi mới.
  - Inbox desktop dùng hệ 2: `features/click-menu/stores/inbox.ts:8,88,110-114` (`INBOX_SNOOZE_MIN = 15`). Inbox này là nguồn `inbox` của summary gửi phone: `features/remote/composables/useRemoteSummary.ts:25-31`.
  - Hệ quả: phone hoãn qua core (C1) thì rail next, inbox desktop và inbox phone **không ẩn**; hoãn qua hệ 2 thì core / toast / `snooze.updated` không biết, mất khi reload.
- Sửa: chủ nhân chốt (Q1). Đề xuất: op ghi đi qua webview như `ws.*` (`useRemotePhone.ts:214-225` thêm handler `ws.snooze {paneKey, minutes}` gọi `useInboxStore().snooze` / `railSnooze`), đồng thời gọi `useSnoozeStore().snooze` khi pane có `terminalId` để toast core cũng im. Key theo `paneKey` (phone có sẵn), không phải `terminalId`. Ghi rõ trong spec hệ nào là nguồn thật; cân nhắc lưu railSnooze (ít nhất `sessionStorage`) và bỏ hoãn khi `since` của câu hỏi đổi.

### C3. G1: "ô trả lời chữ" cho câu hỏi mở không chạy được như spec viết
- Spec: "ô trả lời chữ (nếu là câu hỏi mở)"; gửi qua `ws.send`; thẻ hiện "câu hỏi (đã mask)".
- Code:
  - `ws.send` → `useRemotePhone.ts:168-177` đặt `prefill + prefillSubmit + prefillNow`; `TermPane.vue:750-753` `prefillReady()` trả false khi status `waiting` (chỉ idle, hoặc `prefillNow` + working, hoặc late + không working/waiting). Pane đang chờ thì chữ **không được gõ** cho tới khi hết chờ.
  - Summary phone không có nội dung câu hỏi khi không phải menu: `remoteSummary.ts:46,56` `waitText = waitReason(...)` là lý do, không phải câu hỏi. Chỉ `menu.question` có chữ (masked, `chat_src.rs:352-353`).
- Sửa: v114 chỉ hỗ trợ thẻ có `menu` (trả lời bằng nút); câu hỏi mở / permission không có menu thì thẻ chỉ có "Mở pane" (mở chat). Nếu chủ nhân muốn ô chữ (Q3) thì phải spec đường riêng (chọn mục "Other" của menu rồi gõ, hoặc `ws.keys`), có test pane `waiting`.

### C4. G4: không có nguồn "3 dòng cuối" ở chỗ spec chỉ; nguồn thật là buffer xterm, và với TUI agent thì vô nghĩa
- Spec: "3 dòng cuối của transcript / terminal ... nguồn: `features/monitor` card preview, `pane_preview` phía phone".
- Code:
  - `TerminalSession` không có trường dòng cuối (`packages/shared/src/types.ts:22-70`); monitor card không có preview transcript.
  - Phone `pane_preview.dart:16-27` dựng từ `waitText`, tool + summary, số subagent, nhãn trạng thái, không phải transcript.
  - Nguồn có thật: `features/workspace/lib/paneSnapshots.ts:19-22` `snapshotOf(key, rows)` đọc buffer xterm đang mount (`TermPane.vue:34` register), chỉ có khi pane đã mount (`hasSnapshot`).
  - Với Claude Code / Codex TUI, các dòng cuối màn hình là ô nhập + footer, không phải câu trả lời.
- Sửa: ghi rõ nguồn `snapshotOf`, cách bỏ phần khung TUI (vd lấy 3 dòng không rỗng phía trên ô nhập, hoặc bỏ hẳn cho pane agent và chỉ dùng cho shell), pane chưa mount thì "Chưa có hoạt động". Hoặc bỏ mục 3 dòng, chỉ giữ tool + thời gian + câu hỏi (Q2).

### C5. G3: transition 160 ms trên `height` / `padding` của pane header gây resize xterm + PTY liên tục
- Spec: "Đổi mật độ có chuyển tiếp 160 ms trên khoảng cách (chỉ padding / height qua CSS var)"; phạm vi có pane header.
- Code: đổi chiều cao header làm đổi chiều cao thân pane → ResizeObserver fit xterm và gửi resize PTY cho **mọi** pane terminal trong suốt 160 ms (nhiều frame). Trái `ide-performance-first`.
- Sửa: đổi mật độ tức thì, không transition layout; nếu muốn motion thì crossfade opacity 160 ms toàn shell. Test: đổi mật độ chỉ gây ≤ 1 lần resize PTY mỗi pane.

---

## VỪA

### V1. G1: thứ tự "chờ lâu nhất trước (cùng quy tắc `nextAttention`)" sai
- Code: `workspace.ts:1353-1360` sắp theo thứ tự tab / leaf, waiting trước done, không theo thời gian chờ.
- Sửa: sắp theo `inbox.since` hoặc `menu.since` (phone có `InboxInfo.since`, `inbox_model.dart:33-55`; `RemoteMenu.since`, `chat_protocol.dart:546`), thiếu thì theo thứ tự summary.

### V2. G1: "view-only phone bị từ chối như op ghi khác" dựa trên trạng thái không có ở bản thật
- Code: `Host::control()` mặc định `true` (`remote/ctx.rs:76-78`), `AppHost` không override (`remote/host.rs`), chỉ `test_util.rs:78-80` có `view_only`. Op `ws.*` không kiểm `control()` (`phone_ws.rs:160-178`); chỉ `clip.rs:113`, `fs_put.rs:227` kiểm.
- Sửa: ghi rõ guard = `self.ctx.host.control()` như `fs_put.rs:227` (test qua `view_only` của test host), và ghi rằng bản thật hiện "tắt điều khiển" = tắt remote.

### V3. G1: menu trong thẻ, giới hạn của đường có sẵn + mất bước hỏi lại menu nguy hiểm
- Code: `apps/mobile/lib/app/menu_action.dart:15-27` `menuItemFor` loại mục `other`, `submit`, checkbox; `runMenuAction` (33-72) có undo 3 s. IDEAS G1 yêu cầu "vẫn hỏi lại với menu nguy hiểm (E5)", spec bỏ.
- Sửa: thẻ dùng lại `runMenuAction`; menu chỉ có checkbox / Other thì ẩn nút, hiện "Mở pane"; thêm lại bước xác nhận menu nguy hiểm hoặc ghi lý do bỏ (Q4).

### V4. G1: "chạm thông báo khi ≥ 2 pane chờ" không biết số lúc chạm
- Code: payload push theo 1 pane (`push/push_host.dart:36-60`); app mở lạnh thì state cũ / chưa nối.
- Sửa: định nghĩa: mở pane như cũ, sau khi nối xong nếu còn ≥ 2 pane chờ thì hiện nút "Còn N đang chờ"; hoặc đếm từ state đã cache và ghi rõ khả năng lệch. Thêm route `m/<m>/queue` vào `app/pane_link.dart` (82 dòng) + test.

### V5. G1: undo và thẻ trượt ra chưa định nghĩa
- Spec: trả lời xong thẻ trượt ra 220 ms; undo trả lời 3 s, undo hoãn 5 s.
- Thiếu: bấm undo thì thẻ quay lại vị trí nào; trong 3 s undo, `ws.menu` chưa gửi (`menu_action.dart:57-60`) nên desktop vẫn chờ, thẻ không được tính "đã trả lời trên máy tính". Undo hoãn theo C2: hệ 2 có `undo()` sẵn (`railSnooze.ts:13-22`), hệ 1 phải gửi `minutes: null`.
- Sửa: thêm bảng state thẻ (pending-undo, sent, stale, gone) + test.

### V6. G4: `Space` trên hàng rail đang là click mở pane; tooltip native chồng popover
- Code: hàng rail focus vào `<button data-rail-open>` `RailRowItem.vue:115-126` (`@click` mở pane) nên Space = mở pane. `:title="openTitle"` (122) và `:title="row.title"` (98) hiện tooltip trình duyệt đè popover. Hàng là nguồn kéo (`data-rail-drag`, 75).
- Sửa: chọn phím khác (vd `Shift+F10` không được vì context menu; đề xuất phím `i` hoặc `Alt+↓`) qua `emit('key')` (125); bỏ `title` khi peek bật; đóng peek khi bắt đầu kéo.

### V7. G4: "một listener `pointerenter` trên rail, delegate" không chạy
- `pointerenter` không bubble nên không delegate được. Sửa: `pointerover` / `pointerout` + `relatedTarget` trên container, đọc `data-row-key` (`RailRowItem.vue:80`).

### V8. G4: thiếu rail thu gọn / chế độ icon
- Code: `RailCollapsed.vue` (134 dòng), `railTools.ts:11-16` `footModeFor` có mode `icon`. Spec không nói peek có hiện ở chế độ này và đặt popover phía nào.
- Sửa: ghi rõ (đề xuất: có, popover bên phải rail, cùng delay).

### V9. G4: nguồn tool đang chạy nên dùng cái rail đã dùng
- Code: `LiveRail.vue:89` `agent?.currentTool` khi working; hàng đã hiện tool (`RailRowItem.vue:100`). Thời gian chạy: `ToolSpan.start` (`types.ts:474-484`) trong `core/stores/toolSpans.ts:98` `spansFor`.
- Sửa: tên tool = `currentTool`, thời gian = span mở mới nhất của terminal; không có span thì không hiện thời gian.

### V10. G3: token đặt sai chỗ, kích thước đang hard-code, list ảo dùng JS
- Code: token UI next ở `features/next/tokens.css` (1190 dòng), đã có `--ccm-gap-shell` (`NextShell.vue:413`), tên `--ccm-gap` dễ lẫn. Hàng rail hard-code `min-h-[48px] py-1.5` (`RailRowItem.vue:71`). List ảo có chiều cao JS: `ExplorerSearch.vue:14`, `ExplorerTodo.vue:15`, `LogTable.vue:32`.
- Sửa: liệt kê đúng component đổi class sang token; loại list ảo JS khỏi phạm vi (hoặc đọc token qua `getComputedStyle` 1 lần khi đổi); đặt tên `--ccm-density-*`.

### V11. G5: nguồn dữ liệu tốt hơn "chênh lệch trạng thái" đã có; `leftAt` phải lưu bền
- Code: snapshot core có `activity` (`core-store/src/store/batch.rs:132-140`), mỗi mục có `at` + `kind` gồm `terminal.status` với `data.from/to` (`types.ts:148-202`). Phone nhận lại snapshot khi nối (`dashboard.rs` `snapshot` / `resync_event`). Giới hạn: `activity_limit`, `tool_activity` chiếm chỗ, `fit_snapshot(SNAPSHOT_BUDGET)` có thể cắt.
- `AwaySummary` không có "Lỗi" (`features/history/stores/away.ts:11-27`), spec vừa nói "giữ tên trường" vừa thêm Lỗi.
- Sửa: tính Xong / Lỗi từ `terminal.status` trong `activity` có `at ≥ leftAt`; khi mục cũ nhất của buffer mới hơn `leftAt` thì đánh dấu "không đủ số liệu" và lùi về chênh lệch trạng thái. Lưu `leftAt` vào storage (app bị kill trong nền). Thêm trường `failed` rõ ràng.

### V12. G2: đường prefill không submit có thật nhưng hành vi khác spec
- Code: mẫu đúng là `useSessionActions.ts:160-169` `typeInto` (`prefillSubmit: undefined`, chặn khi `prefill || compact`). `TermPane.vue:750-777`: chờ tới khi agent idle (pane đang working thì chữ hiện sau, có thể lúc chủ nhân đang gõ); nhiều dòng bị gộp 1 dòng nếu không có bracketed paste (771); 1 dòng bắt đầu `/` được `conn.write` thẳng (768).
- Sửa: pane không idle thì báo "Agent đang chạy, sẽ chèn khi rảnh" hoặc chặn (Q5); test assert `prefillSubmit` falsy (không phải `false`); test prompt nhiều dòng và prompt bắt đầu bằng `/`.

### V13. G2b: `mask.rs` không tồn tại; mask thân prompt rồi chèn là chèn sai
- Code: không có `apps/shell/src/**/mask*.rs`; hàm mask là `remote/push_content.rs:217` `mask_text`, `chat_src.rs` `clean`.
- Sửa: dùng `push_content::mask_text`. Prompt bị mask thì phone chèn chữ đã che → gửi sai. Đề xuất: không mask, mà prompt có chuỗi giống secret thì không đồng bộ ra phone (đánh dấu "chỉ desktop") (Q6).

### V14. PLAN: chồng file lượt 2 và chưa rõ quan hệ với InboxPage
- WP-114d (lối vào "Chờ bạn (N)" trên màn machine) và WP-114e (thẻ đi vắng đầu màn machine) cùng sửa `ui/workspace/workspace_body.dart` / `workspace_page.dart` trong lượt 2.
- Phone đã có Inbox (`ui/workspace/inbox_page.dart`) từ inbox desktop (gồm menu / permission / question). Spec không nói hàng đợi thay, nối tiếp hay nằm cạnh Inbox.
- Sửa: cho 114e sau 114d hoặc giao màn machine cho main; ghi rõ hàng đợi = chế độ "từng thẻ" của Inbox (lọc nhóm menu / permission / question) để không có 2 nguồn.

### V15. File > 400 dòng bị sửa thêm mà spec chỉ nhắc LiveRail
- `CommandPalette.vue` 1077 dòng (palette `;`, dùng `parseNextPrefix`, `next/lib/palette.ts:8`), `TermPane.vue` 1944, `workspace.ts` 2044, `next/tokens.css` 1190.
- Sửa: logic mới ở file riêng (`features/prompts/lib/palettePrompts.ts`, provider cho palette), mỗi file lớn chỉ thêm 1-3 dòng nối; thêm `';': 'prompts'` vào `NEXT_PREFIX` + `NextScope`.

### V16. Tiêu chí test không đo được
- G4 "FPS Workspace không tụt": không ngưỡng, không cách đo. G4 E2E "popover có 3 dòng" phụ thuộc C4.
- Sửa: CDP trace rê qua 30 hàng: 0 long task > 50 ms, không re-render Workspace (đếm render của `WorkspaceView`), popover mount ≤ 1. E2E dùng pane shell có output biết trước.

---

## THẤP

### T1. G3 hiện trạng thiếu
- `settings.ts:120-129` Appearance còn theme, sao, file icon, font UI. UI next dùng `features/settings/components/SettingsMistAppearance.vue` (193 dòng), không phải `SettingsAppearance.vue`. Sửa: ghi đúng trang đặt segmented.

### T2. G4 "hàm mask hiện có" mơ hồ
- Có nhiều: `core/lib/mask.ts:27` `maskText`, `packages/shared/src/secretMask.ts:29` `maskSecrets(text, home, platform)`, `notify/trayWaiting.ts:46`. Sửa: chỉ định `secretMask.ts::maskSecrets`.

### T3. G1 cap
- Cap desktop gửi phone nằm ở `ctx.rs:357` `DESKTOP_CAPS` (hot file của main). Ghi rõ tên cap và desktop cũ không có cap thì phone ẩn Hoãn.

### T4. G2 biến
- `{branch}` nên lấy `TerminalSession.gitBranch` qua `usePaneInfo().sessionOf` (như `useRemoteSummary.ts:46`) hoặc `LiveRow.branch` (`railLive.ts:52`). IDEAS có `{file}`, spec bỏ không ghi lý do.

### T5. G2 cách thêm prompt rẻ
- Đã có tìm prompt cũ `features/history/stores/promptSearch.ts`. Có thể thêm "Lưu thành prompt" từ kết quả tìm thay cho "Lưu từ ô nhập" đã bỏ.

### T6. G5 "chạm chip → lọc danh sách pane"
- Màn machine phone hiện chưa có lọc pane; là phần việc mới, nên ghi vào WP-114e và test.

---

## Câu hỏi cần chủ nhân

- Q1 (C2): Hoãn từ phone dùng hệ nào: (a) hoãn của UI next (rail + inbox, theo pane) và đồng bộ sang core để toast cũng im [đề xuất]; (b) chỉ core (monitor classic, không ẩn trên rail next / inbox).
- Q2 (C4): Peek có hiện dòng chữ terminal không: (a) chỉ tool + thời gian + câu hỏi, bỏ 3 dòng [đề xuất]; (b) 3 dòng từ buffer xterm, bỏ khung TUI, chỉ pane đã mount.
- Q3 (C3): Câu hỏi không có menu trên hàng đợi phone: (a) chỉ nút "Mở pane" [đề xuất]; (b) làm ô trả lời chữ (việc thêm, cần đường gõ khi pane đang chờ).
- Q4 (V3): Trả lời menu từ hàng đợi có hỏi lại với menu nguy hiểm (IDEAS E5) không.
- Q5 (V12): Chèn prompt đã lưu vào pane đang chạy: (a) chặn, báo "agent đang chạy" [đề xuất]; (b) chờ tới khi rảnh rồi chèn.
- Q6 (V13): Prompt có chuỗi giống secret: (a) không đồng bộ ra phone [đề xuất]; (b) đồng bộ bản đã che.
- Q7 (C5): Bỏ transition khi đổi mật độ (đổi tức thì) được không.
- Q8: Hoãn trên phone có tắt luôn push nhắc của pane đó trong thời gian hoãn không.
