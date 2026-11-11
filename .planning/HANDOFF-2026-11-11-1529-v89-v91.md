# HANDOFF 2026-11-11 15:29 — Orrery 4.0.2 (v89–v91) đã push, chủ nhân sang MacBook test

Đọc file này là làm tiếp được. Trả lời tiếng Việt, gọi "chủ nhân", xưng "con". Thay bản 10:17 (Tauri gaps, còn trong git history `d5d5e53b`).

## 1. Đang ở đâu
- `master` = `origin/master` = `4bd8a614` (+ `41bbc597` Tauri Windows drop / fit / CSP). Working tree sạch trừ rác không track (`.vitest/`, `docs/qa/v79|v80`, `spikes/layoutlib`, `bg_native_ad_cta.xml`, `git-gui.bat`): **không commit** mấy file đó.
- Windows: `npm run qa --e2e=v89-pane,v89-session-tools,v90-bug-report,v91-themes` 16/16 pass (typecheck, vitest related, cargo core-desk + shell, 5 E2E Tauri). Review sonnet: 2 lỗi đã sửa (pre-paint theme khi storage bị chặn; `writeAtomic` giữ mode 0600 của `~/.claude.json` trên unix — TS `apps/desktop/src/extensions/project.ts`, Rust `apps/core/crates/core-desk/src/claude/mod.rs`).
- Release note nháp: `.planning/RELEASE-4.0.2.md` (en + vi). Chưa bump version, chưa release.

## 2. Đã làm (plan = nguồn chi tiết)
- **v89** `.planning/PLAN-v89-session-tools-context.md`: sheet MCP & plugin theo session (menu ⋯ pane + card Monitor), tắt theo project (`~/.claude.json` `projects[dir].disabledMcpServers`, `<dir>/.claude/settings.local.json` `enabledPlugins`), nút Khởi động lại (`--resume`). Chip context trên pane / SessionCard / BoardCard, toast 1 lần khi vượt `settings.contextWarnTokens` (500000, 0 = tắt). Kênh `ccm:ext-project`, `ccm:ext-project-toggle` (Rust `extensions/project.rs` + Electron `extensions/project.ts`), docs §11aq.
- **v90** `.planning/PLAN-v90-bug-report.md`: Settings → Giới thiệu → zip báo lỗi max 50 MB (report, page-errors, crash events/WER/.ips, logs, dumps tuỳ chọn), mask secret + home. Toast nhẹ sau crash / lỗi UI (`useBugReportPrompts`). Rust `apps/shell/src/bug_report.rs`, Electron `apps/desktop/src/bugReport.ts`.
- **v91** `.planning/PLAN-v91-themes-orrery-console.md`: theme Orrery (`orrery-light`/`orrery-dark`, mặc định) + Console; token hình dáng trong `style.css` (bảng ở `.planning/COLOR-SYSTEM.md`); migration 1 lần `ccm.themeMigration='4.0.2'` (index.html pre-paint + `lib/themes.ts` `startTheme`). Đổi hành vi: theme không có cặp → toggle sáng/tối sang Orrery (trước là Apple). Orrery dark accent fill `#6f5ee8`.
- **qa-fast** `scripts/qa-fast.mjs` (`npm run qa`), map E2E ở `E2E_MAP`; single-instance tắt khi E2E isolated (`apps/shell/src/config.rs` `e2e_isolated`).

## 3. Bước tiếp — trên MacBook (E2E không chạy được: WKWebView không có CDP)
```
git pull && npm install
npm run typecheck && npm test
cd apps/shell && cargo test && cargo tauri dev   # hoặc build như thường lệ
```
Checklist test tay:
1. Lần đầu mở sau update → Orrery (theo sáng/tối hệ thống); chọn theme khác, mở lại vẫn giữ.
2. Sheet MCP & plugin: tắt 1 MCP → Khởi động lại → session chạy với `--resume`; `ls -l ~/.claude.json` vẫn `-rw-------`.
3. Settings → Giới thiệu → tải zip báo lỗi: có `.ips` crash macOS, log; không lộ home path / secret.
4. Explorer, Git Log, console không lỗi CSP.
5. Chip token trên pane + card Monitor; toast khi vượt ngưỡng (hạ ngưỡng trong Settings → Nhắc nhở để thử).
Lỗi gặp trên Mac → sửa, chạy `npm run qa --no-e2e`, báo chủ nhân (commit chỉ khi chủ nhân bảo).

## 4. Còn treo (hỏi chủ nhân)
- Crash 4.0.1 Windows `0xc000001d` (ud2) tại offset `0xc58e4` Orrery.exe, không phải Rust panic; dump ở `%LOCALAPPDATA%\CrashDumps`. Cần PDB: build release local có PDB hay sửa CI giữ PDB?
- App tự mở claude.ai trên trình duyệt: chờ chủ nhân gửi URL. Nghi pane Claude chạy với config dir chưa login → OAuth.
- Chưa làm: Console thanh accent trái cho dòng đang chọn.
- `desk-agent-data.twin.test.ts` EPERM symlink trên Windows (cần Developer Mode) — có từ trước, không liên quan.

## 5. Ràng buộc chủ nhân đã chốt
- Không tự commit/push; cho phép lần nào tính lần đó. Không trailer `Co-Authored-By` / "Generated with Claude".
- Không viết comment trong code. Không đụng `.planning/RELEASE-4.0.0.md`.
- Subagent luôn đặt `model`; máy ≤80% tài nguyên. Code xong chạy `npm run qa`.
