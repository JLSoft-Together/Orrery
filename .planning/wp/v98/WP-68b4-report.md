# WP-68b4 báo cáo: Workspace nhóm project, dock phím, Viewer, iPad rail

Trạng thái: code xong, vitest + typecheck pass. **E2E chưa chạy được** vì ổ D: đầy 100% (`LNK1180 insufficient disk space` khi link `pwa-e2e-peer.exe`). Chưa có ảnh `docs/qa/v98/WP-68b4/`, chưa có số đo cuộn / terminal.

## Đã làm
1. **Nhóm project** (`lib/workspaceModel.ts`): `WsTab.groups: WsGroup[]` đọc `tabs[].groups` (tone 0-7, `folded`, pane lạ / trùng bỏ, pane thừa vào nhóm `~rest`). Không có `groups` thì `[]` và UI giữ như cũ.
   - A (`PaneChips.vue`): nhãn nhóm (monogram tone + tên) trước chip của nhóm, testid `chip-group-<project>`; chip vẫn là `button` trực tiếp.
   - B (`PaneGrid.vue`): section `ws-group-<project>` có header gập (`ws-group-toggle-<project>`, `aria-expanded`, `data-folded`, mặc định theo `folded`, animation grid-rows); lọc Tất cả / Đang chạy / Chờ bạn vẫn áp lên từng nhóm.
   - Tab (`WorkspaceView.vue`): pill kính có monogram + số pane chờ bạn / lỗi (`ws-attn-<tab>`). Thao tác pane WP-61a giữ nguyên.
2. **Terminal**: `[data-testid=dock]` bọc KeyBar + InputBar, nền `--ccm-material` đặc, `backdrop-filter: none`; stage terminal bo góc, `--ccm-term-bg`. `xtermStyles.ts` không đụng.
3. **Viewer**: code / diff / log đã dùng `--ccm-term-bg` đặc, nền trang `--ccm-canvas`; không đổi thêm (chưa soi ảnh).
4. **iPad rail** (`components/IpadRail.vue`, `lib/railModel.ts`, `App.vue`): >= 768px, route monitor / session / workspace / settings có máy thì rail trái 300 px thay tab bar: máy + `LinkChip`, segmented Giám sát / Workspace, Monitor = session lùi vào theo project, Workspace = tab > nhóm project > pane, hạn mức ở chân rail (dùng `QuotaBars` của WP-68b3), thu gọn 68 px (`prefs.railCollapsed`, nhớ qua reload), hàng icon Máy / Ghép / Cài đặt. Chi tiết session / pane thay cột master (`.master.off`, `aside.detail` giữ testid `detail-pane`). Home / Ghép giữ master + detail cũ (phần của b3). iPhone giữ tab bar viên nổi. Blur chỉ ở rail.
5. i18n `en` + `vi`: `rail.*`, `workspace.attention`, `workspace.groupToggle`. `styles/base.css`: `.tone-mono`.
6. E2E soạn sẵn nhưng chưa chạy: `e2e/lib/mist-ws-steps.mjs` (3 bước: nhóm, dock + không blur + burst 64 KiB, iPad rail + cuộn), peer `groups` / `ungroups` (`e2e/peer/src/{main,data}.rs`), `qaDirG` trong `run.mjs`, `STEPS_PER_BROWSER` 44 -> 47 (**cộng dồn nếu b3 cũng thêm bước**).

## File chung đã sửa tối thiểu
`App.vue` (rail, `hasDetail`), `stores/prefs.ts` + `storage/localPrefs.ts` (`railCollapsed`), `i18n/en.ts` + `vi.ts`, `styles/base.css`, `e2e/lib/scenario.mjs`, `e2e/run.mjs`. Dùng lại `QuotaBars.vue` và `lib/projectModel.ts` của b3 (`workspaceModel` re-export `toneOf` / `letterOf`).

## Test (lệnh chạy thật)
- `npm run typecheck -w @ccm/pwa`: sạch.
- `npm test -w @ccm/pwa`: 25 file, 235 / 235 (thêm `lib/wsGroups.test.ts`: groups, rail, prefs).
- `npm run qa -- --no-e2e --no-cargo`: không map workspace nào (0/0).
- `npm run e2e -w @ccm/pwa -- --browsers=webkit ...`: **không chạy được** (lượt 1 chờ cargo của pane khác quá 10 phút; lượt 2 link peer fail do hết đĩa).

## Việc dở
- Giải phóng ổ D: rồi chạy `npm run e2e -w @ccm/pwa -- --relayPort=8795 --inspectorPort=9355` (build peer lại để có lệnh `groups`); sửa các lỗi selector có thể có trong `mist-ws-steps.mjs` (chưa từng chạy); sau đó `git checkout --` ảnh cũ bị ghi đè trong `docs/qa/v98/WP-55b|56a|61a|61b|65|68b1/`.
- Ảnh phone + iPad sáng / tối + Ngân hà vào `docs/qa/v98/WP-68b4/` (step đã soạn sẽ sinh), soi mắt với ảnh `WP-68a`.
- Đo FPS dùng rAF (không phải Playwright trace); WebKit Windows không vẽ blur nên cần đo tay trên iPhone.

## Test macOS / iPhone
- [ ] iPad ngang và dọc: rail trái thay tab bar; chọn session / pane mở chi tiết bên phải; thu gọn rail rồi mở lại app vẫn nhớ.
- [ ] iPad: rail blur không làm giật khi cuộn danh sách dài; terminal output nhanh vẫn 55+ fps.
- [ ] Desktop có tab chia nhóm project: Workspace A hiện nhãn nhóm, B hiện section gập (nhóm `folded` đóng sẵn), màu tone khớp rail desktop.
- [ ] iPhone: dock phím đặc, không thấy chữ terminal / nền sương xuyên qua dock; bàn phím ảo mở lên dock không bị che.
- [ ] Viewer Tệp / Diff / Git / Chạy không blur, cuộn mượt.
