# UX review v102 Sương kính, Workspace W2

Phạm vi: review heuristic và đề xuất mockup, không lặp factual inventory. Căn cứ plan v102, coordination v100/v101, code hiện tại và ảnh QA v79/v80. Kích thước mockup lấy theo window 1440 x 900, trừ khi ghi khác.

## 1. Nhận định nhanh

W2 hợp lý với Orrery vì phiên terminal là đơn vị làm việc, còn project rail hỗ trợ chuyển workspace. Điều kiện thành công là rail phải trả lời nhanh “phiên nào cần tôi, vì sao, đã chờ bao lâu”, còn pane giữ chỗ cho terminal. Nếu rail nhồi metadata hoặc nền kính gây nhiễu, người dùng sẽ phải mở Mission Control để tìm phiên.

Ưu tiên thiết kế:

1. Thứ tự attention: permission/question đang chờ, lỗi/failed, completed cần review, working, idle. Không gộp needs input với done thành một màu hoặc badge.
2. Chốt hành vi focus: mở phiên cần bạn không được làm mất pane đang xem hoặc thay terminal active bất ngờ.
3. Thiết kế rail cho 10+ phiên bằng scroll, nhóm/counter và filter, không co nhỏ row.
4. Terminal well đục và ổn định. Glass chỉ dùng cho chrome, có reduced-transparency và kiểm tra chữ trên palette sáng/tối.
5. Có responsive breakpoint. Khi rail, canvas và Explorer cùng tranh chiều ngang, Explorer overlay, canvas không co xuống kích thước không dùng được.

## 2. Heuristic audit theo UI hiện tại

### Visibility of system status, match với cách user scan

- **P1, trạng thái nhiều pane nằm rải ở tab và footer.** Tab chỉ hiển thị một phần, phần còn lại vào overflow; footer đếm running/waiting nhưng không chỉ ra session nào cần hành động. Nhiều tab hoặc nhiều pane buộc user mở attention popover/Mission Control để tìm. Căn cứ [WorkspaceView.vue](../../apps/web/src/components/workspace/WorkspaceView.vue#L1750), [WorkspaceView.vue](../../apps/web/src/components/workspace/WorkspaceView.vue#L1887), [WorkspaceFooter.vue](../../apps/web/src/components/workspace/WorkspaceFooter.vue#L105).
- **P1, waiting chưa đồng nghĩa với cần user.** WaitingQueue tập hợp pane status waiting, kèm reason và elapsed time. W2 cần phân biệt prompt cần trả lời, permission cần duyệt, process đang chờ, completed cần review. Dùng một màu vàng/chuông sẽ gây interruption sai mức. Căn cứ [WaitingQueue.vue](../../apps/web/src/components/WaitingQueue.vue#L38), [WaitingQueue.vue](../../apps/web/src/components/WaitingQueue.vue#L91).
- **P2, working không nói agent có tiến triển hay bị treo.** Tool gần nhất hoặc context gauge chưa thể hiện tiến độ. Có thể dùng thời điểm output/tool event gần nhất và nhãn “không có cập nhật” sau ngưỡng rõ; tránh cảnh báo giả.

### Information hierarchy và recognition over recall

- **P1, tên phiên và tên project dễ trùng.** Tab lấy tabName, còn project grouping thể hiện bằng tone/edge nhỏ. Dùng task label tùy chỉnh làm text chính, repo/worktree làm dòng phụ. Đừng bắt user nhớ “tab xanh là phiên sửa test”. Căn cứ [WorkspaceView.vue](../../apps/web/src/components/workspace/WorkspaceView.vue#L1773), [WorkspaceView.vue](../../apps/web/src/components/workspace/WorkspaceView.vue#L1823).
- **P2, tab có nhiều metadata cạnh tranh nhau.** Tab hiện có status, tên, peer, Telegram, dirty, pane count, zoom, attention. Pill redesign không tự giảm độ phức tạp. Chỉ giữ một trạng thái ưu tiên, phần còn lại ở hover, rail row hoặc details. Căn cứ [WorkspaceView.vue](../../apps/web/src/components/workspace/WorkspaceView.vue#L1815), [WorkspaceView.vue](../../apps/web/src/components/workspace/WorkspaceView.vue#L1849).
- **P1, context gauge không nên cạnh tranh với attention.** Model hữu ích để chọn phiên, gauge hữu ích để quản lý quota, nhưng needs reason quan trọng hơn. Ở width 264px, hiển thị gauge dạng mini hoặc chỉ khi gần limit; ưu tiên task/reason có thể đọc được.

### User control, interruption handling, error prevention

- **P1, nút Mở cần giữ nguyên ngữ cảnh làm việc.** Waiting row hiện focus pane tương ứng [WaitingQueue.vue](../../apps/web/src/components/WaitingQueue.vue#L146). W2 nên chuyển active tab/session và focus đúng pane, giữ split layout. Nếu session đã đóng hoặc đổi state, thông báo và đưa tới phiên có thể phục hồi.
- **P1, snooze phải có thể undo và hiện phạm vi.** Vẽ menu thời lượng, trạng thái snoozed, thời điểm hiện lại và thao tác khôi phục. Không cho snooze lỗi hoặc permission lâu dài như snooze câu hỏi không khẩn. Căn cứ [WaitingQueue.vue](../../apps/web/src/components/WaitingQueue.vue#L120), [WaitingQueue.vue](../../apps/web/src/components/WaitingQueue.vue#L134).
- **P2, attention burst có thể tạo interruption cascade.** Gộp toast, không steal focus, mặc định không phát âm thanh. Hiển thị count ổn định như “2 phiên cần bạn” để user xử lý theo lô.
- **P1, status color phải giữ nghĩa giữa các palette.** Token hiện tách lifecycle khỏi warning/success [style.css](../../apps/web/src/style.css#L156), [style.css](../../apps/web/src/style.css#L168). Provider color, working/waiting/error/done, dirty/modified và focus ring cần semantics riêng. Không dùng cùng xanh cho active và done, hoặc cam cho working lẫn warning nếu thiếu icon/label.

### Keyboard flow, accessibility, consistency

- **P1, Ctrl+K là đường điều hướng trung tâm.** Palette hiện có modal dialog, combobox, listbox, active descendant và keyboard hints [CommandPalette.vue](../../apps/web/src/components/CommandPalette.vue#L747), [CommandPalette.vue](../../apps/web/src/components/CommandPalette.vue#L802). W2 nên có nhóm Sessions: Cần bạn, Đang chạy, Hoàn tất chưa xem, Gần đây. Query khớp task, repo, branch, provider, model. Enter mở, Shift+Enter alternate action.
- **P2, rail collapsed cần label khi keyboard focus, không phụ thuộc hover.** RailColumn dùng monogram và title [RailColumn.vue](../../apps/web/src/components/workspace/rail/RailColumn.vue#L91). Tooltip trễ không giúp scan 10 project. Focus phải hiện label/popover ngay; needs badge có shape và accessible name, không chỉ màu.
- **P2, Mission Control không nên là “bản đồ bí mật”.** Đã có search, group theo project và needs counter [MissionControl.vue](../../apps/web/src/components/workspace/MissionControl.vue#L882), [MissionControl.vue](../../apps/web/src/components/workspace/MissionControl.vue#L941). Dùng cho overview, multi-select và layout; không bắt user mở để tìm session thứ 9.
- **P2, Explorer overlay cần focus behavior đoán được.** Escape đóng và trả focus về trigger; pinned state thành column với separator/resizer. Narrow window thì overlay, không ép tab strip xuống hàng.

## 3. W2 plan, thiếu và rủi ro

### Density và task identity

- Rail 264px có thể dùng được ở 1440px, nhưng row 2 dòng với title, status, tool, model, result, gauge sẽ chật ở 200-230px hoặc tên project dài. Đề xuất row 56px, 64px khi needs input. Dòng 1: task title và status chip. Dòng 2: project/worktree, provider/model; reason hoặc last event quan trọng hơn gauge.
- Trên 10 phiên, chia section: Cần bạn, Đang chạy, Hoàn tất chưa xem, Paused/Idle. Chỉ Cần bạn tự mở khi có item; các section khác collapsed với count. Có filter và recent ordering. Không auto-collapse khi status đổi vì row nhảy dưới pointer.
- Status mới đến không nên đảo thứ tự liên tục. Mặc định sắp attention trước, rồi activity gần nhất; pin/favorite giữ ổn định qua status change.
- Cần định nghĩa quan hệ session, tab, pane, project, worktree. Một tab có nhiều pane/agent thì row đại diện cái nào? Mockup phải cho thấy pane count và cách mở đúng pane.

### Attention queue và mức ưu tiên

- Chốt taxonomy trước khi chọn màu: needs input, permission, failed, completed unreviewed, working, idle, snoozed. Mỗi loại có icon + text hoặc icon + shape; màu là kênh phụ.
- “Cần bạn” chỉ gồm action user phải làm để tiếp tục. Completed cần review là section riêng. Failed là urgent nhưng có thể không cần action ngay nếu agent tự retry.
- Sort: blocking permission/question, failed, completed có output chưa xem, waiting lâu nhất, working, idle. Needs hiển thị elapsed; completed hiển thị “xong 4 phút trước”.
- Mỗi needs row có action Mở, action phụ snooze/resolve, reason ngắn. Khi user xử lý trong terminal, item biến mất có undo toast 5s.

### Glass, contrast, terminal

- backdrop-filter blur(20px) saturate(1.5) có thể đặt text trên vùng sáng/tối khác nhau khi terminal output thay đổi. Không đặt chữ nhỏ trên nền alpha thấp. Rail, drawer, palette cần tint đủ đục, edge 1px, shadow; kiểm contrast trên background bất lợi, không chỉ token rời.
- Terminal well phải opaque theo palette terminal, bất kể theme sáng/tối. Gradient chỉ thấy qua gutter/empty space. Tránh transparency làm ANSI text, cursor, selection và diff khó đọc. Ngân hà giữ sao trong panel nhưng không để sao xuyên terminal output.
- Blur + xterm WebGL + resize/scroll là rủi ro hiệu năng. Có reduced transparency, reduced motion, high contrast và fallback khi blur không hỗ trợ. Không blur pane hoặc list dài.
- Glass edge không thay focus ring. Active pane có outline accent 2px; keyboard focus ring 2px riêng.

### Window nhỏ và pane count

- **Window 1024px:** rail 240px, canvas tối thiểu 520px, Explorer overlay 280px. Dưới 900px chuyển rail collapsed 56px hoặc flyout; Explorer overlay rộng min(320px, 85vw). Dùng hysteresis hoặc preference để tránh rail tự bật/tắt liên tục khi resize.
- **Window 800px:** rail collapsed, tab strip hiện active tab và overflow +N, toolbar actions vào menu; canvas tối thiểu 480px. Explorer overlay tối đa 360px, có scrim và close.
- **6 panes:** layout 2x3 hoặc 3x2 không thể đọc đủ nội dung. Inactive pane có header 30px giữ title/status; active pane có zoom/focus nhanh. Mock cả hai layout.
- **10+ sessions:** rail scroll độc lập, Cần bạn sticky, section counters, filter và keyboard navigation. Canvas không scroll cùng rail. Chụp case 2 needs, 1 failed, 5 working, 2 completed chưa xem.
- Plan chưa nói drag reorder trên rail dài/sectioned. Reorder giữ vị trí trong section; status change không mất pin; drag giữa project chỉ đổi group nếu affordance rõ.

### Giao diện và rollout

- Settings so sánh palette bằng preview thật có rail, pane và terminal, không chỉ swatch. Có light/dark/system, reduced transparency, restore defaults, UI generation switch và mô tả persistence.
- Bản đầu mặc định classic: entry point “Thử giao diện mới” phải nhớ generation độc lập theme; Settings next có đường về classic.
- Contrast test cần thêm placeholder, disabled, focus, selected, hover, badge, terminal output và glyph trên well; kiểm 9 palette ở light/dark.

## 4. Pattern từ công cụ khác

| Sản phẩm | Pattern đáng học | Áp dụng cho W2 | Tránh sao chép |
|---|---|---|---|
| Warp | Agent Management Panel theo dõi nhiều conversation; conversation gắn terminal session, chạy qua windows/tabs/panes. [Docs](https://docs.warp.dev/agents/local-agents/interacting-with-agents/), [overview](https://docs.warp.dev/agents/) | Mapping session ↔ pane rõ, mở lại conversation, xem recent activity. Tách quản lý list khỏi terminal output. | Đừng tách conversation khỏi local PTY session thật. W2 cần runtime status, không chỉ lịch sử chat. |
| Zed | Thread list/sidebar và Agent Panel; chọn thread để mở lại panel. [Parallel Agents](https://zed.dev/docs/ai/parallel-agents), [Agent Panel](https://zed.dev/docs/ai/agent-panel) | Chuyển thread nhanh, identity rõ, panel là vùng phụ. | Không để agent chat chiếm trung tâm khi workflow terminal-first. |
| Cursor Background Agents | Sidebar có list/search/start, xem status và follow-up/take over. [Docs](https://docs.cursor.com/background-agent) | Search và tiếp tục phiên từ list; phân biệt local/remote nếu Orrery hỗ trợ sau này. | Không sao chép cloud assumptions, retention hoặc remote execution vào local PTY product. |
| VS Code Agent Sessions | Session grouped theo workspace/time; change count cho edits chưa review; archive/done tách delete. [Manage sessions](https://code.visualstudio.com/docs/agents/run/sessions/manage-sessions), [Agents window](https://code.visualstudio.com/docs/agents/run/agents-window) | Mượn trạng thái changes chưa review, archive để dọn list nhưng giữ transcript, mở session history. | Đừng chỉ group theo thời gian; Orrery cần repo/worktree và live state. |
| Conductor | Nhiều workspace cô lập; chọn share workspace hay tách worktree theo nhiệm vụ. [Parallel agents](https://www.conductor.build/docs/concepts/parallel-agents), [Harnesses](https://www.conductor.build/docs/reference/harnesses) | Hiện project/worktree để phân biệt cùng repo nhưng khác checkout. | Không biến rail thành manager git workflow. Orrery tập trung monitor và điều khiển session; diff/branch detail ở Explorer/Mission Control. |
| tmux-style tools | Session/window/pane hierarchy, hotkey switching, detachable session, split layouts. | Giữ keyboard-first, labels tùy chỉnh, restore layout, zoom pane và switcher nhanh. | Không yêu cầu nhớ pane number/prefix khó discover; Ctrl+K và visible key hints bao phủ điều hướng. |

Tổng hợp: kết hợp live rail của terminal multiplexer với session metadata và review states của IDE agent views. Cần bạn là queue hành động, không phải toàn bộ session list; rail là danh sách chính, Mission Control là overview/bulk layout.

## 5. Mockup độ fidelity cao

Artboard mặc định 1440 x 900, trừ responsive variants. Titlebar Electron 32px, shell usable 868px. Font UI Manrope 13px, body 14px, mono terminal 13px. Ghi kích thước trên từng mockup.

### A. Workspace normal, một project, 2 session, 2 pane

- **Shell:** x=0..1440, y=32..900. Background sương tĩnh, padding 12px, gap 10px. Không có AppHeader cũ.
- **Rail:** x=12, y=44, w=264, h=844, radius 18. Header h=44 có segmented Giám sát/Workspace, project selector và +. Search h=32. Cần bạn h=110 khi có item. Session list chiếm phần còn lại; footer h=48 cho Hạn mức, Inbox, Thông báo, Cài đặt.
- **Cần bạn:** title h=24, count chip, 2 row x 40px. Mỗi row có status icon 16px, task 1 dòng, reason 1 dòng 12px, elapsed, action Open. Vẽ hover/focus và snooze menu.
- **Session row:** 56px. Dòng 1 task semibold 13px + status chip 11px. Dòng 2 project/worktree 11px, provider/model 11px, mini context meter 40px chỉ khi không cần show reason. Active có accent edge 2px + background; focus ring tách biệt.
- **Canvas:** x=286, y=44, w=1142, h=844. Tab strip h=40, tabs 30px. Monogram 16px, task label, needs badge khi actionable, overflow +N, nút +. Toolbar h=36 theo session active.
- **Pane:** 2 pane 50/50, gap 8px, radius 16. Header 34px, title trái, provider/model phụ, status và split/zoom/more phải. Terminal opaque chiếm phần còn lại, padding 12px, không blur.
- **States:** active/inactive pane, hover tab, attention badge, context gần limit, empty rail search, terminal output dài, focus ring, reduced-transparency.

### B. Workspace 8 sessions, 2 cần input

- Giữ shell/canvas của A, rail w=264px.
- Cần bạn sticky h=170px: header Cần bạn 2, action Mở kế tiếp, 2 card 64px. Card gồm task, reason như “Cho phép chạy lệnh” hoặc “Trả lời câu hỏi”, project, waited time, Open; failed có icon/error label riêng.
- Dưới queue: Đang chạy 4, Hoàn tất chưa xem 2 collapsed. Hai needs không lặp lại ở Running. Có tổng 8 sessions bằng counts.
- Vẽ active session thứ 5 trên canvas, không tự chuyển sang phiên cần input. Toast “2 phiên cần bạn” ở góc dưới, không lấy focus, có dismiss/link queue.
- **States:** rail scroll đến cuối nhưng queue sticky, needs mới không reorder dưới cursor, snoozed row, resolve + undo, badge overflow 9+.

### C. Rail collapsed

- Rail x=12, y=44, w=56, h=844. Workspace/Monitor icon 32px, project marker 32px, gap 8px. Cần bạn ở đầu list có count, không chỉ dot.
- 8 sessions không biến thành avatar khó phân biệt. Group theo project: project glyph, tối đa 3 active/recent session glyph, sau đó +N. Click project mở flyout w=280px full rows.
- Tooltip sau 300ms; keyboard focus hiện label ngay. Nội dung project, task, status, model. Needs badge hỗ trợ 2 chữ số và accessible name.
- **States:** focus glyph, active project, needs count, failed marker, flyout giữ mở khi pointer vào, scroll flyout.

### D. Explorer drawer open

- Drawer x=1136, y=44, w=292, h=844, trượt từ phải, border trái 1px, shadow. Canvas giữ layout; scrim 8-12% trên canvas hoặc shadow cạnh drawer, không làm chữ terminal xuyên drawer.
- Header h=44: root, pin, close. Tabs/icon row h=40: Files, Changes, Search, Git. Search h=34. Tree row 28px, indent 16px, icon 12px, selection rõ.
- Pinned: column w=292px, canvas co lại. Nếu canvas dưới 520px, chuyển overlay và báo “Ghim Explorer giảm vùng làm việc”.
- **States:** loading, empty, permission denied, search results, dirty file, selected file, pinned, overlay, narrow 800px full-height 85vw.

### E. Ctrl+K

- Overlay x=400, y=112, w=640, max h=620, radius 18, scrim 32%. Search h=52, icon, query, clear, Esc. Surface đục để chữ ổn định trên terminal/gradient.
- Scope row h=36: All, Sessions, Files, Commands, Projects; active có underline và keyboard hint.
- Session result h=56: status icon 16px, task 13px, repo/worktree + provider/model 11px, reason/status phụ, elapsed hoặc changed count. Groups Cần bạn, Đang chạy, Hoàn tất chưa xem, Gần đây.
- Footer h=36: arrows navigate, Enter open, Shift+Enter alternate action, Esc close. Active selection có contrast đầy đủ.
- Empty query hiện 3 needs và 3 recent. Search khớp task, repo, branch, provider, model, status. **States:** empty, no matches, loading, 10 sessions, keyboard selection, command prefix.

### F. Mission Control

- Overlay x=286, y=44, w=1142, h=844, radius 18. Header h=48: title, active/total, needs count, search w=240, close. Project sections accordion.
- Grid padding 20px, gap 12px. Card 260 x 150px, 3 cột ở canvas 1142px. Với 8 sessions: task, repo/worktree, provider/model, lifecycle, last event/elapsed, needs/review badge. Không dùng terminal screenshot mặc định vì stale/load.
- Click card mở session; secondary menu rename/move/close. Multi-select checkbox chỉ trong selection mode. Drag chỉ reorder layout, không move sang project vô tình.
- Header/filter sticky, content scroll. Needs group trước, completed group collapsible. **States:** 1/8/20 cards, 2 needs, search, empty, drag preview, multi-select, 1024px grid 2 cột, 800px 1 cột.

### G. Settings > Giao diện

- Settings shell 1120 x 760 center, nav 220px, content 820px, padding 24px, section gap 24px. Dựa cấu trúc settings hiện tại, preview theo W2.
- UI generation: Classic/Sương kính segmented h=40, generation hiện tại, đổi ngay; mô tả persistence và action quay lại.
- Palette grid 3 cột, card 248 x 136px. Mini sample 210 x 82 có rail, terminal well, needs row, tab, light/dark selector. Tên, mode badge, selected check. Không chỉ show dải màu.
- Transparency: toggle Giảm trong suốt, help text, preview live; bên dưới reduced motion, font UI/terminal. Preview có terminal opaque, ANSI samples, contrast state.
- **States:** Sương kính light/dark, Ngân hà sao on/off, selected palette, system mode, reduced transparency, contrast warning, Classic/current. Palette đổi không thay terminal content đang chạy.

## 6. Screen/state cần duyệt trước WP8-WP10

1. Workspace normal, one pane, expanded rail, Explorer closed.
2. Workspace 8 sessions, 2 needs, rail scrolled, stale working, failed.
3. Collapsed rail, project flyout, keyboard focus.
4. Explorer overlay, pinned, narrow window.
5. Ctrl+K empty, needs results, no result, keyboard selection.
6. Mission Control 8 sessions, 2 projects, needs và unreviewed changes.
7. Settings appearance light/dark, reduced transparency, palette, generation switch.
8. Split canvas 6 panes, active pane zoom, terminal dense, resize handle.
9. Accessibility: focus rings, high contrast, reduced motion/transparency, keyboard only.
10. Empty state, no project/session, restore prompt, PTY error.

## 7. Nguồn

- Plan: [PLAN-v102-mist-redesign.md](../PLAN-v102-mist-redesign.md)
- Coordination: [COORD-2026-11-15-v100-v102.md](../COORD-2026-11-15-v100-v102.md)
- QA images: docs/qa/v79/18-workspace-1-pane-dark.png, 19-workspace-1-pane-light.png, 20-workspace-split-2-pane.png, 21-workspace-rail-7-projects.png, docs/qa/v80/04-palette-prefix-settings.png
- Product pattern references are linked in section 4.
