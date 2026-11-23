# RELEASE next (số version chủ nhân đặt)

Dòng changelog chờ (đưa vào `apps/web/src/features/update/lib/changelog.ts` cùng lúc bump `apps/shell/package.json` + `package-lock.json`, test `changelog.test.ts` đòi version đầu = version shell; mỗi dòng ≤ 100 ký tự). Nguồn: `.planning/wp/WP-111*-report.md`, `WP-112*-report.md`, `WP-bug-hide-restore-report.md`.

Mới:
- en: Create, watch and merge GitHub PRs from Git Log, with CI status and a toast when CI ends
- vi: Tạo, theo dõi và merge PR GitHub trong Git Log, có trạng thái CI và toast khi CI xong
- en: Export a chat session as Markdown: copy, save a .md file or send it to Telegram
- vi: Xuất session chat thành Markdown: chép, lưu file .md hoặc gửi Telegram
- en: Worktrees set up and clean up by themselves via .orrery/worktree.json
- vi: Worktree tự setup và tự dọn theo .orrery/worktree.json
- en: Pre-merge checklist per worktree, agents can add todos
- vi: Checklist trước merge theo worktree, agent thêm được việc
- en: Git Log shows which Claude / Codex session made a commit; click opens the chat at that turn
- vi: Git Log hiện session Claude / Codex đã làm commit; bấm mở chat đúng lượt đó
- en: Git Stash from Changes, stash list in Git Log, undo the last unpushed commit
- vi: Stash từ Changes, danh sách stash trong Git Log, hoàn tác commit cuối chưa push
- en: Dev server URLs in a terminal open a preview and show a header chip
- vi: Terminal in địa chỉ dev server thì tự mở preview và hiện chip trên header
- en: Shell command history (Ctrl+R) and terminal bookmarks
- vi: Lịch sử lệnh shell (Ctrl+R) và mốc terminal
- en: Run history: reopen old logs, pin a baseline, compare two runs
- vi: Lịch sử Run: mở lại log cũ, ghim mốc, so sánh 2 lần chạy
- en: Launcher templates and per-project default agent
- vi: Launcher có mẫu khởi chạy và agent mặc định theo project
- en: Pick element sends source file, styles and a cropped image to the agent
- vi: Chọn phần tử gửi kèm file nguồn, style và ảnh cắt cho agent
- en: Send selected code from the editor to an agent
- vi: Gửi đoạn code đang chọn trong editor cho agent
- en: Tray lists sessions waiting for you; notification level per project
- vi: Tray hiện session chờ chủ nhân; mức thông báo theo project
- en: Footer shows only what needs attention; first-run setup moved to Settings
- vi: Footer chỉ hiện chip cần chú ý; cài đặt lần đầu nằm trong Settings
- en: Pane header slimmed to four buttons, tabs show one signal
- vi: Header pane gọn còn 4 nút, tab chỉ 1 tín hiệu ưu tiên
- en: Empty Inbox, Board and Changes now offer a next step
- vi: Inbox, Board, Changes trống có nút việc tiếp theo
- en: Rail filters sessions as you type, with a compact Group menu
- vi: Rail lọc session ngay khi gõ, nút Nhóm gọn một hàng

Sửa lỗi:
- en: Window no longer stays covered or unclickable after minimize and restore
- vi: Cửa sổ không còn bị phủ tối, không click được sau khi thu nhỏ rồi mở lại
- en: Terminal no longer refits while the window is minimized
- vi: Terminal không đo lại kích thước khi cửa sổ đang thu nhỏ
- en: Mist theme drops blur, so the window no longer freezes while resizing
- vi: Theme Sương kính bỏ blur, cửa sổ không còn treo khi kéo đổi kích thước
- en: Safer GitHub PR merge and fuller CI status
- vi: Merge PR an toàn hơn, trạng thái CI đầy đủ hơn

- Chờ thử trên Mac rồi mới ghi dòng: v110 P0.5 (PATH login shell, menu app, Move to Applications, Dock bounce, toast có nút).
