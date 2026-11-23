# Luật chung cho mọi WP v111 (đọc trước brief WP)

Project: F:\work\web\Orrery. Đọc `CLAUDE.md` (lệnh, hard rules), `.code_index/lookup.md` rồi 1-2 module file liên quan. Spec đợt 1: `.planning/SPEC-v111-git-preview.md`. Quyết định chủ nhân: `.planning/RESEARCH-2311-proposals.md` mục "Chủ nhân chốt", `.planning/PLAN-2026-11-23-v111.md` mục "Chủ nhân chốt đợt 2". Wireframe đã duyệt: `.planning/wireframes/v111-proposals.html`, `v111b-proposals.html`.

## Cấm
- KHÔNG git commit / push / stash / reset / checkout / restore. Main commit.
- KHÔNG viết comment trong code (mọi loại: `//`, `/* */`, doc comment, `<!-- -->`, `#`, TODO). Comment có sẵn để nguyên.
- Không sửa file ngoài phạm vi brief. Hot file (dưới) chỉ sửa khi brief ghi rõ WP này là chủ; còn lại ghi vào mục "Hot file cần thêm" của báo cáo.
- Không gọi LLM / API Claude, không tự submit prompt vào pane agent (chỉ prefill / attachments). Token GitHub không ra frontend / log / event.
- Không polling dày; chỉ cập nhật khi đổi, dừng khi app ở nền trừ khi brief cho phép.
- Không kill / đụng Orrery chủ nhân đang chạy; E2E dùng `CCM_DATA_DIR` tạm và cổng trống (không 9333, 4317, 4419).

## Phải
- Tìm bằng `rg` / tool Grep (không `grep -r`, không Select-String).
- Mọi chuỗi UI qua vue-i18n, có cả `en` và `vi`, tiếng Việt có dấu. Không dùng em dash `—` và chấm giữa `·` trong chuỗi UI hay tài liệu. Từ cấm: "trần" (dùng max), "watchdog" (dùng deadline / timer timeout), "bắn", "bay", "nổ", "kho" (dùng cache).
- Màu qua token theme, status = icon + chữ + màu. Không gradient, không blur, không `backdrop-filter`. Không cuộn ngang.
- Ưu tiên mượt: animation chỉ transform / opacity, list dài ảo hoá, mở từ cache trước.
- Rust: theo skill `rust-conventions` (`C:/Users/dauph/.claude/skills/rust-conventions/SKILL.md`): lệnh ngoài có timeout + kill tree, không block thread async, lỗi rõ ràng. `cargo test --locked -j 4` (PowerShell; Git Bash `link` che `link.exe`).
- Web: `npm run typecheck` (hoặc `-w @ccm/web`), `npm test -w @ccm/web -- <path>`. Chuỗi i18n mới: chạy `npm test -w @ccm/web -- src/i18n` và cập nhật snapshot `keys.*.txt` nếu test yêu cầu (`-u`) CHỈ cho key của WP mình.
- Được build để kiểm (repo cho phép): web `npm run build -w @ccm/web`, shim `node scripts/gen-desk-shim.mjs`, shell `cd apps/shell; cargo build -j 4`. E2E: `node apps/shell/e2e/<tên>.cjs`. Viết E2E mới khi brief yêu cầu.
- Sau khi xong: cập nhật `.code_index/` module file liên quan + 1 dòng `.code_index/_meta.md`, chạy `node scripts/index-check.mjs`.
- Ghi nhật ký từng bước vào mục "Nhật ký" cuối brief WP (agent có thể chết giữa chừng).

## Hot file (một chủ mỗi lượt, xem brief)
`packages/shared/src/types.ts`, `apps/web/src/core/lib/preview.ts`, `apps/web/src/features/workspace/stores/workspace.ts`, `apps/web/src/features/workspace/components/TermPane.vue`, `apps/web/src/features/workspace/components/WorkspaceView.vue`, `apps/web/src/features/git/components/log/LogPane.vue`, `apps/web/src/core/stores/settings.ts`, `apps/core/crates/core-desk/src/mcp/tools.json`, `apps/web/src/features/workspace/lib/workspaceKeys.ts`, `apps/web/src/features/workspace/lib/shortcuts.ts`, `apps/web/src/features/update/lib/changelog.ts` (không ai sửa), `.planning/RELEASE-next.md` (main).

## Báo cáo
Ghi `.planning/wp/WP-<id>-report.md` TRƯỚC khi trả lời main, đúng khung:
```
## WP-<id> — DONE | PARTIAL | BLOCKED
- Đã đổi: <file:line — đổi gì>
- Tiêu chí: <n/m đạt; dòng chưa đạt>
- Audit: <...>
- Review: <rủi ro, edge case chưa phủ>
- Test: <lệnh> → <kết quả thật>
- Nhìn: <ảnh docs/qa/v111/WP-<id>/...> | CHƯA NHÌN: <lý do>
- Hot file cần thêm: <...>
- Ngoài phạm vi cần main xử lý: <...>
- Commit message đề xuất: <type(scope): ... tiếng Anh, không trailer>
- Dòng changelog (en / vi, ≤ 100 ký tự): <...>
```
Không có file báo cáo = WP chưa xong. Trả lời main ≤ 12 dòng.
