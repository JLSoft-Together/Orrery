# SPEC v104 - hành vi từng đề xuất

Đi cùng `.planning/PLAN-v104-ux-proposals.md`. Chuỗi i18n mới của desktop gom trong `apps/web/src/i18n/v104-ux.ts` (prefix `v104.*`), nhúng vào `en.ts` / `vi.ts` như `deepseek-v81.ts`. Text dưới đây là bản đề xuất, subagent giữ nguyên nghĩa, được sửa câu chữ nếu chật chỗ. Không em dash, không chấm giữa trong chuỗi.

---

## 0. Terminal điện thoại mất dòng khi resize (WP-104a1, WP-104a2)

### 0.1 Bất biến
- Mọi byte output và mọi event `pty.*` của 1 stream được xử lý theo **đúng thứ tự desktop gửi**. Không nhánh nào (decoder async, rAF, post-frame) được đảo thứ tự output với `pty.resized` / `pty.ready` / `pty.snapshot`.
- Khi nhận `pty.resized`: (1) ghi hết byte đã nhận trước event vào terminal ở cỡ cũ; (2) đẩy toàn bộ màn hình hiện tại vào scrollback; (3) resize terminal về cỡ mới **ngay**, trước byte kế tiếp; (4) byte sau event ghi ở cỡ mới.
- Không mất dòng: sau 1 burst N dòng có resize giữa chừng, buffer + scrollback chứa đủ dòng cuối min(N, scrollback) theo thứ tự, không lặp, không hụt.

### 0.2 Giao thức (desktop ↔ điện thoại, qua Noise như cũ)
- Attach: client gửi thêm `caps: string[]`; client v104 gửi `['pty.snapshot']`. Field mới tuỳ chọn, desktop cũ bỏ qua.
- Event mới `{ t: 'pty.snapshot', stream, cols, rows, gen, data }`: `data` = byte ANSI dựng lại scrollback (max 2000 dòng) + màn hình hiện tại + vị trí con trỏ + chế độ (alt screen, cursor visible), sinh từ terminal ảo phía desktop.
- Desktop gửi `pty.snapshot`: (a) ngay sau mỗi `pty.resized` (thay cho phần ConPTY vẽ lại); (b) khi client nối lại / attach mới, thay `tail` byte thô. Chỉ gửi khi client có cap; client không có cap giữ hành vi cũ (tail thô, `pty.resized` như nay).
- Client nhận `pty.snapshot`: `reset` terminal, resize đúng `cols` / `rows`, ghi `data`, rồi tiếp tục byte sau. Byte ConPTY vẽ lại sau resize vẫn tới (desktop không lọc được) nhưng ghi đè lên màn hình đã đúng cỡ nên không mất dòng.
- Terminal ảo: Rust trong `apps/shell/src/remote/pty_bridge.rs` (`Pump`), crate đề xuất `vt100` (Q2 trong PLAN), 1 instance / stream đang có client attach, scrollback 2000 dòng, giải phóng khi stream không còn client.

### 0.3 Phía client
- Android `apps/mobile/lib/ui/terminal/terminal_pane.dart`: 1 listener nhận cả output và event theo thứ tự (`handle.output` không đi qua `Utf8Decoder().bind` nữa; decode bằng `Utf8Decoder(allowMalformed: true).startChunkedConversion` đồng bộ, giữ phần UTF-8 dở giữa 2 chunk). `_flushOut` gọi trước mọi event. Resize do `pty.resized` gọi thẳng `Terminal.resize` trong `terminal_pane.dart`, không chờ `terminal_surface.dart::_syncDesk` post-frame. Khi đang gõ (`_typing`, cỡ của điện thoại) thì giữ quy tắc DECISIONS 2026-11-12 "bên nào đang gõ thì cỡ bên đó".
- PWA `apps/pwa/src/components/TerminalSurface.vue`: `onEvent('pty.resized')` gọi `flush()` đồng bộ (ghi `queue` hiện có) trước `relayout()`; trước `t.resize` ghi đủ xuống dòng để màn hình hiện tại lên scrollback. `pty.snapshot`: `t.reset()`, `queue = []`, resize, `t.write(data)`.
- Edge: resize liên tiếp 3 lần trong 100 ms (xoay máy) → mỗi lần đủ 4 bước, snapshot cuối thắng; event tới khi terminal chưa mount → giữ trong hàng đợi chung, áp khi mount; `pty.exit` giữa burst → flush trước rồi mới hiện "đã thoát".

### 0.4 Tiêu chí chấp nhận
- [ ] Unit Android `apps/mobile/test/terminal_resize_order_test.dart`: burst 3000 dòng, resize 41 → 19 hàng ở dòng 1963, không mất line-1963..1984.
- [ ] Unit PWA `apps/pwa/src/terminal/terminal.test.ts`: cùng kịch bản trên xterm headless.
- [ ] `cargo test` cho `pty_bridge`: snapshot sau resize đúng cỡ, có 2000 dòng cuối; client không cap nhận tail thô như cũ.
- [ ] E2E PWA (WebKit + Chromium): resize giữa burst và nối lại sau resize, không mất dòng.
- [ ] Kiểm thật trên A33: `seq 1 3000` trong pane, đổi bên gõ / xoay máy giữa burst, cuộn lại đủ dòng; ảnh `docs/qa/v104/WP-104a/`.

---

## 1. Inbox gộp (WP-104e)

### 1.1 Nguồn dữ liệu (chỉ đọc state có sẵn)
| Nhóm (thứ tự hiện) | `InboxReason` | Nguồn |
|---|---|---|
| 1. Chờ trả lời menu | `menu` | `stores/menuQueue.ts::useMenuQueueStore().entries` (paneKey → terminalId qua `workspace.panes`) |
| 2. Xin quyền | `permission` | terminal `status === 'waiting'` và `waitReason(waitingFor) === 'permission'` (`packages/shared/src/status.ts`) |
| 3. Câu hỏi | `question` | `waiting` với `waitReason` `question` hoặc `other` |
| 4. Đụng file | `clash` | `useAgentTouchesStore().clashes` (chỉ khi switch `ccm.notify.clash` bật) |
| 5. Lỗi | `loop`, `checks`, `stuck` | như `lib/inbox.ts` hiện tại (`attention.loops`, `checks.runs`, `quietFor`) |
| 6. Ghi chú agent | `note` | `notify_owner` (v80), như hiện tại |
| 7. Xong chưa xem | `done` | pane `attention === 'done'` |
`INBOX_GROUPS` (thứ tự trên) đặt ở `packages/shared/src/inbox.ts` để desktop, PWA dùng chung; Android chép hằng cùng thứ tự trong `workspace_model.dart` và có test so với file JSON sinh từ shared (hoặc test chuỗi cứng, cùng thứ tự).

- Một agent có thể ở nhiều nhóm? Không: mỗi agent 1 dòng với lý do khẩn nhất (giữ cách `inboxItems` đang làm), trừ `clash` hiện thêm 1 dòng riêng theo file (vì liên quan 2 agent). Trong nhóm: cũ nhất lên đầu (`since`).
- Đếm trên chip = số dòng sau khi trừ mục đã hoãn (`railSnooze`) và đã đánh dấu (`ccm.inbox`).

### 1.2 UI desktop
- Chip `InboxChip.vue` trên thanh tab Workspace (chỗ `[data-attention-chip]` + `MenuQueueChip` cũ), chỉ hiện khi N > 0: icon `Inbox` + "Inbox N". Màu: có mục nhóm 1-3 → `st-waiting`; có Lỗi → `st-error`; chỉ Xong / Ghi chú → `ink-muted`. `data-inbox-chip`.
- Nút Inbox rail foot (`NextShell.vue` `data-inbox-button`) giữ, cùng số, cùng panel. Ctrl+K `c:inbox` giữ.
- Panel `InboxPanel.vue`: tiêu đề nhóm (`data-inbox-group="<key>"`) + dòng (`data-inbox-row`, `data-reason`). Dòng: StatusIcon + tên session + lý do + tuổi + project. Nhóm rỗng không hiện. Rỗng hết: `inbox.empty`.
- Rail next / board: không hiện số đếm riêng; giữ chấm / chip trạng thái từng dòng. Chuông `otherNeeds` (classic) giữ tới 4.1.0.

### 1.3 Phím
| Phím | Ở đâu | Việc |
|---|---|---|
| Ctrl+Shift+A (⇧⌘A) | toàn app | mở / đóng Inbox |
| Ctrl+Alt+Q (⌃⌥Q) | toàn app | nhảy tới mục kế trong Inbox (mọi nhóm, theo thứ tự 1.1), mở pane, không mở panel |
| J / K, ↑ / ↓ | trong panel | đi dòng |
| Enter | trong panel | mở pane (hoặc Monitor nếu không có pane), đóng panel |
| E | trong panel | đánh dấu đã xem (hiện lại khi trạng thái đổi, như cũ) |
| S | trong panel | hoãn 15 phút (`railSnooze.snooze`), toast Undo |
| Esc | trong panel | đóng |
Inbox **không** có nút chọn đáp án menu: Enter chỉ nhảy tới pane, chủ nhân bấm trong pane (ngoại lệ 6). Toast OS trả lời menu v100 (`useMenuToast`) vẫn chạy như cũ, gọi từ `InboxChip.vue` hoặc `App.vue`.

### 1.4 Remote (desktop → PWA + Android)
- `RemotePane.inbox?: { reason: InboxReason; since?: string; file?: string; peer?: string }` (`packages/shared/src/remote.ts`). `file` = path tương đối repo (chỉ `clash`), `peer` = tên session kia. Điền ở `apps/web/src/lib/remoteSummary.ts`; `summaryHash` đổi khi `inbox` đổi.
- PWA: màn Inbox (sheet từ Monitor, badge số trên tab Monitor), nhóm theo `INBOX_GROUPS`, chạm mở Session. Android: `InboxPage` đọc `inbox`, cùng nhóm, cùng thứ tự; không có `inbox` (desktop cũ) thì dùng `attention` như hiện nay.
- Điện thoại không trả lời menu từ Inbox; mở session rồi dùng luồng trả lời hiện có.

### 1.5 i18n (desktop, mới hoặc đổi)
| Key | vi | en |
|---|---|---|
| `v104.inbox.chip` | Inbox {n} | Inbox {n} |
| `v104.inbox.chipTip` | Agent đang cần bạn ({keys} mở, {next} mục kế) | Agents that need you ({keys} to open, {next} for the next one) |
| `v104.inbox.group.menu` | Chờ trả lời menu | Waiting on a menu |
| `v104.inbox.group.permission` | Xin quyền | Asking permission |
| `v104.inbox.group.question` | Câu hỏi | Questions |
| `v104.inbox.group.clash` | Đụng file | Same file |
| `v104.inbox.group.error` | Lỗi | Errors |
| `v104.inbox.group.note` | Ghi chú của agent | Agent notes |
| `v104.inbox.group.done` | Xong, chưa xem | Done, not seen |
| `v104.inbox.snooze` | Hoãn 15 phút | Snooze 15 minutes |
| `v104.inbox.snoozed` | Đã hoãn {name} 15 phút | Snoozed {name} for 15 minutes |
| `v104.inbox.viewChanges` | Xem thay đổi | View changes |
| `inbox.keys` (đổi) | J/K, Enter mở, E đã xem, S hoãn | J/K, Enter opens, E seen, S snooze |
PWA (`apps/pwa/src/i18n/*`) và Android (`app_en.arb`, `app_vi.arb`) dùng cùng nghĩa, key theo quy ước từng app.

### 1.6 Edge case
- Pane có menu nhưng terminal không còn trong `terminals.live`: vẫn hiện (nhóm menu dựa trên pane).
- Agent vừa ở `menu` vừa `permission` (Claude hiện menu xin quyền): 1 dòng nhóm `menu`.
- Mục hoãn hết hạn: hiện lại, không toast.
- Đóng pane: mục biến mất trong ≤ 1 lần render.
- Desktop cũ ↔ điện thoại mới và ngược lại: field tuỳ chọn, không vỡ.

### 1.7 Tiêu chí chấp nhận
- [ ] Thanh tab chỉ còn 1 chip Inbox; `[data-attention-chip]`, `[data-menu-queue]` không còn ở next.
- [ ] Chip và nút rail foot cùng số với panel.
- [ ] Thứ tự nhóm giống nhau trên desktop, PWA, Android (test so `INBOX_GROUPS`).
- [ ] Ctrl+Alt+Q đi qua mục menu, quyền, lỗi theo thứ tự; Ctrl+Shift+A mở panel.
- [ ] Toast OS trả lời menu v100 vẫn hiện khi Orrery không focus (E2E `v100-menu-queue.cjs`).
- [ ] Không có đường nào gửi phím vào pane từ Inbox.
- [ ] `npm test -w @ccm/web -- src/lib/inbox.test.ts src/components/inbox src/lib/shortcuts.test.ts src/lib/remoteSummary.test.ts` pass; E2E PWA Inbox; `flutter test` model Inbox.

---

## 2. Settings (WP-104c)

### 2.1 Hành vi
- Mọi entry search hiển thị phải có đúng 1 `[data-setting-row="<id>"]` trên trang đích (cả next và classic). Bấm kết quả: chuyển trang, cuộn, nháy `.ccm-setting-flash`. Không tìm thấy (lỗi lập trình): toast `v104.settings.notFound` thay vì im lặng.
- `fileIcons`, `dust` chỉ ở trang **Workspace** (`SettingsWorkspace.vue:56-58`); bỏ ở `SettingsAppearance.vue:142-145`. Trang `workspace` mở ở cả classic (bỏ khỏi `NEXT_ONLY_PAGES`).
- Trang `reminders` gộp vào `notify`: section "Nhắc nhở" (break, stuck, contextWarn, budget), gập được kiểu `[data-notify-more]`. `bugReportNudge` về `about`.
- `ccm.settings.page = 'reminders'` (cũ) → mở `notify`, cuộn tới section "Nhắc nhở". Link / lệnh mở `reminders` từ chỗ khác (`rg -n "'reminders'" apps/web/src`) đổi sang `notify`.
- Số trang: 15 → 14 (12 sau 4.1.0 khi `workspace` / `disk` thành trang chung, không còn tách next).

### 2.2 i18n
| Key | vi | en |
|---|---|---|
| `v104.settings.notFound` | Không tìm thấy dòng cài đặt này trên trang | This setting is not on the page |
| `v104.settings.reminders` | Nhắc nhở | Reminders |

### 2.3 File đổi / xoá và cách kiểm
| File | Đổi |
|---|---|
| `apps/web/src/components/settings/index.ts` | `SETTINGS_PAGES`, `NEXT_ONLY_PAGES`, `NEXT_PAGE_OF`, `entries`, `loadPage` |
| `apps/web/src/components/SettingsMenu.vue` | bỏ nhánh `SettingsReminders` (dòng 391), icon `reminders` (125), `flash()` báo khi không thấy |
| `components/settings/SettingsNotify.vue` | nhúng `SettingsReminders.vue` dưới section mới |
| `components/settings/SettingsAppearance.vue` | bỏ 2 dòng `fileIcons` / `dust` |
| `components/settings/rows.test.ts` (mới) | mount từng trang, so entry ↔ `[data-setting-row]` |
Kiểm không vỡ: `npm test -w @ccm/web -- src/components/settings src/lib/settingsFlash.test.ts`, typecheck, E2E bước search trong `v103-polish.cjs`.

### 2.4 Tiêu chí chấp nhận
- [ ] `rows.test.ts` pass cho next + classic.
- [ ] Gõ "file icons", "bụi", "nhắc" đều mở đúng dòng.
- [ ] Máy có `ccm.settings.page=reminders` mở Settings vào trang Thông báo.

---

## 3. Gỡ classic (WP-104b, WP-104i, WP-104j)

### 3.1 Lộ trình
| Mốc | Việc |
|---|---|
| Ngay (v104) | WP-104b: E2E mặc định next; sửa `v51-compact.cjs`, `v25.cjs`; file chỉ classic chạy `--ui=classic` |
| 4.0.8 | WP-104i: classic đóng băng (chỉ fix vỡ), changelog + hint ở Appearance "Giao diện cũ sẽ bỏ ở 4.1.0" |
| 4.1.0 | WP-104j: xoá classic, migration 1 lần classic → next |

### 3.2 Danh sách xoá / đổi ở 4.1.0 (WP-104j)
- Xoá: `apps/web/src/components/AppHeader.vue`, `components/workspace/AttentionPopover.vue` (nếu WP-104e chưa xoá), nhánh `isNext` / `uiGen.isNext` (`WorkspaceView.vue` 45 chỗ, `CommandPalette.vue` 26, `NextShell.vue`, `TermPane.vue`, `DeviceUiTestDialog.vue`), `<component :is="isNext ? PaneHeader : 'div'">` trong `ChangesPane.vue:718`, `BrowserToolbar.vue:164`, `FilePane.vue:1372` (giữ `PaneHeader`), các block `:root:not([data-ui='next'])` trong `style.css` / `themes.css`.
- Gộp: `SettingsAppearance.vue` + `SettingsMistAppearance.vue`; `stores/uiGen` chỉ còn palette / motion; `NEXT_ONLY_PAGES`, `nextEntries` / `entries` gộp 1.
- Migration: khoá mới `ccm.uiMigration = '4.1.0-next'`; `ccm.uiGeneration` bị bỏ qua; `index.html` boot không đọc classic.
- E2E: bỏ `--ui=classic`, `CLASSIC_ONLY`, `ui-gen.cjs` chỉ còn next.

### 3.3 Cách kiểm không vỡ
- `.planning/AUDIT-v104-classic-parity.md` không còn dòng đỏ (mỗi tính năng classic có tương đương next hoặc ghi "bỏ có chủ ý", chủ nhân duyệt).
- `npm run typecheck`, `npm test`, `node apps/desktop/e2e/run.cjs` 0 FAIL; `rg -n "isNext" apps/web/src` = 0; `rg -n "data-ui='classic'|data-ui=\"classic\"" apps/web/src` chỉ còn migration.
- Tag `last-classic` trước commit xoá.

### 3.4 Tiêu chí chấp nhận
- [ ] v104: E2E chạy next mặc định, 0 FAIL.
- [ ] 4.0.8: chuỗi báo trước có ở changelog + Appearance (en + vi).
- [ ] 4.1.0: máy đang classic mở lên vào next, không mất layout Workspace.

---

## 4. Electron (WP-104k, WP-104l)

### 4.1 v104
- `.planning/AUDIT-v104-tauri-parity.md`: cột Tính năng, File Electron (`apps/desktop/src/...`), File Tauri (`apps/shell/src/...`), Trạng thái (đủ / thiếu / khác), Rust còn gọi TS?, macOS đã test?.
- `apps/desktop/e2e/cdp.cjs:14`: `SHELL` mặc định Tauri; Electron khi `--shell=electron` hoặc `CCM_E2E_SHELL=electron`.

### 4.2 Sau khi `.planning/QA-mac-checklist.md` đạt (WP-104l)
- Tag `last-electron`.
- Xoá `apps/desktop/src/**` trừ `preload.ts` (contract cho `scripts/gen-desk-shim.mjs`) và file TS mà Rust còn gọi (vd `apps/collector/src/toast.ts` qua `serve-toast.ts`) theo bảng.
- GIỮ: `apps/desktop/package.json` (version của `apps/shell/tauri.conf.json`), `apps/desktop/e2e/`.
- Bỏ: `CCM_CORE`, `CCM_CORE_PTY`, `CCM_CORE_USAGE`, `CCM_CORE_TRANSCRIPT`, `CCM_CORE_CODEX`, `CCM_CORE_SIDEFILES`, `CCM_CORE_PROCS` giá trị `node`; 33 file `*.twin.test.ts`; `apps/desktop/src/deepseek/`.
- CLAUDE.md: viết lại Commands (`npm run desktop`, `desktop:dev`, `e2e`), Architecture (`apps/desktop` → chỉ e2e + package.json), env.
- Kiểm không vỡ: `npm run typecheck`, `npm test`, `cargo test` crate shell, `node apps/desktop/e2e/run.cjs` (Tauri) 0 FAIL, `npm run dist:win` chủ nhân tự build.

### 4.3 Tiêu chí chấp nhận
- [ ] v104: bảng không còn ô "chưa rõ"; E2E mặc định Tauri 0 FAIL (file chỉ Electron ghi lý do).
- [ ] Khi xoá: `rg -n "electron" apps/web/src apps/shell/src` chỉ còn tên lịch sử / migration userData.

---

## 5. Code chết, DeepSeek lazy, `.code_index` (WP-104f, WP-104g)

### 5.1 Xoá (WP-104f)
| File | Kiểm trước khi xoá |
|---|---|
| `apps/web/src/components/ActivityFeed.vue` + `components/activityPeer.test.ts` | `rg -n "ActivityFeed" apps/web/src` chỉ ra test này |
| `components/ToolTimeline.vue` + `stores/ui.ts` `initialTimelineWindow`, `timelineWindow` | `rg -n "timelineWindow" apps/web/src` chỉ ra `ui.ts` + `ToolTimeline.vue` |
| `components/MetricsStrip.vue`, `WaitingQueue.vue`, `SessionList.vue`, `SessionSummary.vue`, `FlipClock.vue` | 0 importer |
| `components/monitor/OrbitRings.vue`, `components/next/ui/GlassDialog.vue`, `components/workspace/DevServerChip.vue` | 0 importer |
| key i18n chỉ các file trên dùng | `rg -n "'<key>'" apps/web/src --glob '!i18n/**'` = 0 |
Kiểm không vỡ: `npm run typecheck`, `npm test -w @ccm/web`, `npm run build -w @ccm/web` không lỗi import; `rg -n "import\(.*(ActivityFeed|ToolTimeline|MetricsStrip|WaitingQueue|SessionList|SessionSummary|FlipClock|OrbitRings|GlassDialog|DevServerChip)" apps/web/src` = 0. Comment nhắc Mission Control: không tự sửa (Q3 PLAN).

### 5.2 DeepSeek lazy (WP-104g)
- Thư mục `apps/web/src/features/deepseek/`: `SettingsDeepSeek.vue`, `store.ts` (từ `stores/deepseek.ts`, giữ id Pinia `deepseek`), `gate.ts` (từ `lib/deepseekGate.ts`), `i18n.ts` (từ `i18n/deepseek-v81.ts`), `index.ts::loadDeepSeek()`.
- Hành vi: `hasDeepSeek === false` → không request chunk DeepSeek, trang Settings DeepSeek ẩn (như nay, `needs: 'deepseek'`). `true` → `App.vue` gọi `loadDeepSeek()` lúc khởi động (sau first paint), SettingsMenu render qua `defineAsyncComponent`; preset `deepseek` trong `stores/workspace.ts` chờ module trước khi mở pane.
- Không đổi khoá `ccm.*` hay IPC DeepSeek.
- Kiểm không vỡ: test như PLAN WP-104g; với `CCM_DEEPSEEK_FEATURE=1` mở Settings → DeepSeek, mở pane DeepSeek, không lỗi console; không cờ thì DevTools Network không có chunk `deepseek`.

### 5.3 `.code_index` (WP-104f)
- `_meta.md`: mục trước v90 → `.code_index/archive/_meta-pre-v90.md`, giữ 1 dòng trỏ archive.
- `web.md` → `web.md` (mục lục + store chung), `web-workspace.md`, `web-monitor.md`, `web-settings.md`; mỗi file ≤ ~40 KB.
- CLAUDE.md mục "Code index" thêm tên file mới.
- Tiêu chí: `du -k .code_index/*.md` mỗi file ≤ 45 KB (trừ archive); mọi path trong index còn tồn tại (script nhỏ `rg -o` path → `test -f`, chỉ in số path hỏng).

---

## 6. Cảnh báo 2 agent sửa cùng 1 file (WP-104d)

### 6.1 Định nghĩa
- "Đụng file": file `F` có ≥ 2 terminal khác nhau với `kind === 'edit'` trong lượt hiện tại (hoặc ≤ `AFTER_TURN_MS` 120 s sau lượt), theo `lib/agentTouches.ts::buildTouchIndex` (`state === 'conflict'`). Không tính `read`.
- Cặp báo = `(fileKey, idA, idB)` với `idA < idB`. File có 3 agent → 3 cặp, nhưng UI gộp theo file.
- "Lượt": từ lúc 1 trong 2 terminal chuyển `working` (edges `turnStart` trong `stores/agentTouches.ts`) tới khi cả 2 rời `working`. Mỗi cặp báo toast **1 lần / lượt**; lượt mới của 1 trong 2 bên mà vẫn đụng thì báo lại.

### 6.2 Switch
- `ccm.notify.clash` (`stores/settings.ts::clashWarn`), **mặc định tắt** (`'0'`). Settings → Thông báo, dòng `[data-setting-row="clash"]`, entry `{ page: 'notify', id: 'clash', labelKey: 'v104.clash.setting', hintKey: 'v104.clash.settingHint' }`.
- MỘT switch điều khiển cả 4 chỗ: chip rail, chip header pane, nhóm Inbox "Đụng file", toast. Tắt → không tính `clashes` (selector trả rỗng), không hiện gì. Explorer tree (`ExplorerTree.vue`) vẫn hiện trạng thái `conflict` như cũ (không đổi, không thuộc switch).
- Không phụ thuộc `settings.notifyEnabled` cho chip / Inbox; toast thì tôn trọng `notifyEnabled` + giờ yên lặng (`inQuietHours`).

### 6.3 UI
- Chip trên dòng rail next (`RailRowItem.vue`): icon `FileWarning` + "Đụng file" + số file nếu > 1, màu token `warning`, `data-clash-chip`. Tooltip: "Đụng file với {peer}: {file}" (tối đa 3 file, còn lại "+{n}").
- Chip trên header pane (`TermPane.vue`, nhánh `PaneHeader`): cùng nội dung, bấm mở popover nhỏ: danh sách file (tên + session kia), mỗi dòng 2 nút "Xem diff", "Bỏ qua file này"; cuối có gợi ý "Tách worktree" (mở hộp `wt.add` có sẵn, không tự tạo).
- Container hẹp: chỉ còn icon + số (container query như header hiện tại).
- Toast trong app (`useToastsStore`, kind `warning`): tiêu đề "{a} và {b} cùng sửa {file}", 2 action "Xem diff", "Bỏ qua file này". Cửa sổ không focus → thêm `Notification` web (Q1 PLAN về OS toast Tauri).
- Inbox: nhóm 4 "Đụng file", 1 dòng / file: "{file}" + "{a}, {b}".
- Classic: không thêm (đóng băng).

### 6.4 Hành động
- "Xem diff": `workspace.openChanges(repoRoot, { select: relPath })` (repo root = checkout chứa file, `lib/whoIsEditing.ts::checkoutOf`). File không thuộc repo git → mở file (`kind: 'file'`).
- "Bỏ qua file này": thêm `fileKey` vào `ccm.clash.ignored` (max 200, bỏ khoá cũ nhất); chip / toast / Inbox của file đó tắt tới khi chủ nhân bỏ ignore hoặc mọi terminal liên quan đóng (prune theo `terminals.live`). Toast Undo 5 s.
- Không bao giờ chặn, không gửi gì vào pane, không gọi MCP. MCP `who_is_editing` có sẵn cho agent, không đổi.

### 6.5 Data / store / protocol
- `lib/clash.ts`: `interface Clash { fileKey: string; path: string; peers: { terminalId: string; name: string; at: number }[] }`; `clashesByTerminal(index): Map<terminalId, Clash[]>`; `newClashes(prev: Set<string>, next: Clash[], turnKey: (id) => string): Clash[]` (cặp chưa báo trong lượt này).
- `stores/agentTouches.ts`: `clashes` (computed, rỗng khi switch tắt), `ignored`, `ignoreFile`, `unignoreFile`, `reported` (Set khoá `clashKey + turnStart`, memory).
- Protocol desktop: không đổi (dữ liệu đã có từ `tool.span`). Remote: `RemotePane.inbox` reason `clash` với `file` tương đối + `peer` (WP-104e).
- Không thêm `ToastKind` ở v104 (Q1).

### 6.6 i18n
| Key | vi | en |
|---|---|---|
| `v104.clash.setting` | Cảnh báo 2 agent sửa cùng file | Warn when two agents edit the same file |
| `v104.clash.settingHint` | Hiện chip trên rail và header pane, nhóm trong Inbox và thông báo khi 2 session cùng sửa 1 file trong lượt này. Chỉ cảnh báo, không chặn | Shows a chip on the rail and pane header, an Inbox group and a notification when two sessions edit one file in the same turn. Warns only, never blocks |
| `v104.clash.chip` | Đụng file | Same file |
| `v104.clash.chipN` | Đụng {n} file | {n} shared files |
| `v104.clash.tip` | Đụng file với {peer}: {file} | Same file as {peer}: {file} |
| `v104.clash.toastTitle` | {a} và {b} cùng sửa {file} | {a} and {b} are both editing {file} |
| `v104.clash.viewDiff` | Xem diff | View diff |
| `v104.clash.ignore` | Bỏ qua file này | Ignore this file |
| `v104.clash.ignored` | Đã bỏ qua {file} | Ignoring {file} |
| `v104.clash.worktree` | Tách worktree | Split into a worktree |

### 6.7 Edge case
- 1 agent sửa file 2 lần: không phải đụng.
- Agent A đọc, B sửa: không đụng (`read` không tính).
- 2 agent ở 2 worktree khác nhau cùng path tương đối: khác `pathKey` tuyệt đối, không đụng.
- Path khác hoa / thường trên Windows: `pathKey` đã chuẩn hoá.
- Terminal đóng giữa lượt: cặp biến mất, không toast trễ.
- > `MAX_AGENTS_PER_FILE` (6): giữ giới hạn có sẵn.
- Span đến trễ (collector gửi bù sau reconnect): vẫn qua `isFresh`, nếu cặp đã báo trong lượt thì không báo lại.

### 6.8 Tiêu chí chấp nhận
- [ ] Mặc định tắt: cài mới không thấy chip / toast / nhóm Inbox.
- [ ] Bật: 2 session sửa `a.ts` → chip trên 2 dòng rail + 2 header pane, nhóm Inbox "Đụng file" 1 dòng, toast đúng 1 lần trong lượt.
- [ ] Lượt mới của A vẫn sửa `a.ts` khi B còn trong cửa sổ lượt → toast thêm 1 lần.
- [ ] "Bỏ qua file này" tắt cả 4 chỗ cho file đó, Undo trả lại.
- [ ] "Xem diff" mở Changes chọn đúng file.
- [ ] Status có icon + text + màu; chip cắt chữ, không cuộn ngang; en + vi.
- [ ] `lib/clash.test.ts`, `lib/agentTouches.test.ts`, E2E `v104-clash.cjs` pass.

---

## 7. Changes theo session (WP-104h)

### 7.1 Dữ liệu
- `lib/turnEdits.ts`: `interface TurnEdits { terminalId: string; name: string; turnStart: number; turnEnd?: number; paths: Set<string> }` (paths = `pathKey` của span `access === 'edit'` có `start ≥ turnStart - TURN_SLACK_MS`).
- `stores/turnEdits.ts::useTurnEditsStore`: `current: Map<terminalId, TurnEdits>` (lượt đang chạy), `last: Map<terminalId, TurnEdits>` (lượt xong gần nhất). Khi terminal chuyển `working → idle`: `current` thành `last` (snapshot, không phụ thuộc `toolSpans` `KEEP_MS` 2 giờ hay `AFTER_TURN_MS`). Lượt mới bắt đầu: `last` cũ GIỮ tới khi đánh dấu đã xem, lượt mới chỉ vào `current`; khi lượt mới xong mà `last` cũ chưa xem → gộp path (union) và giữ `turnStart` sớm nhất, để không bỏ sót.
- "Đã xem": bấm "Đánh dấu đã xem" trong Changes khi đang lọc session S → xoá `last[S]`, đồng thời `lib/reviewQueue.ts::markSeen` cho các file đó. Terminal đóng → xoá cả 2.
- Lưu: memory + `sessionStorage` key `ccm.turnEdits` (sống qua reload trang, không qua restart app; restart thì nhóm "Không rõ" gánh). Max 2000 path / terminal.

### 7.2 UI Changes pane
- Thanh segmented "Theo session" phía trên danh sách (`data-session-filter`), chỉ hiện khi repo có ≥ 1 session có `last` hoặc `current` không rỗng: "Tất cả" | mỗi session (StatusIcon + tên, đếm file) | "Không rõ".
- Lọc session S: chỉ file trong git status ∩ (`current[S]` ∪ `last[S]`), theo path tuyệt đối → tương đối repo. File cũng nằm trong session khác: icon đụng file (khi switch clash bật) hoặc chữ "+{peer}" nhỏ.
- "Không rõ": file trong git status không thuộc session nào (Bash, sed, codegen, formatter, sửa tay). **Luôn hiện** khi có ít nhất 1 file như vậy, kể cả khi đang lọc session khác (segment không bị ẩn); tooltip giải thích.
- Nhóm "Chưa xem" (`reviewSplit`) giữ nguyên, áp sau bộ lọc.
- Pane khôi phục mà session không còn: về "Tất cả". Lưu lựa chọn ở pane field `sessionFilter` (không lưu đĩa).
- Container hẹp: segmented thành dropdown.

### 7.3 Lối vào
- Toast `done` (in-app, `useNotifications.ts`) có action "Xem thay đổi" khi `last[S]` không rỗng → `openChanges(repoRoot, { session: S })`. `Notification` web bấm vào làm cùng việc.
- Inbox nhóm "Xong, chưa xem": nút "Xem thay đổi" trên dòng.
- Rail next menu chuột phải (`lib/rail/railMenu.ts::rowMenu`): "Xem thay đổi của lượt này" (`changesOfTurn`), tắt khi không có path.
- Ctrl+K: lệnh `c:turnChanges` cho pane đang active.

### 7.4 i18n
| Key | vi | en |
|---|---|---|
| `v104.turn.filter` | Theo session | By session |
| `v104.turn.all` | Tất cả | All |
| `v104.turn.unknown` | Không rõ | Unknown |
| `v104.turn.unknownTip` | File đổi không qua Edit / Write của agent (lệnh shell, formatter, sửa tay) | Changed outside an agent's Edit / Write (shell commands, formatters, by hand) |
| `v104.turn.view` | Xem thay đổi | View changes |
| `v104.turn.menu` | Xem thay đổi của lượt này | View this turn's changes |
| `v104.turn.markSeen` | Đánh dấu đã xem lượt này | Mark this turn as seen |
| `v104.turn.alsoBy` | cũng do {peer} | also by {peer} |

### 7.5 Edge case
- File agent sửa rồi revert về như cũ: không còn trong git status, không hiện.
- File agent tạo rồi xoá: git status không có, không hiện.
- Session ở worktree khác repo: chỉ hiện trong Changes của worktree đó.
- Rename (git status `R`): khớp theo path mới hoặc cũ.
- Span không có path (Bash): không vào tập nào, file rơi vào "Không rõ".
- 2 session cùng file: hiện ở cả 2 bộ lọc, không ở "Không rõ".

### 7.6 Tiêu chí chấp nhận
- [ ] Lọc session A chỉ ra file A sửa bằng Edit / Write trong lượt cuối.
- [ ] File sửa bằng Bash luôn ở "Không rõ", segment "Không rõ" không bao giờ bị ẩn khi có file.
- [ ] Sau 10 phút (quá `AFTER_TURN_MS`), tập lượt cuối vẫn còn tới khi "Đánh dấu đã xem".
- [ ] Toast `done` và Inbox nhóm Xong có "Xem thay đổi" mở Changes đã lọc.
- [ ] Menu rail "Xem thay đổi của lượt này" mở đúng bộ lọc.
- [ ] en + vi; không cuộn ngang ở pane hẹp.
- [ ] `lib/turnEdits.test.ts`, `lib/reviewQueue.test.ts`, `lib/rail/railMenu.test.ts`, E2E `v104-turn-changes.cjs` pass.
