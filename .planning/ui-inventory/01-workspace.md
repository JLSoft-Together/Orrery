# 01 - Workspace UI inventory (hiện trạng, phục vụ mockup v102 W2)

Phạm vi: `apps/web/src/components/workspace/**`, `lib/rail/*`, `stores/workspace.ts`, `lib/splitLayout.ts`, `composables/usePaneInfo.ts`. Đường dẫn dưới đây tính từ `apps/web/src/` trừ khi ghi khác. "chưa rõ" = chưa xác minh trong code.

Lưu ý lệch với PLAN-v102: rail mặc định hiện là **240px** (không phải 264; 264 là `SIDE_NARROW`), Explorer mặc định **280px** (plan ghi 270), cột avatar thu gọn **52px** (plan ghi 56).

## 1. Region map

```
AppHeader (cao = --ccm-header-h, mặc định 52px, đo động, cộng cả WorkspaceFooter)       App.vue:146
+----------------------------------------------------------------------------------------------+
| <main> px-3 (sm:px-4) pt-2 pb-3, gap-3 giữa các cột                  WorkspaceView.vue:1680  |
| +--------+ +--------------------------------------------------------+ +-----------------+  |
| | RAIL   | | STAGE (flex-1, gap --ccm-gap-shell = 8px)              | | EXPLORER aside  |  |
| | 240px  | | +----------------------------------------------------+ | | 280px (200-560) |  |
| | (200-  | | | Tab row h-7 = 28px                                 | | | ccm-panel p-1.5 |  |
| |  560)  | | | [rail?][Project btn][tabs...][+N][+][paneBar]      | | | rounded-card    |  |
| | hoặc   | | |  [chip Cần bạn N][chip Chờ trả lời N][|][Explorer] | | | seg tabs h-6    |  |
| | 52px   | | +----------------------------------------------------+ | | Files|Changes|  |
| | cột    | | | CANVAS (relative, mọi pane absolute theo box)      | | | Search|Checks|  |
| | avatar | | |  pane head h-7 = 28px (gap-1.5 = 6px dưới head)    | | | TODO            |  |
| |        | | |  well rounded-card p-2, bg-well, shadow-card       | | | tree/list ...   |  |
| |        | | |  khe giữa pane: 8px (HALF 4px mỗi bên)             | | +-----------------+  |
| |        | | |  divider hit-area, line 2px khi hover              | |                      |
| |        | | |  GroupFrames: head 30px, pad 4px                   | |                      |
| |        | | |  MissionControl phủ canvas (absolute inset-0 z-30) | |                      |
| |        | | +----------------------------------------------------+ |                      |
| +--------+ +--------------------------------------------------------+ +-----------------+  |
+----------------------------------------------------------------------------------------------+
WorkspaceFooter (App.vue:203): ccm-footer, chỉ hiện khi showFooter, nằm dưới cùng
```

Số px thật:
- Rail: `SIDE_DEFAULT 240`, `SIDE_MIN 200`, `SIDE_MAX 560` (`lib/sideWidth.ts:4-7`); kéo bằng `SideResizer` (`SideResizer.vue`), double click reset. Cột avatar `RAIL_COLUMN_W = 52` (`view/useSidePanels.ts:11`). `SIDE_NARROW = 264`: dưới mức này run badge ở header project co thành icon (`useSidePanels.ts:12`). Ẩn hoàn toàn dưới breakpoint `md` (`hidden md:flex`).
- Explorer: `EXPLORER_DEFAULT 280`, min 200, max 560; `STAGE_MIN = 480` giữ chỗ cho stage (`lib/sideWidth.ts:8-12`). Ẩn dưới `md`. Lưu `ccm.workspace.explorerWidth`, `ccm.workspace.sideWidth`.
- Tab row: tab `h-7` (28px), `max-w-[260px]` nút tab, nút `ccm-btn-plain` min 28x28.
- Pane head term: `h-7` (28px), icon mark 18x18, chip `h-5` (20px), font `text-xs` / chip `text-2xs`.
- Viewer toolbar (File/Changes/Run/Log/Browser): `h-9` (36px), viền dưới `border-line`, nền `ccm-material`.
- Khe pane: `GAP = 8` (`view/geometry.ts:3`), `--ccm-gap-shell: 8px` (`style.css:287`). `MIN_PANE = 220x140`, `MIN_RATIO 0.15`, `MAX_RATIO 0.85` (`lib/splitLayout.ts:25-61`).
- Radius: `--radius-card 12px`, `--radius-control 8px`, `--radius-inner 6px` (`style.css:95-97`).
- Mission Control: header `h-10`, card head `h-9`, tile head `h-5`, `LAYOUT = {head 36, gap 12, sectionHead 34, sectionGap 24, min 280, max 720}`, aspect 1.1-2.4 (`MissionControl.vue:59`, `mission/missionLayout.ts:22`).

## 2. Region / component

### 2.1 Rail (`WorkspaceSidebar.vue` 1028 dòng, `rail/*`, `lib/rail/*`)
Cây: cluster (project) > checkout (worktree/folder, chỉ hiện khi có >1 hoặc worktree, `showsCheckouts` `lib/rail/railModel.ts:217`) > group (tab nhiều pane) > row (pane). Mô hình ở `lib/rail/railModel.ts` (`RailCluster`, `RailCheckout`, `RailGroup`, `RailRow`, `RailState`).

Khung (`WorkspaceSidebar.vue` template): `nav.ccm-panel rounded-card p-1.5`, `aria-label` = "Dự án" (`workspace.rail.title`). Trên cùng:
- Ô lọc `ccm-field` placeholder `railV79.filter`, nút xoá lọc, nút `+` Open Project (`workspace.rail.openProject` = "Mở Project", `size-8`), nút "thu gọn các project khác" (`railCollapse.others`, icon ChevronsDownUp, sáng accent khi `autoCollapse`).
- Danh sách cuộn `ccm-scroll`; nhãn nhóm `ccm-label` (`railV92.agents` "Đang có agent", `railV92.opened` "Đang mở", `railV92.recent` "Gần đây"); nút "+N nữa / Ẩn" cho project cũ (`railV79.others`).
- Chân: thanh chọn nhiều (`railSelect.count`), `status` text, `p.sr-only` live region.

**Header project** (`rail/RailClusterView.vue`): `min-h-9` (36px), nền `bg-selection` khi active. Phần tử: chevron (ẩn khi solo/opener), monogram 22x22 (`ccm-mono`, 8 tone, chữ cái đầu, mờ khi không live), tên (`text-xs`, đậm khi active), nếu **solo** (1 pane): StatusIcon + thời gian + từ trạng thái; nếu không: "ago" (`railV92.ago.*`: "vừa xong / N phút / N giờ / hôm qua / N ngày"). Dòng 2 (`text-[11px]`): `{n} chạy` xanh working, `{n} chờ` waiting (icon Hand), nhánh git. Bên phải: icon Mail khi có peer news, chip `BellRing N` (đỏ nếu có failed, vàng nếu chỉ asked), `RunBadge` (Building/Running/Run failed, compact khi narrow), nút `+` (hover mới hiện, `workspace.rail.newIn` "Tab mới trong {name}"), nút `...` menu project (hover), nút X "Bỏ khỏi danh sách" nếu không live.
Tooltip head: root path, nhánh, dòng disk size (`projectDisk.lines`, v101), gợi ý kéo.

**Row pane** (`rail/RailRow.vue`): lưới `grid-cols-[10px_minmax(0,1fr)_auto_24px]`, `min-h-6` (24px), indent `pl-9` (36px) hoặc `pl-12` khi là member của group.
- Cột 1: chấm trạng thái 6px (`bg-st-working/waiting/error/done/idle/stale`) hoặc spinner khi `starting`; badge peer 8px ở góc khi cần bạn.
- Cột 2 `first` (`text-xs`, đậm khi focused): tên từ `rowLines` (task hoặc agent + ordinal "Claude 2", `twin`).
- Cột 3 (`text-[11px] tabular-nums`, `max-w-[9rem]`): **chỉ hiện dòng second khi cần bạn hoặc đang chạy tool**: attention thì hiện từ trạng thái ("Cần bạn" hoặc `waitDisplay` text), working thì hiện **tên tool đang chạy**; sau đó `· {time}` (`workspace.activity.tool` = `{tool} · {time}`), `+ins −del` diff (`text-diff-add/del`), icon Network khi đang điều phối, icon Telegram, glyph peer.
- Cột 4: nút X đóng (hover mới hiện, 24x24).
- Trạng thái trực quan: `ccm-rail-bar` (thanh chạy khi busy), vạch accent 2px bên trái khi focused (`--ccm-pane-accent`), sweep peer, flash 1 lần khi chuyển sang asked/done/failed (`ccm-flash`), highlight khi đổi tên, `data-rail-selected` (chọn nhiều) nền `bg-accent-soft` + outline accent, drop target `data-drop-on`.
- Tooltip (`title`) gộp: first, second, thời gian, diff (`workspace.activity.diffTitle`), quan hệ peer, Telegram.
- Từ trạng thái (vi, `workspace.rail.state.*`): failed "Lỗi", asked "Cần bạn", ended "Đã kết thúc", working "Đang chạy", subagents "Chờ subagent", done "Đã xong", starting "Đang khởi động…", idle "Sẵn sàng", parked "Đang ngủ". `stateWord` thêm biến thể "đang chạy N subagent" (`workspace.subagentState.*`).

**Group head** (`rail/RailGroupHead.vue`): `min-h-7`, glyph bố cục tab (`TabLayoutGlyph`), chấm màu project (`ccm-group-dot`), tên group, dòng attention, badge trạng thái 12px, nút X; click = activate, double click = rename, folded/unfold. **Sub-header** project trong group: `h-5`, monogram nhỏ + số pane.
**Checkout row**: `h-6`, icon nhánh/folder, nhãn, tag "worktree"/"thư mục".

**Cột avatar thu gọn** (`rail/RailColumn.vue`): `ccm-panel rounded-card py-2`; nút mở project `size-9` viền đứt; mỗi project live một nút `size-9` chữ viết tắt (`avatarInitials`), `~` cho Home, badge `BellRing N` góc trên phải; click mở **flyout** 272px (`WorkspaceSidebar` với `only` + `flat`); chân có `ResourcesButton variant="rail"`. Không có vòng trạng thái hiện nay (plan W2 muốn thêm).

States rail: rỗng (không cluster: chỉ nút Open Project, menu rỗng `workspace.rail.menu.empty` = "Dự án"), lọc không khớp (`railV79.noMatch`), project không live (mờ, hiện ago, nút X), narrow (<264: RunBadge compact), thu gọn (cluster `collapsed` dùng `ccm-rail-fold`), đang kéo (`data-dragging:bg-fill`), chọn nhiều.

Interactions rail: click row = `workspace.activate(paneKey)`; click header = mở/đóng cụm, solo thì focus pane; kéo thả đổi thứ tự project (`useRailDrag.ts`, `lib/rail/railDrag.ts`), kéo row/tab giữa tab (`data-rail-drag`/`data-rail-drop`); roving tabindex + phím mũi tên (`useRail.ts`, `lib/rail/railSelect.ts`); chọn nhiều bằng Ctrl/Shift click; F2 đổi tên; Alt+↑/↓ chuyển project; ←/→ thu/mở.
Context menu:
- Project (`lib/rail/railMenu.ts:49-83`): Tab mới > (Claude Code, Codex, [DeepSeek], Shell, Trình duyệt, Mở file, Thay đổi, Git Log), Run, Scripts, Plan, Env check, i18n scan, Perm advisor, Build clean, Bố cục > (Lưu, Quản lý, từng layout), Hiện trong Explorer, Mission Control (Ctrl+Shift+O), Sao chép đường dẫn, Hiện trong File Explorer/Finder, Chuyển lên/xuống, Thu gọn/Mở rộng, Đóng dự án hoặc Bỏ khỏi danh sách.
- Row/pane (`railMenu.ts:116-146`): Chuyển tới, Chia sang phải/xuống dưới (con: preset + "dự án khác"), Đổi tên tab (F2), Chuyển vào tab khác > / Tách khỏi tab, Sao chép thư mục làm việc, Hiện trong Explorer, Xem phiên trong Monitor, Nguồn context, Liên kết Telegram, mục compact, Đóng, Đóng các pane khác trong tab.
- Menu "..." header project dùng `ProjectMenu.vue` (Scripts, Run, Lưu/Quản lý layout, Đóng project).

### 2.2 Tab strip (`WorkspaceView.vue:1707-1975`)
Thứ tự trái sang phải: nút toggle rail `PanelLeft` (md+); **nút project hiện tại** (`activeCluster`, `max-w-[24ch]`, icon FolderOpen + tên + chip `BellRing N` của project khác + Mail; click = Mission Control); `ccm-tab-rail` chứa dải tab cuộn ngang (`ccm-tabstrip`, fade 24px hai đầu, `data-fade-*`); nút `+N` overflow tab (`tabsV93.more` "+N", `moreMenu` "Tab khác", đổi màu waiting nếu tab ẩn cần bạn); nút `+` (`workspace.newLabel` "Terminal hoặc tab mới", mở `LaunchPicker` popover 380px, phím Ctrl+Shift+T / Cmd+T); slot `paneBar` (head của pane đơn được "mượn" lên thanh tab khi chỉ có 1 pane, `headMode` = `bar`, `lib/paneHead.ts:38`); chip "N cần bạn" (`workspace.attention.chip`, mở `AttentionPopover`); `MenuQueueChip` ("Chờ trả lời N"); vạch ngăn; nút toggle Explorer `PanelRight` (Ctrl+Shift+E).
Mỗi tab (`h-7`): vạch trái 3px màu tone project (`ccm-tab-edge`, 8 tone), pill nền chung trượt theo tab active (`ccm-tab-on`), chấm trạng thái 7px hoặc `PaneIcon` 13px, tên (`font-medium`, truncate), icon Mail (peer), icon Telegram, chấm `●` vàng dirty, "N pane" (`text-[11px]`, khi >1 pane), icon Maximize (zoom), chip "chờ" (`paneV92.wait`) hoặc icon xong, nút X (hiện khi hover hoặc tab active). Phân nhóm tab theo project: `ml-2` + vạch 1px (`data-tab-group-start`). Flash khi đổi trạng thái, highlight khi đổi tên.
Interactions: click, double click đổi tên (`TabRenameInput`, max 60 ký tự `TAB_NAME_MAX`), kéo đổi chỗ (caret accent 2px), kéo ra khỏi dải, wheel cuộn dải, context menu tab (`onTabContext`), double click vùng trống dải = tab mới (`onStripDblClick`), phím: Ctrl+Tab/Ctrl+Shift+[ ], Ctrl+Alt+1..9, Ctrl+Shift+PgUp/PgDn (`lib/shortcuts.ts:36-90`).
Empty state (`WorkspaceView.vue:1977`): không có tab thì không có tab row/rail; giữa màn: tiêu đề `workspace.empty.title` ("Chưa có terminal nào"), mô tả `workspace.empty.body`, `RestorePicker` (khôi phục workspace trước, `workspace.restore*`) và `LaunchPicker` trong `ccm-group`, `max-w-[400px]`.

### 2.3 Canvas và pane frame
Mọi pane render phẳng trong 1 canvas, đặt bằng box tỉ lệ (`boxStyle` `view/geometry.ts`), không remount khi split/đổi tab. Divider (`tabDividers`): `role=separator`, tabindex 0, phím mũi tên chỉnh ratio, kéo chuột, aria-valuemin 15 max 85. Overlay kéo pane: drop preview viền 2px accent + nền `accent-soft`, ghost nổi `h-7` (`data-drag-ghost`), overlay chặn webview khi kéo.
Frame pane: `ccm-pane-ring` (outline `--ccm-pane-ring-w` 2px, màu `--ccm-pane-accent` theo agent), `ccm-pane-active` khi focused + multi, `ccm-pane-idle` (opacity `--ccm-dim-inactive`) khi multi và không focus, lớp `data-pane-dim` (`bg-canvas/45`) khi dim.
Trạng thái pane: zoomed (chỉ 1 pane, chip `+N` ẩn, nút Minimize), dimmed, grouped (ẩn chip project vì frame đã nói), parked (xem 2.4), ended (well mờ 60%).

**GroupFrames** (`view/GroupFrames.vue`): khung gom pane theo project trong 1 tab: `ccm-group-frame` viền màu tone, head `h-[30px]` kéo được (`cursor-grab`): chip monogram + tên (đổi tên inline max 40 ký tự `GROUP_NAME_MAX`), "N pane", nút `+` session mới, nút thêm project bên cạnh, nút chuyển ra tab riêng, nút đóng nhóm. Folded: cột/hàng hẹp với monogram + tên (dọc nếu `vertical`) + số pane + `ChevronsLeftRight`; click mở lại. Menu nhóm: Đổi tên, Màu (8 tone: Cam, Tím, Xanh dương, Nâu, Xanh lá, Hồng, Hổ phách, Màu nhấn, hoặc Tự động), Thu gọn, Đóng. `GROUP_CHROME = {head 30, pad 4}`.

### 2.4 TermPane (`TermPane.vue` 1553 dòng)
Cấu trúc: `section.@container` (container query theo bề rộng pane) > `header` (`h-7`) > `ContextMeter` (nếu agent running) > hàng [well + `AttachmentsPanel`] > `TurnCard`.
**Header (terminal)**, trái sang phải, các `@max-[...]` là ngưỡng bề rộng pane:
1. Mark 18x18 (`PresetIcon` 12px, nền accent 16%) + tên preset (`workspace.preset.*`: "Claude Code", "Codex", "DeepSeek", "Shell"; ẩn chữ khi <=420px; ẩn cả hai khi `inBar`).
2. Chip project (`ownProject`, khi pane thuộc project khác và không grouped; ẩn <=360px).
3. Diff `+N −M` (`data-pane-diff`; ẩn <=480px).
4. Sau `ml-auto`: `SubagentOrbit` (chỉ Claude), `ContextGauge` w-24 (ẩn <=600px), `ContextTokens` (ẩn <=600px), nút nhánh git `h-5` mono (mở `WorktreeMenu`, tên nhánh max 16ch), chip gợi ý Compact (vàng, "Ngữ cảnh {pct}% · Compact…", ẩn <=300px, chữ ẩn <=460px), chip bước Compact (writing/..., `workspace.compact.step.*`), **chip trạng thái phiên** (`sessionChip`: StatusIcon + từ trạng thái chỉ hiện khi asked/failed/ended + `· {tool} · {time}` khi working, ẩn <=620px; badge peer khi cần bạn), hoặc `procChip` (parked "Đang ngủ" / "Đang khởi động…" / "Đã thoát (code)" / lỗi), chip peer (glyph + câu "đã gửi/đã nhận ... ", ẩn chữ <=720px) hoặc icon coordinating (Network), `OverlapChip` (2 agent cùng nhánh), `AgentStatusChip` (phase + progress bar từ `.orrery/status.json`, pill `rounded-full`), `LimitResumeChip` (Hourglass "chờ reset limit" / AlarmClock "đã hẹn", popover đặt thời gian + câu resume), `PaneMore` (menu "Thao tác khác"), chip `+N` pane ẩn (khi zoomed), nút zoom (khi multi), nút split (`Columns2`, mở `SplitMenu`), nút đóng.
5. Menu "Thao tác khác" (`moreEntries` `TermPane.vue:376`): Mở dev server `host`, Compact…, Đính kèm (n), Scripts, [Shell] Quét lỗi terminal, [agent] Hàng đợi prompt, [Claude] Công cụ session, Khởi động lại / Kết nối lại.
6. Head cho pane đơn (`inBar`): teleport lên tab row, ẩn mark và tên, ẩn gauge dưới `lg`.
7. Pane nhiều: header là grip kéo (`cursor-grab`, tooltip `workspace.drag.paneHint`), double click vùng trống = zoom (`isHeadBlank`), focused có `ccm-pane-head-on` (nền `--ccm-pane-head-tint`).

**ContextMeter** (`ContextMeter.vue`): vạch 3px dưới head, `bg-ink-faint` ok / `bg-warning` warn (>=80% `METER_WARN_AT`) / `bg-st-error` over; khi over hiện hàng `h-6` "Context {now} vượt limit {max}" + nút "Compact…". Ngưỡng từ `settings.contextWarnTokens`. Chỉ hiện với agent (không shell) đang running.
**Well**: `ccm-pane-ring ccm-sky-well bg-well shadow-card rounded-card`, xterm padding `p-2 pr-0.5` (`pl-4` khi có shell marks), mờ 60% khi ended, các lớp phủ z-11/12: **boot** (quỹ đạo CSS + "Đang khởi động {agent}… {s}s" + gợi ý sau `BOOT_SLOW_MS`), **parked** (icon Moon, "Đang ngủ" + nút Chạy/Tiếp tục), **dim** (opacity 45%), toast quét lỗi, `TermFind` (Ctrl+F, góc phải trên), `ContextMenu` đánh dấu shell, flash `ccm-flash` khi `pane.attention`, overlay thả file (`attach.drop`, viền đứt focus).
**Thanh nút menu click** (v100, `TermMenuBar.vue`, 43 dòng): dưới terminal trong well, `max-h-24` cuộn, `flex-wrap gap-1.5`, mỗi nút `ccm-btn h-6 text-xs`: số thứ tự (nếu `numbered`), nhãn đáp án, icon: ChevronRight = đang chọn (nền `ccm-btn-primary`), Check = đã tick (multi), TextCursorInput = "Other", CornerDownLeft = Submit (`clickMenu.submit`). Disabled khi `busy`. Hover dòng trong terminal: lớp `bg-accent/20` (`useTermMenuMouse.ts`, `data-term-menu-hover`). Tên `aria-label` = `clickMenu.label` "Đáp án menu".
**Hàng đợi hỏi** (`MenuQueueChip.vue`): chip `ccm-btn text-st-waiting` "Chờ trả lời N" trên tab row; popover 320px liệt kê pane đang chờ menu (title = câu hỏi hoặc loại `menuQueue.kind.*`: Câu hỏi, Câu hỏi chọn nhiều, Xin quyền, Duyệt plan, Lựa chọn, Codex xin duyệt; dòng 2 = project, tab; tuổi chờ). Phím Ctrl+Alt+Q nhảy pane kế (`wsAsked`). Toast OS có nút đáp án (`useMenuToast.ts`).
**TurnCard** (`paneV93.turn.*`): thẻ "Lượt vừa rồi" dưới well: danh sách file, Xem diff, Hoàn tác lượt (xác nhận), Ẩn.
**AttachmentsPanel**: cột phải trong pane, file/ảnh đính kèm. **TermFind**, **DropTerminal** (terminal thả xuống Ctrl+`).

### 2.5 Viewer panes (không dùng `TermPane`, bọc trong div có ring)
Tất cả có thanh trên `h-9` `ccm-material` + `data-pane-head`, đuôi gắn `ViewPaneControls` (grip khi multi, chip `+N`, zoom, split, X; `border-l pl-1`).
- File (`FilePane.vue`): editor CodeMirror + minimap/ruler + outline; toolbar: Sửa/Xong, Lưu, trạng thái lưu, chấm chưa lưu (`editor.unsaved`), layout diff, gutter diff, linksV80 badge, "Thao tác khác". Tab tiêu đề = tên file.
- Changes (`ChangesPane.vue`): danh sách file đổi + `DiffView`/`SplitDiff`, `CommitBox`, `ConflictView`, `HunkPopup`; tiêu đề "Thay đổi (N)".
- Log (`log/LogPane.vue`): `LogTable` + đồ thị (`LogGraphCell`), `LogRefsTree`, `LogFilterBar`, `LogCommitDetails`/`Files`; tiêu đề "Log: {repo}".
- Run (`RunPane.vue`): `RunBar` (chọn config, Run/Stop, Release check), hàng trạng thái `h-8` (Building/Running/Stopped/Lỗi + progress), `RunLog`; empty: "không chạy được" + danh sách project phát hiện.
- Browser (`BrowserPane.vue` + `BrowserToolbar.vue`): back/forward/stop/reload, ô địa chỉ `h-7`, `DeviceMenu` (responsive + xoay + zoom), `HandoffMenu`, `ConsolePopover`, webview Electron; tiêu đề = host.
- Preview XML / Flutter: `LayoutPreview.vue`, `FlutterPreviewPanel.vue`, `DrawablePreview.vue`, `LayoutBoxes.vue` (là panel trong FilePane, không phải pane kind riêng; chưa rõ chỗ gắn chính xác).
Trạng thái chung: loading (spinner `ccm-spin`), lỗi (hàng `bg-st-error-soft`), dim (`data-pane-dim` overlay `bg-canvas/45`).

### 2.6 Explorer aside (`ExplorerPanel.vue` + `ExplorerTree/Changes/Search/Todo/Checks.vue`)
`aside` rộng `explorerWidth`, `ccm-panel rounded-card p-1.5`, `SideResizer edge="left"`. Trong: thanh seg `ccm-seg` `h-6` 5 tab icon + chữ (ẩn chữ <=360px): Files "File", Changes "Thay đổi" (kèm `+N −M`, hoặc chấm 6px khi <440px), Search "Tìm", Checks "Kiểm tra", TODO "TODO"; vô hiệu hoá Changes/Checks khi không phải repo. Dưới: hàng trạng thái (`explorer.status`, nền `st-error-soft` nếu fail, nút ẩn) rồi view. Tree: dấu git trên file (`changes.marks`), chọn so sánh (compare), icon file Atom Material (v59), menu file (tạo, đổi tên, xoá vào thùng rác có `DiscardConfirm`, so sánh 2 file, mở split). Empty: `workspace.explorer.noRoot` "Mở terminal trong một thư mục dự án để duyệt file." / `unavailable` "Explorer cần ứng dụng desktop.". Dialog nội bộ: `CreateEntryDialog`, `PushDialog`, `TextCompareDialog`. Mở bằng Ctrl+Shift+E, ẩn trong focus mode (Ctrl+.). Mở/đóng đang chiếm 1 cột cố định (không phải drawer) và co stage lại.

### 2.7 Mission Control (`MissionControl.vue` 1070 dòng, `mission/*`, `lib/rail/mission*.ts`)
Phủ canvas (`absolute inset-0 z-30 bg-canvas rounded-card`, modal), mở Ctrl+Shift+O hoặc nút project ở tab row. Header `h-10`: tiêu đề "Mission Control", gợi ý phím, ô lọc `w-56`, nút đóng. Thân cuộn: mỗi project một section (head `h-6`: FolderOpen/Folder, tên, "N tab", chip cần bạn), lưới card (`gap-3`). `MissionCard`: head `h-9` (phím số 1-9 `kbd`, StatusIcon 14px, tiêu đề, chip số pane, nút X hover), thân `bg-fill` cao `bodyH` chứa các **tile** pane (mỗi tile head `h-5`: icon + tên + cảnh báo overlap + dòng peer, thân là snapshot text tối đa `SNAP_ROWS = 48` hàng của terminal; có `ask` card khi đang hỏi: `MissionCard.ask.test.ts`). Card hiện tại `ring-2 ring-focus`. Tương tác: click = vào tab (`enter`), click tile = vào pane, kéo card/tile đổi thứ tự (`useMissionDrag.ts`), double click tên = đổi tên, context menu (`lib/rail/missionMenu.ts`), phím 1..9 và mũi tên (roving), Esc đóng, lọc theo tên.

### 2.8 Chip và popover khác
- `AttentionPopover.vue`: danh sách "Cần bạn" từ `lib/rail/attentionList.ts` (`buildAttentionList`: failed trước asked, cũ nhất trước; entry = `paneKey, tabKey, state, first, agent, place, branch, run, since`; footer đếm `working/done/idle/ended`: "{n} đang chạy" ...). Nút trigger `workspace.attention.chip` "N cần bạn" (Ctrl+Shift+A). Rỗng: "Hiện không có gì cần bạn". Plan v102 dùng lại làm thẻ "Cần bạn" đầu rail.
- `AgentStatusChip.vue` (47 dòng), `LimitResumeChip.vue` (132), `ContextMeter.vue` (41), `SubagentOrbit.vue` (chấm quỹ đạo subagent: "N subagent"), `OverlapChip.vue`, `RunBadge.vue`, `PromptQueueButton.vue`.
- `DevServerChip.vue` (33 dòng): **không còn được dùng ở đâu** (rg không thấy import); link dev server nay nằm trong menu "Thao tác khác".
- `WorkspaceFooter.vue` (176 dòng, mount ở `App.vue:203`): `{n} chạy` (xanh), `{n} chờ` (vàng), `Port {list}`; phải: nhánh, context, `Hôm nay {n} token`, nút khác. Chưa đọc hết nút cuối (chưa rõ).
- Toast, dialog, popover khác dùng class chung `ccm-menu`, `ccm-sheet`, `ccm-btn`, `ccm-btn-plain`, `ccm-panel`, `ccm-field`, `ccm-seg`, `ccm-label` (`style.css`).

### 2.9 Dialog khởi chạy từ Workspace (chỉ tên + 1 dòng)
`AccessPrompt` xin quyền truy cập; `AgentChoiceDialog` chọn agent nhận nội dung; `ApkSizeDialog` phân tích dung lượng APK; `CloseConfirmDialog` xác nhận đóng pane/tab đang chạy hoặc chưa lưu; `ContextSourcesDialog` nguồn context của session; `CreateEntryDialog` tạo file/thư mục; `DiscardConfirm` xác nhận xoá/bỏ thay đổi; `EnvCheckDialog` kiểm tra môi trường dự án; `I18nScanDialog` quét chuỗi i18n; `LayoutsDialog` lưu/mở bố cục tab; `LibUpdatesDialog` cập nhật thư viện; `LocalHistoryDialog` lịch sử cục bộ file; `PermAdvisorDialog` gợi ý quyền Claude; `PlanBoard` bảng plan; `NotesPanel` ghi chú dự án; `ProjectSwitcher` (Ctrl+Alt+P) chuyển project nhanh; `PromptPicker` (Ctrl+Shift+P) thư viện prompt; `QuickOpen` (Ctrl+P) mở file nhanh; `ReleaseCheckDialog` soát trước release; `RestorePicker` khôi phục workspace; `RunCloseDialog` dừng run rồi đóng; `RunConfigsDialog`, `RunToolchainDialog` cấu hình run/toolchain; `ScriptsDialog` chạy script project; `SessionToolsSheet` công cụ session Claude; `StringsDialog` bảng chuỗi; `TelegramLinkDialog` liên kết topic Telegram; `TextCompareDialog` so sánh text; `NavPicker`, `OutlinePicker` điều hướng/outline code; `LaunchPicker` popover tạo terminal; `ShortcutTip` gợi ý phím; `CheckpointsPanel`, `HttpPanel`, `JsonTree` (viewer phụ trong file pane).

## 3. Loại pane và nội dung header

| Loại (`Pane.kind` / preset) | Header | Màu (`--ccm-pane-accent`) | Ghi chú |
|---|---|---|---|
| term claude (`kind term`, preset `claude`) | mark + "Claude Code", project chip, diff, SubagentOrbit, ContextGauge, tokens, nhánh, compact, trạng thái, peer, overlap, AgentStatus, LimitResume, more, zoom, split, X | `--ccm-agent-claude` (cam) | có meter, TurnCard, thanh menu click |
| term codex | như claude trừ SubagentOrbit và công cụ session | `--ccm-agent-codex` (tím) | model lấy từ rollout (`codex/rollout.ts:146`) |
| term deepseek | như claude, tắt mặc định (`CCM_DEEPSEEK_FEATURE=1`) | `--ccm-agent-deepseek` | |
| term shell | mark + "Shell", diff, nhánh, procChip, more (Quét lỗi, Scripts), split, X | `--ccm-agent-shell-mark` (xanh ngọc) | không meter; shell chạy `claude` thì `presetOf` đổi theo agent (`stores/workspace.ts` `agent`) |
| file | toolbar 36px: tên file, Sửa/Lưu, dirty, layout, gutter diff, more | tone `file` | tiêu đề `viewLabel` = tên file |
| changes | toolbar: root repo, "Thay đổi (N)", layout, options, commit | | |
| log | toolbar: "Log: repo", lọc, refs | | |
| run | CirclePlay + "Chạy" + RunBar + Flutter version/FVM | | tiêu đề `run.titleIn` |
| browser | back/forward/stop/reload, địa chỉ, device, handoff | tone `browser` | tiêu đề = host |
| xml/flutter preview | panel con trong File pane (chưa rõ vị trí mount) | | |

Nguồn: `PaneKind = 'term'|'browser'|'file'|'changes'|'log'|'run'` (`stores/workspace.ts:66`), `PaneState` running/starting/parked/exited/error/gone (`TermPane.vue:313`), nhãn `usePaneInfo.viewLabel` (`composables/usePaneInfo.ts:84`).

## 4. Mật độ thông tin

- Font: row/tab/header `text-xs` (12px), chip và nhãn phụ `text-2xs` (`--text-2xs: 0.75rem`, line-height 1rem, tức 12px; **trùng cỡ `text-xs`**), dòng phụ rail `text-[11px]`, dialog/empty `text-sm`. Số liệu dùng `tabular-nums`.
- Chiều cao hàng: head project 36, row pane 24, group head 28, checkout 24, sub-head 20, tab 28, pane head 28, viewer toolbar 36, seg Explorer 24, card Mission head 36.
- Cỡ icon: 10-15px (lucide), mark preset 12 trong ô 18.
- Điển hình: 1-6 project, 1-4 tab mỗi project, 1-4 pane mỗi tab (tối đa `AGENT_VIEWERS_MAX = 4` viewer agent; số pane không có trần cứng đã thấy), rail vài chục dòng.
- Truncate: tên row/tab/head `truncate`; tab `max-w-[260px]`; chip tool `max-w-[22ch]`; nhánh `max-w-[16ch]`; peer text `max-w-[17ch]`; AgentStatus phase `max-w-[16ch]`; nút project strip `max-w-[24ch]`; dòng second rail `max-w-[9rem]`; tiêu đề tab tối đa 60 ký tự lưu, nhóm 40. Tooltip `title` luôn giữ chuỗi đầy đủ.
- Responsive trong pane: container query `@max-[300|360|420|440|460|480|520|600|620|720px]` ẩn dần chip (xem 2.4). Rail dưới `md` ẩn; dưới 264px compact RunBadge.
- Animation: `ccm-fade`, `ccm-list`, `ccm-tab`, `ccm-rail-word`, `ccm-flash`, `ccm-peer-sweep`, view transition theo pane (`view-transition-name: ccm-pane-<key>`).

## 5. Phải giữ khi redesign / điểm đau

**Must keep**
1. Mọi trạng thái = icon + chữ + màu (không chỉ màu). Từ trạng thái giữ đúng 9 giá trị `RailState`.
2. Cần bạn: failed trước asked, cũ nhất trước (`buildAttentionList`); nút nhảy tiếp (Ctrl+Shift+A, Ctrl+Alt+Q); số đếm ở project, tab, rail, strip; flash khi chuyển trạng thái; toast OS có nút đáp án.
3. Menu click v100: thanh nút dưới pane (numbered, selected, checked, Other, Submit, disabled khi busy), hover dòng, hàng đợi "Chờ trả lời N".
4. Pane không remount khi split/đổi tab/zoom (terminal giữ state); split/dock/swap bằng kéo, divider có phím, ratio 15-85%, `MIN_PANE`.
5. Mọi cấu trúc cây: project > checkout > group > pane; group frame (gom nhiều project trong 1 tab), fold/unfold, 8 tone.
6. Kéo thả (project, tab, pane, group, row) + roving tabindex + chọn nhiều + F2 + phím mũi tên; toàn bộ shortcut `lib/shortcuts.ts` (wsNew... wsGroupMove, wsDropTerm).
7. Zoom + chip `+N` + tab glyph zoom; undo đóng (`undo.*`, Ctrl+Shift+T); dim pane không focus; focus mode (Ctrl+.).
8. Peer/messaging: glyph, câu "đã gửi/nhận", sweep, `goToPeer`; coordinating (Network); Telegram glyph; OverlapChip.
9. Chip: AgentStatus (progress `.orrery/status.json`), LimitResume (hẹn tiếp tục), compact gợi ý và bước, context gauge/tokens/meter + cảnh báo over, subagent orbit, diff +/−, nhánh + WorktreeMenu, RunBadge.
10. Boot, parked (Ngủ/Tiếp tục), ended (mờ), error + Kết nối lại/Khởi động lại.
11. Explorer: 5 view, status line, git marks, trash an toàn, resize, ghim cột (plan: drawer, ghim lại cột).
12. Mission Control: phím 1-9, lọc, kéo, tile snapshot, ask card.
13. i18n en+vi mọi chuỗi; màu qua token; không cuộn ngang trang; reduce motion.

**Pain points / không nhất quán thấy trong code**
- Cỡ chữ: `text-2xs` = 12px bằng `text-xs`, nên chip và nhãn không nhỏ hơn thân; rail dùng `text-[11px]` lẻ. Lưới cỡ chữ không có bậc thật.
- Hard-code px rải rác: `h-5/h-6/h-7/h-9`, `size-[18px]`, `size-[13px]`, `size-[7px]`, `left-[39px]`, `pl-9/pl-12`, `max-w-[Nch]`, `w-[min(380px,...)]`, `w-24` gauge, LAYOUT mission; không qua token.
- Ba kiểu chip: `rounded-inner` (chip trạng thái, LimitResume, compact), `rounded-full` (AgentStatus, DevServerChip), `ccm-btn` (chip Cần bạn, Chờ trả lời), cao `h-5` vs `h-6` vs 28.
- Header pane term và toolbar viewer khác hẳn nhau: 28px trong suốt trên canvas vs 36px `ccm-material` có border-b; viewer có `ViewPaneControls` riêng lặp lại zoom/split/X (code trùng với TermPane header, `WorkspaceView.vue:2059-2158` lặp 5 lần).
- Head nhồi 8-12 phần tử, dựa hoàn toàn vào `@max-[...]` ẩn dần; nhiều ngưỡng khác nhau (300/360/420/440/460/480/520/600/620/720).
- Màu ad-hoc: `bg-canvas/45`, `bg-accent/20`, `color-mix(... 16%)`, `opacity-45/60` cho dim (plan COLOR-SYSTEM nói "ink ladder not opacity").
- Rail chỉ cho row thấy tool khi working hoặc cần bạn (`showSecond`), còn lại 1 dòng 24px: không có model, context, subagent count; header project solo gánh trạng thái.
- Cột avatar thu gọn 52px không có vòng trạng thái, chỉ chữ cái + badge cần bạn.
- Explorer là cột cố định chiếm bề rộng stage (không drawer), mở là stage mất 280px.
- Rail + Explorer + tab row + header app đều có nút toggle riêng; tab row quá tải (project btn, tabs, +N, +, paneBar, 2 chip, toggle).
- `DevServerChip.vue` mồ côi (không import).
- `style.css` có cả lớp sao, meteor, `ccm-sky-well`: dependency trực tiếp của pane (well), cần quyết khi chuyển sang nền sương.
- Plan nói rail 264 và Explorer 270 nhưng code 240 và 280 (lệch số liệu mockup).

## 6. Dữ liệu có sẵn nhưng chưa dùng cho rail (W2 live rail)

Plan: dòng 2 rail hiện **tên model** cạnh tool, kèm context gauge, thời gian.

| Dữ liệu | Nguồn | Hiện ở đâu hôm nay |
|---|---|---|
| **Model id** (Claude: `claude-opus-5-5`, `claude-sonnet-5-5`; Codex: model từ rollout) | `Agent.model` (`packages/shared/src/types.ts:94`) qua `useAgentsStore().mainOf(terminalId)` (`stores/agents.ts:52`); Codex nạp ở `apps/collector/src/codex/rollout.ts:146` và `monitor.ts:655` | `MainNode.vue:56`, `SessionCard.vue:233`, `SessionSummary.vue:49` (Monitor), **không có trong Workspace** |
| Nhãn model đẹp ("Opus 5.5", "· 1M") | `modelLabel(id, contextWindow, extras.modelNames)` (`lib/model.ts:8`), `extras.modelNames` từ snapshot (`stores/extras.ts:11`, `types.ts:233`) | Monitor/History/Recap |
| Effort ("low..max") | `Agent.effort` + `effortLabel(effort, t)` (`lib/model.ts:23`) | Monitor |
| Context % và token | `Agent.contextPct`, `contextTokens`, `contextWindow` -> `contextPctOf` (`lib/provider.ts:12`); `TermPane.vue:275` | chỉ head pane (ContextGauge) và meter; **không có ở rail** |
| Tool đang chạy + tóm tắt | `Agent.currentTool`, `currentToolSummary`, `currentToolStartedAt` (`types.ts:100-102`); `usePaneInfo.activityOf` chỉ trả `tool` (`usePaneInfo.ts:151`), **bỏ `currentToolSummary`** | rail dùng `tool` (khi working) |
| Thời gian đã chạy / trong trạng thái | `TerminalSession.statusSince` (`types.ts:48`), `startedAt`, `lastActivityAt` | rail hiện `since` khi working |
| Số subagent đang chạy | `runningSubagents(agents.byTerminal[id])` -> `usePaneInfo.subagentsOf` (`lib/subagents.ts`) | chỉ từ trạng thái "Chờ subagent" |
| Chi phí phiên | `TerminalSession.costUsd` (`types.ts:55`, cần bridge status line opt-in) | Monitor |
| Compaction đang chạy / số lần | `compactingSince`, `compactTrigger`, `compactions` (`types.ts:53-58`) | chỉ head pane (bước compact) |
| Lượt bị từ chối bởi classifier | `deniedCount`, `deniedTools` (`types.ts:60-61`) | chưa rõ nơi dùng trong workspace |
| Chế độ quyền | `permissionMode` (`types.ts:51`) | chưa rõ trong workspace |
| Limit hit + giờ reset | `TerminalSession.limitHit` (`types.ts:72`) | chỉ `LimitResumeChip` ở head |
| Workflow phase + progress | `useAgentStatusStore().statusOf(paneKey)` (`.orrery/status.json`) | chỉ `AgentStatusChip` ở head |
| Kết quả gần nhất / tóm tắt lượt | `TurnCard` (`paneV93.turn`: danh sách file đổi từ `cwd`), `useBoardLastTurn.ts` | chỉ dưới pane; rail không dùng (chưa rõ dạng "last result" text) |
| Diff +/− | `TerminalSession.diff` (`GitDiffStat`) | rail đã có |
| Nhánh git | `session.gitBranch` hoặc `pane.branch` | header project |
| Câu hỏi đang chờ (menu) | `useMenuQueueStore().entries` (`question`, `menu.kind`, `since`) | chip hàng đợi |
| Chờ gì (waitingFor) | `TerminalSession.waitingFor` -> `waitDisplay` (`lib/wait.ts`) | đã vào từ trạng thái asked |
| Dung lượng đĩa project | `useDiskSizes.ts` (`projectDisk.lines`) | tooltip header project |

Cách lấy model cho 1 row rail: `pane.terminalId` -> `agents.mainOf(terminalId)?.model` -> `modelLabel(model, contextWindow, extras.modelNames)`. `usePaneInfo` hiện không export hàm này; cần thêm `modelOf(pane)` (dùng chung 100% logic theo plan). Pane không có session (`terminalId` rỗng, shell, mới khởi động) thì để trống như plan chốt. Shell không có model. DeepSeek: chưa rõ có `Agent.model` hay không (chưa kiểm collector deepseek).
