# RELEASE 4.0.6

Gom cùng v100 click menu + v101 Ổ đĩa + v102 UI mới Sương kính (feat_mist_ui); release khi cả 3 xong.

## Tiếng Việt

### Sửa lỗi bản Tauri (so với bản Node cũ)
- MCP `list_sessions`, `send_to_session`, `notify_owner`, `run_*`, `who_is_editing` và lệnh Telegram gửi vào session không còn báo "Orrery window is not ready" khi đang mở cửa sổ compact.
- Thoát app hỏi trước khi tắt terminal / run đang chạy; file đang sửa chưa lưu được lưu (auto save) hoặc hỏi Save / Don't save / Cancel.
- Link `agent-monitor://` mở được cả khi app chưa chạy; nhiều link focus liền nhau không đè nhau.
- Build clean không xoá khi project còn dev server / run / Gradle daemon; backup có danh sách tài khoản GitHub.
- Core terminal khởi động lại thì `list_sessions` không còn hiện pane cũ.

### Click chọn đáp án (v100)
- Menu chọn của Claude / Codex trong pane: rê chuột làm sáng dòng, click để chuyển tới dòng, double click để chọn. Menu nhiều lựa chọn: click để tick, có nút Submit.
- Thanh nút dưới pane hiện đáp án khi agent đang hỏi, có số 1-9 trên mỗi nút.
- Chip "Chờ trả lời" gom mọi pane đang hỏi (nhiều project), phím Ctrl+Alt+Q nhảy tới câu kế tiếp. Card trong Mission Control cũng trả lời được.
- Khi app ở nền, thông báo của hệ điều hành có sẵn nút đáp án cho câu hỏi ngắn.
- Tắt được ở Settings > Workspace; tắt thì chuột hoạt động như cũ.

### Ổ đĩa (v101)
- Mở từ Settings > Data > Ổ đĩa: quét ổ, xem dung lượng dạng treemap hoặc sunburst, tìm file theo tên gần như tức thì, lọc theo size / loại / ngày, xem file lớn và file cũ.
- Windows NTFS có nút "Quét siêu nhanh" (hỏi quyền admin mỗi lần bấm).
- Dọn dẹp: rác dev (node_modules, build...), rác hệ thống, SDK / emulator cũ, Docker, dọn 1 chạm. Mỗi nhóm ghi rõ xoá xong ảnh hưởng gì.
- File trùng, sắp xếp thư mục Downloads (xem trước, tự động, có Undo), gỡ app kèm file sót, xem process tốn RAM.
- Chuyển thư mục lớn sang ổ khác và để lại link; so sánh dung lượng giữa các lần quét; quét nền khi máy rảnh; báo Telegram khi ổ sắp đầy.
- Mọi lần xoá đều hỏi xác nhận, mặc định vào Thùng rác; file chỉ có trên cloud không bị tải về. Agent chỉ đề xuất, bạn duyệt mới xoá.

## English

### Tauri fixes (parity with the old Node host)
- MCP page tools and Telegram commands into sessions no longer fail with "Orrery window is not ready" while the compact window is open.
- Quitting asks before killing live terminals / runs, and saves or asks about unsaved editors.
- `agent-monitor://` links work when they start the app; concurrent focus links no longer clobber each other.
- Build clean refuses while dev servers, runs or Gradle daemons use the project; backups include GitHub accounts.
- After the terminal core restarts, `list_sessions` no longer shows dead panes.

### Click to answer (v100)
- Claude / Codex choice menus in a pane: hover highlights a row, click moves to it, double click selects. Multi-select menus: click ticks, with a Submit button.
- A button bar under the pane shows the answers while the agent asks, with the numbers 1-9 on each button.
- A "Chờ trả lời" chip lists every pane that is asking (across projects); Ctrl+Alt+Q jumps to the next one. Mission Control cards can answer too.
- While the app is in the background, the OS notification carries answer buttons for short questions.
- Turn it off in Settings > Workspace; off means the mouse behaves as before.

### Disk (v101)
- Opens from Settings > Data > Disk: scan drives, see usage as a treemap or sunburst, find files by name almost instantly, filter by size / type / date, see big and old files.
- Windows NTFS has a "Fast scan" button (asks for admin rights each time you press it).
- Clean up: dev junk (node_modules, build...), system junk, old SDKs / emulators, Docker, one-tap clean. Every group says what deleting it affects.
- Duplicate files, sort the Downloads folder (preview, automatic, with Undo), uninstall apps with leftovers, see which processes use the most RAM.
- Move a big folder to another drive and leave a link; compare usage between scans; background scan when idle; Telegram alert when a drive is nearly full.
- Every delete asks first and goes to the Recycle Bin by default; cloud-only files are never downloaded. Agents can only suggest, you approve before anything is deleted.

## Changelog chờ (`apps/web/src/lib/changelog.ts`, ≤100 ký tự)
- Tauri: MCP và Telegram vào session chạy lại khi mở cửa sổ compact
- Tauri: hỏi trước khi thoát, lưu file chưa lưu; link mở được khi app chưa chạy
- Click hoặc double click đáp án của Claude / Codex ngay trong pane; chip Chờ trả lời, toast có nút
- Ổ đĩa mới: quét, treemap, tìm file, dọn rác, file trùng, sắp xếp Downloads, gỡ app
