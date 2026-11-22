# PROGRESS v110 (2026-11-22)

Plan: `.planning/PLAN-v110-modules.md`. Pipeline `v110-modules`.

## Bảng WP
| WP | Model | Việc | Chờ | Trạng thái |
|---|---|---|---|---|
| P1 | opus | Xoá Electron + collector | - | ĐẠT (kiểm full) |
| P3-0 | sonnet | Alias, web-move, importRules | - | ĐẠT |
| P2 | sonnet | Office lazy | P3-0 | ĐẠT |
| P3 lô A-C | sonnet | 14 feature | P2 | ĐẠT (A, B có E2E; C chưa E2E) |
| P3 lô D, 10, 11 | sonnet | files, git, agent-tools, monitor, map, workspace, rail, next | - | ĐẠT (kiểm nhẹ) |
| P3 lô 12 | opus | core / app | - | ĐẠT (kiểm nhẹ) |
| P4 | sonnet | Index + CLAUDE.md | - | ĐẠT |
| P5 | main | Đo dòng | - | ĐẠT; gate full CHỜ máy rảnh |

## Nhật ký
- Commit 6 commit v109 (b913af1e..6f4a0428), jin-workflow 8b91ef0, jin-tools ded3dbf + d1ebbdb. Chưa push.
- P3-0 ĐẠT: alias, `scripts/web-move.mjs`, `importRules.test.ts`, snapshot key i18n.
- P2 ĐẠT: `features/office/` lazy (70 file dời), registry `stores/connectionEvents.ts`, chunk chính 2217 → 2184 kB, 84 key Office nạp khi mở. Tauri luôn trả `coreServe=rust` nên dòng "Văn phòng chưa có trên bản này" luôn hiện khi bật Office.
- P3 chia lô: A (đợt 1-3: usage, extensions, update, history, resources) đang chạy; brief chung `.planning/wp/v110/P3-brief.md`.
- P1 ĐẠT: xoá `apps/desktop` (519 file, ~94k dòng) + `apps/collector` (144 file, ~27k dòng), lockfile -4030 dòng; dời preload → `apps/shell/shim/` (+ tsconfig giữ "use strict", shim.js không đổi), daemon-kit → `scripts/lib/`, fixture → crate tests, 115 E2E → `apps/shell/e2e/`; version `apps/shell/package.json`; `npm run dev` = `orrery-core serve --dev` + Vite. Kiểm: cargo core 421, shell 603, vitest web 2935 / pwa 306 / habblaud 538, E2E external-open 5/5, footer-sys 30/30, v91 classic 29/29. Giữ `CCM_ELECTRON_USER_DATA` (legacy.rs đọc), `CCM_TOAST_BACKEND=electron` (mac toast). Còn: `trace-compare/replay/samples/store-record` không còn nguồn trace, `cdp.cjs` ghi `electron.log` (đổi tên ở P4); CLAUDE.md sai nhiều chỗ → P4.
- Lô A ĐẠT: usage 16, extensions 15, update 13, history 23, resources 18 file → `features/<f>/` + index + README; vitest 2935, build PASS. Core stores còn import index usage/history/update/resources (gỡ ở đợt 12). Lô B (devices, disk, telegram, remote) đang chạy.
- Lô B ĐẠT: devices 42 (+ devPerf/devFiles từ resources), disk 79, telegram 17, remote 51 file; vitest 2935, build PASS; E2E devices, v101-diskclean, v98-remote-next, v101-diskmap-fast PASS. Treo cho gate cuối: `v97-remote` [12] F2 "Không rõ" (component + key không đổi, nghi test lệch trang Remote mới), `v101-diskmap-ui` / `v101-dupes` / `v101-organize` timeout chờ scan thật, `v97-device` Terminated. Lô C (click-menu, palette, settings, run + dời disk-clean-v101) đang chạy.
- Lô C ĐẠT (chưa E2E): click-menu 69, palette 6, settings 59, run 62 file; disk-clean-v101 vào features/disk (sửa path 2 E2E). vitest 2935 PASS 5 lần, có lần fail lẻ 1-9 test khác nhau (nghi tải máy, chưa so baseline). `stores/{settings,ui,uiGen}` chờ core. Lô D (bước 0 chạy E2E lô C, rồi files+git, agent-tools) đang chạy.
- Tạm dừng 2026-11-22 theo chủ nhân (cần máy, CPU 100%): lô D bị dừng giữa chừng (có thể đã chạy E2E lô C / đã dời 1 phần files-git). Làm tiếp: `git status` xem file đã dời dưới `apps/web/src/features/{files,git}`, chạy typecheck + vitest web, rồi giao lại lô D.
- Chủ nhân 22:1x: code tiếp, chỉ test nhẹ. Brief thêm mục "CHẾ ĐỘ NHẸ" (không cargo / E2E / build / vitest full, vitest 2 worker). Đang chạy 1 agent sonnet: hoàn tất lô D (files, git, agent-tools đã git mv) + lô 10 (monitor + sensitive, map).
- [x] 22:19 Handoff mới: `.planning/HANDOFF-2026-11-22-2219-v110.md`
- Lô D + 10 ĐẠT (kiểm nhẹ): files/git/agent-tools đủ index+README, vitest 483/483; monitor 54 + map 48 file (447 import), vitest 546/546 + related 28/28, typecheck PASS. `scripts/gen-file-icons.mjs` sửa 1 chuỗi đường dẫn. Core còn import sâu monitor/map (tránh vòng), monitor ↔ map import lẫn nhau. Chờ máy rảnh: build, E2E explorer, file-editor, log, diff, v104-clash, v104-turn-changes, v89-pane, v109-diff-comments, vitest full. Lô 11 (workspace, rail, next) đang chạy.
- Lô 11 ĐẠT (kiểm nhẹ): workspace 139 (gồm stores workspace/attention/awake), rail 37, next 73 file; vitest workspace 418, rail 133, next+i18n 153, stores/composables/lib/components 233; typecheck PASS. Vòng: workspace↔rail 21/28, workspace↔next 13/25, core→workspace 51, core→next 18. E2E chờ máy rảnh: term-ime, v109-term-search, v109-chat-view, rail-reorder, v109-project-menu, v102-next, v103-rail, rail, sidebar, tabs-cwd, v50-groups, v52-polish, ui-gen, v91-themes, v109-themes-all. Lô 12 (opus: core = không import feature, app = có import feature) đang chạy.
- Lô 12 ĐẠT (kiểm nhẹ): 128 file → core 92 (+4 file type/hằng tách từ feature), app 12, feature 24; `src/{components,lib,composables,stores}` đã xoá; `main.ts`, `App.vue`, `src/i18n` (en/vi/index + 7 file chung) giữ chỗ cũ. importRules: core ↛ features, core ↛ app (FAIL được, 0 vi phạm). Còn: feature → app 44 cạnh (connection 39), import sâu feature ↔ feature 638 (cảnh báo). vitest nhẹ: 156 + 831 + 758 + 545 PASS; typecheck PASS. P4 (index + CLAUDE.md) đang chạy.
- P4 ĐẠT: `.code_index` 696 KB (19 file) → 88 KB (38 file ≤ 8 KB, `lookup.md` 6.8 KB), file cũ → `archive/`; `CLAUDE.md` 25.6 → 7.3 KB (8 ngoại lệ giữ 1 dòng + `docs/core-principle.md` nguyên văn), docs mới `features-history.md`, `core-principle.md`, `visual-style.md`, `env.md`; `scripts/index-check.mjs` (0 lỗi, trong jin-scripts); `cdp.cjs` log → `shell.log`. Còn: P5 đo + gate full khi máy rảnh.
- [x] 23:05 Handoff mới: `.planning/HANDOFF-2026-11-22-2305-v110.md`
- Việc nhẹ sau 23:05 (máy bận): rà tĩnh không thấy đường dẫn chết trong `?raw` / `readFileSync` / glob / E2E_MAP (42 tên E2E đều có file); đổi 16 path web cũ trong `docs/*.md` + `bug_pool/` theo `git mv`; sửa 5 dòng Vai trò sai trong `.code_index` (mcp, disk, serve, telegram), lookup 48/48 symbol khớp, index-check 0 lỗi; dòng changelog chờ v109 + v110 ghi ở `.planning/RELEASE-next.md` (test changelog chặn thêm version trước khi bump); giữ `scripts/trace-*` vì còn dùng với `store-replay` Rust. Gate full vẫn chờ máy rảnh.
