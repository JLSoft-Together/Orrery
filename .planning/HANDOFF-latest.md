# HANDOFF 2026-11-11 16:21 — 4.0.2 test macOS xong, IME tiếng Việt + 6 lỗi nhỏ đã sửa, chủ nhân sang Windows chạy E2E

Đọc file này là làm tiếp được. Trả lời tiếng Việt, gọi "chủ nhân", xưng "con". Thay bản 15:29 (`HANDOFF-2026-11-11-1529-v89-v91.md`).

## 1. Đang ở đâu
- `master` có commit "🐛 4.0.2 macOS test fixes" (sau `e3bc9726`), đã push. Version vẫn **4.0.1**, chưa bump, chưa release.
- macOS: `npm run typecheck` pass; `npm test` pass (1 test twin fail lúc đầu do `apps/core/target/release/orrery-core` cũ, build lại `cargo build -p core-bin --release` là hết; crate của binary `orrery-core` là `core-bin`); `cargo test` shell + core-desk pass (`layout_model.rs::refresh_failures_are_reported_with_codes` fail 1 lần rồi pass 5 lần: timing, theo dõi). `npm run qa --no-e2e` 6/6.
- Test tay trên `apps/shell/target/debug/orrery-shell` (data tạm `$TMPDIR/orrery-shell-dev`, cổng 4419), lái app bằng System Events / AX (WKWebView không có CDP): checklist 5 mục của handoff 15:29 đều đạt (theme Orrery lần đầu + giữ Console sau mở lại; sheet MCP ghi `disabledMcpServers` / `enabledPlugins`, `~/.claude.json` vẫn 600, `--resume` đúng id, `/mcp` thấy blender tắt; zip báo lỗi không chứa home path / secret; Explorer + Git Log + editor; chip context + toast).
- Ghi chú: "Khởi động lại" trên session chưa có prompt mở session mới (đúng thiết kế, `resumeIdOf` bỏ qua untouched). Markdown preview không hiện ảnh là cố ý (`lib/markdown.ts:19`).

## 2. Đã sửa phiên này
- **IME tiếng Việt trong terminal (bug chủ nhân báo: gõ rồi Delete ra `/uuuu`, mất chữ)**: Apple Vietnamese Telex trên WKWebView bắn `beforeinput`/`input` `insertReplacementText` (thay cả từ trong textarea ẩn, không composition) → xterm 6 bỏ qua; thêm `insertText` đến trước keydown nên `_keyPressHandled` của xterm bỏ chữ đầu sau dấu cách. Sửa `apps/web/src/lib/termIme.ts` `installReplacementIme` (TermPane + DropTerminal): `\x7f`×phần bị thay + phần mới, textarea khớp dòng lệnh (Backspace bỏ 1 code point, Enter/mũi tên/ctrl/meta xoá), gửi lại `insertText` mà xterm bỏ (không khi đang composition). Test tay: shell `echo tiếng việt ný đây oké xong`, Claude pane `thử gõ tiếng việt ný đây nhé`, ASCII không trùng, dead key Option+e đúng.
- 6 lỗi nhỏ: mask không che `Session: <1–4 chữ số>` (Rust `mask.rs` `is_count` = TS `claudeHooks/model.ts` `isCount`); `Node )` trống bỏ (`useDiagnosticsReport.ts`); tên zip / log theo giờ máy (`core_desk::util::local_file_stamp`, `bugReport.ts` `localFileStamp`); `errorLog.recordError` bỏ noise ResizeObserver / Script error; toast context nhớ qua lần mở app (`ccm.contextWarned`); What's new không hiện khi cài mới, máy đã qua setup / tour vẫn thấy (`changelog.ts` `isFreshInstall`).
- Review `typescript-reviewer` (sonnet) đã áp 4 điểm. Bỏ qua có chủ ý: Linux `local_date_time` vẫn UTC (Linux không phải target), mirror textarea lệch khi click chuột / TUI redraw (rủi ro thấp).

## 3. Việc tiếp — Windows (chủ nhân chạy)
```
git pull && npm install
npm run qa -- --e2e=core-pty,p3,p4,file-editor,v89-pane,v89-session-tools,v90-bug-report,v91-themes
```
Mục tiêu: chắc `termIme.ts` không làm gõ bị trùng / mất trên WebView2 (Chromium dùng composition, khác WebKit). Gõ tay thêm Telex (Unikey / EVKey) trong Claude pane, bấm Delete giữa chừng.
Pass → `/jin-release`: release-check, bump 4.0.2, release note `.planning/RELEASE-4.0.2.md` (đã có dòng IME).
Chủ nhân confirm IME OK → ghi `bug_pool/` + `signatures.tsv`.

## 4. Còn treo
- Crash 4.0.1 Windows `0xc000001d` tại `0xc58e4` Orrery.exe: cần PDB (build local có PDB hay CI giữ PDB?).
- App tự mở claude.ai trên trình duyệt: chờ URL.
- Console: thanh accent trái cho dòng đang chọn chưa làm.
- Chưa có `.ips` crash Orrery thật trên Mac để kiểm zip.
- `CCM_SHELL_WEBVIEW_DIR` / data dir dev: xoá `$TMPDIR/orrery-shell-dev` mà WKWebView vẫn giữ localStorage (recent, tour) → nghi WebKit lưu chỗ khác trên macOS; chưa điều tra.

## 5. Ràng buộc chủ nhân đã chốt
- Không tự commit/push (lần này chủ nhân cho phép). Không trailer `Co-Authored-By` / "Generated with Claude". Không đụng `.planning/RELEASE-4.0.0.md`.
- Không viết comment trong code. Subagent luôn đặt `model`; máy ≤80% tài nguyên. Code xong chạy `npm run qa`.
- Trên Mac bộ gõ Telex đang bật: gõ phím giả lập phải dán qua clipboard (trả clipboard lại), không `keystroke` chữ có thể bị Telex đổi.
