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

## Workspace
- Terminal / Browser pane, tab, split, layout, undo đóng pane, zoom pane, dust (v50, v52) -> `web-workspace.md`, `core-pty.md`
- Rail classic + UI next "Sương kính", palette, glass, drop compass (v92, v102, v108) -> `web-rail.md`, `web-next.md`
- Ctrl+K palette (v80), phím tắt tuỳ biến, MRU, tìm trong terminal (v109) -> `web-small.md`
- Trả lời menu Claude / Codex bằng chuột, Inbox "Chờ trả lời" (v100) -> `web-small.md`
- Chat view session, shell done toast (OSC 633), share mode (v109) -> `web-small.md`
- Compact handoff, prompt library, turn review, clash, permission advisor, context meter (v51, v72, v89, v104) -> `web-agent-tools.md`
- MCP `orrery` (`open_session`, `list_sessions`, `run_app`...), deep link `agent-monitor://` (v49) -> `core-desk-mcp.md`

## Code
- Explorer + editor CodeMirror 6, icon Atom Material, auto save (v59, v62) -> `web-files.md`
- Diff inline / split, Changes, commit, Git Log, branches, worktree, checkpoints, secret guard (v30, v36, v58, v109) -> `web-git.md`
- Run pane Android / Flutter, XML Layout / Drawable / Flutter preview, strings, release check, lib updates (v31, v32) -> `web-run.md`
- Devices (adb USB / Wi-Fi / QR), emulator, capture, APK size (v61, v73) -> `web-devices.md`
- Extensions (MCP / plugin cho Claude Code), Hooks panel (v28, v74) -> `web-settings.md`
- DeepSeek CLI Codewhale, `hand_back` (v81, tạm dừng) -> `web-small.md`

## Hệ thống
- Disk: scan / MFT, treemap, Clean up, dupes, Organize Downloads, Apps, RAM, Move, snapshots, MCP `disk_*` (v101) -> `web-disk.md`, `core-disk.md`
- Settings, Backup, GitHub accounts, Capture, bug report, tour, What's new, update -> `web-settings.md`
- Telegram bot nhận lệnh + nút trả lời (v82) -> `web-remote.md`, `core-desk-telegram.md`
- Orrery Remote: app Flutter + PWA, Noise, relay Cloudflare, push FCM / Web Push, chat, chạy lệnh từ xa (v97, v98, v107) -> `web-remote.md`, `shell-remote.md`, `pwa.md`, `relay.md`
- Theme (Orrery, Ngân hà, Orrery dark, legacy sau "View all"), font tuỳ chọn, sao nền (v91-v96, v109) -> `web-core.md`, `docs/visual-style.md`
