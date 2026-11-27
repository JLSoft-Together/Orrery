export interface ChangeItem {
  en: string
  vi: string
}

export interface Release {
  version: string
  /** Tag date, YYYY-MM-DD. */
  date: string
  items: ChangeItem[]
}

/** Version of the running app, from apps/desktop/package.json (vite.config.ts `define`). */
export const APP_VERSION: string = __APP_VERSION__

export const SEEN_KEY = 'ccm.whatsNew.seen'

// Summaries of `git log <prev tag>..<tag>` and the matching .planning plans; newest first.
export const CHANGELOG: Release[] = [
  {
    version: '4.0.11',
    date: '2026-11-27',
    items: [
      { en: 'Three step phone connect in Settings: install over USB or scan a QR, then a connection test', vi: 'Kết nối điện thoại 3 bước trong Settings: cài qua cáp USB hoặc quét QR, rồi kiểm kết nối' },
      { en: 'The installer ships the Orrery Remote app: install over USB in one click or save the APK', vi: 'Bộ cài có sẵn app Orrery Remote: cài qua cáp bằng 1 nút hoặc lưu file APK' },
      { en: 'Settings, Devices: phones, adb devices and emulators in one list, revoke with Undo', vi: 'Settings, Thiết bị: điện thoại, máy adb và emulator chung 1 danh sách, thu hồi có Hoàn tác' },
      { en: 'Answer agent questions right from the phone notification', vi: 'Trả lời câu hỏi của agent ngay trên thông báo điện thoại' },
      { en: 'Running notice with a timer on the phone, open a pane on the phone from the pane menu', vi: 'Thông báo đang chạy có đồng hồ, mở pane trên điện thoại từ menu pane' },
      { en: 'Phone home screen widget and Quick Settings tile with the waiting count', vi: 'Widget màn hình chính và ô Quick Settings trên điện thoại hiện số session đang chờ' },
      { en: '"Waiting for you" queue and "Since you were away" card on the phone', vi: 'Hàng đợi "Chờ bạn" và thẻ "Từ lúc bạn đi vắng" trên điện thoại' },
      { en: 'Saved prompts: type ; in Ctrl+K, inserted without sending, shared with the phone', vi: 'Prompt đã lưu: gõ ; trong Ctrl+K, chèn không tự gửi, dùng chung với điện thoại' },
      { en: 'Peek a session from the rail: running tool, run time, pending question', vi: 'Xem nhanh session khi rê chuột trên rail: tool đang chạy, thời gian, câu hỏi đang chờ' },
      { en: 'Interface density: compact, regular, comfortable', vi: 'Mật độ giao diện Gọn, Vừa, Thoáng' },
      { en: 'Dangerous commands from the phone need fingerprint or PIN', vi: 'Lệnh nguy hiểm gửi từ điện thoại cần vân tay hoặc PIN' },
      { en: 'Pre-merge checklist, Run log and crash hand-off on the phone', vi: 'Checklist trước merge, log Run và gửi crash cho agent trên điện thoại' },
      { en: 'Run on several Android devices with light, dark and large font variants and a screenshot grid', vi: 'Chạy trên nhiều máy Android với biến thể sáng, tối, chữ lớn và lưới ảnh chụp' },
      { en: 'Send a freshly built APK to the phone', vi: 'Gửi APK vừa build lên điện thoại' },
      { en: 'Shared clipboard between phone and computer, only on a tap', vi: 'Clipboard chung giữa điện thoại và máy tính, chỉ khi bấm' },
      { en: 'Dictate or take a photo into a pane from the phone', vi: 'Nói ra chữ hoặc chụp ảnh gửi vào pane từ điện thoại' },
    ],
  },
  {
    version: '4.0.10',
    date: '2026-11-25',
    items: [
      { en: 'Window no longer stays covered or unclickable after minimize and restore', vi: 'Cửa sổ không còn bị phủ, không click được sau khi thu nhỏ rồi mở lại' },
      { en: 'Create, watch and merge GitHub PRs from Git Log, with CI status and a toast when CI ends', vi: 'Tạo, theo dõi và merge PR GitHub trong Git Log, có trạng thái CI và toast khi CI xong' },
      { en: 'Export a chat session as Markdown: copy, save a .md file or send it to Telegram', vi: 'Xuất session chat thành Markdown: chép, lưu file .md hoặc gửi Telegram' },
      { en: 'Worktrees set up and clean up by themselves via .orrery/worktree.json', vi: 'Worktree tự setup và tự dọn theo .orrery/worktree.json' },
      { en: 'Pre-merge checklist per worktree, agents can add todos', vi: 'Checklist trước merge theo worktree, agent thêm được việc' },
      { en: 'Git Log shows which Claude / Codex session made a commit; click opens the chat at that turn', vi: 'Git Log hiện session Claude / Codex đã làm commit; bấm mở chat đúng lượt đó' },
      { en: 'Git Stash from Changes, stash list in Git Log, undo the last unpushed commit', vi: 'Stash từ Changes, danh sách stash trong Git Log, hoàn tác commit cuối chưa push' },
      { en: 'Dev server URLs in a terminal open a preview and show a header chip', vi: 'Terminal in địa chỉ dev server thì tự mở preview và hiện chip trên header' },
      { en: 'Shell command history (Ctrl+R) and terminal bookmarks', vi: 'Lịch sử lệnh shell (Ctrl+R) và mốc terminal' },
      { en: 'Run history: reopen old logs, pin a baseline, compare two runs', vi: 'Lịch sử Run: mở lại log cũ, ghim mốc, so sánh 2 lần chạy' },
      { en: 'Launcher templates and per-project default agent', vi: 'Launcher có mẫu khởi chạy và agent mặc định theo project' },
      { en: 'Pick element sends source file, styles and a cropped image to the agent', vi: 'Chọn phần tử gửi kèm file nguồn, style và ảnh cắt cho agent' },
      { en: 'Send selected code from the editor to an agent', vi: 'Gửi đoạn code đang chọn trong editor cho agent' },
      { en: 'Tray lists sessions waiting for you; notification level per project', vi: 'Tray hiện session đang chờ; mức thông báo theo project' },
      { en: 'Footer shows only what needs attention; first-run setup moved to Settings', vi: 'Footer chỉ hiện chip cần chú ý; cài đặt lần đầu nằm trong Settings' },
      { en: 'Pane header slimmed to four buttons, tabs show one signal', vi: 'Header pane gọn còn 4 nút, tab chỉ 1 tín hiệu ưu tiên' },
      { en: 'Empty Inbox, Board and Changes now offer a next step', vi: 'Inbox, Board, Changes trống có nút việc tiếp theo' },
      { en: 'Rail filters sessions as you type, with a compact Group menu', vi: 'Rail lọc session ngay khi gõ, nút Nhóm gọn một hàng' },
    ],
  },
  {
    version: '4.0.9',
    date: '2026-11-23',
    items: [
      { en: 'Mist uses solid panels: no more blur that froze the window after many resizes', vi: 'Sương kính dùng panel đặc: bỏ blur làm cửa sổ mờ, đứng khi đổi size nhiều lần' },
      { en: 'Minimizing Orrery no longer squeezes terminals and wipes what agents printed', vi: 'Thu nhỏ Orrery không còn bóp terminal làm mất nội dung agent đã in' },
      { en: 'Phone remote never resizes the desktop terminal; phone terminal fits the width', vi: 'Điện thoại không còn đổi cỡ terminal trên máy; terminal điện thoại vừa bề ngang' },
      { en: 'One grouped notification per burst, with a badge on the taskbar / Dock', vi: 'Gộp thông báo dồn dập thành 1, kèm badge trên taskbar / Dock' },
      { en: 'Ctrl+Tab switches panes by recent use; shortcuts can be changed in Settings', vi: 'Ctrl+Tab đổi pane theo lần dùng gần nhất; đổi được phím tắt trong Cài đặt' },
      { en: 'Quota bar shows when the limit will run out at the current pace', vi: 'Thanh quota báo giờ sẽ hết theo tốc độ đang dùng' },
      { en: 'Comment on diff lines and send them to the agent as a prefilled prompt', vi: 'Góp ý trên từng dòng diff, gửi sang agent thành prompt điền sẵn' },
      { en: 'Long shell commands tell you when they finish', vi: 'Lệnh shell chạy lâu báo khi xong' },
      { en: 'Commit and push warn before a secret leaves the machine', vi: 'Commit và push cảnh báo trước khi lộ secret ra ngoài' },
      { en: 'Chat view for Claude / Codex sessions on the desktop', vi: 'Xem session Claude / Codex dạng chat ngay trên máy' },
      { en: 'Share mode hides paths, emails and pairing codes while you share the screen', vi: 'Chế độ chia sẻ che path, email, mã ghép khi chia sẻ màn hình' },
      { en: 'Search across every terminal with Ctrl+Alt+F', vi: 'Tìm trong mọi terminal bằng Ctrl+Alt+F' },
      { en: 'Footer shows machine CPU and RAM', vi: 'Footer hiện % CPU và % RAM của máy' },
      { en: 'Shorter project menu; drag to reorder projects in the rail', vi: 'Menu project gọn hơn; kéo để sắp xếp project trên rail' },
      { en: 'Older themes moved behind View all', vi: 'Theme cũ gom vào mục Xem tất cả' },
      { en: 'Electron and the Node collector are gone; the app runs on Tauri and Rust only', vi: 'Bỏ hẳn Electron và collector Node; app chỉ chạy trên Tauri + Rust' },
      { en: 'Office loads only when you turn it on, so the app opens lighter', vi: 'Văn phòng chỉ tải khi bật, mở app nhẹ hơn' },
    ],
  },
  {
    version: '4.0.8',
    date: '2026-11-20',
    items: [
      { en: 'Orrery Remote: Android app and iPhone web app to follow and drive sessions from your phone', vi: 'Orrery Remote: app Android và web app iPhone để theo dõi, điều khiển session từ điện thoại' },
      { en: 'Phone chat: inbox with waiting first, send into the session, Stop, tap to answer menus', vi: 'Chat trên điện thoại: hộp thư ưu tiên chờ bạn, gửi vào session, Dừng, chạm để trả lời menu' },
      { en: 'Remote over LAN or your own Cloudflare relay, end-to-end encrypted; one tap relay setup', vi: 'Remote qua LAN hoặc relay Cloudflare của bạn, mã hoá đầu cuối; dựng relay 1 chạm' },
      { en: 'Push notifications with secret-masked content; Remote has its own Settings page', vi: 'Push có nội dung đã che secret; Remote có trang riêng trong Cài đặt' },
      { en: 'Phone and Telegram messages go into the session even while the agent is working', vi: 'Tin nhắn từ điện thoại và Telegram vào thẳng session, kể cả khi agent đang chạy' },
      { en: 'Inbox (Ctrl+Shift+A) gathers menus, permissions, questions, file clashes, errors and done', vi: 'Inbox (Ctrl+Shift+A) gom menu, xin quyền, câu hỏi, đụng file, lỗi và việc xong' },
      { en: 'A warning when two sessions edit the same file; Changes can show one session\'s edits', vi: 'Cảnh báo khi 2 session sửa cùng 1 file; Changes xem được phần sửa của từng session' },
      { en: 'New UI is the default: rail rows show status, running tool and time', vi: 'Giao diện mới là mặc định: rail hiện trạng thái, tool đang chạy và thời gian' },
      { en: 'New UI: file, Changes and log panes sit under a collapsible Tools line in the rail', vi: 'Giao diện mới: pane file, Changes, log nằm trong nhóm Công cụ thu gọn được trên rail' },
      { en: 'Settings regrouped into 8 pages, with search', vi: 'Cài đặt gom lại còn 8 trang, có ô tìm kiếm' },
      { en: 'MCP & plugins: tick your changes, then apply them all at once', vi: 'MCP & plugin: đánh dấu thay đổi rồi áp dụng một lần' },
      { en: 'Colours: plain Đen trắng palette and Ngẫu nhiên, a new palette at every launch', vi: 'Màu: bảng Đen trắng và Ngẫu nhiên, mỗi lần mở app một bảng màu mới' },
      { en: 'Classic UI keeps Orrery light, Orrery dark and Ngân hà; other classic themes were removed', vi: 'Giao diện cũ còn Orrery sáng, Orrery tối, Ngân hà; các theme cũ khác đã bỏ' },
      { en: 'Disk: scans and idle rescans cover only the drives you picked', vi: 'Disk: quét và quét lại khi rảnh chỉ trên các ổ đã chọn' },
      { en: 'macOS: Cmd+Q asks before closing running terminals; the Dock icon reopens the window', vi: 'macOS: Cmd+Q hỏi trước khi đóng terminal đang chạy; bấm icon Dock mở lại cửa sổ' },
      { en: 'macOS: Esc closes dialogs, Cmd+K works inside terminals', vi: 'macOS: Esc đóng hộp thoại, Cmd+K dùng được trong terminal' },
      { en: 'Windows: Ctrl+C reaches programs running in terminal panes', vi: 'Windows: Ctrl+C tới được chương trình đang chạy trong pane terminal' },
      { en: 'Fixed a tab bar freeze that could leave a file pane empty', vi: 'Sửa lỗi thanh tab treo làm pane file trống' },
      { en: 'The classic UI will be removed in 4.1.0', vi: 'Giao diện cũ sẽ bỏ ở bản 4.1.0' },
    ],
  },
  {
    version: '4.0.7',
    date: '2026-11-16',
    items: [
      { en: 'Smaller minimize, maximize and close buttons that match the app theme', vi: 'Nút thu nhỏ, phóng to, đóng cửa sổ nhỏ gọn, theo màu theme của app' },
      { en: 'New UI: the close button no longer sits on the tab bar; drag the window by its top strip', vi: 'Giao diện mới: nút đóng không còn đè thanh tab; kéo cửa sổ ở dải trên cùng' },
      { en: 'New UI: + in the session rail opens a project folder', vi: 'Giao diện mới: nút + trên rail session để mở thư mục project' },
      { en: 'New UI: the + menu and rail menus show above terminals, blurred and uncut', vi: 'Giao diện mới: menu + và menu trên rail hiện trên terminal, mờ kính đúng, không bị cắt' },
      { en: 'New UI: Limits and status opens upward in full instead of being cut off', vi: 'Giao diện mới: Hạn mức và trạng thái mở lên trên, hiện đủ, không bị cắt' },
      { en: 'New UI: the same 12px spacing around the rail, tab bar and panes', vi: 'Giao diện mới: khoảng cách 12px đều nhau quanh rail, thanh tab và pane' },
      { en: 'Pane header: Attachments and MCP & plugins are one click, next to the branch', vi: 'Header pane: nút Đính kèm và MCP & plugin bấm thẳng, cạnh branch git' },
    ],
  },
  {
    version: '4.0.6',
    date: '2026-11-16',
    items: [
      { en: 'Try the new Mist glass UI: session rail, Explorer drawer, pane drag compass, 9 palettes', vi: 'Thử giao diện Sương kính: rail session, Explorer ngăn kéo, la bàn kéo pane, 9 bảng màu' },
      { en: 'New UI: custom theme with a color picker, light and dark made for you', vi: 'Giao diện mới: theme tự chọn với color picker, tự sinh bản sáng và tối' },
      { en: 'Click or double click a Claude / Codex answer in the pane; Waiting chip, toasts with buttons', vi: 'Click hoặc double click đáp án Claude / Codex trong pane; chip Chờ trả lời, toast có nút' },
      { en: 'New Disk view: scan, treemap, file search, clean up, duplicates, sort Downloads, uninstall', vi: 'Ổ đĩa mới: quét, treemap, tìm file, dọn rác, file trùng, sắp xếp Downloads, gỡ app' },
      { en: 'Every dropdown shares one style and works from the keyboard', vi: 'Mọi dropdown dùng chung một kiểu, dùng được bằng bàn phím' },
      { en: 'Tauri: MCP and Telegram into sessions work while the compact window is open', vi: 'Tauri: MCP và Telegram vào session chạy lại khi mở cửa sổ compact' },
      { en: 'Tauri: asks before quitting, saves unsaved files; links open when the app is closed', vi: 'Tauri: hỏi trước khi thoát, lưu file chưa lưu; link mở được khi app chưa chạy' },
    ],
  },
  {
    version: '4.0.5',
    date: '2026-11-13',
    items: [
      { en: 'Workspace: Chrome-style project groups, several projects side by side in one tab', vi: 'Workspace: nhóm project kiểu Chrome, nhiều project cạnh nhau trong một tab' },
      { en: 'Project groups: header menu, colours, collapse, Ctrl+Alt+G, saved layouts', vi: 'Nhóm project: menu header, màu, thu gọn, Ctrl+Alt+G, lưu layout' },
      { en: 'Sidebar: Ctrl/Shift+click several sessions, then Group / Ungroup / Close', vi: 'Sidebar: Ctrl/Shift+click chọn nhiều session rồi Group / Ungroup / Đóng' },
      { en: 'Drag a project from the sidebar onto a pane to put it beside', vi: 'Kéo project từ sidebar thả vào pane để mở cạnh bên' },
      { en: 'Galaxy: bigger twinkling stars and meteors, Star effect in Appearance', vi: 'Ngân hà: sao to nhấp nháy và sao băng, chọn ở Giao diện → Hiệu ứng sao' },
      { en: 'Fix: in-app notifications were blocked on Windows', vi: 'Sửa: thông báo trong app bị chặn trên Windows' },
      { en: 'Fix: pasted images had no preview in Attachments', vi: 'Sửa: ảnh paste không có preview trong Attachments' },
    ],
  },
  {
    version: '4.0.4',
    date: '2026-11-13',
    items: [
      { en: 'Galaxy theme: stars inside Workspace panels and terminals, switch in Appearance', vi: 'Theme Ngân hà: sao trong panel Workspace và terminal, bật/tắt ở Giao diện' },
      { en: 'Galaxy: brighter stars, thin terminal scrollbar', vi: 'Ngân hà: sao sáng hơn, scrollbar terminal mảnh' },
    ],
  },
  {
    version: '4.0.3',
    date: '2026-11-12',
    items: [
      { en: 'macOS: dragging files from Finder or Explorer into a pane inserts @path again', vi: 'macOS: kéo file từ Finder hoặc Explorer vào pane lại ra @path' },
    ],
  },
  {
    version: '4.0.2',
    date: '2026-11-11',
    items: [
      { en: 'New default theme Ngân hà (Galaxy): deep black with faint static stars; Orrery stays for light', vi: 'Theme mặc định mới Ngân hà: đen sâu, sao mờ tĩnh; Orrery cho nền sáng' },
      { en: 'Pane colours by agent: Claude orange, Codex purple, shell teal; status bar at the bottom', vi: 'Màu pane theo agent: Claude cam, Codex tím, shell xanh ngọc; thanh trạng thái ở đáy' },
      { en: 'Monitor Board: richer cards, empty columns fold, group by status, project or agent', vi: 'Giám sát › Bảng: thẻ đủ thông tin, cột rỗng thu gọn, nhóm theo trạng thái / project / agent' },
      { en: 'Monitor Orbit is a star map: one system per project, sessions are suns, subagents planets', vi: 'Quỹ đạo thành bản đồ sao: mỗi project một hệ, session là mặt trời, subagent là hành tinh' },
      { en: 'Pane header shows running subagents and a context meter; a card lists files of the last turn', vi: 'Header pane hiện subagent đang chạy và thước context; thẻ liệt kê file của lượt vừa rồi' },
      { en: 'Tabs group by project with +N overflow; focus mode (Ctrl+.) hides rail and Explorer', vi: 'Tab nhóm theo project, tràn thành +N; chế độ tập trung (Ctrl+.) ẩn rail và Explorer' },
      { en: 'New tab opens a launcher to pick agent and project; Explorer marks files agents touch', vi: 'Tab mới mở launcher chọn agent và project; Explorer đánh dấu file agent đang sửa / đọc' },
      { en: 'A starting animation shows while a Claude or Codex pane boots', vi: 'Pane Claude / Codex có hiệu ứng đang khởi động trong lúc agent mở lên' },
      { en: 'Monitor status bar shows tokens used today; Settings theme cards preview each theme', vi: 'Thanh đáy Giám sát hiện token hôm nay; Settings có thẻ theme kèm preview' },
      { en: 'Per-session MCP & plugins sheet: turn servers or plugins off for one project', vi: 'Bảng MCP & plugin theo session: tắt server hay plugin cho riêng một project' },
      { en: 'Context warning chip and toast past 500K tokens, with a Compact… button', vi: 'Chip và toast cảnh báo context khi vượt 500K token, kèm nút Compact…' },
      { en: 'Bug report in Settings → About: one .zip with logs, secrets masked', vi: 'Báo lỗi trong Settings → Giới thiệu: 1 file .zip gồm log, đã che secret' },
      { en: 'Vietnamese typing in terminal panes fixed on Windows and macOS', vi: 'Sửa gõ tiếng Việt trong terminal trên Windows và macOS' },
      { en: 'Windows: dropping files from Explorer into a pane inserts @path again', vi: 'Windows: kéo file từ Explorer vào pane lại ra @path' },
    ],
  },
  {
    version: '4.0.1',
    date: '2026-11-11',
    items: [
      { en: 'Windows: Monitor no longer stuck on "Connection lost" after updating to 4.0.0', vi: 'Windows: Monitor hết kẹt "Mất kết nối" sau khi lên 4.0.0' },
      { en: 'Errors are written to shell.log in the data folder (tray → Open log folder)', vi: 'Lỗi được ghi vào shell.log trong thư mục dữ liệu (tray → Mở thư mục log)' },
    ],
  },
  {
    version: '4.0.0',
    date: '2026-11-11',
    items: [
      { en: 'Monitor Board tab: Needs you, Running, Idle, Done today; Alt+1 / Alt+2 switch to Orbit', vi: 'Monitor có tab Bảng: Cần anh, Đang chạy, Rảnh, Xong hôm nay; Alt+1 / Alt+2 chuyển sang Quỹ đạo' },
      { en: 'Orbit tab: priority rings around you, detail panel, focus view with tool arcs', vi: 'Tab Quỹ đạo: vòng ưu tiên quanh bạn, panel chi tiết, màn tiêu điểm có vệt tool' },
      { en: 'Running commands panel: full command (secrets masked), run time, open pane; ` toggles it', vi: 'Panel "Lệnh đang chạy": lệnh đầy đủ (che secret), thời gian chạy, mở pane; phím ` để ẩn / hiện' },
      { en: 'Runs on Tauri + Rust (no Electron / Node): RAM −53 to −59%, CPU up to −72%, installer 147 → 24 MB', vi: 'Chạy bằng Tauri + Rust (bỏ Electron / Node): RAM −53 đến −59%, CPU max −72%, bộ cài 147 → 24 MB' },
      { en: 'Telegram: sign in to your account once more for topics and large files; Office not in this version', vi: 'Telegram: đăng nhập lại tài khoản 1 lần để quét topic và gửi file lớn; bản này chưa có Office' },
      { en: 'XML Preview installs on demand; no blur; Linux and Windows portable builds dropped', vi: 'XML Preview chỉ cài khi cần; bỏ blur; bỏ bản Linux và bản Windows portable' },
    ],
  },
  {
    version: '3.4.0',
    date: '2026-11-08',
    items: [
      { en: 'Agents can open a pane and start the prompt themselves (open_session submit: true)', vi: 'Agent mở pane mới và tự gửi prompt luôn (open_session submit: true)' },
      { en: 'open_session returns the pane id; new wait_session waits until a pane is idle or closed', vi: 'open_session trả id pane; tool mới wait_session chờ pane rảnh hoặc đóng' },
      { en: 'list_sessions shows when a Claude pane hit its usage limit and since when a pane is idle', vi: 'list_sessions báo pane Claude chạm usage limit và pane rảnh từ lúc nào' },
      { en: 'A workflow can write .orrery/status.json: phase and progress show on the pane and card', vi: 'Workflow ghi .orrery/status.json: phase và tiến độ hiện trên pane và thẻ' },
      { en: 'run_log filters lines by a regex and reads only new lines with a cursor', vi: 'run_log lọc dòng theo regex và chỉ đọc dòng mới bằng cursor' },
      { en: 'Telegram: agents can add reply buttons; your tap goes to that pane, sent only when idle', vi: 'Telegram: agent gắn button trả lời; bạn bấm thì chữ vào pane đó, chỉ gửi khi pane rảnh' },
      { en: 'Resources: system services no longer show as an agent\'s servers', vi: 'Tài nguyên: service hệ thống không còn bị tính là server của agent' },
    ],
  },
  {
    version: '3.3.1',
    date: '2026-11-06',
    items: [
      { en: 'Telegram: pick a topic from a list, live session state, drag and drop with Undo', vi: 'Telegram: chọn topic trong danh sách, trạng thái session live, kéo-thả có Hoàn tác' },
      { en: 'Sign in with your Telegram account to list every topic (messages still go via the bot)', vi: 'Đăng nhập tài khoản Telegram để thấy đủ topic (tin nhắn vẫn do bot gửi)' },
      { en: 'Telegram: Esc and "last 20 lines" buttons; /compact, /close and /focus; /new picks a project', vi: 'Telegram: nút Esc và "20 dòng cuối"; lệnh /compact, /close, /focus; /new chọn project' },
      { en: 'Telegram: one pinned status per topic, send health per topic, limit alerts to the topic', vi: 'Telegram: 1 tin ghim trạng thái mỗi topic, sức khoẻ gửi tin, cảnh báo hạn mức vào topic' },
      { en: 'Telegram: photos from the phone go into the Claude pane; ✈ opens the topic in Telegram', vi: 'Telegram: ảnh từ điện thoại vào pane Claude; ✈ mở topic trong Telegram' },
      { en: 'Compact sends /compact and the resume line by itself (switch in Settings)', vi: 'Compact tự gửi /compact và dòng làm tiếp (công tắc trong Cài đặt)' },
      { en: 'One gateway for menus, links, the bot and agent tools; agents can list and message sessions', vi: 'Một cổng chung cho menu, link, bot và tool agent; agent liệt kê và nhắn session khác' },
      { en: 'claude / codex typed in a shell pane get the Orrery tools and resume like an agent pane', vi: 'claude / codex gõ trong pane shell có tool Orrery và resume như pane agent' },
    ],
  },
  {
    version: '3.3.0',
    date: '2026-11-05',
    items: [
      { en: 'Telegram: a session\'s topic is freed when the session ends or /clear, unless 📌 Keep', vi: 'Telegram: topic của session tự ngắt khi session kết thúc hoặc /clear, trừ khi 📌 Giữ' },
      { en: 'Freed topics get Resume and Connect buttons; dead sessions are freed after a restart', vi: 'Topic vừa ngắt có nút Resume và Nối; session chết lúc tắt app được ngắt khi mở lại' },
      { en: 'New Topics tab: topic map with filter, keep, test, show pane and disconnect', vi: 'Tab Topic mới: bản đồ topic có lọc, giữ, gửi thử, mở pane và ngắt' },
      { en: 'Topics the bot can no longer post to are marked; "where" tells what a topic is on', vi: 'Topic bot không gửi được được đánh dấu; lệnh "where" cho biết topic đang nối gì' },
      { en: 'Optional topic icons: ⚡ working, ❓ waiting for you, ✔ idle', vi: 'Tuỳ chọn icon topic: ⚡ đang làm, ❓ chờ bạn, ✔ rảnh' },
      { en: 'Setup steps strip in Settings › Telegram; the ✈ mark opens the link dialog', vi: 'Dải bước cài đặt ở Cài đặt › Telegram; dấu ✈ mở hộp nối topic' },
    ],
  },
  {
    version: '3.2.0',
    date: '2026-11-05',
    items: [
      { en: 'Telegram bot: reports go to each project or session topic, with a pinned status', vi: 'Bot Telegram: báo vào topic của từng dự án hoặc session, kèm tin ghim trạng thái' },
      { en: 'Command sessions from Telegram: status, new, stop, last, tail, run, screen and the "/" menu', vi: 'Ra lệnh từ Telegram: status, new, stop, last, tail, run, screen và menu "/"' },
      { en: 'Telegram buttons allow / deny tools and answer Claude\'s questions', vi: 'Nút Telegram cho phép / từ chối tool và trả lời câu hỏi của Claude' },
      { en: 'Send photos and files into a session; agents send files back, up to 2 GB', vi: 'Gửi ảnh, file vào session; agent gửi file về, tới 2 GB' },
      { en: 'Connect a topic to a session or project from its menu or from Telegram itself', vi: 'Nối topic với session hoặc dự án từ menu chuột phải hoặc ngay trong Telegram' },
      { en: 'Only your paired account commands the bot; add members with view or full access', vi: 'Chỉ tài khoản đã ghép ra lệnh; thêm thành viên quyền xem hoặc đầy đủ' },
      { en: 'Commands sent while Orrery was off are asked about before they run', vi: 'Lệnh gửi lúc Orrery tắt được hỏi lại trước khi chạy' },
      { en: 'Settings window is larger and resizable; Telegram has its own page', vi: 'Cửa sổ Cài đặt lớn hơn, kéo giãn được; Telegram có trang riêng' },
    ],
  },
  {
    version: '3.1.1',
    date: '2026-11-03',
    items: [
      { en: 'Monitor shows DeepSeek CLI (Codewhale) sessions from its outbox file', vi: 'Monitor hiện session DeepSeek CLI (Codewhale) từ file outbox' },
      { en: 'DeepSeek panes and setup are built in but off; nothing calls DeepSeek', vi: 'Pane và cài đặt DeepSeek có sẵn nhưng tắt; app không gọi DeepSeek' },
      { en: 'Internal: Codex process scan shared with other agent CLIs', vi: 'Nội bộ: quét process Codex dùng chung cho các CLI agent khác' },
    ],
  },
  {
    version: '3.1.0',
    date: '2026-11-03',
    items: [
      { en: 'Agents can notify you, open a file or diff for you and ask who is editing a file', vi: 'Agent có thể báo chủ nhân, mở file hoặc diff cho chủ nhân xem và hỏi ai đang sửa file' },
      { en: 'Agent tools for Run: start / reload the app, read its log, screenshot the device', vi: 'Tool Run cho agent: chạy / reload app, đọc log, chụp màn thiết bị' },
      { en: 'Agent tools: render an XML layout, list and stop the ports a pane left open', vi: 'Tool cho agent: dựng layout XML, xem và dừng cổng pane còn mở' },
      { en: 'Codex panes get the Orrery MCP tools too', vi: 'Pane Codex cũng dùng được tool MCP của Orrery' },
      { en: 'Changes lists unreviewed files first; History can continue an ended session', vi: 'Changes đưa file chưa xem lên đầu; History có nút Tiếp tục session đã kết thúc' },
      { en: 'Run can reload when the agent finishes and remembers each project\'s device', vi: 'Run có thể tự reload khi agent xong và nhớ thiết bị của từng dự án' },
      { en: 'Limits warn ahead with the time they are forecast to run out', vi: 'Hạn mức báo trước giờ dự kiến hết' },
      { en: 'Ctrl+K searches sessions, projects, files, settings and actions in one box', vi: 'Ctrl+K tìm session, dự án, file, cài đặt và hành động trong một ô' },
      { en: 'Agent Map shows each sub-agent\'s recent tool calls', vi: 'Agent Map hiện các lệnh gọi tool gần đây của từng sub-agent' },
      { en: 'One toast stack for undo, break and phone notices; rarely used dialogs load later', vi: 'Một chồng toast chung cho hoàn tác, nhắc nghỉ, điện thoại; hộp thoại ít dùng tải sau' },
    ],
  },
  {
    version: '3.0.0',
    date: '2026-11-03',
    items: [
      { en: 'Settings is a window: ten pages, a search box and one table of events by channel', vi: 'Cài đặt là một cửa sổ: mười trang, ô tìm và một bảng sự kiện theo kênh báo' },
      { en: 'Monitor: sessions grouped by need, three-line cards, ⋯ menu for the extra actions', vi: 'Monitor: session chia theo việc cần làm, thẻ ba dòng, thao tác phụ trong menu ⋯' },
      { en: 'Agent Map keeps colour only on nodes waiting for you', vi: 'Agent Map chỉ tô màu node đang chờ bạn' },
      { en: 'Workspace rail: one line per session, agents first, filter box, Open project icon', vi: 'Thanh bên Workspace: mỗi phiên một dòng, agent lên đầu, ô lọc, nút mở dự án dạng icon' },
      { en: 'Pane headers are quieter; a lone pane lends its header to the tab bar', vi: 'Header pane gọn hơn; pane đứng một mình đưa thông tin lên thanh tab' },
      { en: 'Workspace tab bar has a single ⋯ button; one-pane projects take one rail row', vi: 'Thanh tab Workspace chỉ còn một nút ⋯; dự án một pane chiếm một dòng ở thanh bên' },
      { en: 'Top bar: limits, keep-awake, disk, resources and connection in one Limits button', vi: 'Thanh trên: hạn mức, giữ máy thức, ổ đĩa, tài nguyên và kết nối trong một nút Hạn mức' },
      { en: 'Limits shows the lowest percent left; Dim, Shortcuts and Guide moved into the ⋯ menu', vi: 'Hạn mức hiện % còn lại thấp nhất; Dim, Phím tắt và Hướng dẫn chuyển vào menu ⋯' },
      { en: 'Cleaner History, Usage, Devices, Extensions and Git branches toolbar', vi: 'Gọn lại Lịch sử, Thống kê, Thiết bị, Tiện ích và thanh nhánh Git Log' },
      { en: 'Status labels are fully Vietnamese in the Vietnamese UI', vi: 'Nhãn trạng thái hoàn toàn bằng tiếng Việt khi dùng giao diện tiếng Việt' },
    ],
  },
  {
    version: '2.6.1',
    date: '2026-11-02',
    items: [
      { en: 'Go to project (Ctrl+Alt+P): fuzzy-find open, closed and favourite projects', vi: 'Chuyển tới dự án (Ctrl+Alt+P): tìm nhanh dự án đang mở, đã đóng và yêu thích' },
      { en: 'Hold Ctrl+Alt and press P again to step through projects; let go to jump', vi: 'Giữ Ctrl+Alt, bấm P tiếp để chọn dự án; thả phím là chuyển' },
      { en: 'Project list shows branch, last used, run state and agents that need you', vi: 'Danh sách dự án hiện nhánh, lần dùng gần nhất, trạng thái Run và agent cần bạn' },
      { en: 'Rail: collapse other projects, or collapse them on every project switch', vi: 'Thanh bên: thu gọn dự án khác, hoặc tự thu gọn mỗi khi chuyển dự án' },
    ],
  },
  {
    version: '2.6.0',
    date: '2026-11-01',
    items: [
      { en: 'Resources: RAM / CPU of sessions, servers, daemons, emulators; alert and stop forgotten ones', vi: 'Tài nguyên: RAM / CPU của session, server, daemon, emulator; báo và tắt thứ bị bỏ quên' },
      { en: 'Listening ports with their process; name the holder when a port is already in use', vi: 'Cổng đang lắng nghe kèm process; báo process giữ cổng khi cổng đã bị chiếm' },
      { en: 'Keep the computer awake while agents work (only on power by default)', vi: 'Giữ máy thức khi agent đang làm (mặc định chỉ khi cắm sạc)' },
      { en: 'Capture folder: recordings kept, screenshots reach agents as files', vi: 'Thư mục chụp / quay: giữ video quay, ảnh chụp tới agent dạng file' },
      { en: 'Sensitive watch: warn on .env / keystore edits, force push, deletes outside the project', vi: 'Canh nhạy cảm: cảnh báo khi sửa .env / keystore, force push, xoá ngoài project' },
      { en: 'Block next time: turn a warning into a Claude Code deny rule after a confirm', vi: 'Chặn lần sau: biến cảnh báo thành luật deny của Claude Code sau khi xác nhận' },
      { en: 'Phone notifications through ntfy or Discord when a session waits and you are away', vi: 'Báo ra điện thoại qua ntfy hoặc Discord khi session chờ mà bạn rời máy' },
      { en: 'Agent data: measure ~/.claude and ~/.codex, clean caches, move them to another drive', vi: 'Dữ liệu agent: đo ~/.claude và ~/.codex, dọn cache, chuyển sang ổ khác' },
      { en: 'Clean build output: node_modules, build, .gradle, dist, target of your projects', vi: 'Dọn đồ build: node_modules, build, .gradle, dist, target của các project' },
      { en: 'Low disk warning: top bar badge and one alert when a drive gets below 10 GB or 5 %', vi: 'Cảnh báo ổ đầy: badge trên thanh trên và một thông báo khi ổ dưới 10 GB hoặc 5 %' },
      { en: 'Prompt library: 12 ready-made prompts, export and import as JSON', vi: 'Thư viện prompt: 12 mẫu dựng sẵn, xuất và nhập bằng JSON' },
      { en: 'Reopen the last workspace after a restart or crash; shells return to their folder', vi: 'Mở lại workspace sau khi khởi động lại hoặc crash; shell về đúng thư mục' },
      { en: 'History: find a past session by what you asked and resume it', vi: 'History: tìm session cũ theo những gì bạn đã hỏi rồi tiếp tục' },
      { en: 'CLAUDE.md / memory check: broken links, stale paths, oversized files', vi: 'Kiểm CLAUDE.md / memory: link gãy, đường dẫn cũ, file quá lớn' },
      { en: 'Library updates: known vulnerabilities from osv.dev with the fixed version', vi: 'Kiểm bản mới thư viện: lỗ hổng đã biết từ osv.dev kèm bản đã vá' },
      { en: 'Release check for Android / Flutter: versionCode, targetSdk, manifest, signing key', vi: 'Kiểm phát hành Android / Flutter: versionCode, targetSdk, manifest, khoá ký' },
      { en: 'Devices: accessibility check on screenshots, screen recording up to 30 s', vi: 'Thiết bị: kiểm trợ năng trên ảnh chụp, quay màn tối đa 30 s' },
      { en: 'Devices: app performance (janky frames, memory, cold start) and a file browser', vi: 'Thiết bị: đo hiệu năng app (giật khung, bộ nhớ, mở nguội) và duyệt file' },
    ],
  },
  {
    version: '2.5.0',
    date: '2026-10-29',
    items: [
      { en: 'In-app updates: notice of a new release, download, verify and restart to install', vi: 'Cập nhật trong app: báo bản mới, tải, kiểm tra file rồi khởi động lại để cài' },
      { en: 'Devices › Emulators: list AVDs, start, cold boot, stop and wipe them', vi: 'Thiết bị › Emulator: liệt kê AVD, chạy, cold boot, tắt và xoá dữ liệu' },
      { en: 'Settings › Backup: export your setup to a file and import it on another machine', vi: 'Cài đặt › Sao lưu: xuất cấu hình ra file và nhập lại trên máy khác' },
      { en: 'Strings table for strings.xml and ARB: missing keys, placeholder checks, safe edits', vi: 'Bảng chuỗi cho strings.xml và ARB: key thiếu, kiểm placeholder, sửa an toàn' },
      { en: 'Check libs.versions.toml, pubspec.yaml or package.json for newer library versions', vi: 'Kiểm bản mới của thư viện trong libs.versions.toml, pubspec.yaml, package.json' },
      { en: 'Extensions › Hooks: Claude Code hooks by event, turn them off and on, test run', vi: 'Tiện ích › Hook: hook Claude Code theo sự kiện, tắt / bật, chạy thử' },
      { en: 'Shell panes: command marks with exit code and time, Ctrl/⌘+↑↓ jumps, fill output to agent', vi: 'Pane shell: đánh dấu lệnh kèm mã thoát, thời gian, Ctrl/⌘+↑↓ nhảy lệnh, điền output cho agent' },
      { en: 'Agent stopped at the usage limit: schedule a continue text filled in at the reset', vi: 'Agent dừng vì hạn mức: hẹn điền sẵn chữ tiếp tục lúc reset' },
      { en: 'History: top tools, skills, subagent types and MCP servers per project, with unused ones', vi: 'History: tool, skill, loại subagent, MCP dùng nhiều nhất theo dự án, kèm mục không dùng' },
      { en: 'APK / AAB size: groups, largest files, manifest, compare with the previous build', vi: 'Kích thước APK / AAB: nhóm, file lớn nhất, manifest, so với bản build trước' },
      { en: "Devices: browse a debug app's SQLite and SharedPreferences, edit prefs with restore", vi: 'Thiết bị: xem SQLite và SharedPreferences của app debug, sửa prefs có nút khôi phục' },
      { en: 'Vietnamese is now the default language for new installs', vi: 'Tiếng Việt là ngôn ngữ mặc định khi cài mới' },
    ],
  },
  {
    version: '2.4.0',
    date: '2026-10-29',
    items: [
      { en: "What's new dialog, also opened from Settings and Ctrl+K", vi: 'Hộp "Có gì mới", mở lại được trong Cài đặt và Ctrl+K' },
      { en: 'Explorer Search, Checks and TODO views; Replace in files, previewed per line', vi: 'Explorer có Tìm kiếm, Checks và TODO; Thay trong nhiều file, xem trước từng dòng' },
      { en: 'Stage, commit and blame in the app; merge conflicts solved block by block', vi: 'Stage, commit và blame ngay trong app; giải xung đột gộp theo từng khối' },
      { en: 'Checkpoints per turn and Turn review to keep or drop each hunk', vi: 'Checkpoint theo lượt và Xem lượt này để giữ hoặc bỏ từng đoạn' },
      { en: 'Compare and merge worktrees; a warning when agents edit the same file', vi: 'So sánh và gộp worktree; cảnh báo khi các agent cùng sửa một file' },
      { en: 'Prompt library, a prompt queue per agent pane and an inbox of what needs you', vi: 'Thư viện prompt, hàng chờ prompt cho từng pane agent và hộp việc cần chủ nhân' },
      { en: 'Editor: Outline, Go to Symbol, bookmarks, recent files, Local History, Format', vi: 'Editor: Outline, Go to Symbol, bookmark, file gần đây, Local History, Format' },
      { en: 'JSON / YAML tree, log viewer, .http requests, Markdown split preview, colour swatches', vi: 'Cây JSON / YAML, xem log, gửi request .http, xem trước Markdown song song, ô màu' },
      { en: 'Drop-down terminal (Ctrl+`), project notes, Plan board and terminal errors in Problems', vi: 'Terminal thả xuống (Ctrl+`), ghi chú dự án, Plan board và lỗi terminal vào Problems' },
      { en: 'Ctrl+K developer tools, file and image compare, .env check and i18n key scan', vi: 'Công cụ dev trong Ctrl+K, so file và ảnh, kiểm .env và quét khoá i18n' },
      { en: 'Device screenshot from Devices or the Run pane, attached to an agent pane', vi: 'Chụp màn thiết bị từ tab Thiết bị hoặc Run pane, đính kèm vào pane agent' },
      { en: 'Inspect views on the screenshot: id, text, bounds, Find in files or fill into an agent', vi: 'Soi view trên ảnh chụp: id, chữ, khung, tìm trong file hoặc điền vào agent' },
      { en: 'Test UI on a device: dark mode, font size, display size, app language, animations off, Restore all', vi: 'Thử UI trên máy: giao diện tối, cỡ chữ, cỡ hiển thị, ngôn ngữ app, tắt animation, Trả về như cũ' },
      { en: "Open the project's deep links and exported activities on a device, or type a URI", vi: 'Mở deep link và activity exported của dự án trên máy, hoặc tự nhập URI' },
      { en: 'Run pane catches Android and Flutter crashes and ANRs; project frames open the file', vi: 'Run pane bắt crash và ANR của Android, Flutter; frame trong dự án bấm là mở file' },
      { en: 'Fill a crash into the agent: exception, project frames, log before, secrets masked', vi: 'Điền crash vào agent: exception, frame của dự án, log trước đó, đã che bí mật' },
      { en: 'Permission prompts per project: repeated asks and allow rules to tick', vi: 'Hỏi quyền theo dự án: các lần hỏi lặp lại và luật allow để tích chọn' },
      { en: 'Ticked rules go to settings.local.json after a confirm with the exact diff', vi: 'Luật đã tích được ghi vào settings.local.json sau khi xác nhận đúng phần thay đổi' },
      { en: 'Background context of a session: first-reply tokens and CLAUDE.md, memory, skills sizes', vi: 'Ngữ cảnh nền của session: token lượt đầu và kích thước CLAUDE.md, memory, skill' },
    ],
  },
  {
    version: '2.3.0',
    date: '2026-10-28',
    items: [
      { en: 'Auto save in the file editor, like VS Code (off by default)', vi: 'Tự lưu trong editor như VS Code (mặc định tắt)' },
      { en: 'HTML files render in the file viewer like Markdown', vi: 'File HTML hiển thị trong trình xem file như Markdown' },
      { en: 'Office theme packs, full screen and a 3D office style (beta)', vi: 'Gói giao diện Office, toàn màn hình và kiểu văn phòng 3D (beta)' },
      { en: 'The header now sits in the window title bar', vi: 'Thanh header nằm luôn trên thanh tiêu đề cửa sổ' },
      { en: 'Restore the workspace with only the projects you pick running', vi: 'Khôi phục workspace chỉ chạy những dự án chủ nhân chọn' },
      { en: 'Simpler Monitor: session bar, activity panel and timeline view removed', vi: 'Monitor gọn hơn: bỏ thanh session, bảng hoạt động và chế độ timeline' },
      { en: 'Lighter Agent Map and Office: fewer reflows, calmer motion', vi: 'Agent Map và Office nhẹ hơn: ít tính lại layout, chuyển động êm hơn' },
      { en: 'Atom Material icons for file panes and project folders', vi: 'Icon Atom Material cho pane file và thư mục dự án' },
      { en: 'Fixed duplicated shell prompts when resizing terminal panes', vi: 'Sửa lỗi lặp dấu nhắc shell khi đổi cỡ pane terminal' },
    ],
  },
  {
    version: '2.2.0',
    date: '2026-10-22',
    items: [
      { en: 'Office view in the Monitor: a pixel-art room per project, a character per agent', vi: 'Office trong Monitor: mỗi dự án một phòng pixel-art, mỗi agent một nhân vật' },
      { en: 'Office day timelapse, sound cues and a volume slider', vi: 'Office có timelapse cả ngày, âm báo và thanh chỉnh âm lượng' },
      { en: 'Click an asking agent in the Office to jump to its session', vi: 'Bấm agent đang hỏi trong Office để nhảy tới session của nó' },
      { en: 'Devices tab: connect Android devices over USB or Wi-Fi, by pair code or QR', vi: 'Tab Thiết bị: kết nối máy Android qua USB hoặc Wi-Fi, bằng mã ghép hoặc QR' },
      { en: 'Remembered devices with aliases and auto-connect', vi: 'Ghi nhớ thiết bị, đặt tên riêng và tự kết nối' },
      { en: 'Compact… works for claude typed in a shell pane', vi: 'Compact… dùng được cho claude gõ trong pane shell' },
      { en: 'Terminal panes fall back to installed Nerd Fonts', vi: 'Pane terminal dùng Nerd Font đã cài khi thiếu font' },
    ],
  },
  {
    version: '2.1.0',
    date: '2026-10-18',
    items: [
      { en: 'Renamed to Orrery, with a new logo', vi: 'Đổi tên thành Orrery, kèm logo mới' },
      { en: 'Several GitHub accounts in Settings, used per repo for push, fetch and pull', vi: 'Nhiều tài khoản GitHub trong Cài đặt, dùng theo từng repo khi push, fetch, pull' },
      { en: 'Push dialog in the Log Branches toolbar and a ↑N push button in Changes', vi: 'Hộp Push trên thanh Branches của Log và nút ↑N để push trong Changes' },
      { en: 'The Explorer has its own panel on the right; each file opens in its own viewer', vi: 'Explorer có panel riêng bên phải; mỗi file mở trong trình xem riêng' },
      { en: 'Atom Material file icons in the Explorer tree', vi: 'Icon file Atom Material trong cây Explorer' },
      { en: 'A shell pane running claude or codex shows as that agent', vi: 'Pane shell đang chạy claude hoặc codex được nhận là agent đó' },
      { en: 'Wrapped links in terminal panes open and copy correctly', vi: 'Link bị xuống dòng trong pane terminal mở và chép đúng' },
      { en: 'Flutter Widget Preview lists the widgets of the open Dart file', vi: 'Flutter Widget Preview liệt kê widget của file Dart đang mở' },
    ],
  },
  {
    version: '2.0.0',
    date: '2026-10-16',
    items: [
      { en: 'Desktop-only app with a Workspace of terminals: split panes, tabs and groups', vi: 'Chỉ còn app desktop, có Workspace terminal: chia pane, tab và nhóm tab' },
      { en: 'Browser, file and Changes panes; file Explorer with drag to @mention', vi: 'Pane trình duyệt, file và Changes; Explorer kéo file vào để @mention' },
      { en: 'Project sidebar, saved layouts, scripts menu and multi-project workspaces', vi: 'Sidebar dự án, lưu bố cục, menu script và workspace nhiều dự án' },
      { en: 'Git Log pane with graph, branches and diffs; create, fetch and delete branches', vi: 'Pane Git Log có đồ thị, nhánh và diff; tạo, fetch và xoá nhánh' },
      { en: 'Android Studio style diff with minimap and split view', vi: 'Diff kiểu Android Studio có minimap và xem hai cột' },
      { en: 'Run pane for Android / Flutter, XML Layout Preview and Flutter Widget Preview', vi: 'Pane Run cho Android / Flutter, XML Layout Preview và Flutter Widget Preview' },
      { en: 'Extensions tab installs MCP servers and plugins through the claude CLI', vi: 'Tab Extensions cài MCP server và plugin qua claude CLI' },
      { en: 'Colour themes, bundled fonts, Ctrl+wheel zoom and Quick setup', vi: 'Theme màu, font kèm sẵn, Ctrl+lăn chuột để zoom và Thiết lập nhanh' },
      { en: 'Agents can open Workspace panes; Compact…, pane zoom, undo close, Mission Control', vi: 'Agent mở được pane Workspace; Compact…, phóng pane, hoàn tác đóng, Mission Control' },
      { en: 'Redesigned Monitor map and Usage, with messages between main agents', vi: 'Thiết kế lại bản đồ Monitor và Usage, hiện tin nhắn giữa các agent chính' },
    ],
  },
  {
    version: '1.2.3',
    date: '2026-10-12',
    items: [
      { en: 'macOS: dock badge and bounce when a session needs you', vi: 'macOS: badge và nảy icon trên dock khi session cần chủ nhân' },
      { en: 'macOS: focus the terminal tab, launch sessions and pick folders', vi: 'macOS: chuyển tới tab terminal, mở session và chọn thư mục' },
      { en: 'Waiting sessions say whether they need approval or ask a question', vi: 'Session đang chờ ghi rõ cần duyệt hay đang hỏi' },
      { en: 'Tool timeline view in the Agent Map panel', vi: 'Chế độ timeline tool trong panel Agent Map' },
      { en: 'Activity text wraps instead of being cut off', vi: 'Chữ trong Hoạt động xuống dòng thay vì bị cắt' },
      { en: 'A clear message when the port is taken, instead of restarting over and over', vi: 'Báo rõ khi cổng đã bị chiếm thay vì khởi động lại liên tục' },
      { en: 'Stopped background jobs stay hidden after a restart', vi: 'Job nền đã dừng vẫn ẩn sau khi mở lại app' },
      { en: 'Faster reading of large transcripts', vi: 'Đọc transcript lớn nhanh hơn' },
    ],
  },
  {
    version: '1.2.2',
    date: '2026-10-11',
    items: [
      { en: 'Codex CLI runs show up under the Claude agent that started them', vi: 'Lượt chạy Codex CLI hiện dưới agent Claude đã gọi nó' },
      { en: 'Renamed to Agent Monitor', vi: 'Đổi tên thành Agent Monitor' },
    ],
  },
  {
    version: '1.2.1',
    date: '2026-10-10',
    items: [{ en: 'Smoother motion across the new layout', vi: 'Chuyển động mượt hơn trên bố cục mới' }],
  },
  {
    version: '1.2.0',
    date: '2026-10-10',
    items: [
      { en: 'Full-screen Agent Map', vi: 'Agent Map toàn màn hình' },
      { en: 'Session bar to switch between sessions', vi: 'Thanh session để chuyển giữa các session' },
      { en: 'Keyboard shortcuts', vi: 'Phím tắt' },
    ],
  },
  {
    version: '1.1.1',
    date: '2026-10-09',
    items: [{ en: 'New app logo', vi: 'Logo app mới' }],
  },
  {
    version: '1.1.0',
    date: '2026-10-09',
    items: [{ en: 'macOS app (.dmg)', vi: 'App macOS (.dmg)' }],
  },
  {
    version: '1.0.2',
    date: '2026-10-09',
    items: [{ en: 'The app takes over when a shared collector stops', vi: 'App tự chạy collector riêng khi collector dùng chung bị dừng' }],
  },
  {
    version: '1.0.1',
    date: '2026-10-09',
    items: [
      { en: 'Watch every running Claude Code session live, with terminal titles and status', vi: 'Theo dõi trực tiếp mọi session Claude Code, kèm tên terminal và trạng thái' },
      { en: 'Agent Map of main agents and their sub-agents', vi: 'Agent Map hiện agent chính và các sub-agent' },
      { en: 'Usage tab with statistics and the plan quota left', vi: 'Tab Usage có thống kê và hạn mức gói còn lại' },
      { en: 'History tab with a daily recap you can export', vi: 'Tab History có tóm tắt theo ngày, xuất ra được' },
      { en: 'Windows toasts with snooze, quiet hours and stuck / context alerts', vi: 'Thông báo Windows có hoãn, giờ yên lặng, cảnh báo kẹt và đầy context' },
      { en: 'Session notes, favourites, git diff stats and open folder', vi: 'Ghi chú session, mục yêu thích, thống kê git diff và mở thư mục' },
      { en: 'Compact mode, onboarding tour and break reminders', vi: 'Chế độ thu gọn, hướng dẫn làm quen và nhắc nghỉ giải lao' },
      { en: 'Windows app: installer and portable exe', vi: 'App Windows: bản cài đặt và bản portable' },
    ],
  },
]

export function compareSemver(a: string, b: string): number {
  const pa = a.split('.').map(Number)
  const pb = b.split('.').map(Number)
  for (let i = 0; i < 3; i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0)
    if (d) return d
  }
  return 0
}

/** Shown once per app version, including a fresh install (nothing seen yet). */
export function shouldShowWhatsNew(seen: string | null, current: string): boolean {
  return !!current && seen !== current
}

const PRIOR_INSTALL_KEYS = ['ccm.setupDone', 'ccm.tourDone', 'ccm.tourWorkspaceDone']

export function isFreshInstall(): boolean {
  try {
    return PRIOR_INSTALL_KEYS.every((k) => localStorage.getItem(k) === null)
  } catch {
    return false
  }
}

export function seenVersion(): string | null {
  try {
    return localStorage.getItem(SEEN_KEY)
  } catch {
    // Storage blocked: treat this version as seen rather than open the dialog on every launch.
    return APP_VERSION
  }
}

export function markSeen(version: string = APP_VERSION): void {
  try {
    localStorage.setItem(SEEN_KEY, version)
  } catch {
    return
  }
}
