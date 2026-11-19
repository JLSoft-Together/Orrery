# RELEASE 4.0.6

Release 2026-11-16 (tag v4.0.6): v100 click menu + v101 Ổ đĩa (cả trên vỏ Tauri) + sửa lỗi Tauri + v102 UI mới Sương kính.

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

### Giao diện mới Sương kính (v102, bật thử)
- Bật ở Settings > Giao diện > Thử giao diện mới (UI next gen) hoặc Ctrl+K; đổi qua lại tức thì, terminal đang chạy không bị mở lại. Giao diện cũ vẫn là mặc định.
- Rail bên trái hiện mọi session theo project hoặc trạng thái, mục "Cần bạn" ở đầu, limit 5h / tuần ở chân; cửa sổ hẹp thu còn icon.
- Explorer thành ngăn kéo (Ctrl+Shift+E), ghim được khi màn đủ rộng.
- Ctrl+K gom session theo trạng thái (Cần bạn, Đang chạy, Xong chưa xem, Gần đây).
- Kéo pane có la bàn 5 ô, hít 1/3 1/2 2/3, nút Bố cục với các mẫu chia sẵn, toast Hoàn tác sau khi thả.
- 9 bảng màu (Sương kính, Tinh vân, Cực quang, Than chì, Bình minh sương, Ngân hà, Orrery sáng, Orrery tối, Console), chỉnh chuyển động đầy đủ / giảm / tắt và "Giảm trong suốt" cho máy yếu.
- Theme tự chọn: thẻ "Tuỳ chỉnh" có color picker kính cho accent, 2 vệt sáng nền, sắc nền và độ trong kính; chọn 1 màu, app tự sinh bản sáng + tối và tự chỉnh để chữ luôn đủ rõ.
- Hướng dẫn ở giao diện mới gọn thành 1 thẻ mẹo nhỏ góc dưới phải, không che màn hình. Giao diện cũ có thẻ mời thử giao diện mới (hiện 1 lần).

### Chung
- Mọi dropdown trong app (Ổ đĩa, Thiết bị, Run, Log, Thông báo...) dùng chung một kiểu, có thể dùng bằng bàn phím, không còn danh sách xám của hệ điều hành.
- Menu chuột phải của pane đủ rộng, không cụt chữ.

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

### New Mist glass UI (v102, opt-in)
- Turn it on in Settings > Appearance > Try the new UI (UI next gen) or from Ctrl+K; it switches instantly and running terminals are not reopened. The classic UI stays the default.
- A left rail lists every session by project or by status, with a "Needs you" list on top and the 5h / weekly limits at the bottom; narrow windows shrink it to icons.
- The Explorer becomes a drawer (Ctrl+Shift+E) that can be pinned when the window is wide enough.
- Ctrl+K groups sessions by state (Needs you, Running, Done not seen, Recent).
- Dragging a pane shows a 5-tile compass, snaps to 1/3 1/2 2/3, a Layout button offers ready splits, and an Undo toast follows every drop.
- 9 palettes (Mist glass, Nebula, Aurora, Graphite, Misty dawn, Galaxy, Orrery light, Orrery dark, Console), motion full / reduced / off and "Reduce transparency" for slower machines.
- Custom theme: a "Custom" card with a glass color picker for the accent, two background glows, the base tint and glass opacity; pick one color and the app derives light + dark and keeps text readable.
- The guide in the new UI is a small tips card at the bottom right that never covers the screen. The classic UI shows a one-time card inviting you to try the new UI.

### General
- Every dropdown in the app (Disk, Devices, Run, Log, Notifications...) uses one style, works from the keyboard, and no longer shows the grey OS list.
- The pane context menu is wide enough that no label is cut.

## Changelog chờ (`apps/web/src/lib/changelog.ts`, ≤100 ký tự)
- Tauri: MCP và Telegram vào session chạy lại khi mở cửa sổ compact
- Tauri: hỏi trước khi thoát, lưu file chưa lưu; link mở được khi app chưa chạy
- Click hoặc double click đáp án của Claude / Codex ngay trong pane; chip Chờ trả lời, toast có nút
- Ổ đĩa mới: quét, treemap, tìm file, dọn rác, file trùng, sắp xếp Downloads, gỡ app
- Thử giao diện Sương kính: rail session, Explorer ngăn kéo, la bàn kéo pane, 9 bảng màu
- Giao diện mới: theme tự chọn với color picker, tự sinh bản sáng và tối
- Mọi dropdown dùng chung một kiểu, dùng được bằng bàn phím
- Mục "Có gì mới" đề xuất (vi): Giao diện mới Sương kính, bật thử ngay ở thẻ mời góc dưới phải hoặc Cài đặt > Giao diện; quay lại cũ lúc nào cũng được
- "What's new" proposal (en): New Mist glass UI, try it from the card at the bottom right or Settings > Appearance; go back to the classic UI any time
