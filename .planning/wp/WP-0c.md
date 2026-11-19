## WP-0c: click thẳng dòng menu trong terminal + chặn mouse Codex + mẫu thật còn thiếu

- Mục tiêu: khi parser (`apps/web/src/lib/termMenu.ts`) confident và công tắc `clickMenu` bật, hover dòng đáp án trong xterm thì dòng sáng lên; click = chuyển highlight tới dòng đó (Up/Down); double click = chọn (menu có số: phím số; không số: tới dòng rồi Enter); multiSelect: click = tick/bỏ tick; dòng `Type something` / Other: chuyển highlight rồi để con trỏ gõ; Submit: như logic thanh nút. Không có menu hoặc click ngoài dòng menu → giữ hành vi cũ (bôi chọn chữ, double click chọn từ). Đọc `.planning/PLAN-v100-click-menu.md`, `.planning/wp/WP-0a-report.md`, `.planning/wp/WP-0b-report.md` (cách bar gửi phím + đọc lại màn, dùng lại `termMenuKeys.ts` + `useTermMenu.ts`).
- Codex bật mouse mode (`1003h` + `1006h`): xterm đang gửi sự kiện chuột vào Codex. Khi click rơi vào dòng menu đã nhận diện → chặn trước khi xterm gửi (capture listener / `attachCustomMouseEventHandler` nếu xterm có, hoặc tương đương), rồi tự gửi phím. Click ngoài dòng menu → để nguyên cho Codex.
- Hover: dùng decoration / overlay của xterm hoặc lớp phủ DOM cùng toạ độ ô; màu từ token có sẵn (vd selection / `ccm-pane-*`), không thêm màu / CSS token mới, không sửa `style.css` / `themes.css`. Con trỏ chuột đổi sang pointer trên dòng menu.
- Chỉ trên màn live: đang cuộn scrollback thì không bắt.
- Mẫu thật còn thiếu: chụp lại bằng `scripts/capture-term-menu.mjs` menu **hỏi quyền tool** Claude, **duyệt plan** (ExitPlanMode) và **Codex approval**, thay 3 fixture synthetic; được chạy tối đa 4 prompt ngắn `claude --model haiku` / `codex` model rẻ nhất trong thư mục scratchpad (không trong repo). **Không được đổi `~/.claude/settings.json` hay config Codex**: chụp bằng `--settings` / env tạm hoặc HOME tạm nếu cần; trước khi chạy, sao lưu `~/.claude/settings.json`, chạy xong so sánh, khác thì trả lại và ghi vào báo cáo. Không bấm vào menu `/model`, `/config`.
- Phạm vi file: `apps/web/src/components/workspace/TermPane.vue`, mới `apps/web/src/composables/useTermMenuMouse.ts` (+test), `termMenu.ts` / `termMenuKeys.ts` / `useTermMenu.ts` nếu cần mở rộng, fixtures `apps/web/src/lib/__fixtures__/term-menu/*` + `termMenu.test.ts`, E2E `apps/desktop/e2e/v100-menu-click.cjs`, `apps/web/src/i18n/click-menu-v100.ts` (nếu cần tooltip).
- Cấm đụng: `style.css`, `themes.css`, `apps/desktop/src/**`, `apps/core/**`, `App.vue`, `AppHeader.vue`, `CommandPalette.vue`, `stores/ui.ts`, Settings, `apps/web/src/components/disk/**`, `lib/treemap.ts`, `lib/sunburst.ts`, `lib/diskMap.ts`, `stores/diskMap.ts` (WP khác đang làm).
- Model: sonnet. Phụ thuộc: WP-0b (04e390e1).

## Tiêu chí nghiệm thu

- [ ] Unit test map toạ độ (row trong viewport ↔ item), quyết định chặn hay để nguyên, chuỗi phím click / double click cho từng loại menu.
- [ ] 3 fixture synthetic được thay bằng mẫu thật (hoặc ghi rõ lý do không chụp được); parser vẫn đúng 100% dương, 0 false positive.
- [ ] `~/.claude/settings.json` không đổi sau khi chụp (so hash trước / sau, ghi vào báo cáo).
- [ ] E2E trên **bản dev tách biệt** (harness `apps/desktop/e2e/cdp.cjs`, cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): hover sáng, click di highlight, double click gửi phím đúng, multiSelect tick, click ngoài menu vẫn bôi chữ, pane Codex giả bật mouse mode (in `\x1b[?1003h\x1b[?1006h`) thì click dòng menu không lọt sự kiện chuột vào PTY. Chụp `docs/qa/v100/WP-0c/`.
- [ ] `npm test -w @ccm/web` pass toàn bộ; `npm run typecheck -w @ccm/web` sạch.
- [ ] Không comment trong code.

- Lệnh test: `npm test -w @ccm/web && npm run typecheck -w @ccm/web`; build web + desktop; `node apps/desktop/e2e/v100-menu-click.cjs`.
- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-0c-report.md`.

## Nhật ký
- 17:25 code xong: termMenuHit.ts, termMenuKeys (highlight/confirm), useTermMenu.click, useTermMenuMouse (+test), TermPane nối; unit pass. Tiếp: chụp mẫu thật, E2E.
- 17:35 fixture thật claude-permission/plan + codex-approval thay synthetic, test pass; settings.json hash không đổi. Codex lần 1 vô tình Enter vào hộp 'Update now' (npm i -g codex 0.162.1 -> 0.169.0), ghi báo cáo. Tiếp: E2E.
- 17:55 E2E v100-menu-click 24/24 PASS (cổng 4412/9412). Tiếp: chạy lần cuối + full test + typecheck + báo cáo.
- 18:05 xong, báo cáo ở WP-0c-report.md
