# PLAN — web app iPhone / iPad đổi sang "Sương kính" (WP-68b, 2026-11-19)

Nguồn: mockup `docs/qa/v98/WP-68a/` (`compare.html`, `mockup/index.html`), báo cáo `.planning/wp/v98/WP-68a-report.md`.

## Chủ nhân chốt (2026-11-19)
1. Hạn mức Claude Code / Codex: desktop gửi sang điện thoại, hiện trên thẻ máy (Home) và chân rail iPad, như mockup.
2. Bảng màu chọn riêng trên điện thoại (Cài đặt → Giao diện), mặc định Sương kính.
3. Tab bar iPhone: viên nổi như mockup.
4. iPad: bỏ tab bar dưới, dùng rail trái giống desktop (máy, session theo project lùi vào, hạn mức ở chân rail).
- Main mặc định (không hỏi): đóng gói Manrope + JetBrains Mono như desktop (offline, ~110 KB); "Giảm trong suốt" mặc định tắt (tự bật khi iOS bật Reduce Transparency); tách token sang `packages/mist-tokens`.

## Gói
| Gói | Model | Nội dung | Phụ thuộc |
|---|---|---|---|
| WP-68b1 | sonnet | `packages/mist-tokens` (tách biến `apps/web/src/next/{tokens,palettes}.css` + `palettes.ts`, desktop import lại, không đổi giao diện desktop) + khung pwa: `data-ui="next"`, nền sương, kính thanh trên / tab bar viên nổi / sheet, `--ccm-bar`, font bundle, Cài đặt → Giao diện (bảng màu, sáng / tối, Giảm trong suốt), CSP relay không đổi | — |
| WP-68b2 | sonnet | Hạn mức sang điện thoại: desktop đưa số hạn mức thật (Claude Code 5h / tuần, Codex) vào dữ liệu gửi điện thoại (trường tuỳ chọn), không có số thì không gửi; pwa store đọc; Android chỉ parse bỏ qua an toàn | — (song song b1, không đụng CSS) |
| WP-68b3 | sonnet | Màn Home (thẻ máy + hạn mức), Ghép, bước Face ID, Monitor A / B, chi tiết, banner limit theo mockup | b1, b2 |
| WP-68b4 | sonnet | Workspace A / B (vẽ `tabs[].groups` của WP-67), Terminal + dock phím đặc, Viewer (Tệp / Diff / Git / Chạy), iPad rail trái | b1 |
| WP-68b5 | sonnet | QA: E2E WebKit + Chromium toàn bộ, ảnh so mockup, đo FPS cuộn (Playwright trace), kiểm CSP 0 vi phạm, bundle relay sinh lại | b3, b4 |

## Luật chung
- Không comment trong code. Chuỗi qua i18n `en` + `vi` (không em dash, không chấm giữa). Màu qua token. Status = icon + chữ + màu.
- Blur chỉ ở thanh trên, tab bar, rail iPad, sheet / popover; thẻ, danh sách, terminal, code, log, dock phím không blur; blur điện thoại ≤ 16px; không bật / tắt blur khi cuộn; nền sương vẽ 1 lần trên lớp cố định (iOS bỏ qua `background-attachment: fixed`).
- CSP relay `style-src 'self'` (không inline style / script); xterm dùng shim `xtermStyles.ts` có sẵn.
- Không `git commit` / `push`. Không deploy relay. `JIN_FORCE=1` nếu resource-guard chặn vì tải app khác. Tắt tiến trình nền khi xong.
- Test: `npm run typecheck -w @ccm/pwa`, `npm test -w @ccm/pwa`, `npm run e2e -w @ccm/pwa` (ảnh cũ trong `docs/qa/v98/WP-55b|56a|61a|61b|65/` bị ghi đè → `git checkout --`); desktop đụng thì typecheck + vitest web.
- Báo cáo `.planning/wp/v98/<WP>-report.md` có mục "Test macOS / iPhone" (main chép vào `.planning/QA-mac-checklist.md`).
