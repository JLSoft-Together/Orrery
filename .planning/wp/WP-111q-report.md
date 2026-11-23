## WP-111q — DONE
- Đã đổi: `scripts/qa-fast.mjs` (+17 dòng E2E_MAP cho E2E đợt v111 / v112); `apps/shell/e2e/v111-preview.cjs` (banner sống lâu + Ctrl+C, khớp 112h); `apps/shell/e2e/v109-footer-sys.cjs` (chờ mẫu CPU); `apps/shell/e2e/whats-new.cjs` (mở shell pane trước, khớp 112b); mới `apps/shell/e2e/v111q-launch-argv.cjs` (đo argv thật của claude giả), `apps/shell/e2e/v111q-overview.cjs` (ảnh tổng quan); `docs/qa/v111/QA/*`. Không sửa code sản phẩm.
- Tiêu chí: 6/6 việc làm xong; mục (c) chỉ kiểm bằng code + unit + v111-pr, chưa có E2E riêng "không mở Git Log".
- Audit: whitelist cờ launch khớp `claude --help` 2.1.318 và codex; `pty.rs` env_clear chỉ cho 11 biến lõi.
- Review: v25 vẫn FAIL 2 check tour (112b), thử dời `open` thì rớt check tip nên revert; chủ nhân/main quyết.
- Test: typecheck sạch; vitest 3152/3153 (1 flaky telegramAnswer, riêng lẻ pass); cargo core 81 suite 0 fail; cargo shell 621 pass; index-check ok; E2E 19 file mới + v109 đều ĐẠT; `run.cjs` 57 script: 35 fail ban đầu, còn lại là lỗi cũ (p3 Ctrl+Tab MRU, v73 sql.js, v79-shots chip 9px) + v25.
- Nhìn: docs/qa/v111/QA/01..04 (rail, header, footer, Git Log, PR + CI chip).
- Hot file cần thêm: không.
- Ngoài phạm vi cần main xử lý: restore `docs/qa/v79/*` (25) và `docs/qa/v104/WP-104d|e|h/*` (15) bị E2E ghi đè; chỉnh v25; thêm E2E mới vào `run.cjs`; `FooterSysChip` 9px vi phạm check v79 (cũ).
- Commit message đề xuất: test(qa): v111 QA, E2E map entries, fix preview / footer-sys / whats-new E2E, launch argv and overview E2E
- Dòng changelog (en / vi, ≤ 100 ký tự): (không có, chỉ test)

## Nhật ký
- Gate, build, E2E mới, run.cjs đầy đủ, ảnh tổng quan và RESULT.md xong.
