# WP-104e báo cáo: Inbox gộp, desktop + remote + PWA + Android

Inbox giờ là 1 chỗ duy nhất trên desktop, PWA và Android. Thứ tự nhóm chung (`INBOX_GROUPS`): Chờ trả lời menu, Xin quyền, Câu hỏi, Đụng file, Lỗi, Ghi chú của agent, Xong chưa xem. Inbox chỉ nhảy tới pane / session, **không bao giờ** trả lời menu hay gửi phím (ngoại lệ 6 giữ nguyên).

## File + symbol

### Shared
- `packages/shared/src/inbox.ts` (mới): `INBOX_REASONS`, `INBOX_GROUPS`, `inboxGroupOf`, `isInboxReason`, `isUrgentInboxGroup`, `RemoteInbox`, `parseRemoteInbox` (cắt max 120 ký tự), `sortInbox`. Export trong `index.ts`.
- `packages/shared/src/remote.ts`: `RemotePane.inbox?: RemoteInbox`, `RemoteWorkspaceSnapshot.inbox?: true`.

### Desktop (apps/web)
- `lib/inbox.ts`: viết lại.
  - `InboxReason` gồm menu, permission, question, clash, failed, loop, checks, stuck, note, done. `waiting` tách theo `waitReason`.
  - `InboxInput` thêm `menus`, `clashes`, `panes`, `paneOf`, `snoozed`.
  - Item có `terminalId` / `paneKey`. Key là terminal id, hoặc `pane:<key>` cho pane không có session, hoặc `clash:<fileKey>` (1 dòng / file).
  - Hàm `inboxGroups`, `nextInboxItem`, `inboxTone`.
  - Trong nhóm, cũ nhất lên đầu.
- `stores/inbox.ts::useInboxStore`:
  - Nạp `menuQueue.entries`, `agentTouches.clashes`, pane lỗi / gone / Run failed (`run.sessionsFor`) / attention waiting, `railSnooze.isSnoozed`.
  - API mới: `groups`, `count`, `tone`, `trigger`, `toggle(el)`, `close`, `reveal`, `next`, `goNext`, `snooze`.
- `components/inbox/InboxPanel.vue`:
  - Nhóm `data-inbox-group`, dòng `data-inbox-row` / `data-reason` / `data-pane`, có tên project.
  - Phím S hoãn 15 phút, có toast Hoàn tác. Nhóm Xong có nút Xem thay đổi.
  - Panel lật lên khi neo ở nửa dưới. Bug cũ: mở từ nút rail foot thì panel rơi ra ngoài màn hình, giờ đã sửa.
- `components/inbox/InboxChip.vue` (mới): `data-inbox-chip`, `data-tone` (waiting / error / warning / muted). Thay `[data-attention-chip]`, `AttentionPopover` và `MenuQueueChip` trong `components/workspace/WorkspaceView.vue`.
- Đã xoá: `components/workspace/AttentionPopover.vue`, `MenuQueueChip.vue` + `.test.ts`, `lib/rail/attentionList.ts` (bỏ luôn 2 ca test needs-you trong `lib/rail/rail.test.ts`). Giữ `stores/menuQueue.ts`.
- `composables/useInboxGlobal.ts` (mới, gọi trong `App.vue`):
  - Chuyển `useMenuToast` từ chip sang đây, nên toast OS v100 vẫn chạy khi chip không mount.
  - Phím toàn app (`inboxKeyAction`): Ctrl+Shift+A / ⇧⌘A mở / đóng, Ctrl+Alt+Q mục kế.
  - `WorkspaceView` bỏ qua 2 action này.
- `NextShell.vue`, `AppHeader.vue`: panel neo theo `inbox.trigger`. Ctrl+Shift+A neo vào chip nếu chip đang hiện.
- `CommandPalette.vue` + `lib/next/palette.ts::bucketOf({ needs })`: nhóm `needs` lấy từ mục Inbox (trừ note / done).
- `next/tokens.css`: style `[data-inbox-chip]` theo tone.
- i18n `i18n/v104-ux.ts` `v104.inbox.*` (en + vi). `en.ts` / `vi.ts`: đổi nhãn `shortcuts.wsAttention` / `wsAsked`.
- Remote:
  - `lib/remoteSummary.ts`: `RemoteSummaryInput.inbox`, `inboxOf`, cờ `inbox: true` ở gốc snapshot, `summaryHash` gồm cả cờ.
  - `composables/useRemoteSummary.ts`: mỗi pane gửi mục khẩn nhất của nó.
  - Ghi chú agent chỉ gửi reason `note`, không gửi chữ. Không thêm push kind.
- `apps/shell`: không phải đổi gì, `Board` chuyển nguyên JSON. Có thêm test `remote/ws_tests.rs::the_inbox_field_and_flag_reach_the_phone_unchanged`.
- Doc: `docs/remote-integration.md` §4.3 thêm field `inbox`, cờ `inbox: true` và luật fallback.

### PWA (apps/pwa, subagent)
- `lib/inboxModel.ts`: `inboxRows`, `inboxSections`, `inboxCount`, dùng `sortInbox` / `INBOX_GROUPS` của shared. Có cờ thì đọc `pane.inbox`; không có cờ thì fallback `attention`.
- `lib/workspaceModel.ts`: `WsPane.inbox`.
- `components/InboxSheet.vue`: mở từ nút `inbox-open` trên Monitor.
- Badge `App.vue` `tab-badge` và `IpadRail.vue` `rail-badge` bằng `inboxCount`.
- i18n `en.ts` / `vi.ts`.
- E2E `e2e/lib/inbox-steps.mjs`, peer có lệnh `inbox` / `inboxold` / `inboxoff`.
- Sửa thêm ngoài phạm vi: `e2e/lib/ctx.mjs` `shoot` ghi file có retry (Windows lỗi "UNKNOWN open"), `ui-steps.mjs` đổi kỳ vọng badge.
- Chưa có Inbox sheet trong Workspace view.

### Android (apps/mobile, subagent)
- `lib/ui/model/inbox_model.dart` (mới): `inboxGroups`, `inboxReasons`, `groupOf`, `InboxInfo.parse`, `buildInboxRows`, `groupInboxRows`.
- `workspace_model.dart`: `PaneInfo.inbox`, `inboxMode`, `inboxRows`.
- `lib/ui/workspace/inbox_page.dart`: có header nhóm, chạm dòng thì `openPane`.
- `app_en.arb` / `app_vi.arb`: thêm `inboxGroup*` / `inboxReason*`, sinh lại bằng `flutter gen-l10n`.

## Test
- `npm test -w @ccm/web`: 317 file, 2741 test pass. Riêng nhóm theo tiêu chí PLAN (`inbox`, `menuQueue`, `components/inbox`, `shortcuts`, `remoteSummary`, `rail`, `palette`, `useMenuToast`, `workspaceKeys`): 118 pass.
- `npm run typecheck`: web, pwa, shared, collector, desktop đều 0 lỗi.
- `cargo test -j 4 the_inbox_field` (apps/shell): 1 pass.
- E2E desktop trên Tauri (cổng 4511 / 9511, web build riêng ở scratch qua `CCM_WEB_DIST`):

  | File | Kết quả |
  |---|---|
  | `v100-menu-queue.cjs` | 15/15 PASS (selector `[data-inbox-chip]`, `[data-inbox-row][data-reason=menu]`) |
  | `v69-ide.cjs` | 22 PASS, 0 FAIL |
  | `rail.cjs` | 19 PASS, 0 FAIL |
  | `run-pane.cjs` | 64 PASS, 0 FAIL |
  | `v104-inbox.cjs` (mới) | 14 PASS, chạy 2 lần đều 0 FAIL |

  `v104-inbox.cjs` đã thêm vào `run.cjs` và `scripts/qa-fast.mjs` E2E_MAP.
- E2E PWA: WebKit 52/52, Chromium 52/52 (chạy riêng từng browser). Có 3 step Inbox: badge, thứ tự nhóm, chạm mở session và không gửi phím, fallback desktop cũ, rỗng.
- Android: `flutter test --concurrency=8` +317 pass, `node apps/mobile/scripts/check.mjs` OK (file Dart ≤ 400 dòng), `flutter analyze` sạch.
- Ảnh `docs/qa/v100/WP-0d/*` bị E2E ghi đè đã `git checkout`.

## Ảnh (`docs/qa/v104/WP-104e/`)
- `desktop-inbox-light.png`, `desktop-inbox-dark.png`: mở bằng Ctrl+Shift+A, neo ở chip.
- `desktop-inbox-rail.png`: mở từ nút rail foot, panel lật lên.
- `pwa-inbox-light.png`, `pwa-inbox-dark.png`.

## QA tay Windows (đã làm qua E2E, cần main nhìn lại)
- Thanh tab chỉ còn 1 chip "Inbox N". Số trên chip bằng badge rail foot và bằng số dòng trong panel.
- Ctrl+Alt+Q đi lần lượt menu, quyền, lỗi theo thứ tự. Ctrl+Shift+A mở panel.
- Toast OS trả lời menu vẫn hiện khi Orrery không focus. Bước này chưa kiểm bằng tay vì E2E chạy `CCM_TOAST=0`.

## Còn treo
- Đổi nghĩa Ctrl+Alt+Q cần 1 dòng "What's new". Con chưa sửa `lib/changelog.ts` vì WP-104i đang làm khối 4.0.8. Dòng đề xuất:
  - en: `Inbox: one chip for menus, permissions, questions, same-file edits, errors, notes and done; Ctrl+Shift+A opens it, Ctrl+Alt+Q jumps to the next item`
  - vi: `Inbox gộp: 1 chip cho menu, xin quyền, câu hỏi, đụng file, lỗi, ghi chú và việc xong; Ctrl+Shift+A mở, Ctrl+Alt+Q tới mục kế`
- Clash chỉ gắn vào pane của người sửa sau cùng (remote 1 lý do / pane). Pane kia vẫn có chip đụng file của WP-104d.
- Key i18n cũ không còn dùng (`workspace.attention.chip`, `workspace.rail.needsList.*`, `menuQueue.chip*`, `inbox.reason.*`, `inbox.keys`) để lại cho WP-104f dọn.
- Mobile còn key `inboxWaiting` / `inboxDone` không còn dùng.

## Test macOS
- [ ] ⇧⌘A mở / đóng Inbox ở Workspace và Monitor. ⌃⌥Q nhảy mục kế, panel không mở.
- [ ] Chip `Inbox N` có kbd ⌃⌥Q (UI next). Màu: vàng khi có menu / quyền / hỏi, đỏ khi chỉ có lỗi, cam khi chỉ có đụng file, xám khi chỉ có Xong / Ghi chú.
- [ ] Mở Inbox từ nút rail foot thì panel nằm trọn trong cửa sổ.
- [ ] Phím S hoãn 15 phút có toast Hoàn tác. E đánh dấu đã xem. Enter mở pane.
- [ ] Toast trả lời menu (v100) vẫn hiện khi app ở nền (dock badge / alert theo `CCM_TOAST_BACKEND`).

## Kiểm A33 (main làm trên máy thật)
1. Dùng desktop bản mới (snapshot có `inbox: true`). Tạo đủ tình huống: menu, xin quyền, câu hỏi, 2 agent cùng sửa 1 file (bật switch đụng file), lỗi, `notify_owner`, xong chưa xem.
2. Mở Inbox trên app. Header nhóm phải đúng thứ tự: Chờ trả lời menu, Xin quyền, Câu hỏi, Đụng file, Lỗi, Ghi chú của agent, Xong chưa xem. Nhóm rỗng không hiện. Mỗi pane 1 dòng.
3. Dòng đụng file hiện "Cùng sửa {file} với {peer}". Dòng có tuổi, trong nhóm cũ nhất ở trên.
4. Chạm 1 dòng thì mở đúng pane. Không có phím nào vào terminal. Không có nút trả lời menu trong Inbox.
5. Đổi vi / en, chữ khớp bảng.
6. Badge Inbox ở Workspace bằng số dòng.
7. Desktop cũ (không có `inbox`): Inbox vẫn hiện theo `attention`, không crash.
8. Hoãn (S) hoặc đánh dấu (E) một mục trên desktop thì dòng đó biến khỏi điện thoại sau 1 lần cập nhật.
