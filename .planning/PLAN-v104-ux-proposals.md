# PLAN v104 - terminal điện thoại khi resize + 7 đề xuất UX 2026-11-20

Nhánh: `feat/v98-pwa` (sau v103, UI next mặc định, chưa release; bản kế 4.0.8). Nguồn: `.planning/PROPOSALS-2026-11-20-ux.md`. Spec hành vi: `.planning/SPEC-v104-ux-proposals.md`. Triển khai chế độ manager: mỗi gói nghiệm thu xong thì commit + push lên `feat/v98-pwa`, không trailer attribution.

## Quyết định đã chốt (chủ nhân, 2026-11-20)
| # | Việc | Quyết định |
|---|---|---|
| 1 | Gói đầu tiên | WP-104a: terminal điện thoại mất dòng khi resize. Làm cả 2 hướng: (a) điện thoại xử lý output + event đúng thứ tự, flush rồi resize ngay; (b) desktop giữ terminal ảo, gửi snapshot (scrollback max 2000 dòng) sau mỗi resize và khi nối lại |
| 2 | Inbox | Desktop + web app `apps/pwa` + Android `apps/mobile`: cùng nhóm, cùng thứ tự. Desktop gửi thêm nhóm "Chờ trả lời menu" và "Đụng file" qua remote |
| 3 | UI classic | Bước 1 (E2E sang next) làm ngay; 4.0.8 classic đóng băng + changelog báo trước; xoá hẳn ở **4.1.0** |
| 4 | Electron | v104 chỉ làm bảng so tính năng Tauri vs Electron + E2E mặc định Tauri. Xoá `apps/desktop/src` sau khi `.planning/QA-mac-checklist.md` đạt, giữ tag `last-electron` |
| 5 | Đụng file | MỘT switch chung (Settings → Thông báo) điều khiển chip rail + header pane + nhóm Inbox "Đụng file" + toast. **Mặc định TẮT**. Bật: mỗi cặp (file, 2 session) báo 1 lần / lượt; nút "Bỏ qua file này" + "Xem diff"; chỉ cảnh báo, không chặn |
| 6 | Settings | Làm cả 3: sửa search + test; `fileIcons` / `dust` chỉ ở trang Workspace; gộp `reminders` vào `notify`, map trang cũ |
| 7 | Changes theo session | Luôn hiện nhóm "Không rõ"; giữ tập file lượt cuối tới khi đánh dấu đã xem; notify `done` có nút "Xem thay đổi" |
| 8 | DeepSeek | Tách module lazy (không xoá) |
| 9 | Triển khai | Manager; commit + push từng gói sau nghiệm thu, không trailer |

Phát hiện lúc soát code (đã tính vào gói):
- Đã có **6** lối "cần bạn": ngoài 5 lối trong đề xuất còn nút Inbox ở rail foot `components/next/NextShell.vue` (`data-inbox-button`) + `components/inbox/InboxPanel.vue` + `stores/inbox.ts::useInboxStore` + `lib/inbox.ts::inboxItems`, Ctrl+K `c:inbox` (`CommandPalette.vue:525`). Android đã có `apps/mobile/lib/ui/workspace/inbox_page.dart::InboxPage`. Inbox v104 dựng trên các thứ này, không làm mới từ đầu.
- Settings next đã map `dust` / `fileIcons` sang `workspace` (`components/settings/index.ts::NEXT_PAGE_OF`) và lọc `stars*` (`nextEntries`). Còn thiếu: test bảo đảm mọi entry có dòng thật; `bugReportNudge` lẫn trong khối `reminders`; classic vẫn 2 bản.
- `MenuQueueChip.vue` gọi `useMenuToast` (toast OS có nút trả lời menu v100). Bỏ chip phải dời lời gọi, không thì mất toast.
- `ActivityFeed.vue` còn 1 importer là test `components/activityPeer.test.ts`.
- `apps/shell/tauri.conf.json` lấy version từ `../desktop/package.json`: gỡ Electron không được xoá file này.
- PWA `components/TerminalSurface.vue`: output vào `queue` chờ `requestAnimationFrame(flush)`, còn `pty.resized` gọi `relayout()` ngay, nên byte cũ (vẽ cho cỡ cũ) bị ghi SAU resize: cùng lớp lỗi với Android, chiều ngược.
- Cargo lock chưa có crate terminal ảo (`vt100`, `alacritty_terminal`): WP-104a(b) thêm dependency mới.

## Gói
| Gói | Việc | Cỡ | Phụ thuộc | Song song với | Model |
|---|---|---|---|---|---|
| WP-104a1 | Terminal resize, phía điện thoại (Android + PWA) | M | không | a2, b, c | opus |
| WP-104a2 | Terminal resize, desktop terminal ảo + snapshot | M-L | không (giao thức chốt trong SPEC 0) | a1 | opus |
| WP-104b | Đề xuất 3 bước 1: E2E mặc định next, sửa E2E Mission Control | S | không | a1, a2, c | sonnet |
| WP-104c | Đề xuất 2: Settings search, trang Workspace, gộp reminders | S-M | không | a, b, f | sonnet |
| WP-104d | Đề xuất 6: cảnh báo đụng file (desktop) | M | b | f, g | opus |
| WP-104e | Đề xuất 1: Inbox desktop + remote + PWA + Android | L | b, d | không | opus (desktop + giao thức), sonnet (PWA), sonnet (Android) |
| WP-104f | Đề xuất 5: code chết, `.code_index` | S-M | không | c, d | haiku (xoá) + sonnet (index) |
| WP-104g | Đề xuất 5: DeepSeek lazy module | M | f | d | sonnet |
| WP-104h | Đề xuất 7: Changes theo session | M | d, e | không | opus |
| WP-104i | Đề xuất 3 bước 2: classic đóng băng + changelog 4.0.8 | S | b | bất kỳ | haiku |
| WP-104k | Đề xuất 4 phần v104: bảng Tauri vs Electron + E2E mặc định Tauri | M | b | i | sonnet |
| WP-104j | Đề xuất 3 bước 3: xoá classic | L | release 4.1.0 mở, i | không | opus |
| WP-104l | Đề xuất 4 phần sau: xoá `apps/desktop/src` + fallback Node | L | k, QA Mac đạt | không | opus |

Lượt (≤ `JIN_MAX_SUBAGENTS`, mặc định 3):
- Lượt 1: WP-104a1 ‖ WP-104a2 ‖ WP-104b.
- Lượt 2: WP-104c ‖ WP-104f ‖ WP-104i.
- Lượt 3: WP-104d ‖ WP-104g ‖ WP-104k.
- Lượt 4: WP-104e (3 subagent con: desktop + giao thức trước, rồi PWA ‖ Android).
- Lượt 5: WP-104h.
- Sau v104: WP-104j (nhánh 4.1.0), WP-104l (khi QA Mac đạt).

Mỗi gói về: main chạy lại test liên quan, xem ảnh, nghiệm thu, commit + push `feat/v98-pwa` (message không trailer), cập nhật `.planning/PROGRESS-2026-11-20-v104.md`.

---

## WP-104a1: Terminal điện thoại, xử lý đúng thứ tự (M, opus)
Nguyên nhân (đã tái hiện trên Galaxy A33, mất line-1963..1984): ConPTY tạo với `PSEUDOCONSOLE_RESIZE_QUIRK` (`apps/core/crates/core-pty/src/pty_win.rs:21, 158`) vẽ lại viewport theo cỡ mới ngay sau resize. Android resize terminal muộn: `apps/mobile/lib/ui/terminal/terminal_surface.dart::_syncDesk` chạy trong post-frame callback (`didUpdateWidget`), sau khi `terminal_pane.dart::_flushOut` đã ghi byte cùng frame; output đi qua `Utf8Decoder().bind(handle.output)` (`terminal_pane.dart:179`) thêm 1 nhịp async so với `handle.events`. Kết quả: phần vẽ lại cho 41 hàng ghi vào terminal 19 hàng, mất đúng 22 dòng.
Phạm vi:
- Android `apps/mobile/lib/ui/terminal/terminal_pane.dart`: bỏ `Utf8Decoder().bind`, decode đồng bộ bằng `Utf8Decoder` dạng chunked (`startChunkedConversion`) trong cùng listener với event; 1 hàng đợi chung output + event theo thứ tự nhận (`_onEvent` `case 'pty.resized'`: `_flushOut()` → resize terminal ngay (không chờ frame) → đẩy màn hình hiện tại vào scrollback → nhận byte sau).
- Android `terminal_surface.dart::_syncDesk`: chỉ còn đồng bộ khi layout đổi; resize do `pty.resized` do `terminal_pane.dart` làm trực tiếp trên `Terminal` (xterm.dart) trước khi ghi byte kế.
- PWA `apps/pwa/src/components/TerminalSurface.vue::onEvent` `case 'pty.resized'`: gọi `flush()` đồng bộ trước `relayout()`; `layoutDesk` đẩy viewport vào scrollback trước `t.resize` (ghi `\r\n` x rows hoặc `scrollLines` tuỳ bản xterm.js, chốt trong SPEC 0.3).
- PWA `apps/pwa/src/terminal/ptyStreams.ts`: bảo đảm event và output của 1 stream đi qua cùng thứ tự (không tách hàng đợi).
Tiêu chí xong:
- Unit Android: `apps/mobile/test/terminal_resize_order_test.dart` (mới): burst 3000 dòng `line-N`, chèn `pty.resized` 41 → 19 hàng giữa burst, buffer + scrollback chứa đủ line-1..3000 liên tục. `flutter test test/terminal_resize_order_test.dart --concurrency=$JIN_MAX_THREADS`.
- Unit PWA: thêm ca vào `apps/pwa/src/terminal/terminal.test.ts` (cùng kịch bản, xterm headless). `npm test -w @ccm/pwa -- src/terminal/terminal.test.ts`.
- E2E PWA: ca mới trong `apps/pwa/e2e/` (peer giả gửi burst + `pty.resized`, kiểm không mất dòng) trên WebKit + Chromium.
- Kiểm thật trên A33 (USB, bản debug): chạy `seq 1 3000` trong pane Claude / shell, xoay máy / đổi bên đang gõ giữa burst, cuộn lại thấy đủ dòng; ảnh vào `docs/qa/v104/WP-104a/`.
Rủi ro: xterm.dart / xterm.js reflow khi resize có thể tự cắt dòng dài; giảm bằng test so nội dung dòng chứ không so số hàng.

## WP-104a2: Desktop terminal ảo + snapshot (M-L, opus)
Phạm vi:
- `apps/shell/src/remote/pty_bridge.rs::Pump`: mỗi stream giữ 1 terminal ảo (crate mới, đề xuất `vt100` vì nhẹ, có scrollback; chốt trong SPEC 0.2), nạp mọi byte `flush` gửi đi; trong `text()` nhánh `"resized"`: `flush()` → `set_size` → gửi `pty.resized` → gửi `pty.snapshot` (nếu phía điện thoại báo hỗ trợ).
- Khi điện thoại nối lại (`Attach`, chỗ đang gửi tail byte thô từ ring `core-pty/src/ring.rs::snapshot`): gửi `pty.snapshot` thay tail nếu client hỗ trợ; client cũ giữ tail như cũ.
- Capability: `RemoteAttach` có `caps: ['pty.snapshot']` (thêm vào `packages/shared/src/remote.ts` và Dart model), desktop chỉ gửi snapshot khi có cap.
- Client Android + PWA xử lý `pty.snapshot` (thuộc a1 nếu a1 xong trước, hoặc làm ở a2 sau a1).
Tiêu chí xong: `cargo test -p orrery-shell remote::pty_bridge` (test mới: burst + resize giữa chừng, snapshot chứa đủ 2000 dòng cuối, đúng cỡ mới; client không có cap nhận tail như cũ); test tương thích client cũ; E2E PWA nối lại sau resize thấy đúng màn hình; kiểm thật A33 như a1 với mạng bị cắt / nối lại.
Rủi ro: (1) CPU / RAM khi parse mọi stream: chỉ dựng terminal ảo cho stream có điện thoại đang attach, max 2000 dòng scrollback / stream; (2) snapshot to qua relay: nén giống `Out`, cắt theo `MAX_BACKLOG` hiện có; (3) client cũ: có cap mới gửi.

## WP-104b: E2E mặc định UI next (S, sonnet)
Phạm vi: `apps/desktop/e2e/ui-gen.cjs::uiGenFromArgs` default `'next'`; `apps/desktop/e2e/v51-compact.cjs` (dòng 51-55) và `apps/desktop/e2e/v25.cjs` (dòng 207-209) bỏ bước Mission Control, đọc pane trực tiếp; `apps/desktop/e2e/run.cjs` thêm map `CLASSIC_ONLY` (file nào chỉ có nghĩa ở classic thì chạy kèm `--ui=classic`).
Tiêu chí xong: `node apps/desktop/e2e/run.cjs` 0 FAIL ở next; `v51-compact.cjs`, `v25.cjs` pass; `rg -n "data-mission" apps/desktop/e2e` chỉ còn các check "không còn Mission Control" (`rail.cjs:74`, `v100-menu-queue.cjs:89`, `v103-polish.cjs:101`).
Rủi ro: fail hàng loạt do selector classic. Giảm: phân loại (sửa selector / gắn classic); lỗi app thật ghi `bug_pool/`, báo main, không sửa code app trong gói.

## WP-104c: Settings (S-M, sonnet)
Phạm vi:
- `apps/web/src/components/settings/index.ts`: `SETTINGS_PAGES` bỏ `reminders`; entries `break`, `stuck`, `contextWarn`, `budget` → `page: 'notify'`; `bugReportNudge` dời xuống khối `about`; `loadPage` map `reminders` → `notify` (khoá `ccm.settings.page`); `fileIcons` / `dust` khai báo thẳng `page: 'workspace'` (bỏ khỏi `NEXT_PAGE_OF`), classic thì `pageAvailable` vẫn cho trang workspace (bỏ `workspace` khỏi `NEXT_ONLY_PAGES`, trang workspace classic chỉ chứa 2 dòng này) hoặc classic giữ ở appearance tới 4.1.0: chọn cách đầu.
- `apps/web/src/components/SettingsMenu.vue`: bỏ `SettingsReminders` ở dòng 391, icon `reminders: Timer` (dòng 125); `flash()` không tìm thấy dòng thì toast `v104.settings.notFound`.
- `components/settings/SettingsNotify.vue` nhúng `SettingsReminders.vue` dưới section "Nhắc nhở" (giữ file, đổi vai thành section).
- `components/settings/SettingsAppearance.vue` (dòng 142-145) bỏ `fileIcons` / `dust`; `SettingsWorkspace.vue` (dòng 56-58) giữ.
- Test mới `apps/web/src/components/settings/rows.test.ts`: mount từng trang (next + classic), mỗi entry hiển thị có đúng 1 `[data-setting-row="<id>"]`.
Tiêu chí xong: `npm test -w @ccm/web -- src/components/settings src/lib/settingsFlash.test.ts`; `npm run typecheck -w @ccm/web`; bước E2E trong `apps/desktop/e2e/v103-polish.cjs` (search "file icons" → bấm → `.ccm-setting-flash` trên trang Workspace; mở app với `ccm.settings.page=reminders` → vào notify).

## WP-104d: Cảnh báo đụng file (M, opus)
Phạm vi:
- Mới `apps/web/src/lib/clash.ts` (thuần) + `lib/clash.test.ts`: `clashesByTerminal(index: TouchIndex)`, `clashKey(fileKey, idA, idB)`, `newClashes(prev, next, turnOf)` (mỗi cặp 1 lần / lượt).
- `apps/web/src/stores/agentTouches.ts::useAgentTouchesStore`: `clashes`, `ignoreFile(fileKey)`, `ignored` (`ccm.clash.ignored`, max 200), chỉ tính khi `settings.clashWarn`.
- `apps/web/src/stores/settings.ts`: `clashWarn` (`ccm.notify.clash`, default `'0'` = tắt).
- `components/settings/index.ts`: entry `{ page: 'notify', id: 'clash', ... }`; `SettingsNotify.vue` 1 switch `[data-setting-row="clash"]`.
- Rail: `components/next/rail/LiveRail.vue::extrasOf` + `RailRowItem.vue` chip. Header pane: `components/workspace/TermPane.vue` (nhánh `PaneHeader`). Classic không thêm.
- Toast: `composables/useNotifications.ts` `NotifyKind` thêm `'clash'`, dùng `useToastsStore` (kind `warning`, action "Xem diff", "Bỏ qua file này") + `Notification` khi cửa sổ không focus.
- "Xem diff": `stores/workspace.ts::openChanges(cwd, { select })`.
- i18n `apps/web/src/i18n/v104-ux.ts` (mới, nhúng `en.ts` / `vi.ts`).
Tiêu chí xong: `npm test -w @ccm/web -- src/lib/clash.test.ts src/lib/agentTouches.test.ts src/components/settings`; typecheck; E2E mới `apps/desktop/e2e/v104-clash.cjs` (switch tắt: không chip / toast; bật: chip trên 2 dòng rail + header, toast 1 lần / lượt, "Bỏ qua file này" tắt chip, "Xem diff" mở Changes đúng file); thêm vào `run.cjs` và `scripts/qa-fast.mjs::E2E_MAP`.
Rủi ro: báo nhầm khi 2 agent sửa 2 đoạn hợp lệ (chỉ cảnh báo, mặc định tắt); CPU (chỉ duyệt `files` có `state === 'conflict'`).

## WP-104e: Inbox gộp, desktop + remote + PWA + Android (L, opus / sonnet)
Phạm vi desktop:
- `apps/web/src/lib/inbox.ts`: `InboxReason` thêm `'menu' | 'permission' | 'question' | 'clash'`, `waiting` tách theo `waitReason` (`packages/shared/src/status.ts`); `InboxInput.menus`, `InboxInput.clashes`; `SEVERITY`; `inboxGroups()`; `INBOX_GROUPS` (thứ tự chung) chuyển vào `packages/shared/src/inbox.ts` (mới) để PWA dùng chung.
- `apps/web/src/stores/inbox.ts::useInboxStore`: nạp `useMenuQueueStore().entries`, `useAgentTouchesStore().clashes`, `lib/next/railSnooze.ts::isSnoozed`; `next(current)`.
- `components/inbox/InboxPanel.vue`: nhóm, phím `S`, nút "Xem thay đổi" (nhóm Xong).
- Mới `components/inbox/InboxChip.vue` thay `[data-attention-chip]` + `AttentionPopover` + `<MenuQueueChip />` trong `components/workspace/WorkspaceView.vue` (khoảng dòng 2043-2063); dời `useMenuToast` sang `InboxChip.vue` hoặc `App.vue`.
- `lib/workspaceKeys.ts` (dòng 78, 96) + `lib/shortcuts.ts`: Ctrl+Shift+A mở Inbox, Ctrl+Alt+Q mục kế.
- `CommandPalette.vue` nhóm `needs` lấy từ `useInboxStore().items`.
- Xoá khi hết importer: `components/workspace/AttentionPopover.vue`, `lib/rail/attentionList.ts`, `components/workspace/MenuQueueChip.vue` + `.test.ts` (ca test sang `InboxChip.test.ts`). Giữ `stores/menuQueue.ts`.
Phạm vi remote:
- `packages/shared/src/remote.ts::RemotePane`: field tuỳ chọn `inbox?: { reason: InboxReason; since?: string; file?: string; peer?: string }`; `apps/web/src/lib/remoteSummary.ts` điền từ `useInboxStore`; `apps/shell/src/remote/fit.rs` không cắt field này; `docs/remote-integration.md` §4.3 thêm field.
- PWA: `apps/pwa/src` màn Inbox (tab hoặc sheet từ Monitor) dùng `INBOX_GROUPS`; chạm mục mở session.
- Android: `apps/mobile/lib/ui/workspace/inbox_page.dart::InboxPage` + `apps/mobile/lib/ui/model/workspace_model.dart` đọc `inbox`, nhóm theo cùng thứ tự; l10n `apps/mobile/lib/l10n/app_en.arb`, `app_vi.arb`.
Tiêu chí xong: `npm test -w @ccm/web -- src/lib/inbox.test.ts src/stores/menuQueue.test.ts src/components/inbox src/lib/shortcuts.test.ts src/lib/remoteSummary.test.ts`; typecheck mọi workspace; E2E `apps/desktop/e2e/v100-menu-queue.cjs` (selector `[data-inbox-chip]`, `[data-inbox-row][data-reason=menu]`), `v69-ide.cjs`, `rail.cjs`, `run-pane.cjs` pass; E2E PWA màn Inbox; `flutter test` cho model Inbox; Android kiểm thật A33.
Rủi ro: (1) ngoại lệ thứ 6 (v100): Inbox chỉ nhảy tới pane, không trả lời menu; điện thoại trả lời menu vẫn qua luồng hiện có; (2) quên dời `useMenuToast` → test kiểm composable chạy khi chip không mount; (3) đổi nghĩa Ctrl+Alt+Q → dòng "What's new" (`lib/changelog.ts`); (4) điện thoại cũ: field tuỳ chọn, không có thì dùng `attention` như cũ.

## WP-104f: Code chết + `.code_index` (S-M, haiku + sonnet)
Xoá (kiểm `rg -n "<Tên>" apps/web/src` = 0 importer ngay trước khi xoá, cả `import(` động):
- `apps/web/src/components/ActivityFeed.vue` + `components/activityPeer.test.ts` (nếu test có ý cho `lib/`, chuyển sang test lib).
- `components/ToolTimeline.vue`, `MetricsStrip.vue`, `WaitingQueue.vue`, `SessionList.vue`, `SessionSummary.vue`, `FlipClock.vue`, `monitor/OrbitRings.vue`, `next/ui/GlassDialog.vue`, `workspace/DevServerChip.vue`.
- `apps/web/src/stores/ui.ts`: `initialTimelineWindow`, `timelineWindow` (dòng 78, 342-346, 528).
- Key i18n chỉ các file trên dùng (kiểm từng key bằng `rg`).
- Comment nhắc Mission Control (`components/workspace/DropTerminal.vue:21`, `lib/paneSnapshots.ts:1`): không tự xoá (rule global), ghi vào báo cáo.
- `.code_index/`: `_meta.md` trước v90 → `.code_index/archive/_meta-pre-v90.md`; `web.md` tách `web-workspace.md`, `web-monitor.md`, `web-settings.md`, `web.md` còn mục lục; mỗi file ≤ ~40 KB; CLAUDE.md mục "Code index" cập nhật danh sách.
Tiêu chí xong: `npm run typecheck`, `npm test -w @ccm/web`, `npm run build -w @ccm/web` sạch; mỗi file `.code_index/*.md` ≤ 45 KB (trừ archive).

## WP-104g: DeepSeek lazy module (M, sonnet)
Phạm vi: dời vào `apps/web/src/features/deepseek/`: `components/settings/SettingsDeepSeek.vue`, `stores/deepseek.ts` (+ test), `lib/deepseekGate.ts`, `i18n/deepseek-v81.ts`; `features/deepseek/index.ts::loadDeepSeek()` (dynamic `import()`, `mergeLocaleMessage`). Điểm nối tĩnh: `lib/desktop.ts::hasDeepSeek`, `lib/provider.ts::isDeepseek`; đổi sang động: `components/SettingsMenu.vue:33` (`defineAsyncComponent`), `App.vue:14`, `stores/workspace.ts:57-58`, `components/next/rail/LiveRail.vue:16` (chỉ cờ). Không đụng `apps/desktop/src/deepseek/`, `apps/core/crates/core-desk/src/deepseek/`.
Tiêu chí xong: `npm test -w @ccm/web` (gồm `stores/handBack.test.ts`, `stores/workspace.test.ts`, `lib/launcher.test.ts`, `lib/rail/railMenu.test.ts`); typecheck; sau build web có chunk riêng cho `features/deepseek`; E2E `apps/desktop/e2e/v104-deepseek-lazy.cjs` (mới) với `CCM_DEEPSEEK_FEATURE=1` mở Settings → DeepSeek không lỗi console, không có cờ thì không request chunk.
Rủi ro: key i18n DeepSeek dùng ngoài module (launcher, rail menu) hiện key thô. Giảm: key dùng ngoài module giữ ở i18n chính.

## WP-104h: Changes theo session (M, opus)
Phạm vi:
- Mới `apps/web/src/lib/turnEdits.ts` + test: tập path `edit` theo terminal cho lượt hiện tại + lượt xong gần nhất, từ spans + edges.
- Mới `apps/web/src/stores/turnEdits.ts`: giữ lượt gần nhất tới khi "Đã xem" (dùng `lib/reviewQueue.ts::markSeen` / `markAllSeen`) hoặc terminal đóng; không phụ thuộc `AFTER_TURN_MS` và `stores/toolSpans.ts` `KEEP_MS` 2 giờ. Edges dùng chung với `stores/agentTouches.ts` (tách ra `lib/turnEdges.ts`).
- `components/workspace/ChangesPane.vue`: segmented "Theo session" (cạnh `reviewSplit` / `groups`, dòng 130-137); pane field `sessionFilter?: string` (`stores/workspace.ts` Pane kind `changes`, dòng 72).
- `stores/workspace.ts::openChanges(cwd, { select, session })`.
- `composables/useNotifications.ts`: toast `done` có action "Xem thay đổi"; `InboxPanel.vue` nhóm Xong cùng nút.
- `lib/rail/railMenu.ts::rowMenu`: mục `changesOfTurn`.
Tiêu chí xong: `npm test -w @ccm/web -- src/lib/turnEdits.test.ts src/lib/reviewQueue.test.ts src/lib/rail/railMenu.test.ts`; typecheck; E2E mới `apps/desktop/e2e/v104-turn-changes.cjs` (2 session sửa 2 file + 1 file sửa bằng Bash: lọc A chỉ ra file A; "Không rõ" luôn có file Bash; menu rail mở đúng bộ lọc; "Đã xem" xoá tập lượt cuối).
Rủi ro: file Bash / formatter không có span (nhóm "Không rõ" luôn hiện); 2 session cùng file (hiện ở cả 2, có icon đụng file khi switch bật); pane khôi phục mà session không còn (về "Tất cả").

## WP-104i: Classic đóng băng 4.0.8 (S, haiku)
Phạm vi: `apps/web/src/lib/changelog.ts` dòng 4.0.8 "Giao diện cũ sẽ bỏ ở 4.1.0"; `components/settings/SettingsAppearance.vue` hint ở dòng chọn classic (`v104.classic.sunset`); CLAUDE.md câu "classic stays the default" sửa đúng hiện trạng (next mặc định từ v103, classic đóng băng, xoá ở 4.1.0).
Tiêu chí xong: typecheck web; test changelog nếu có.

## WP-104k: Bảng Tauri vs Electron + E2E mặc định Tauri (M, sonnet)
Phạm vi: `.planning/AUDIT-v104-tauri-parity.md` (từng tính năng `apps/desktop/src/*` ↔ `apps/shell/src/*`, từ `.code_index/desktop.md` + `.code_index/shell.md`; cột "Rust còn gọi TS?", vd `apps/collector/src/toast.ts` qua `serve-toast.ts`; cột "macOS đã test?"); `apps/desktop/e2e/cdp.cjs:14` `SHELL` mặc định Tauri, Electron chỉ khi `--shell=electron` / `CCM_E2E_SHELL=electron`; `apps/desktop/e2e/run.cjs` ghi rõ build `apps/shell` debug trước.
Tiêu chí xong: `node apps/desktop/e2e/run.cjs` trên Tauri 0 FAIL (file nào chỉ chạy được trên Electron thì ghi lý do vào bảng, gắn cờ `--shell=electron`); bảng không còn dòng "chưa rõ".

## WP-104j: Xoá classic (L, opus, nhánh 4.1.0)
Bước 1 (sonnet, chỉ đọc): `.planning/AUDIT-v104-classic-parity.md`, mỗi tính năng chỉ có ở classic (chuông `otherNeeds` `WorkspaceView.vue:236, 1816`, explorer cũ quanh `WorkspaceView.vue:2327`, tab strip, drop overlay cũ, `AppHeader.vue`) → tương đương next hoặc "bỏ có chủ ý".
Bước 2: xoá nhánh `isNext` (WorkspaceView 45 chỗ, CommandPalette 26, NextShell, TermPane, DeviceUiTestDialog), `components/AppHeader.vue`, gộp `SettingsAppearance.vue` + `SettingsMistAppearance.vue`, `stores/uiGen` chỉ còn palette / motion, migration `ccm.uiGeneration` classic → next 1 lần; `WorkspaceView.vue` 2440 → ~1200 dòng; bỏ `--ui=classic` ở E2E.
Tiêu chí xong: `npm run typecheck`, `npm test`, `node apps/desktop/e2e/run.cjs` 0 FAIL; `rg -n "isNext" apps/web/src` = 0. Tag `last-classic` trước khi xoá.

## WP-104l: Xoá Electron + fallback Node (L, opus, khi QA Mac đạt)
Phạm vi: tag `last-electron`; xoá `apps/desktop/src` trừ `preload.ts` (contract `scripts/gen-desk-shim.mjs`) và phần Rust còn gọi (theo bảng WP-104k); GIỮ `apps/desktop/package.json` (version cho `apps/shell/tauri.conf.json`) và `apps/desktop/e2e/`; bỏ `CCM_CORE*=node` (`apps/desktop/src/core-serve-host.ts:41`, `main.ts:189, 865, 930`, `desk/forward.ts:22, 65`); 33 file `*.twin.test.ts`; DeepSeek desktop main `apps/desktop/src/deepseek/`; viết lại phần Electron trong CLAUDE.md.
Tiêu chí xong: `npm run typecheck`, `npm test`, `cargo test` crate shell, E2E Tauri 0 FAIL Windows, mục Tauri trong `.planning/QA-mac-checklist.md` đạt.

---

## Không được phá
- **Chỉ quan sát.** Không gửi prompt, không gọi API LLM, không spawn agent. Inbox, đụng file, Changes theo session chỉ đọc state có sẵn (`tool.span`, git status, terminal status). Ngoại lệ thứ 6 (v100): mọi phím vào pane chỉ sau 1 click của chủ nhân; Inbox không tự trả lời menu. Ngoại lệ thứ 7 (remote): pane chỉ submit khi idle; snapshot terminal ảo là nội dung terminal đã được phép ra điện thoại (mã hoá đầu-cuối), không thêm env / secret / app key / PTY token.
- **Dữ liệu nhạy cảm.** `tool.span` chỉ mang `path` + `access` (§11ar). Chip / toast / remote `inbox.file` chỉ tên file tương đối + tên session, không nội dung diff; push điện thoại không thêm kind mới ở v104.
- **i18n en + vi** cho mọi chuỗi mới (`apps/web/src/i18n/v104-ux.ts`, `apps/pwa/src/i18n/*`, `apps/mobile/lib/l10n/app_*.arb`); không em dash, không chấm giữa trong chuỗi mới.
- **Màu qua token**: `warning` cho đụng file, `st-waiting` / `st-error` cho trạng thái; status = icon + text + màu.
- **Không blur ở classic**; blur chỉ dưới `:root[data-ui='next']`.
- **Không cuộn ngang**: chip cắt chữ + tooltip; popover `min(400px, 100vw - 24px)` như `InboxPanel.vue`.
- **Không bịa số**: số Inbox chỉ đếm mục thật.
- **Hiệu năng**: không polling mới; selector derive từ store; terminal ảo chỉ cho stream đang có điện thoại attach.
- Không comment trong code; E2E chạy bản dev tách biệt (cổng riêng, `CCM_DATA_DIR` tạm), không tắt Orrery đang chạy; commit chỉ sau nghiệm thu, không trailer.

## Luật chung cho mọi brief
Như `.planning/PLAN-v103-next-polish.md` mục "Luật chung" (không comment, i18n en + vi, token màu, motion, file mới ≤ ~400 dòng, Grep / `rtk proxy rg`, vitest + typecheck + E2E next, `cargo -j 4`, `flutter test --concurrency=$JIN_MAX_THREADS`). Ảnh sáng / tối vào `docs/qa/v104/<WP>/`; báo cáo `.planning/wp/v104/<WP>-report.md` (file + symbol, test + số, ảnh, QA tay Windows, mục "Test macOS" cho main chép vào `.planning/QA-mac-checklist.md`). Subagent không commit; main commit sau nghiệm thu. Cập nhật `.code_index/` sau mỗi gói.

## Câu hỏi mở
- Q1: Toast `clash` ra OS khi Orrery ở nền trong vỏ Tauri: nếu `Notification` của webview không hiện thì cần `ToastKind` `clash` ở `packages/shared/src/types.ts`, `packages/shared/src/notify.ts::DEFAULT_TOAST_KINDS`, `apps/collector/src/toast.ts`, `apps/core/crates/core-store/src/persist/notify_file.rs::TOAST_KINDS` và đường web → core (chưa có). Đề xuất: đo trong WP-104d, cần thì thêm gói con.
- Q2: Crate terminal ảo: `vt100` (nhẹ, đủ scrollback) hay `alacritty_terminal` (chuẩn hơn, nặng hơn, tăng size bộ cài). Đề xuất `vt100`, chốt sau spike 1 giờ trong WP-104a2.
- Q3: Comment nhắc Mission Control (`DropTerminal.vue:21`, `paneSnapshots.ts:1`): rule global cấm tự xoá comment cũ; chủ nhân có muốn sửa?
