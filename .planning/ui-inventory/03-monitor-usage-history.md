# UI inventory 03: Monitor, Usage, History, Inbox

Nguồn: đọc code ngày 15/11/2026 (nhánh feat_clean_memory). Đường dẫn gốc `apps/web/src/`. Mục đích: mockup v102 (Sương kính) dùng đúng cấu trúc và nội dung thật. "chưa rõ" = không xác nhận được từ code.

## 0. Routing và khung chung

- `App.vue:190-196`: `CompactView` (khi `ui.compact`) > `MonitorView` (lazy) > `UsageView` > `HistoryView`, cùng 1 ô grid, crossfade `ccm-view`. `WorkspaceView` mount riêng, `v-show`.
- Inbox không phải view: popover `InboxPanel` mở từ nút trên `AppHeader.vue:178-211` (`inbox.open`).
- Monitor cao `h-[calc(100dvh-var(--ccm-header-h,4.5rem))]`, min 420px (`MonitorView.vue:75`). Usage và History là `<main>` cuộn theo trang, `max-w-[1560px]`, `px-4 sm:px-6`.
- `ui.narrow` = `max-width: 767px` (`stores/ui.ts:197`). Dưới ngưỡng này panel thành bottom sheet.
- Component KHÔNG còn mount ở đâu (chỉ còn file + test, `rg` không thấy import): `SessionList`, `WaitingQueue`, `ActivityFeed`, `ToolTimeline`, `MetricsStrip`, `SessionSummary`, `OrbitRings`, `SessionTabs`. `FavoritesList` + `JobsList` chỉ được `SessionList` (đã chết) dùng. Mockup không cần vẽ lại, nhưng `ui.sessionTab` vẫn tồn tại (xem 1.6), chưa rõ ai set nó.

## 1. Monitor (`components/MonitorView.vue`, 144 dòng)

### 1.1 Layout hiện tại
```
AppHeader (var --ccm-header-h, mặc định 72px)
+---------------------------------------------------------------------------------+
| [AwaySummary]  [PortConflictBar]  [WaitingStrip h28, chỉ mode Quỹ đạo]  px12 pt8 |
| ModeBar: (◔ 3 agent đang chạy, ✋ 2 chờ chủ nhân) [Bảng|Quỹ đạo]   [Nhóm theo: ..] [ProjectStrip]
|----------------------------------------------------------------------------------
| MODE BẢNG (SessionBoard, scroll, p12):                                            |
|  +Cần chủ nhân 2+ +Đang chạy 3--+ +Rảnh 1---+ +Xong hôm nay 4+   cột 1fr, cột rỗng = strip 40px dọc
|  | card 190px  | | card        | | card     | | timeline   |   gap 12, card gap 8
| MODE QUỸ ĐẠO (OrbitView):                                                         |
|  [OrbitStarMap flex-1, zoom/legend nổi] | [aside w320 border-l: OrbitDetail]      |
|  hoặc OrbitFocus: [rail w224] [stage SVG + toolbar] [feed w288]                   |
|-----------------------------------------------------------------------------------
| CommandPanel: thanh h32 "Lệnh đang chạy | 2 đang chạy | Lâu nhất ... | ` v"  (mở: max 40vh, bảng)
+---------------------------------------------------------------------------------+
```
Mode Quỹ đạo kèm `ui.sessionTab`: aside trái (SessionCard, rộng `ui.panelSize`, ghim hoặc nổi `inset-y-2 left-2`) + `AgentMap` (xem 1.6). Narrow: aside thành sheet `h-[min(70%,560px)]` bo trên (`MonitorView.vue:54-56`).

### 1.2 ModeBar (`monitor/ModeBar.vue`, 65 dòng)
- Tiêu đề h2: StatusIcon working 14px + "N agent đang chạy" + ", " + "N chờ chủ nhân" (màu warning + icon Hand) hoặc "không ai chờ chủ nhân" (muted). Nguồn `countOf(groups,'needs'|'running')` từ `useMonitorSessions` (terminals store). `ModeBar.vue:38-41`, i18n `boardV95.summary.*`.
- Tab segment `Bảng` (LayoutGrid) / `Quỹ đạo` (Orbit), h28, `role=tablist`, tooltip `Alt+1` / `Alt+2` (`monitorV88.modeHint`). Mũi tên trái/phải đổi mode (`ModeBar.vue:27-33`).
- Mode Bảng: `SegmentedControl` "Nhóm theo": Trạng thái / Project / Agent (`boardV95.groupBy.*`, `ui.monitorGroupBy`), đẩy về phải (`ms-auto`). Nhóm theo Trạng thái thì thêm `ProjectStrip` (AppSelect size sm, icon FolderTree, nhãn "Tất cả dự án", hint = số session + "N đang chờ bạn" + dung lượng thư mục từ Disk index + "junk" nếu có). Chỉ hiện khi có >1 project.
- Phím toàn view (`composables/useMonitorKeys.ts`): `Alt+1`, `Alt+2` đổi mode, phím ` (backtick) bật/tắt CommandPanel. Bỏ qua khi đang gõ hoặc có dialog mở.

### 1.3 SessionBoard (`monitor/SessionBoard.vue`, 159 dòng)
- Hai bố cục: (a) nhóm theo Trạng thái: 4 cột cố định `MONITOR_COLUMNS = needs, running, idle, ended` (`lib/monitorRows.ts:80`), template `minmax(0,1fr)` hoặc 40px khi cột rỗng và chưa mở (`lib/boardV95.ts:35-41`); trong cột, card lưới `repeat(auto-fill,minmax(min(100%,320px),1fr))`. (b) nhóm theo Project hoặc Agent: lane `repeat(auto-fill,minmax(min(100%,340px),1fr))`, mỗi lane `rounded-card bg-raised p-2`, lane có việc cần chủ nhân thì nền `warning-soft` (`.ccm-board-needs`, `SessionBoard.vue:149-152`).
- Tiêu đề cột `.ccm-label`: "Cần chủ nhân" (+ Hand, màu warning) / "Đang chạy" / "Rảnh" / "Xong hôm nay" + số đếm. Cột rỗng: nút thu `ChevronsLeft` + chữ "Không có session". Cột rỗng khi thu: strip dọc `[writing-mode:vertical-rl]`, aria "Mở cột rỗng {label}".
- Lane agent: chấm màu `bg-agent-claude|codex|deepseek`; lane project: tên folder hoặc "Không có project".
- Data: `terminals.byId` (stores/terminals), `connection.hasData`. State: `empty` = đã có data mà 0 terminal, hiện hộp nét đứt "sessions.empty". Chưa có data/loading: chưa rõ ở Bảng (không có skeleton, chỉ rỗng).
- Item `content-visibility:auto`, `contain-intrinsic-size 190px` (nhiều item). `freezeBind` đóng băng thứ tự khi hover/focus để card không nhảy.
- Tương tác: Enter/Space trên card = `ui.focusTerminal`; chuột phải hoặc phím menu = `ContextMenu` (`useSessionMenu`, nhãn "Thao tác cho {title}"); toast nổi `fixed bottom-12` cho kết quả menu.

### 1.4 BoardCard (`monitor/BoardCard.vue`, 214 dòng): thẻ session
```
| |bar2px màu agent  [M 28x28 monogram tone project]  Tiêu đề session (14px bold)   (SubagentOrbit) |
| |                  [icon agent] project, Claude Code (12px muted)                                 |
| |  [chip dòng trạng thái: ⟳ "Bash · 12s" | ✎ "Đang sửa App.vue" | ✋ "Cần duyệt: Bash 3m 20s"]    |
| |  02:14   ⎇ feat/login                                       (mono 12px)                          |
| |  Context 128k, cảnh báo ở 500k   [meter h4 ▬▬▬▭▭]                                               |
| |  Lượt cuối sửa 3 file +120 −8                                                                    |
| |  [Mở trong Workspace]  [Xem diff]                                                                |
```
- Padding `py-2.5 ps-3.5 pe-3`, `rounded-card`, `shadow-card`; viền `border-warning` khi chờ, không viền khi thường; ring focus khi `ui.focusedTerminalId`. Quiet (rảnh/xong): bỏ shadow. Ended: chữ muted, monogram rỗng viền.
- Nguồn: `terminal` (status, statusSince, cwd, gitBranch, diff.files, waitingFor, provider), `agents.mainOf` (currentTool, currentToolStartedAt, contextTokens, currentToolSummary), `toolSpans.spansFor` (file đang sửa/đọc, `fileAction`), `useBoardLastTurn` (lượt cuối), `settings.contextWarnTokens` (mặc định 500_000, `lib/contextWarn.ts`), `workspace` (mở diff).
- Dòng trạng thái ưu tiên: chờ duyệt (`waitDisplay`: "Cần duyệt: {tool}" / "Cần duyệt plan" / "Đang hỏi bạn" + thời gian chờ) > đang chạy có tool (`toolRunning`: "{tool} · {time}") > nhãn status (Đang chạy, Rảnh, Xong, Đã đóng, Lỗi).
- Meter context 3 tone: ok (xám) / warn từ 80% ngưỡng (vàng) / over (đỏ). `role=meter`.
- Nút: "Mở trong Workspace" (`OpenPaneButton` primary compact, disabled + tooltip "Session này chưa có pane trong Workspace" nếu chưa có pane; chỉ khi `hasTerminals`), "Xem diff" (mở Changes của cwd rồi `setView('workspace')`). Ẩn cả hai khi quiet/ended.
- Monogram: `projectTone` 8 tone (agent-claude, codex, browser, file, success, peer, warning, accent), chữ cái đầu tên project.
- SubagentOrbit (`workspace/SubagentOrbit.vue`) hiện khi có subagent sống; `ORBIT_MAX_DOTS = 5` (`lib/paneV93.ts:4`).

### 1.5 BoardDone (`monitor/BoardDone.vue`, 63 dòng): cột "Xong hôm nay"
Timeline dọc: đường 1px + chấm tròn viền `success` 11px, giờ `HH:MM` mono (w40), tiêu đề + dòng "project, N file". Hiện `DONE_VISIBLE = 3`, nút "Xem thêm N" / "Thu gọn" (`lib/boardV95.ts:8`). Click = `ui.focusTerminal`. Sắp xếp mới nhất trước.

### 1.6 Mode Quỹ đạo (tab "Quỹ đạo")
`OrbitView.vue` (109 dòng) có 2 trạng thái:
1. **Tổng quan** = `OrbitStarMap` (508 dòng) + aside `OrbitDetail` w320 (hoặc sheet khi narrow).
   - StarMap (HTML + SVG trong "world" có pan/zoom, `lib/starMap`): mỗi project = 1 "hệ" (nhãn tên project + "+N session" nếu bị ẩn, hiện tối đa 3 mặt trời/hệ); mỗi session = mặt trời (nút tròn, StatusIcon 12px, to theo mức hoạt động, màu theo agent, viền `ring-ink` khi chọn, `star-needs` nhấp nháy khi chờ); subagent = hành tinh 10px trên vòng nét đứt, quay (`star-spin`, tắt khi `prefers-reduced-motion`, dừng khi ngoài viewport/tab ẩn), đặc = chạy, vòng rỗng = chờ, vuông = xong; dưới mặt trời: project, "Đang chờ chủ nhân trả lời", hoạt động ("Edit App.vue"), "3 subagent", nút Mở pane khi cần chủ nhân.
   - Liên kết peer giữa session: đường cong nét đứt `accent` 45% + pill "2 tin nhắn" (`peers` store).
   - Điều khiển nổi trái-dưới: Phóng to / Thu nhỏ / Vừa khung / Về hệ đang chọn (`ccm-material`). Legend nổi phải w256 thu gọn được ("Cách đọc bản đồ", 6 dòng chú giải). Hint "Phím mũi tên để dời bản đồ". Empty: "Chưa có session để hiện".
   - Tương tác: click chọn, double click hoặc Enter mở Focus, chuột phải menu, mũi tên điều hướng theo vòng (`lib/orbitRings.navigate`), kéo pan, wheel zoom.
   - OrbitDetail (122 dòng): tiêu đề + icon agent, status + thời gian, hộp cảnh báo "Cần duyệt: Bash" + lệnh mono + nút "Mở pane trả lời" (primary) khi chờ, nút "Xem hệ" (Enter), danh sách "Subagent" (loại + tool · thời gian hoặc chờ/xong), "Tool gần nhất" 5 dòng giờ + tên tool (lookback 30 phút, `connection.loadToolSpans`). Empty text: "Chọn một session để xem chi tiết", "Không có subagent", "Chưa có tool call".
   - Segmented Bản đồ / Văn phòng (Office) nổi góc phải trên, chỉ khi `settings.officeEnabled` (`OrbitView.vue:95`).
2. **Focus 1 hệ** = `OrbitFocus.vue` (245 dòng), Esc quay lại:
   - Cột trái w224 "Các hệ": mỗi session 1 nút (SVG 28px: vòng nét đứt + mặt trời tô theo nhóm + chấm accent nếu có subagent sống) + tên + status. Mũi tên lên/xuống đổi hệ.
   - Giữa: toolbar (Quay lại tổng quan, Hệ trước/kế, tiêu đề, status · thời gian, Mở pane) + stage SVG: vòng nét đứt đồng tâm, mặt trời tô theo nhóm (needs `warning`, running `accent`, idle `ink-faint`, ended `line-strong`), vệt cung 5px = tool call gần nhất (dài = chạy lâu; màu accent chạy/xong, `st-error` lỗi, `ink-faint` bị ngắt), hành tinh 16 (nhãn tên subagent + tool, cắt 18/24 ký tự). Hint "Vệt cung trên quỹ đạo là các tool call gần nhất...".
   - Phải w288 "Dòng tool" (lookback 60 phút): giờ mono `HH:MM:SS`, tên tool (+ " · tên subagent"), thời gian chạy (`formatMs`) hoặc "lỗi/bị ngắt". Empty "Không có tool call trong 1 giờ qua".
   - Narrow: rail ẩn, feed xuống dưới `max-h-[40%]`.

### 1.7 CommandPanel (`monitor/CommandPanel.vue`, 91 dòng)
Thanh đáy h32 `border-t bg-canvas`: icon Terminal, "Lệnh đang chạy", "N đang chạy" / "Không có lệnh nào đang chạy", "Lâu nhất: {command} · {time}" (mono, cắt), kbd `` ` ``, chevron. Mở: bảng max 40vh với cột Session (icon agent + tên, subagent thêm " · subagent") / Lệnh (mono; chờ quyền thì nền warning-soft + ShieldQuestion) / Tool (ẩn dưới md) / Đã chạy / Pane (nút Mở pane). Nguồn `runningCommands(agents.list, terminals.byId)` (`lib/monitorRows`), tick `now`. Không thời gian = "chưa rõ".

### 1.8 AgentMap (`components/AgentMap.vue`, 1581 dòng) + node
- Khi nào hiện: chỉ ở mode Quỹ đạo khi `ui.sessionTab` có giá trị (xem `MonitorView.vue:58,138`: `orbit = !sessionTab && mapView==='map'`) hoặc `mapView==='office'`. Chưa rõ UI nào set `sessionTab` hiện nay (SessionTabs không mount). Mockup coi là phụ.
- Vue Flow (`@vue-flow/core`), zoom 0.2 đến 2, nền chấm gap 18, `only-render-visible-elements`.
- `MainNode.vue` (342 dòng): thẻ 300x96 (min, resize 240..720 x 96..480), `rounded-card`, dòng 1 StatusIcon 16 + tiêu đề 15px bold + `ProviderBadge`; dòng 2 model · effort, peer chip, đếm bị chặn (ShieldX), `ContextRing`, `TokenCount`; dòng 3 tool/wait/compacting/peer line. Viền `st-waiting` + `ccm-attention` khi chờ.
- `SubPill.vue` (116 dòng): viên thuốc 240x30: StatusIcon 14, tên (max 60%), badge, chi tiết tool, thời gian; nút gập `ChevronDown` + "N ẩn".
- `DoneTray.vue` (61 dòng): viên nét đứt "N subagent xong", các chấm `st-done`/`st-stale` tối đa `MAX_DOTS`, mở/gập.
- `AgentDetails.vue` (208 dòng): popover 300px cạnh node (hoặc góc phải trên), eyebrow + tên + StatusBadge + ProviderBadge, `dl` (label/value: model, tokens, ...), mục "Tools" cho subagent (icon trạng thái + tool mono + kết quả + giờ · ms). Esc đóng.
- Chrome: nút "Session"/"Sessions" (phím S) mở lại panel trái, toolbar nổi (Zoom in/out, Fit `F`, Reset layout), ô tìm Ctrl+F (`map.find.*`, đếm "i/n"), capsule peer (Mail + text), menu chuột phải, toast. Empty: ảnh `agent-map-bg` 25% + `map.empty`.
- Phím: mũi tên điều hướng cây (`lib/mapNav`), Enter chọn/bỏ chọn, Shift+Enter hiện trong Workspace, Space gập/mở, Ctrl+C chép node, Escape bỏ chọn nhiều.

### 1.9 Dải trên cùng và các panel phụ
- `AwaySummary.vue` (103 dòng): thẻ `surface` + vạch accent trái 3px + nút X: "Trong lúc chủ nhân vắng {thời gian}", nhóm (chờ / xong / lỗi liên tục) mỗi nhóm là các nút `ccm-btn` tiêu đề + thời gian chờ, dòng facts "2 session mới, 1 subagent xong, ..." (nối bằng " · " trong code). Nguồn `away` store.
- `PortConflictBar.vue` (1.6K): chưa đọc kỹ, chưa rõ nội dung chữ.
- `WaitingStrip.vue` (44 dòng): nút h28 nền `st-waiting-soft`: Hand + "Cần chủ nhân · N" + "{title} đã chờ {waited}"; click mở panel session (`ui.panels.sessions.open`). Chỉ hiện mode Quỹ đạo, khi không focus 1 session, không phải overview orbit (`MonitorView.vue:81`). Nguồn `waitingSummary(terminals.live, extras.blockedJobs, snooze)`.
- `CompactView.vue` (220 dòng): thay cả Monitor khi `ui.compact` ("Chế độ gọn"): cột `max-w-3xl`, chữ to: mục "Cần chủ nhân (N)" (hàng 18px bold + thời gian chờ 20px, đỏ khi chờ lâu `LONG_WAIT_MS`, nút snooze 48px), mục "Đang chạy (N)", "Session khác (N)", empty `all-quiet`. Click = nhảy cửa sổ hoặc focus session. Hợp cửa sổ nhỏ cạnh màn làm việc.
- `SessionCard.vue` (470 dòng) + `SessionCardHeader` (149) + `SessionCardSignals` (154): thẻ chi tiết dùng trong aside khi có `sessionTab` (và trước đây trong SessionList). Nhiều signal: quiet (Hourglass), cache lạnh (Snowflake), compacting, bị chặn (ShieldX), sensitive (mở danh sách hit), loop, xung đột file (GitMerge), RAM/CPU, ghi chú (StickyNote, sửa tại chỗ, 140 ký tự), đổi tên (80 ký tự), Pin, Star, RunBadge, mode, branch, diff +/-, cwd rút gọn, compactions, cost, ContextRing, TokenCount. Mockup chỉ cần tham chiếu.
- `FavoritesList.vue` (452), `JobsList.vue` (177): không hiện ở Monitor hiện tại (dead).

### 1.10 Trạng thái chung của Monitor
- Disconnected có data: `main` opacity 90% (`MonitorView.vue:76`); không có chỉ báo riêng trong Monitor, `connection.state` hiện ở header ("Đang hiện trạng thái cuối, collector offline" = `connection.stale`).
- Empty: "Chưa có session" (`sessions.empty`) hộp nét đứt.
- Nhiều item: Bảng cuộn dọc, `content-visibility:auto`; Quỹ đạo: hệ >3 session thì "+N session", >5 subagent thì "+N".
- Narrow (<768): Bảng 1 cột, strip ngang h40, Orbit detail thành sheet, Focus bỏ rail.
- Keyboard: Alt+1/2, backtick, mũi tên mode, Esc, Enter, S, F, Ctrl+F (map).
- Chart/đồ họa: không dùng thư viện chart; SVG tay (Orbit), Vue Flow (AgentMap). Token màu: `agent-claude|codex|deepseek`, `accent`, `warning`/`warning-soft`, `st-working|waiting|idle|error|stale|done` (+`-soft`), `success`, `ink-faint`, `line-strong`, `fill`, `raised`, `surface`, `canvas`, `diff-add|del`, `peer`.

## 2. Usage (`components/usage/UsageView.vue`, 151 dòng)

### 2.1 Layout
```
main max-w 1560, px16/24, py16/20
Thống kê (h2 24px) (i)  [chip lọc: model x][chip: project x]      [Hôm nay|7 ngày|30 ngày|Tất cả] [Token|Chi phí] [Model v] [Project v 12]
[ scanning card: radar Lottie + "Đang đọc lịch sử transcript… 120 / 480 file" + progress h6 ]
UsageHero  (rounded-card bg-surface shadow-card, grid [1fr | 15rem] từ >=900px)
 |  "Token · 7 ngày"        (nút bảng/biểu đồ)                         | Giới hạn gói (UsageLimits)
 |  1.9M  (36px)   TB 270k/ngày   ▲ 12% so với 7 ngày trước đó         |  5 giờ   62% còn lại  ▬▬▬▭
 |  [biểu đồ cột chồng cao 200px]                                       |  Weekly  41% còn lại
 |  [legend 4 nút: Input / Output / Cache write / Cache read + số + %]  |  Gói: Max 5x. Cập nhật 2 phút trước
[ UsageProjects (3fr) ][ UsageModels (2fr) ]   (1 cột dưới 1100px)
```
- Dialog `ProjectPickerDialog` (248 dòng, Teleport body, kéo đổi size, phóng to) chứa `ProjectTable` (450 dòng): ô tìm, "Gộp theo repo", segment ổ đĩa, mật độ, "Đặt lại cột", bảng `table-fixed` cột resize được (Project, Message, Input, Output, Cache write, Cache read, Token, Chi phí ƯT, Số ngày, Lần cuối), phân trang "Mỗi trang", "Trang 1 / 5". Mở bằng nút Project hoặc "Xem tất cả N project" (mode `all`, chỉ xem) hoặc `pick` (chọn để lọc).

### 2.2 Phần tử, data, state
- Store `stores/usage` (`range`, `metric`, `model`, `project`, `selectedDay`, `hidden`, `daily`, `seriesTotals`, `heroTotal`, `topProjects`, `byModel`, `scan`, `received`, `hasData`), `extras.limits` + `extras.codexLimits` cho hạn mức.
- Hero (`UsageHero.vue:89-184`): nhãn "Token" hoặc "Chi phí ước tính" + phạm vi (ngày đã chọn hoặc khoảng, có nút X "Xem lại cả khoảng (Esc)"); số lớn `CountNum` 36px; "TB x/ngày"; trend (icon + % + "so với 7 ngày trước đó"); "không tính ..."; "N message từ model chưa có giá"; `UsageInfo` popover giải thích chi phí (footnote "Chi phí là ước tính theo giá API niêm yết (kiểm tra ngày ...), không phải hoá đơn"). Nút đổi biểu đồ/bảng.
- Legend: 4 nút toggle series, mỗi nút có ô vuông màu, tên, giá trị, % phần, "· 83% đọc từ cache" cho Cache read.
- Hạn mức bên phải (`UsageLimits.vue`, 120 dòng): theo provider (Claude, Codex; chỉ hiện nhãn provider khi có >1 nhóm): 2 cửa sổ "5 giờ" và "Weekly": "62% còn lại" (icon theo mức: Gauge ok / CircleAlert >=80% dùng / OctagonAlert >=95%), bar h6 `accent` (warn `warning`, critical `st-error`, stale `ink-faint`), "reset sau 2h 15m", dự báo "hết ~15:40 (1h 20m)" màu warning nếu chạm trước reset; chân "Gói: Max 5x. Cập nhật 3 phút trước" + ghi chú stale ("Hiện không có session nào gửi số liệu"). Ẩn hẳn cột nếu không có limits (`hasLimits`).
- States: scanning (card + progress), `waiting` = skeleton shimmer (384px + 268px x2), `empty` (ảnh `empty-stats`, "Chưa có token nào được ghi nhận."), có data. Disconnected: dòng sr-only `sessions.loading`. Lỗi riêng: chưa rõ (không thấy nhánh error).
- Chart: `UsageDailyChart.vue` (327 dòng), SVG tay, không thư viện: cao 200px, pad `{top10,right30,bottom24,left56}`, cột max 28px, gap 2, bo 4; 4 series chồng (fill `series-input|output|write|read` = `--ccm-cat-1..4`), lưới `--ccm-grid`, đường TB nét đứt `ink-faint`, nhãn trục 11px; tooltip 220px (ngày dài, 4 dòng series, Tổng, hint "Bấm để xem riêng ngày này"). Hover cột `raised`, ngày chọn `fill`, ngày khác mờ 35%. Bảng thay thế `max-height 200px`.
- Rank list (`UsageRankList.vue`): hàng h32 `grid [1fr | 1.1fr bar | 5rem | 2.75rem]`: nhãn (+ ô màu model), bar h6 `accent` (project) hoặc `bg-model-1..8` (model), giá trị compact, % (`<1%` nếu nhỏ). Click = lọc cả trang (toggle, `aria-pressed`, nền `accent-soft`). Tiêu đề động "Dùng nhiều nhất: Orrery" / "Model dùng nhiều nhất: Opus 4.7". `TOP_PROJECTS` giới hạn (giá trị chưa đọc).
- Tương tác: Esc bỏ chọn ngày; trái/phải chuyển ngày trên chart, Enter chọn; click cột chọn ngày; legend ẩn/hiện series; chip lọc xóa bằng click.
- Narrow: hero 1 cột, limits xuống dưới có `border-t`; 2 card dưới xếp dọc.
- `PlanLimits.vue` (287 dòng) KHÔNG nằm trong view này: nó là nội dung popover header (`HeaderStatusPopover.vue:13`): thẻ cho từng provider với 2 vòng/bar lớn (số 24px "62% còn lại"), `dl` Đã dùng / Reset / Tốc độ "~8% mỗi giờ" / Dự báo ("Đủ dùng tới lúc reset" màu success hoặc "hết ~15:40 (1h 20m)"), chân Gói + Cập nhật + ghi chú stale; dạng compact trong header (lưới 3 cột: nhãn, bar, "N% còn").

## 3. History (`components/history/HistoryView.vue`, 242 dòng)

### 3.1 Layout (cuộn trang dọc, mỗi khối cách `mt-6 border-t pt-5`)
```
Lịch sử (h2 24px)  "Các session đã làm gì hôm nay và những session đã kết thúc..."          [Tổng kết ngày] (primary)
[PromptSearchCard]  Tìm session cũ: ô "vd. paywall khôi phục giao dịch" | Dự án v | [7|30|... ngày] | Tìm | (i riêng tư)
   kết quả: mỗi hit = id/title + "Bắt đầu ..." + "3 prompt khớp" + [Tiếp tục trong pane] [Mở thư mục] [Chép id session]
[TimelineChart] "Hôm nay"  tóm tắt "làm 3g 20p, chờ 40p, rảnh 1g"; mỗi session 1 hàng: tên | thanh h10 chia working/waiting/idle | "78%"; nút "Xem theo giờ" -> làn h28 theo giờ trong ngày + vạch "bây giờ"
[HeatmapChart] "Agent làm việc lúc nào": lưới 7 x 24 ô 16px, 5 mức (bg-fill, ccm-heat-1..4 = accent 25/50/..%), legend Ít..Nhiều, "Bận nhất: T3 khoảng 14:00"
[ToolStatsCard] "Tools, skills và subagent": [Dự án v][7 ngày|30 ngày]; 4 cột (tools / skills / subagents / MCP), mỗi hàng h28: tên mono | bar h6 accent | số; cảnh báo lỗi "N% lỗi" (warning); "Xem tất cả N"; mục "chưa dùng" (skill + MCP không gọi 30 ngày) nút kiểm tra
[Session đã kết thúc] card bg-surface: tiêu đề + ô tìm w288; bảng 8 cột (Session | Kết thúc | Chạy | Model | Token | Thời gian | Chi phí | [Tiếp tục]); phân trang 20/trang
```

### 3.2 Chi tiết
- Nguồn: `stores/history` (`sessions`, `loaded`, `heatmap`), `connection.loadTimeline/loadHistory/loadHeatmap` (gọi khi mount và khi reconnect; timeline poll 30s `useActiveInterval`), `extras.modelNames`, `workspace` (resume), `terminals` (đang chạy ở nơi khác), `search` store (prompt search).
- Bảng "Session đã kết thúc" (`HistoryView.vue:176-234`): `table-fixed`, cột tự ẩn theo breakpoint (Chạy ẩn <md, Model ẩn <lg, Thời gian ẩn <sm, Chi phí ẩn <md). Ô Session: tiêu đề 14px medium + `shortPath(cwd) · branch` mono 11px. Ô Kết thúc: "15 thg 11, 09:42" + "2 giờ trước". Token compact ("1.2M"). Thời gian: "làm 1h 05m" (st-working), "chờ 12m" (st-waiting). Chi phí `$1.24`. Nút "Tiếp tục" (Play) disabled kèm lý do tooltip: không có terminal app desktop, id không hợp lệ, thiếu thư mục, "vẫn đang chạy ở terminal khác"; nếu đã có pane thì nhãn "Xem pane".
- States: `loading` "Đang tải lịch sử…"; empty (ảnh `empty-history`, "Session sẽ hiện ở đây khi kết thúc." + nút "Mở Workspace"); không khớp (ảnh `empty-project-search`, "Không có session nào khớp." + nút "Xoá ô tìm"). Lưu 1000 session gần nhất, chỉ lưu số token.
- Prompt search: phase idle/listing/scanning ("Đang đọc transcript: 12 / 340")/found/capped/cancelled/failed/none; nút Huỷ khi đang chạy; không prompt thì nút Tìm disabled. Palette có thể focus ô này (`ui.promptSearchFocus`).
- Tương tác: phím tắt chưa thấy riêng; click tên session trong timeline nhảy tới (disabled khi không live); hover đoạn timeline hiện tooltip "làm 12m, 14:02 – 14:14".
- Chart: tay, không thư viện. Timeline = thanh flex % với `bg-st-working|waiting|idle`; làn giờ = đoạn tuyệt đối `ccm-grow-x`, vạch `accent` "bây giờ"; heatmap = `<table>` ô màu `color-mix(accent, fill)`; tool stats = bar `scaleX`.
- Narrow: bảng cuộn ngang trong card (`overflow-x-auto`), cột phụ ẩn.

## 4. Inbox (`components/inbox/InboxPanel.vue`, 134 dòng)

```
popover w<=400 (ccm-menu, fixed, neo dưới nút Inbox ở header, top = anchor.bottom+6, max-h min(70vh,520px))
| ▣ Cần chủ nhân                                     J/K · Enter · E đã xem |
| ◐ Tiêu đề session A                                              [✓ hover]|
|   Chờ chủ nhân trả lời · 4m                                              |
| ⚠ Tiêu đề session B   Lỗi lặp lại liên tục · 12m                          |
```
- Hàng min-h36: StatusIcon 14 (waiting / idle cho note / error cho loop+checks / working cho stuck / completed cho done) + tiêu đề 12px medium + lý do 11px muted (+ " · thời gian" bỏ giây). Nút ✓ "Đánh dấu đã xem (hiện lại khi trạng thái đổi)" chỉ hiện khi hover/focus.
- Lý do (`lib/inbox.ts`, `SEVERITY`): waiting 0, note 0.5 (ghi chú agent gửi qua MCP, có level info/attention/done, gộp "+N"), loop 1 ("Lỗi lặp lại liên tục"), checks 2 ("Checks lỗi trong thư mục của nó"), stuck 3 ("Im lặng ..."), done 4. Mỗi agent 1 hàng với lý do khẩn nhất; sắp theo mức rồi theo thời gian.
- Data: `stores/inbox.items` + `terminals.byId`; `go(id)`: có pane Workspace thì `setView('workspace')` + `workspace.activate(pane.key)`, không thì `setView('monitor')` + `focusTerminal`. Đóng khi click ngoài.
- Keyboard: J/K hoặc mũi tên lên xuống (vòng), Enter đi tới, E đánh dấu đã xem, Esc đóng và trả focus nút. Empty: "Không có agent nào cần chủ nhân lúc này." Nút header: "Cần chủ nhân: không có | {n} agent". Loading/stale/error: chưa rõ (không có nhánh riêng).
- Narrow: width = `min(400, innerWidth-24)`.

## 5. Office (tạm dừng từ 2026-10-28, không đề xuất việc Office)
- Vào qua segmented "Bản đồ / Văn phòng" (chỉ khi `settings.officeEnabled`) ở `OrbitView.vue:95` và toolbar `AgentMap.vue:1297`; files `components/office/{OfficeView,OfficeTimelapse,OfficeOptions,OfficeNeedsNode}.vue`.
- Habblaud pixel-art (`packages/habblaud`), có âm lượng, timelapse, fullscreen. v102 chỉ cần khung, theo plan WP10.

## 6. Must keep (bất biến chức năng)
1. Chip/tóm tắt "N agent đang chạy, N chờ chủ nhân" luôn thấy ở Monitor; việc cần chủ nhân phải nổi nhất (màu warning + icon Hand, không chỉ màu).
2. Trạng thái = icon + chữ + màu (CLAUDE.md), với 8 status: Đang chạy, Đang chờ, Rảnh, Xong, Lỗi, Đã huỷ, Đã đóng, Không rõ.
3. Card session: tiêu đề, project, agent, dòng tool/file đang chạy hoặc lý do chờ + thời gian chờ, branch, thời gian chạy, context meter (3 tone, ngưỡng `contextWarnTokens`), tóm tắt lượt cuối (file +/-), nút Mở trong Workspace (disabled có tooltip khi không có pane) và Xem diff, menu chuột phải, subagent orbit.
4. Quỹ đạo: project = hệ, session = mặt trời, subagent = hành tinh, liên kết peer, Focus 1 hệ với vệt cung tool call + dòng tool, zoom/fit/legend. Tab tên "Quỹ đạo" (GLOSSARY).
5. Nhóm theo Trạng thái/Project/Agent, lọc project, cột "Xong hôm nay" với timeline + "Xem thêm".
6. CommandPanel (phím `` ` ``), CompactView, AwaySummary, WaitingStrip.
7. Không hiển thị token/giá trị bịa; chi phí chỉ là "ƯT" kèm footnote; không lộ nội dung session/secret.
8. Usage: lọc chéo (range, metric, model, project, ngày), legend ẩn/hiện series, bảng thay thế biểu đồ (a11y), hạn mức 5h/Weekly với dự báo, picker project dạng bảng.
9. History: tìm prompt cũ (chỉ prompt của chủ nhân, che secret), timeline hôm nay, heatmap 4 tuần, tool stats, bảng session đã kết thúc với Tiếp tục/Xem pane + lý do disabled, Tổng kết ngày.
10. Inbox: phím J/K/Enter/E/Esc, lý do + thời gian, đánh dấu đã xem (hiện lại khi trạng thái đổi).
11. Mọi string qua i18n vi + en; màu qua token; không cuộn ngang trang; không blur (cũ) hoặc blur chỉ ở chrome (mới, theo plan); `prefers-reduced-motion` tắt quay hành tinh.

## 7. Pain points và inconsistencies
1. Một session hiện ở 3 chỗ gần giống: `BoardCard`, `SessionCard` (aside Quỹ đạo), `OrbitDetail`; mỗi nơi tự dựng dòng trạng thái, wait, context (BoardCard dùng meter + chữ, MainNode/SessionCard dùng `ContextRing`, rail Workspace dùng `ContextGauge`): 3 biểu diễn context khác nhau.
2. Mã chết nhiều: 8 component không mount (mục 0), gồm `SessionList`, `ActivityFeed`, `ToolTimeline`, `WaitingQueue`. Dễ gây hiểu nhầm khi redesign.
3. `AgentMap` (1581 dòng, Vue Flow) và `OrbitStarMap`+`OrbitFocus` là 2 mô hình bản đồ song song; AgentMap chỉ còn vào được qua `sessionTab` (chưa rõ nguồn set) hoặc Office.
4. Thứ tự dải trên: `AwaySummary`, `PortConflictBar`, `WaitingStrip` xếp trên `ModeBar`, đẩy nội dung xuống; WaitingStrip chỉ có ở mode Quỹ đạo, Bảng thì không (vì cột "Cần chủ nhân" đã có).
5. Chrome không thống nhất: ModeBar dùng tablist tự dựng, `SegmentedControl` cho nhóm, `AppSelect` cho project; Usage/History dùng `SegmentedControl` + `AppSelect` + nút `ccm-btn`; Inbox là popover `ccm-menu` còn Usage picker là dialog tự dựng.
6. Hai cách hiện "Cần chủ nhân": cột Bảng, WaitingStrip, Inbox popover, CompactView, rail Workspace sắp có: 5 bề mặt cho cùng dữ liệu.
7. Dùng " · " làm ngăn cách trong code UI (`toolRunning`, `facts`, `lastTurn`, hero "Token · 7 ngày") trái GLOSSARY mới (dùng dấu phẩy hoặc gạch ngang).
8. Màu lệch theo ngữ cảnh: chờ dùng `warning` ở Board/CommandPanel nhưng `st-waiting` ở WaitingStrip/Inbox/CompactView/Orbit label.
9. Usage chart: cột chồng 4 màu `--ccm-cat-*` cố định, cột tối đa 28px, nhãn 11px nhỏ; không có empty-per-range riêng; bảng thay thế max 200px cuộn nhỏ.
10. History quá dài 1 trang (5 khối) và mỗi khối là card/border-t khác nhau; "Tổng kết ngày" là nút primary duy nhất cạnh tiêu đề.
11. Narrow chưa rõ có thiết kế riêng cho Usage hero chart (SVG min width 240).
12. Heatmap 16px + bảng 8 cột là phần khó co về mobile.

## 8. Overlap với Workspace rail (live rail W2)
Rail sống (264px, kéo được, thu 56px) lấy từ `usePaneInfo.activityOf`, `railStateOf`, `ContextGauge`, `lib/rail/*`; hàng đợi "Cần bạn" từ `lib/rail/attentionList.ts`; Hạn mức/Inbox/Thông báo về chân rail (plan v102).

| Thông tin | Monitor hiện tại | Rail sống W2 | Gợi ý hợp nhất |
|---|---|---|---|
| Tên session + chip trạng thái + thời gian | `BoardCard` dòng 1 và chip dòng trạng thái | dòng 1 rail | 1 component `SessionRow` dùng chung, 3 mật độ (rail 1-2 dòng, Board card đầy đủ, Mission) |
| Tool đang chạy, file, kết quả gần nhất | `BoardCard` (`fileAction`, `toolRunning`), OrbitDetail "Tool gần nhất", CommandPanel | dòng 2 rail (+ tên model) | nguồn duy nhất `activityOf`; Monitor chỉ thêm lịch sử (feed) |
| Context | BoardCard meter, SessionCard ContextRing | `ContextGauge` | dùng `ContextGauge` mọi nơi |
| Cần chủ nhân + thời gian chờ | cột Board, WaitingStrip, Inbox, CompactView, OrbitDetail hộp cảnh báo | thẻ "Cần bạn" đầu rail | Inbox và WaitingStrip trở thành 1 danh sách "Cần bạn" trên rail; Monitor giữ cột/lane chi tiết + Mở pane trả lời |
| Mở pane / reply | `OpenPaneButton` ở mọi card | click hàng rail = `nextAttention`/focus pane | giữ nút ở Monitor (vì Monitor không có terminal), rail dùng click hàng |
| Subagent | `SubagentOrbit` (card), planets (Quỹ đạo), `SubagentBar` | trạng thái "subagents" trong rail state, vòng avatar | thống nhất `railStateOf`; Quỹ đạo là nơi duy nhất xem chi tiết subagent |
| Hạn mức 5h/Weekly | `UsageLimits` (Usage), `PlanLimits` (header popover) | chân rail | 1 component `LimitMeter`, 3 kích thước (rail mini, popover, Usage cột phải) |
| Inbox | popover header | chân rail | popover neo vào chân rail |
| Project grouping, monogram tone | `projectTone`, `monogram` trong BoardCard | rail dùng `projectRow` | đã chung `lib/rail/projectRow`, giữ |

Phần CHỈ có ở Monitor (rail không thay): Quỹ đạo, so sánh nhiều session cạnh nhau, "Xong hôm nay", CommandPanel (lệnh dài), nhóm theo Agent, Away summary.

## 9. Nội dung thật để mockup
- Project: `Orrery`, `jin-tools`, `jin-workflow`, `PhotoWidget-Android`, `ChatGuru-Flutter`, `habblaud`; monogram O, J, P, C. Thư mục rút gọn `…/web/Orrery`. Branch: `feat_mist_ui`, `feat_clean_memory`, `master`, `fix/ads-capping`.
- Agent (nhãn): `Claude Code`, `Codex`, `DeepSeek`. Model: `Opus 4.7`, `Sonnet 5.5`, `Haiku 4.5`, effort `high`/`xhigh`. Hint Codex `gpt-5.5-codex` (chưa rõ tên model thật).
- Tiêu đề session (displayTitle, thường tự đặt hoặc alias): "Refactor AdsGate backoff", "Fix crash khi mở Explorer", "v102 mist rail: LiveRail.vue", "Viết WP-3a dupes", "Dịch strings vi cho Disk Clean", "Review diff v101".
- Tool: `Bash`, `Edit`, `Read`, `Write`, `Grep`, `Glob`, `Task` (subagent), `WebFetch`, `mcp__orrery__open_session`; subagent type: `general-purpose`, `Explore`, `typescript-reviewer`.
- Chuỗi dòng trạng thái: "Bash · 12s", "Đang sửa SessionBoard.vue", "Đang đọc CLAUDE.md", "Cần duyệt: Bash · 3m 20s", "Cần duyệt plan · 45s", "Đang hỏi bạn · 1m 05s", "Rảnh", "Xong".
- Thời gian (`lib/format.duration`): `12s`, `3m 20s` (có đệm 0: `3m 05s`), `1h 05m`. Giờ `14:32` (Board done), `14:32:07` (feed). Token compact `en-US`: `842`, `12.5K`, `128K`, `1.9M`; context "Context 128K, cảnh báo ở 500K". Chi phí: `$0.042` (<1, 3 số lẻ), `$3.42` (1 đến 100), `$128` (>=100).
- Lượt cuối: "Lượt cuối sửa 3 file", `+120` (diff-add), `−8` (diff-del, dấu trừ U+2212).
- Subagent: "3 subagent đang chạy"; planet "Explore: Grep · 8s", "xong", "chờ".
- Usage: tiêu đề "Thống kê"; hero "Token · 7 ngày" `4.2M`, "TB 600K/ngày", "▲ 12% so với 7 ngày trước đó"; "Chi phí ước tính" `$87.40`; legend: Input `310K 7%`, Output `420K 10%`, Cache write `1.1M 26%`, Cache read `2.4M 57% · 83% đọc từ cache`; hạn mức "5 giờ 62% còn lại, reset sau 2h 15m", "Weekly 41% còn lại, hết ~15:40 (1h 20m)", "Gói: Max 5x. Cập nhật 2 phút trước"; top project: Orrery 2.1M 50%, jin-tools 980K 23%, jin-workflow 610K 15%; model: Opus 4.7 2.6M, Sonnet 5.5 1.3M, Haiku 4.5 0.3M.
- History: heatmap "Bận nhất: T3 khoảng 14:00"; timeline "làm 3g 20p, chờ 40p"; hàng đã kết thúc: "Fix crash khi mở Explorer", `…/web/Orrery · feat_clean_memory`, "15 thg 11, 09:42 / 2 giờ trước", Chạy `1h 12m`, Model `Opus 4.7`, Token `1.2M`, "làm 58m / chờ 9m", `$4.80`; tool stats: `Bash 1.2K`, `Edit 845`, `Read 2.3K`, "3% lỗi".
- Inbox: "Cần chủ nhân"; hàng "Refactor AdsGate backoff / Chờ chủ nhân trả lời · 4m", "Fix crash khi mở Explorer / Lỗi lặp lại liên tục · 12m", "Viết WP-3a dupes / Checks lỗi trong thư mục của nó"; hint "J/K · Enter · E đã xem".
- Trạng thái trống để vẽ: Monitor "Không có session Claude Code nào đang chạy." (Compact) / "Chưa có session" (Bảng); Usage "Chưa có token nào được ghi nhận."; History "Session sẽ hiện ở đây khi kết thúc."; Inbox "Không có agent nào cần chủ nhân lúc này."
