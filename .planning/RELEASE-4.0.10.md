# RELEASE 4.0.10

Release 2026-11-25 (tag v4.0.10): sửa lỗi cửa sổ bị phủ sau khi thu nhỏ, gói v111 / v112 (PR GitHub + CI, xuất chat Markdown, worktree tự setup, stash, lịch sử lệnh, lịch sử Run, launcher mẫu, header / footer gọn).

## Tiếng Việt

### Sửa lỗi
- Cửa sổ không còn bị phủ, không click được sau khi thu nhỏ rồi mở lại.

### Mới
- Tạo, theo dõi và merge PR GitHub trong Git Log, có trạng thái CI và toast khi CI xong.
- Xuất session chat thành Markdown: chép, lưu file .md hoặc gửi Telegram.
- Worktree tự setup và tự dọn theo `.orrery/worktree.json`.
- Checklist trước merge theo worktree, agent thêm được việc.
- Git Log hiện session Claude / Codex đã làm commit; bấm mở chat đúng lượt đó.
- Stash từ Changes, danh sách stash trong Git Log, hoàn tác commit cuối chưa push.
- Terminal in địa chỉ dev server thì tự mở preview và hiện chip trên header.
- Lịch sử lệnh shell (Ctrl+R) và mốc terminal.
- Lịch sử Run: mở lại log cũ, ghim mốc, so sánh 2 lần chạy.
- Launcher có mẫu khởi chạy và agent mặc định theo project.
- Chọn phần tử gửi kèm file nguồn, style và ảnh cắt cho agent.
- Gửi đoạn code đang chọn trong editor cho agent (Ctrl+Shift+L).
- Tray hiện session đang chờ; mức thông báo theo project.
- Footer chỉ hiện chip cần chú ý; cài đặt lần đầu nằm trong Settings.
- Header pane gọn còn 4 nút, tab chỉ 1 tín hiệu ưu tiên.
- Inbox, Board, Changes trống có nút việc tiếp theo.
- Rail lọc session ngay khi gõ, nút Nhóm gọn một hàng.

## English

### Fixes
- The window no longer stays covered or unclickable after minimize and restore.

### New
- Create, watch and merge GitHub PRs from Git Log, with CI status and a toast when CI ends.
- Export a chat session as Markdown: copy, save a .md file or send it to Telegram.
- Worktrees set up and clean up by themselves via `.orrery/worktree.json`.
- Pre-merge checklist per worktree; agents can add todos.
- Git Log shows which Claude / Codex session made a commit; click opens the chat at that turn.
- Git Stash from Changes, stash list in Git Log, undo the last unpushed commit.
- Dev server URLs in a terminal open a preview and show a header chip.
- Shell command history (Ctrl+R) and terminal bookmarks.
- Run history: reopen old logs, pin a baseline, compare two runs.
- Launcher templates and a per-project default agent.
- Pick element sends the source file, styles and a cropped image to the agent.
- Send selected code from the editor to an agent (Ctrl+Shift+L).
- Tray lists sessions waiting for you; notification level per project.
- Footer shows only what needs attention; first-run setup moved to Settings.
- Pane header slimmed to four buttons; tabs show one signal.
- Empty Inbox, Board and Changes now offer a next step.
- Rail filters sessions as you type, with a compact Group menu.

## Chưa có trong bản này
- macOS v110 P0.5 (PATH login shell, menu app, Move to Applications, Dock bounce, toast có nút) chưa thử trên Mac.
- E2E `v25.cjs` còn 2 FAIL ở tour (tour chờ tab đầu từ 112b), chưa sửa.
