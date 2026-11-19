## WP-68a: mockup web app iPhone (`apps/pwa`) theo style "Sương kính" (UI next gen v102) — chỉ mockup, chờ chủ nhân duyệt

- Chủ nhân chốt 2026-11-19: web app đổi sang style Sương kính cho đồng bộ desktop UI next. Bước này **chỉ mockup**, chưa sửa code app.
- Đọc trước: `.planning/PLAN-v98-merge-master.md` (mục C), `.planning/PLAN-v102-mist-redesign.md`, `.planning/SPEC-v102-mist-ui.md`, `apps/web/src/next/{tokens,palettes}.css`, `apps/web/src/next/palettes.ts`, `CLAUDE.md` (rule glass của next: blur chỉ ở chrome + lớp nổi, terminal / list / nội dung không blur, có "Giảm trong suốt"), `.code_index/web-pwa.md`, ảnh hiện tại `docs/qa/v98/WP-55a|55b|61a|61b|65/`.
- Model: opus. Chạy song song với WP-67 (desktop `apps/web`) — không sửa `apps/web`, `apps/pwa/src`.

### Việc
1. Mockup tĩnh HTML trong `docs/qa/v98/WP-68a/mockup/` (1 file HTML + CSS, mở bằng trình duyệt, không build, không font mạng) cho các màn: Home nhiều máy, Ghép, Monitor A + chi tiết, Workspace A (terminal) + B (lưới), viewer (Files / Diff / Git commit / Run), Cài đặt, bước "Bật khoá Face ID", banner lỗi `limit`.
2. Mỗi màn: phone 390×844 + iPad 820×1180, sáng + tối, 2–3 palette của next (chọn palette tiêu biểu), thêm 1 bản "Giảm trong suốt".
3. Glass chỉ ở chrome (thanh trên, tab bar, sheet, popover); terminal, danh sách dài, nội dung không blur; nền mist tĩnh như desktop. Ghi rõ chi phí GPU dự kiến trên iPhone và cách giảm (ít lớp blur, không blur khi cuộn, `Giảm trong suốt`).
4. Đề xuất cách dùng chung token với desktop (tách `apps/web/src/next/{tokens,palettes}.css` sang `packages/` hay copy có test đồng bộ) và CSP relay (`style-src 'self'`, không inline).
5. Chụp ảnh mockup (Playwright WebKit + Chromium) vào `docs/qa/v98/WP-68a/` + 1 trang so sánh cũ | mới.

### Luật (bắt buộc)
- Không viết comment trong code / HTML / CSS.
- Không `git commit` / `push`. Không sửa `apps/pwa/src/**`, `apps/web/**`.
- Bash: dùng tool Grep hoặc `rtk proxy rg`. Tắt tiến trình nền khi xong.

### Báo cáo
`.planning/wp/v98/WP-68a-report.md`: danh sách ảnh, quyết định thiết kế, chi phí GPU + cách giảm, đề xuất token / CSP, câu hỏi cho chủ nhân, ước lượng WP-68b (làm thật).
