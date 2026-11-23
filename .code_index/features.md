# features: catalog tính năng

Rà trước khi đề xuất ý tưởng mới; cập nhật khi thêm / bỏ tính năng. Mỗi dòng: tính năng (plan) -> file index. Ý tưởng đã gạt: `.planning/IDEAS-*.md`. Bản dài cũ: `archive/features-full.md`. Lịch sử: `docs/features-history.md`.

## Quan sát (Monitor)
- Session Claude Code / Codex realtime, card, nhóm theo việc cần làm, waiting strip, done tray (v1, v79) -> `web-monitor.md`, `core-monitor.md`
- Agent Map, Quỹ đạo (Orbit), Board (v47, v95) -> `web-monitor.md`
- History (heatmap, timeline, tool stats, tìm prompt), Away summary, Recap (v73, v75) -> `web-monitor.md`
- Usage: chi phí / token / limit plan, forecast 5 giờ (v53) -> `web-monitor.md`
- Sensitive (deny / settings), Resources (CPU / RAM / port), DevTools, Diagnostics (v76) -> `web-small.md`
- Toast / alert OS, Telegram report, push điện thoại (v13, v82, v98) -> `core-toast-bridge.md`
- Office pixel-art Habblaud (v60, tạm dừng) -> `office.md`
- Tray liệt kê session chờ bạn, mức thông báo theo project (v111 A5 / A6) -> `core-toast-bridge.md`, `shell.md`

## Workspace
- Terminal / Browser pane, tab, split, layout, undo đóng pane, zoom pane, dust (v50, v52) -> `web-workspace.md`, `core-pty.md`
- Rail classic + UI next "Sương kính", palette, glass, drop compass (v92, v102, v108) -> `web-rail.md`, `web-next.md`
- Ctrl+K palette (v80), phím tắt tuỳ biến, MRU, tìm trong terminal (v109) -> `web-small.md`
- Trả lời menu Claude / Codex bằng chuột, Inbox "Chờ trả lời" (v100) -> `web-small.md`
- Chat view session, shell done toast (OSC 633), share mode (v109), xuất session ra Markdown (v111 S1) -> `web-small.md`
- Compact handoff, prompt library, turn review, clash, permission advisor, context meter (v51, v72, v89, v104) -> `web-agent-tools.md`
- MCP `orrery` (`open_session`, `list_sessions`, `run_app`...), deep link `agent-monitor://` (v49) -> `core-desk-mcp.md`
- Footer chỉ chip cần chú ý, header pane 4 nút, tab 1 tín hiệu, Launcher mẫu khởi chạy (v111 A1 / A2 / A4) -> `web-workspace.md`
- Dev server URL trong terminal mở preview + chip header (v111 W1) -> `web-workspace.md`
- Lịch sử lệnh shell Ctrl+R, mốc terminal (v111 C1 / X7) -> `web-workspace.md`, `core-desk.md`
- Chọn phần tử preview gửi file nguồn, style, ảnh cắt (v111 W2) -> `web-core.md`
- Gửi đoạn code đang chọn trong editor cho agent (v111 C2) -> `web-files.md`
- Rail lọc session khi gõ, menu Nhóm (v111 X1) -> `web-next.md`, `web-rail.md`
- Inbox / Board / Changes trống có nút việc tiếp theo (v111 A3) -> `web-core.md`

## Code
- Explorer + editor CodeMirror 6, icon Atom Material, auto save (v59, v62) -> `web-files.md`
- Diff inline / split, Changes, commit, Git Log, branches, worktree, checkpoints, secret guard (v30, v36, v58, v109) -> `web-git.md`
- Run pane Android / Flutter, XML Layout / Drawable / Flutter preview, strings, release check, lib updates (v31, v32) -> `web-run.md`
- Devices (adb USB / Wi-Fi / QR), emulator, capture, APK size (v61, v73) -> `web-devices.md`
- Extensions (MCP / plugin cho Claude Code), Hooks panel (v28, v74) -> `web-settings.md`
- DeepSeek CLI Codewhale, `hand_back` (v81, tạm dừng) -> `web-small.md`
- PR / CI / merge PR trong Git Log, theo dõi CI nền (v111 G1) -> `web-git.md`, `core-desk.md`
- Worktree tự setup / dọn theo `.orrery/worktree.json`, checklist trước merge + MCP `todo_*` (v111 G2 / G3) -> `web-git.md`, `core-desk-preview.md`
- Commit trong Git Log -> session đã làm, mở chat đúng lượt (v111 G4) -> `web-git.md`, `web-small.md`
- Git Stash (list / apply / pop / drop), hoàn tác commit cuối chưa push (v112 B1) -> `web-git.md`, `core-desk-preview.md`
- Run: lịch sử log, ghim mốc, so sánh 2 lần chạy (v112 X6) -> `web-run.md`

## Hệ thống
- Disk: scan / MFT, treemap, Clean up, dupes, Organize Downloads, Apps, RAM, Move, snapshots, MCP `disk_*` (v101) -> `web-disk.md`, `core-disk.md`
- Settings, Backup, GitHub accounts, Capture, bug report, tour, What's new, update -> `web-settings.md`
- Telegram bot nhận lệnh + nút trả lời (v82) -> `web-remote.md`, `core-desk-telegram.md`
- Orrery Remote: app Flutter + PWA, Noise, relay Cloudflare, push FCM / Web Push, chat, chạy lệnh từ xa (v97, v98, v107) -> `web-remote.md`, `shell-remote.md`, `pwa.md`, `relay.md`
- Theme (Orrery, Ngân hà, Orrery dark, legacy sau "View all"), font tuỳ chọn, sao nền (v91-v96, v109) -> `web-core.md`, `docs/visual-style.md`
