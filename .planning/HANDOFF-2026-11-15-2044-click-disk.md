# HANDOFF — Orrery v100 click menu + v101 Ổ đĩa (2026-11-15 20:44)

Đọc file này là làm tiếp được. Trả lời tiếng Việt, gọi "chủ nhân", xưng "con". File này thay `HANDOFF-2026-11-15-2000.md` (đã chuyển `archive/`).
Nhật ký + bảng WP: `.planning/PROGRESS-2026-11-15-click-disk.md`.

## 1. Đang ở đâu

- Repo `F:/work/web/Orrery`, nhánh `feat_clean_memory` (tách từ `master` 6d7054be). 23 commit của luồng (`git log --oneline 6d7054be..HEAD`), HEAD `60438421`. **Chưa push, chưa merge master.**
- Đợt v100 + v101 **đã xong hết**: mọi phase code + nghiệm thu + QA + docs.
- Kiểm tra cuối (WP-QA, 20:3x 15/11): `npm run typecheck` sạch; web 2464 test pass; desktop 1993 pass + 1 fail có từ trước (`desk-agent-data.twin` EPERM symlink Windows); `cargo test -p core-disk -p core-desk` pass; `npm run qa -- --no-e2e` 9/9; 17 E2E v100/v101 PASS trên bản dev tách biệt. Bảng: `docs/qa/v101/RESULT.md`.
- Không còn agent / lệnh nền nào chạy.
- Bị chặn: chờ chủ nhân quyết merge master (mục 3).

| Mảng | Commit |
|---|---|
| v100 click menu 0a-0e | 1e84e87d, 04e390e1, 45e558ee, 6f766885, ce9a9709, 9bb22a0d |
| v101 engine / host / view / tìm kiếm / MFT | fcf8e0c3, 1e6643c5, 34b9fca0, 42f9788d, 6bb96d9e, fd87fcbc |
| v101 Dọn dẹp, vá guard, size project | 59f51156, 07260d5d, 0514ae7c, 1ba491b6 |
| v101 sắp xếp, file trùng, SDK/move, App/RAM, snapshot+MCP | 7d1ea54b, 9a613568, fd211c67, d729fab1, e0bb18c5 |
| docs, QA | 2eb6afe0, 60438421 |

Thống kê: 23 WP ĐẠT, 0 BLOCKED; review độc lập bắt 2 lô lỗi thật (core-disk 1 HIGH, guard xoá 1 HIGH + autoConfirm env bản đóng gói), đã vá có test.

## 2. Phiên này đã làm

- Grill 31 câu + review → `.planning/PLAN-v100-click-menu.md`, `.planning/PLAN-v101-disk-manager.md`.
- v100: parser màn `apps/web/src/lib/termMenu.ts` (data từ màn live, không hook), thanh nút `TermMenuBar.vue`, click trong xterm `useTermMenuMouse.ts` (chặn mouse report Codex), hàng chờ `stores/menuQueue.ts` + Ctrl+Alt+Q, Mission Control, toast `apps/desktop/src/menuToast.ts` (link `agent-monitor://menu?k=<key mỗi lần chạy>`).
- v101: crate `apps/core/crates/core-disk` (walk song song, index nén `CCM_DATA_DIR/disk/`, search, MFT helper elevated qua named pipe, dupes BLAKE3, summary, sizes, forget); host `apps/desktop/src/diskMap/` (guard canonical `guard.ts`, `delete.ts` `runDelete`), `diskClean/`, `diskMove/`, `organize/`, `diskApps/`, `diskRam/`; web `components/disk/**`; MCP `disk_search|disk_usage|disk_propose_delete`.
- Docs: `CLAUDE.md` (Since v100/v101, sixth exception), `docs/claude-code-integration.md` §11as, §11at.

## 3. Việc tiếp (theo thứ tự)

1. **Hỏi chủ nhân** có merge `feat_clean_memory` vào `master` không (COORD: v100/v101 vào master trước v102). Được phép thì: `git checkout master && git merge --no-ff feat_clean_memory`, rồi `npm run qa -- --no-e2e`. Không push nếu chủ nhân không nói.
2. Gộp dòng `.code_index/_meta.md` (dòng đề xuất trong `.planning/wp/WP-DOC-report.md`) **sau khi** session orrery-4e commit xong file đó.
3. `.planning/RELEASE-4.0.6.md` đã có mục v100/v101 (file của orrery-4e, untracked, không commit hộ). Release 4.0.6 = v100 + v101 + sửa Tauri + v102; chỉ bump / build khi chủ nhân bảo.
4. (Đề xuất, chưa giao) kiểm hash / chữ ký `orrery-core.exe` ngay trước `runas` của quét MFT (rủi ro: bản cài per-user, binary ở thư mục ghi được).
5. Luồng Tauri (orrery-4e làm): `apps/shell/src/shim.js` thiếu bridge `diskMapFastScan`, `diskApps*`, `diskRam*`, `diskMove*`, `organize*`, `diskClean*`, `from-clean open`; `apps/shell/src/hosts.rs` thiếu `mcpDesk.disk`.

Còn chờ chủ nhân (đang giữ mặc định):
- (a) Merge master? — mặc định: chưa merge.
- (b) Chạy `node scripts/mft-smoke.mjs C` + bấm UAC để kiểm MFT thật — mặc định: CHƯA KIỂM.
- (c) Codex toàn cục bị update 0.162.1 → 0.169.0 lúc chụp mẫu (lùi: `npm install -g @openai/codex@0.162.1`) — mặc định: giữ 0.169.0; CLAUDE.md vẫn ghi target 0.162.0.
- (d) `~/.claude/settings.json` có `"model": "opus"` do spike `/model` — mặc định: để nguyên.

## 4. Quyết định và ràng buộc

- **Chủ nhân đã chốt** (đừng đề xuất lại): đối tượng dùng chung; vị trí Settings > Data > Ổ đĩa mở view lớn; xoá mặc định Thùng rác, xoá hẳn confirm 2 lần; MFT = UAC mỗi lần helper tự thoát; MCP = đọc + đề xuất xoá qua dialog; Telegram kèm top 5 thư mục, không tên file; click menu có cả 2 cách + công tắc mặc định bật; chi tiết Q1-Q31 trong 2 plan.
- Git: commit từng phần trên nhánh (đã cho cả phiên); **không push, không merge master** khi chưa được phép; không trailer attribution.
- Test app **chỉ bản dev tách biệt** (`apps/desktop/e2e/cdp.cjs`: `--user-data-dir`, `CCM_PORT`, `CCM_DATA_DIR`, `APPDATA` tạm); không dùng 4317/5173, không `npm run desktop:dev`, không kill Orrery đang chạy. `orrery-core.exe` thật bị Orrery khoá → build core vào target-dir trong scratchpad, không tạo thư mục build trong repo.
- File của session khác, **không commit**: `apps/shell/**`, 2 khối `reset` + `#[cfg(test)] mod tests` trong `core-desk/src/mcp/registry.rs`, `apps/desktop/e2e/{browser,p6,shell-agent,tier2,v70-tools,whats-new}.cjs`, `.code_index/_meta.md`, `.planning/DECISIONS.md` (dòng cũ), `.planning/PROGRESS-2026-11-11-*`, `PLAN-v102-*`, `COORD-*`, `RELEASE-4.0.6.md`, `ui-inventory/**`.
- Main.ts / preload.ts dùng chung giữa nhiều WP: commit bằng lọc hunk (`git diff -U0` → patch → `git apply --cached --unidiff-zero`); lọc từng dòng dễ lệch thứ tự, kiểm `git show :<file>` trước khi commit.
- Quirk: `rtk` lọc output `git diff` / vitest → dùng `rtk proxy <lệnh>`; `who_is_editing` MCP hay báo "Orrery window not ready"; vitest ghi đè `apps/desktop/.vitest/json/output.json` (tracked) → `git restore` trước commit; message vue-i18n chứa `|` phải escape (`messages.test.ts` bắt).

## 5. File cần đọc lại (theo thứ tự)

1. `.planning/PROGRESS-2026-11-15-click-disk.md` (luật + bảng WP + ghi chú tồn).
2. `.planning/COORD-2026-11-15-v100-v102.md` (phối hợp nhánh v102).
3. `docs/qa/v101/RESULT.md`, `.planning/wp/WP-QA-report.md`.
4. `.code_index/core-disk.md`, `desktop.md`, `web.md` (mục v100 / v101).
5. `.planning/PLAN-v100-click-menu.md`, `.planning/PLAN-v101-disk-manager.md`.
