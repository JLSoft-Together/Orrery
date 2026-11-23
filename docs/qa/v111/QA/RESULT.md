# QA tổng đợt v111 (23/11, nhánh feat/new_feat_2311, HEAD 4b6113df + bản sửa hide / restore chưa commit)

Cổng riêng 4521 / 9521, `CCM_DATA_DIR` tạm, không đụng Orrery đang chạy.

## Gate

| Mục | Kết quả |
|---|---|
| `npm run typecheck` | ĐẠT (0 lỗi, mọi workspace) |
| `npm test` (vitest, 370 file, 3153 test) | ĐẠT 3152 / 3153. 1 fail `telegramAnswer.test.ts` "works for a pane that moved out of the project folder" chỉ khi chạy cả bộ (tải cao), chạy riêng 5/5 pass: flaky, không do v111 |
| `cargo test --locked -j 4` apps/core | ĐẠT (81 suite, 0 fail) |
| `cargo test --locked -j 4` apps/shell | ĐẠT (621 pass, 0 fail, 4 ignored) |
| `node scripts/index-check.mjs` | ĐẠT (39 file, 0 lỗi) |
| Build web + shim + `cargo build -j 4` shell | ĐẠT |

## E2E mới của đợt (chạy riêng từng file)

| E2E | Kết quả |
|---|---|
| window-restore (bug hide / restore) | ĐẠT 13/13 |
| v111-rail-filter (112f) | ĐẠT 19 |
| v111-export (111b) | ĐẠT 32 |
| v111-preview (111c) | ĐẠT 24 sau khi sửa E2E (xem Lỗi 1) |
| v111-send-selection (112j) | ĐẠT 30 |
| run-pane (112i) | ĐẠT 71 |
| v111-wt (111d) | ĐẠT 16 |
| v111-checklist (111e) | ĐẠT 13 |
| wp112e-notify (112e) | ĐẠT 12 |
| v111-pr (111f) | ĐẠT 27 |
| v111-pick (111h) | ĐẠT 12 |
| v112b-first-run (112b) | ĐẠT 18 |
| v111-commit-chat (111g) | ĐẠT 28 |
| v112-header (112a) | ĐẠT 36 |
| v112d-launch-template (112d) | ĐẠT 9 |
| v112-cmdhist (112h) | ĐẠT 62 |
| v112-stash (112g) | ĐẠT 56 |
| v112c-empty (112c) | ĐẠT 6 |
| v109-shortcuts / v109-notify / v100-menu-toast | ĐẠT |
| v109-footer-sys | ĐẠT 30 sau khi sửa E2E (Lỗi 2) |

## Toàn bộ `run.cjs` (57 script, 35 FAIL ban đầu)

Sau khi loại trừ: còn lại là lỗi cũ / môi trường (xem mục dưới), không có lỗi do code v111.

| Script | Fail | Phân loại |
|---|---|---|
| p3 "Ctrl+Tab cycles to first tab" | 1 | Cũ: Ctrl+Tab là MRU switcher từ v109 (commit khi nhả Ctrl), E2E `chord` không nhả Ctrl. Không liên quan v111 (v109-shortcuts đạt 52/52) |
| v25 "workspace guide auto-opens once", "guide marked done" | 2 | Do v111 (112b): tour workspace chỉ chạy khi đã có tab đầu. E2E mở pane sau khi kiểm tour. Thử dời `open` lên trước thì check tip 3 lần bấm "+" rớt, nên giữ nguyên file (revert) và ghi lại: cần chủ nhân/main chỉnh v25 |
| whats-new "fresh install: only the guide shows" | 1 | Do v111 (112b), cùng nguyên nhân. ĐÃ SỬA (mở shell pane trước vòng quan sát), chạy lại 20/20 |
| v73 | 1 | Môi trường: thiếu module `sql.js` trong node_modules (fixture), không liên quan |
| v79-shots | 30 | Cũ: `FooterSysChip.vue` dùng `text-[9px]` (v109, commit 4573ebcd) vi phạm check "text < 11px" của v79. Chạy này ghi đè docs/qa/v79 (cần restore) |

## Kiểm tích hợp còn treo

| Mục | Kết quả |
|---|---|
| (a) Mẫu khởi chạy 112d ra cờ CLI thật | ĐẠT. E2E mới `v111q-launch-argv.cjs`: `claude.cmd` giả trên PATH, argv thật = `--session-id <id> --model opus --permission-mode plan --effort high --mcp-config ...`. Whitelist khớp `claude --help` 2.1.318 (`manual`, `plan`, `low..max`) và `codex` `-m` / `-s`. Lưu ý: `pty.rs` `env_clear` chỉ cho 11 biến lõi qua (PATH, TEMP...) nên env tùy ý không tới pane |
| (b) Chip dev server mất sau Ctrl+C (112h) | ĐẠT (v112-cmdhist: chip mất sau Ctrl+C, mất cả khi Ctrl+C ngay sau banner, pane vẫn chạy, lần 2 hiện lại chip) |
| (c) Toast + chip CI toàn cục | ĐẠT theo code + E2E: `useCiEvents()` ở App.vue:71 (ref-count 1 subscription, LogPane dùng chung); v111-pr thấy `[data-ci-toast="failure"]` và chip "CI running". CHƯA kiểm riêng ca "không mở Git Log" bằng E2E, chỉ có unit test `useCiEvents.test.ts` |
| (d) i18n snapshot | ĐẠT (vitest `src/i18n` nằm trong 3152 pass) |
| (e) `qa-fast.mjs` E2E_MAP | ĐÃ THÊM 17 dòng map cho E2E của đợt (`node --check` OK) |

## Lỗi tìm thấy và đã sửa (trong phạm vi)

1. `v111-preview.cjs`: E2E chạy lệnh `node -e` thoát ngay nên chip tắt đúng như 112h quy định, rồi test chờ chip còn. Sửa: banner giữ tiến trình sống (`setTimeout 120000`) và Ctrl+C trước mỗi lần chạy kế. Pass 24/24.
2. `v109-footer-sys.cjs`: đọc CPU ngay khi chip vừa hiện, mẫu CPU đầu tiên chưa có (next UI) nên 3 check rớt. Sửa: chờ `[data-sys-cpu]` tối đa 15s. Pass 30/30.
3. `whats-new.cjs`: mở shell pane trước (phụ thuộc 112b). Pass 20/20.

## Ảnh tổng quan desktop UI next (docs/qa/v111/QA/)

- `01-workspace-rail-header-footer.png`: rail (2 project, trạng thái Working / Waiting), header pane agent (nút ... , Needs you), footer (quota, Share, CPU / RAM). ĐẠT. Ghi chú: "20780d" trên rail là do fixture không có thời điểm hoạt động.
- `02-git-log.png`: Git Log có danh sách commit, diff, panel chi tiết. ĐẠT.
- `03-pane-more-menu.png`: menu "..." của pane.
- `03-git-log-pr-ci-chip.png`: Git Log có PR #42, chip "CI running", checks build / lint, nút Merge. ĐẠT.
- `04-ci-toast.png`: toast CI.

## Còn lại cho main

- v25 cần chỉnh theo 112b (tour chờ tab đầu); p3 Ctrl+Tab, v73 `sql.js`, v79-shots 9px là lỗi cũ.
- `run.cjs` chưa liệt kê các E2E mới v111 / v112 (chạy tay từng file ở trên).
- Ảnh docs/qa bị ghi đè cần restore: `docs/qa/v79/*` (25 file kể cả shots-results.txt), `docs/qa/v104/WP-104d|104e|104h/*` (15 file). Ảnh `docs/qa/v111/WP-*` (34 file) được chụp lại bởi E2E, là ảnh mới của đợt này, giữ được.
