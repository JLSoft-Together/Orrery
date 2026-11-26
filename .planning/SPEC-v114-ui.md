# SPEC v114: UI/UX đợt G

Plan: `.planning/PLAN-v114-ui.md`. Review spec: `.planning/REVIEW-v114-spec.md` (27 finding, đã sửa hết vào bản này). Quyết định chủ nhân 27/11 ở mục "Quyết định".

## Quyết định (27/11)
| ID | Câu hỏi | Chốt |
|---|---|---|
| D1 | G1 hoãn từ phone dùng hệ nào | Hệ UI next (rail + inbox, theo `paneKey`) và báo core (`snoozeTerminal`) để toast cũng im; hệ UI next được lưu bền hơn (`sessionStorage` ít nhất) và tự bỏ hoãn khi câu hỏi mới (`since` đổi) |
| D2 | G4 peek hiện gì | Tool đang chạy + thời gian + câu hỏi đang chờ; KHÔNG có dòng chữ terminal |
| D3 | G1 câu hỏi không có menu | Làm ô trả lời chữ (cần đường gõ khi pane đang chờ + gửi nội dung câu hỏi xuống phone) |
| D4 | G2 lưu ở đâu | File `prompts.json` trong data dir, core-desk đọc / ghi, desktop + phone dùng chung |
| D5 | G2 phone | Sửa được cả trên phone (thêm / sửa / xoá, tính quota ghi) |
| D6 | G2 chèn khi agent đang chạy | Cứ chèn ngay vào ô nhập (`prefillNow`), KHÔNG submit |
| D7 | G5 ngưỡng đi vắng phone | 30 phút |
| D8 | G1 chạm thông báo khi ≥ 2 pane chờ | Mở pane của thông báo, sau khi nối hiện nút "Còn N đang chờ" vào hàng đợi; hoãn 1 pane thì desktop không gửi push nhắc pane đó trong lúc hoãn |
| D9 (main) | G3 transition | Đổi mật độ tức thì, không transition layout (tránh resize xterm / PTY liên tục) |
| D10 (main) | G1 menu nguy hiểm | Không hỏi lại thêm; dùng undo 3 s có sẵn của `runMenuAction`. Ô trả lời chữ đi qua danger guard F8 |
| D11 (main) | G2 prompt giống secret | Không đồng bộ ra phone (đánh dấu "chỉ desktop"), không gửi bản đã che |

---

## G4. Xem nhanh (peek) session trong rail (desktop)

**Hiện trạng.** `features/next/components/rail/LiveRail.vue` (931 dòng) + `RailRowItem.vue` (hàng có `<button data-rail-open>` 115-126, `:title` 98 và 122, nguồn kéo `data-rail-drag` 75, `data-row-key` 80). Tool hiện tại: `LiveRail.vue:89` `agent?.currentTool`. Span: `core/stores/toolSpans.ts:98 spansFor`, `ToolSpan.start` (`packages/shared/src/types.ts` ~474-484). Rail thu gọn: `RailCollapsed.vue`, mode `icon` (`railTools.ts:11-16`).

**Hành vi.**
- Rê lên hàng ≥ 400 ms → popover bên phải rail (cả rail đầy đủ lẫn thu gọn / icon), cùng 1 instance `RailPeek.vue`.
- Nội dung: tên pane, trạng thái (icon + chữ + màu theo token), tool đang chạy (`currentTool`) + thời gian từ span mở mới nhất của terminal (không có span thì không hiện thời gian), câu hỏi đang chờ (menu question 1 dòng đã mask qua `packages/shared/src/secretMask.ts::maskSecrets`) khi `waiting`, thời gian ở trạng thái hiện tại. Pane không phải agent: tên + loại + cwd rút gọn. Không dữ liệu: "Chưa có hoạt động".
- Nút "Mở pane" trong popover. Không có nút trả lời.
- Đóng: rời hàng + popover, Esc, cuộn rail, đổi tab, bắt đầu kéo hàng (160 ms fade).
- Khi peek mở: bỏ `title` native của hàng đó (tránh tooltip đè).
- Bàn phím: focus hàng + phím `i` mở / đóng peek (Space giữ nguyên = mở pane); `aria-describedby` trỏ popover.
- Lắng nghe: `pointerover` / `pointerout` + `relatedTarget` trên container rail (delegate, đọc `data-row-key`); 1 timer duy nhất; không subscribe store khi chưa mở.
- Reduced motion: chỉ fade.

**File.** Mới: `features/next/components/rail/RailPeek.vue`, `features/next/lib/railPeek.ts` (thuần: chọn nội dung, delay, quy tắc đóng) + test, composable `useRailPeek.ts`. `LiveRail.vue` / `RailRowItem.vue` / `RailCollapsed.vue` chỉ thêm 1-3 dòng nối.

**Test.** Vitest: delay 400 ms, đóng theo từng nguồn, không mount khi không rê, `title` bị bỏ khi mở, phím `i`, nội dung theo loại pane, mask. E2E `rail-peek.cjs`: rê hàng pane shell → popover → Esc đóng. Hiệu năng: trace CDP rê qua 30 hàng: 0 long task > 50 ms, `WorkspaceView` không render lại, popover mount ≤ 1 (ghi số đo vào báo cáo).

---

## G3. Mật độ giao diện Gọn / Vừa / Thoáng (desktop)

**Hiện trạng.** Settings UI next: `features/settings/components/SettingsMistAppearance.vue` (193 dòng); store `core/stores/settings.ts` (theme, sao, file icon, font UI, `codeFontSize` 129). Token UI next ở `features/next/tokens.css` (1190 dòng, đã có `--ccm-gap-shell`). Hàng rail hard-code `min-h-[48px] py-1.5` (`RailRowItem.vue:71`). List ảo tính chiều cao bằng JS: `ExplorerSearch.vue:14`, `ExplorerTodo.vue:15`, `LogTable.vue:32`.

**Hành vi.**
- `SettingsMistAppearance.vue`: segmented "Mật độ" Gọn / Vừa / Thoáng, mặc định Vừa = đúng pixel hiện tại.
- `:root[data-density="compact|regular|comfy"]`; token mới trong file riêng `features/next/density.css` (import từ tokens), tên `--ccm-density-row-h`, `--ccm-density-row-py`, `--ccm-density-gap`, `--ccm-density-head-h`. Vừa đọc đúng giá trị đang hard-code.
- Phạm vi: hàng rail (đổi class hard-code sang token), danh sách session / inbox, Settings rows, palette rows, pane header. Không đổi: terminal, editor, list ảo dùng chiều cao JS (để sau).
- Đổi tức thì (D9), không transition chiều cao / padding. Mỗi pane terminal resize PTY ≤ 1 lần khi đổi.
- Lưu `ccm.density`; search Settings có "Mật độ"; palette có 3 lệnh. Không rẽ nhánh theo theme id.

**Test.** Vitest: store mặc định / đọc / ghi, attribute; E2E: Gọn → chiều cao hàng rail nhỏ hơn Vừa; đổi mật độ → mỗi pane terminal nhận ≤ 1 resize (đếm qua log resize hoặc spy). E2E cũ chạy ở Vừa không đổi.

---

## G1. Hàng đợi "Chờ bạn" trên điện thoại

**Hiện trạng (đã kiểm).**
- Hai hệ hoãn: core `features/monitor/stores/snooze.ts` (key `terminalId`, lưu `snooze.json`, có `since`), và UI next `features/next/lib/railSnooze.ts` (key `paneKey`, chỉ RAM, mức `[5,15,60]` `railLive.ts:8`). Inbox desktop `features/click-menu/stores/inbox.ts:8,88,110-114` dùng hệ UI next; inbox này là nguồn `inbox` của summary gửi phone (`features/remote/composables/useRemoteSummary.ts:25-31`).
- Phone đã gửi được `terminal.snooze` qua tap tới core (`apps/shell/src/remote/dashboard_msg.rs:8`, `dashboard.rs:216, 293-309`, core `dispatch.rs:130-139`) nhưng chưa gọi.
- Phone đã có Inbox `ui/workspace/inbox_page.dart` (`InboxInfo.since` trong `inbox_model.dart:33-55`), trả lời menu `app/menu_action.dart::runMenuAction` (undo 3 s, loại mục `other` / `submit` / checkbox).
- `ws.send` vào pane `waiting` không gõ được: `useRemotePhone.ts:168-177` đặt `prefill + prefillSubmit + prefillNow`, `TermPane.vue:750-753 prefillReady()` false khi `waiting`.
- Summary phone không có chữ câu hỏi mở: `remoteSummary.ts:46,56` `waitText` là lý do chờ.
- `nextAttention` (`workspace.ts:1353-1360`) sắp theo tab, không theo thời gian chờ.

**Hành vi (phone).**
- Hàng đợi = chế độ "từng thẻ" của Inbox (cùng nguồn dữ liệu, không tạo nguồn thứ 2): nút "Trả lời từng cái" trên Inbox + nút "Chờ bạn (N)" trên màn machine.
- Thứ tự: chờ lâu nhất trước theo `InboxInfo.since` (thiếu thì theo thứ tự summary). Đếm "1 / N".
- Thẻ: tên pane, project, chờ bao lâu, nội dung câu hỏi (đã mask).
  - Có menu: nút lớn ≥ 48dp cho từng lựa chọn hợp lệ (`menuItemFor`), bấm → `runMenuAction` (undo 3 s). Menu chỉ có checkbox / Other → không có nút chọn, chỉ "Mở pane".
  - Không menu (câu hỏi mở / permission dạng chữ): ô trả lời chữ (D3) + nút Gửi; qua danger guard F8; gửi bằng đường mới `ws.reply` (xem dưới).
- Nút dưới: "Hoãn" (menu 15 phút / 1 giờ), "Bỏ qua" (chỉ sang thẻ kế trong lượt, không đổi gì desktop), "Mở pane".
- Vuốt trái = Bỏ qua, vuốt phải = Hoãn 15 phút; luôn có nút tương ứng; hint lần đầu.
- State thẻ: `ready` → (`pendingUndo` 3 s | `snoozed` có undo 5 s) → `sent` (trượt ra 220 ms, thẻ kế vào) ; `stale` (menu đổi → chữ "Câu hỏi đã đổi" + nút Mở pane) ; `gone` (pane hết chờ từ desktop → "Đã trả lời trên máy tính", tự bỏ sau 1.5 s). Undo → thẻ quay lại vị trí cũ. Trong `pendingUndo` desktop chưa nhận gì nên không coi là `gone`.
- Hết thẻ: "Xong hết, không còn gì chờ bạn" + nút về.
- Thông báo (D8): chạm thông báo mở pane như cũ; sau khi nối, nếu còn ≥ 2 pane chờ thì hiện nút nổi "Còn N đang chờ" mở hàng đợi. Deep link `orrery-remote://m/<m>/queue` (thêm vào `app/pane_link.dart` + test).

**Desktop (web + shell).**
- Op mới phone → desktop `ws.snooze {paneKey, minutes: 15|60|null}` đi qua webview như các `ws.*` (`useRemotePhone.ts:214-225` thêm handler): gọi hoãn hệ UI next theo `paneKey` (`useInboxStore().snooze` / `railSnooze`), và nếu pane có `terminalId` thì `useSnoozeStore().snooze` (core) để toast im (D1). `null` = bỏ hoãn (undo).
- Hệ UI next: lưu `sessionStorage` (`ccm.railSnooze`), thêm `since` để tự bỏ hoãn khi câu hỏi mới; thêm mức 60 phút nếu chưa có trong lựa chọn inbox.
- Push (D8): trong lúc pane đang hoãn (hệ UI next), desktop không gửi push nhắc pane đó (push watch kiểm hoãn; WP xác minh chỗ chọn push trong `apps/shell/src/remote/push_watch.rs` / nguồn summary và cách shell biết trạng thái hoãn: ưu tiên field `snoozed` trong summary web publish, không thêm kênh mới).
- Ô trả lời chữ (D3):
  - Summary gửi phone thêm `question` (chữ câu hỏi mở đã mask, ≤ 300 ký tự) khi pane `waiting` không có menu; nguồn: dữ liệu web đang dùng cho toast / card câu hỏi (WP xác minh; không có nguồn chữ tin cậy thì thẻ hiện lý do chờ như hiện tại và vẫn có ô trả lời).
  - Op `ws.reply {paneKey, text, authed?}`: như `ws.send` (danger check ở `phone_ws.rs`, `authed`), nhưng web gõ được vào pane `waiting`: prefill với cờ mới `prefillWaiting` cho phép `prefillReady()` khi `waiting` + submit. Chỉ cho khi pane đang `waiting` và không có menu (có menu thì phải dùng `ws.menu`), sai thì lỗi `notWaiting` / `useMenu`.
  - Đây vẫn là ngoại lệ 7 (prompt gửi từ phone được submit ngay); không thêm ngoại lệ.
- Guard: op ghi kiểm `self.ctx.host.control()` như `fs_put.rs:227` (bản thật hiện "tắt điều khiển" = tắt remote; test bằng `view_only` của test host). Rate theo bảng hiện có.
- Cap mới `queue` (thêm vào `DESKTOP_CAPS` `ctx.rs`, hot file main); thiếu cap → phone ẩn Hoãn và ô trả lời chữ, giữ nút Mở pane.

**Test.** Vitest: handler `ws.snooze` (cả 2 hệ, null = undo), `sessionStorage`, tự bỏ hoãn khi `since` đổi, `ws.reply` gõ vào pane waiting + submit, từ chối khi có menu / không waiting, summary có `question` đã mask. cargo: `ws.reply` qua danger check, view-only bị từ chối, push không gửi khi pane đang hoãn. Dart: thứ tự theo `since`, mọi state thẻ + undo, stale, gone, ẩn theo cap, vuốt, deep link queue, nút "Còn N". E2E desktop: phone giả `ws.snooze` → hàng rail ẩn + core snooze; `ws.reply` vào pane shell giả đang waiting.

---

## G5. "Từ lúc bạn đi vắng" trên điện thoại

**Hiện trạng.** Desktop `features/history/stores/away.ts` (`AwaySummary{done, waiting, started, ended, compactions, failedAgents, completedAgents, jobsDone, loops, limitDelta}`, không có "lỗi"). Snapshot core có `activity` (`core-store/src/store/batch.rs:132-140`), mục `terminal.status` có `data.from/to` (`types.ts:148-202`); phone nhận lại snapshot khi nối (giới hạn `activity_limit`, `fit_snapshot`).

**Hành vi.**
- `leftAt` lưu bền (storage) khi app vào nền (`paused`); mở lại sau ≥ 30 phút (D7) → thẻ đầu màn machine "Từ 14:05 (1 giờ 20 phút trước)": Xong N, Chờ bạn N, Lỗi N, Agent con xong / lỗi, Compact N.
- Tính từ `activity` có `at ≥ leftAt` (`terminal.status` working → idle = Xong, → error = Lỗi; agent / compaction theo `summarizeActivity`). Nếu mục cũ nhất trong buffer mới hơn `leftAt` (buffer bị cắt) → dùng chênh lệch trạng thái lúc rời / lúc về và ghi "Số liệu có thể thiếu". "Chờ bạn" luôn lấy từ state hiện tại.
- Logic thuần `apps/mobile/lib/remote/away_summary.dart` (port `summarizeActivity`, giữ tên trường + thêm `failed`), test bảng dữ liệu đối chiếu test web.
- Chạm chip → lọc danh sách pane theo nhóm (việc mới: thêm lọc vào màn machine); "Đã xem" hoặc cuộn qua → ẩn tới lần đi vắng sau. Nhiều máy: mỗi máy 1 thẻ.

**Test.** Dart: dưới 30 phút không hiện; đủ → số đúng từ chuỗi activity mẫu; buffer bị cắt → nhánh chênh lệch + nhãn; `leftAt` qua kill app; chip lọc; ẩn sau khi xem.

---

## G2. Prompt đã lưu (chèn, không gửi)

**Hiện trạng.** Không có thư viện prompt. Đường chèn không submit: `features/monitor/composables/useSessionActions.ts:160-169 typeInto` (`prefillSubmit` undefined, chặn khi `prefill || compact`); `TermPane.vue:750-777` (chờ idle trừ `prefillNow`; nhiều dòng gộp 1 dòng nếu không có bracketed paste ~771; dòng bắt đầu `/` ghi thẳng ~768). Tìm prompt cũ: `features/history/stores/promptSearch.ts`. Palette prefix: `next/lib/palette.ts:8` `NEXT_PREFIX`. Mask: `packages/shared/src/secretMask.ts::maskSecrets`, Rust `remote/push_content.rs:217 mask_text`.

**Lưu (D4).** core-desk file `<data dir>/prompts.json` `{version:1, items:[{id, title, body, scope:'all'|'<project path>', updatedAt}]}`; max 200 mục, title ≤ 80, body ≤ 4 000 ký tự; ghi nguyên tử (tmp + rename); file hỏng → đổi tên `.bak`, bắt đầu rỗng. Kênh `ccm:prompts-get`, `ccm:prompts-set {op:'upsert'|'delete'|'reorder', ...}`; phát event `prompts.changed` cho web + shell (phone).

**Desktop.**
- Feature mới `features/prompts/` (store, lib thuần `promptVars.ts`, `palettePrompts.ts`).
- Biến: `{branch}` (`TerminalSession.gitBranch` qua `usePaneInfo().sessionOf`), `{project}`, `{cwd}`. Không có giá trị → giữ nguyên chữ và báo nhẹ. (`{file}` của ý tưởng ban đầu bỏ: pane agent không có "file đang mở" tin cậy.)
- Chèn (D6): vào pane agent đang focus bằng `prefill` + `prefillNow: true`, `prefillSubmit` falsy; chèn được cả khi agent đang chạy; chặn khi pane đã có `prefill` / `compact` ("Ô nhập đang bận"). Prompt nhiều dòng: dùng bracketed paste (giữ xuống dòng); prompt bắt đầu bằng `/` vẫn chỉ chèn, không gửi (test).
- Lối vào: palette prefix `;` (thêm vào `NEXT_PREFIX` + `NextScope`, provider ở `palettePrompts.ts`, `CommandPalette.vue` chỉ 1-3 dòng nối), mục "Prompt đã lưu" trong menu header pane agent (`paneHeadActions`), Settings › Prompt đã lưu (thêm / sửa / xoá / sắp xếp / nhập / xuất JSON), "Lưu thành prompt" từ kết quả tìm prompt cũ (`promptSearch`).

**Phone (D5, D11).**
- Viewer op `prompts.list {paneKey?}` → danh sách (bỏ mục giống secret: `mask_text` đổi nội dung → không gửi, kèm số mục bị ẩn), biến đã điền theo pane khi có `paneKey`.
- Op ghi `prompts.save {id?, title, body, scope}`, `prompts.delete {id}`, tính quota ghi, kiểm `control()`; desktop kiểm giới hạn như trên; phone không sửa được mục "chỉ desktop" (không thấy).
- Composer: dải chip "Prompt" (cuộn ngang trong dải, trang không cuộn ngang); chạm → chèn tại con trỏ, KHÔNG gửi; giữ chip → sửa / xoá; nút "+" thêm.
- Cap `prompts`; thiếu cap → ẩn dải.

**Test.** core-desk: đọc / ghi / giới hạn / file hỏng / ghi nguyên tử. Vitest: store, biến, chèn (`prefillSubmit` falsy, `prefillNow` true), nhiều dòng, dòng `/`, palette `;`, Settings CRUD. cargo shell: op list ẩn mục secret, save / delete quota + control, event changed. Dart: chip chèn tại con trỏ không gửi, CRUD, ẩn theo cap.

---

## Bảng test + QA
| Mục | Unit | E2E desktop | Máy thật |
|---|---|---|---|
| G4 | vitest railPeek | rail-peek.cjs + trace | - |
| G3 | vitest density | density.cjs (resize ≤ 1) | Mac 3 mức |
| G1 | vitest ws.snooze / ws.reply, cargo reply + push, Dart queue | phone giả snooze + reply | A33: trả lời menu, trả lời chữ, hoãn, nút Còn N |
| G5 | Dart away_summary | - | A33: tắt app 30 phút rồi mở |
| G2 | core-desk prompts, vitest, cargo, Dart | palette `;` chèn không gửi | A33 chip + sửa |
