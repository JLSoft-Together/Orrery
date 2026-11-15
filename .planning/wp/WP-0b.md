## WP-0b: thanh nút menu nổi dưới pane + công tắc Settings

- Mục tiêu: khi pane Claude / Codex đang hiện menu chọn (parser `apps/web/src/lib/termMenu.ts` trả `confident: true`), dưới pane hiện thanh nút các đáp án; click là chọn bằng phím. Có công tắc Settings > Workspace (mặc định bật). Đọc `.planning/PLAN-v100-click-menu.md`, `.planning/wp/WP-0a-report.md` (kết luận phím), `.planning/COORD-2026-11-15-v100-v102.md` (luật chung với nhánh UI v102).
- Nguồn data: **parser màn** (hook chỉ có câu đầu, max 6 option, không tới page, theo WP-0a). Đọc buffer live của xterm (không đọc khi đang cuộn lên scrollback), throttle ~100ms theo sự kiện render/write, chỉ khi pane đang hiện + agent là claude/codex.
- Phím (theo WP-0a): menu có số → gửi phím số (Claude + Codex chọn luôn). Menu không số → Up/Down theo khoảng cách từ dòng đang highlight rồi Enter. multiSelect → phím số = tick/bỏ tick; nút Submit = Down tới dòng Submit rồi Enter (đọc lại màn sau khi gửi để kiểm đã tới Submit, chưa tới thì không Enter). `Type something` / Other → chỉ chuyển highlight tới đó rồi focus terminal để người dùng gõ.
- Gửi phím qua đường có sẵn: `stores/workspace.ts:1030` `typeKeys` → `TermPane.vue:1127` watch → `conn.write`. `useSessionActions.ts:28` `keysReady` đang chặn Codex cho phím option: không dùng `keysReady` cho click (click là người dùng chủ động), chỉ cần parser confident.
- UI: chỉ dùng token + utility có sẵn (`ccm-chip`, `ccm-seg`, `ccm-label`, `ccm-panel`...), không thêm màu / style mới, không sửa `style.css` / `themes.css`. Mỗi nút hiện số (nếu có) + label, `title` = label đầy đủ. Không che nội dung terminal: đặt dưới vùng terminal, pane co lại, hoặc nổi ở mép dưới không che dòng menu. Ẩn khi menu biến mất.
- Phạm vi file: mới `apps/web/src/composables/useTermMenu.ts` (+test), `apps/web/src/lib/termMenuKeys.ts` (+test, hàm thuần dựng chuỗi phím từ menu + item), `apps/web/src/components/workspace/TermMenuBar.vue` (+test nếu dễ); sửa `TermPane.vue` (gắn bar, cấp buffer), store settings (thêm `clickMenu` mặc định `true`, key lưu `ccm.clickMenu`), component Settings mục Workspace (công tắc), `apps/web/src/i18n/en.ts` + `vi.ts` (namespace `clickMenu.*`, thêm cuối block, không sắp xếp lại file). Được sửa `termMenu.ts` nếu cần thêm field.
- Cấm đụng: `style.css`, `themes.css`, `apps/desktop/**`, `apps/core/**`, `App.vue`, `AppHeader.vue`, `CommandPalette.vue`.
- Phụ thuộc: WP-0a (đã commit). Song song với WP-1a (Rust).
- Model: sonnet.

## Tiêu chí nghiệm thu

- [ ] `termMenuKeys` có unit test cho: menu có số, không số (Up/Down), multiSelect tick, Submit, Other, Codex.
- [ ] Bar hiện đúng khi fixture menu được nạp vào pane (test component/composable với buffer giả), ẩn khi không có menu / `confident: false` / công tắc tắt / đang cuộn scrollback.
- [ ] Công tắc Settings > Workspace lưu được, mặc định bật, text qua i18n en + vi.
- [ ] Không thêm màu / CSS token mới; status/nhãn không chỉ dựa vào màu.
- [ ] `npm test -w @ccm/web` pass toàn bộ; `npm run typecheck -w @ccm/web` sạch.
- [ ] Chạy thật trên **bản dev tách biệt** (không đụng Orrery đang chạy): build web + desktop rồi dùng harness `apps/desktop/e2e/cdp.cjs` (tự cô lập userData / CCM_PORT / CCM_DATA_DIR / APPDATA). Viết E2E `apps/desktop/e2e/v100-menu-bar.cjs` theo mẫu file e2e có sẵn: mở pane shell, in một màn menu fixture (vd `cat` file fixture) để parser nhận, kiểm bar hiện, click nút thì phím đúng được gửi (đọc lại buffer). Chụp ảnh vào `docs/qa/v100/WP-0b/`. Không kill process Orrery/Electron đang chạy, không dùng `npm run desktop:dev`.

- Lệnh test: `npm test -w @ccm/web && npm run typecheck -w @ccm/web`; build: `npm run build -w @ccm/web` + build desktop theo `package.json` của `apps/desktop`; E2E: `node apps/desktop/e2e/v100-menu-bar.cjs`.
- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-0b-report.md`.

## Nhật ký
- 17:13 code + unit tests xong (termMenuKeys, useTermMenu, TermMenuBar, TermPane, settings, i18n). Chạy full test + typecheck, rồi E2E.
- 17:25 E2E 15/15 PASS, ảnh docs/qa/v100/WP-0b. Viết báo cáo.
