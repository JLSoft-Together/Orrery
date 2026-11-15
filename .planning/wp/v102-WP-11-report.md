WP-11 (QA tích hợp v102): ĐẠT, còn vài lệch nhỏ so mockup (liệt kê cuối)

## Số liệu
- Typecheck `npm run typecheck` (5 workspace): pass.
- Vitest `npm test -w @ccm/web`: 291 file, 2576 pass / 0 fail (lần đầu 1 fail: `next/palettes.test.ts` coi `from` / `to` của `@keyframes mist-spring` là selector, đã sửa test bỏ qua keyframe step).
- Build: `npm run build -w @ccm/web` + `npm run build -w @ccm/desktop` (gồm cargo release `orrery-core`) pass.
- E2E (Electron dev `apps/desktop/stage`, cô lập qua `cdp.cjs`, cổng 4460-4473, tối đa 2 song song). Kết quả lượt cuối:

| File | classic pass / fail | next pass / fail |
|---|---|---|
| v102-next (mới) | 23 / 0 | 23 / 0 |
| pane-drag | 13 / 0 | 13 / 0 |
| p2 | 14 / 0 | 14 / 0 |
| p3 | 15 / 0 | 15 / 0 |
| split | 24 / 0 | 24 / 0 |
| sidebar | 5 / 0 | SKIP |
| rail | 47 / 0 | 27 / 0 (+SKIP phần sidebar) |
| explorer | 33 / 0 | 30 / 0 |
| explorer-menu | 11 / 0 | 11 / 0 |
| v50-groups | 32 / 0 (*) | 29 / 0 (+3 SKIP) |
| v52-polish (gồm Mission Control) | 39 / 0 | 39 / 0 |
| v91-themes | 35 / 0 (*) | SKIP |
| whats-new (Ctrl+K, Settings) | 20 / 0 | 20 / 0 |
| v101-diskmap-ui (Ổ đĩa) | 30 / 0 | 30 / 0 |
| v101-qa | 19 / 0 | 19 / 0 |
| devices | 8 / 0 | 8 / 0 |
| v100-menu-queue | 17 / 0 | 17 / 0 |
| tabs-cwd | 17 / 0 | 17 / 0 (*) |

(*) Lượt cuối có flake, chạy lại 1 lần thì pass: `v91-themes` classic "no app page" (app không lên trong 60s lúc máy bận), `v50-groups` classic "dust runs without long frames" 1 frame 54-64 ms (lượt đầu pass, lượt chạy lại pass), `tabs-cwd` next 2 check Ctrl+click path (lượt đầu + chạy lại pass). Classic: 18 file, 0 fail sau khi chạy lại; next: 16 file chạy + 2 SKIP, 0 fail.

- `SKIP` ở next (có chủ đích, sidebar classic bị ẩn ở next, rail mới phủ bởi `v102-next.cjs`): `sidebar.cjs`, `v91-themes.cjs` (theme classic), phần Alt+↓ / Shift+F10 / multi-select / kéo project / cột avatar của `rail.cjs`, 3 check hình học sidebar của `v50-groups.cjs`.

## Hiệu năng (6 pane shell, 1600x900, Sương kính tối, CDP rAF trong trang)
| Chế độ | Resize viewport 40 bước (fps / p95 / max ms / frame > 33ms) | Kéo divider 90 bước | Idle 3s |
|---|---|---|---|
| classic | 90.1 / 27.8 / 41.7 / 7 | 116.3 / 13.9 / 20.9 / 0 | 144.1 |
| next (blur) | 130.3 / 13.9 / 27.8 / 0 | 109.4 / 20.8 / 27.8 / 0 | 144.3 |
| next, Giảm trong suốt | 133.1 / 7.1 / 34.8 / 1 | 122.8 / 13.9 / 20.9 / 0 | 144.2 |
- Màn 144 Hz. Blur chỉ trên 3 phần tử (rail `glass-side`, tab strip `glass`, `ccm-next-extra-in`); 0 phần tử blur trên `[data-pane]` / `.xterm`. Kéo divider ở next chậm hơn classic khoảng 6% (p95 20.8 vs 13.9 ms), vẫn trên 100 fps, không cần giảm blur rail. Mỗi số đo 1 lần, có nhiễu.
- `Browser.setWindowBounds` không đổi kích thước cửa sổ Electron qua CDP (innerWidth giữ 1360), nên resize đo bằng `Emulation.setDeviceMetricsOverride` (layout + xterm fit như resize thật, không có chi phí compositor của cửa sổ OS).

## Bug đã sửa (code v102)
- Ctrl+Shift+E ở next không mở Explorer: `WorkspaceView` và `ExplorerDrawer` cùng nghe keydown capture trên window, 1 bên mở 1 bên đóng. `components/workspace/WorkspaceView.vue` bỏ action `explorer` khi next.
- Explorer drawer báo "Không đọc được thư mục" với project mới: drawer luôn mount nên đọc trước khi PTY cấp quyền, `retryFailed` chỉ gắn với panel classic. `components/next/explorer/ExplorerDrawer.vue`: gọi `explorer.retryFailed()` khi pane running và khi drawer hiện.
- Kéo file từ drawer xuống terminal bị scrim chặn: `ExplorerDrawer.vue` scrim `pointer-events: none` trong lúc kéo path.
- `PaneHeader` là `<div>` nên mọi selector `[data-pane] header` (kéo pane, zoom, dim...) không thấy header ở next: `components/next/ui/PaneHeader.vue` root `<header>`.
- `GlassMenu` thiếu `data-menu-id` (menu "..." của rail không chọn được bằng id như ContextMenu): `components/next/ui/GlassMenu.vue`.
- Rail head: segmented có icon làm "Workspace" bị cắt ("Workspa..."), mockup chỉ chữ: `components/next/NextShell.vue` `textOptions`.
- Khối "Cần bạn" của rail xuống 2-3 dòng ("Cần / bạn", kbd gãy): `components/next/rail/RailNeeds.vue` nowrap, nhãn nút truncate + title.
- Ctrl+K next ghi "sau 36 giây nữa" cho session vừa đổi trạng thái (mốc now làm tròn xuống phút): `components/CommandPalette.vue` dùng cuối phút hiện tại.

## E2E đã sửa / thêm
- `apps/desktop/e2e/cdp.cjs`: `launch({ ui })`, mặc định `--ui=` / `CCM_E2E_UI`; next thì đặt `ccm.uiGeneration` rồi reload; export `UI`. Classic không đổi.
- Nhánh next (nhánh classic giữ nguyên): `pane-drag.cjs` (ghost chỉ tên pane, hành động nằm ở nhãn vùng thả), `p2.cjs` (nút Workspace ở segmented rail), `whats-new.cjs` (sheet kính có blur thay vì đặc), `v101-diskmap-ui.cjs` (Settings Disk ở trang `disk`), `explorer.cjs` (drawer: không resize panel, thả path ngoài la bàn, Ctrl+Shift+E đóng), `explorer-menu.cjs` (mở drawer bằng phím), `v52-polish.cjs` (row menu ở live rail, Dim ở `GlassMenu`), `rail.cjs`, `v50-groups.cjs`, `sidebar.cjs`, `v91-themes.cjs` (SKIP như trên).
- Mới `apps/desktop/e2e/v102-next.cjs` (23 check): classic -> next -> classic giữ nguyên node `.xterm` + id terminal, rail hiện 1600, không cuộn ngang, Ctrl+Shift+E mở / Esc đóng drawer, Ctrl+K có nhóm + row session, la bàn 5 ô khi kéo pane, toast Hoàn tác + Undo trả layout, Settings có trang Workspace + Ổ đĩa, 9 thẻ palette, chọn Tinh vân áp ngay, M5 (không sao băng trong panel / well, sao băng nền Ngân hà vẫn chạy), motion off dừng loop.

## Ảnh `docs/qa/v102/` (47 ảnh, demo fixture 8 session / 3 subagent, 2 pane gắn session thật để rail đủ trạng thái)
- Sương kính sáng + tối (`-mist-light` / `-mist-dark`) cho: `next-workspace-1600`, `next-explorer-drawer`, `next-explorer-pinned`, `next-ctrlk`, `next-drag-compass`, `next-mission`, `next-rail56-1100`, `next-rail56-flyout`, `next-window-1024`, `next-monitor-board`, `next-monitor-orbit`, `next-usage`, `next-history`, `next-disk`, `next-devices`, `next-settings-appearance`.
- 12 ảnh palette khác ở Workspace: `next-workspace-1600-{nebula,aurora,graphite,dawn,orrery}-{light,dark}`, `-galaxy-dark`, `-console-dark`.
- Classic (không đổi): `classic-workspace-galaxy`, `classic-monitor-galaxy`, `classic-settings-appearance-galaxy`.
- Không ảnh nào cuộn ngang (đo `scrollWidth - clientWidth` mỗi ảnh). Ảnh Devices / Ổ đĩa là dữ liệu thật của máy (adb + ổ đĩa), không phải fixture.

## Lệch còn lại so mockup (`.planning/mockups/v102/src/*.dc.html`, render bằng Electron)
- Rail: nút "Mở kế tiếp" trong khối Cần bạn bị cắt ở 264px (font 12 bold, mockup 11); nhóm Project / Trạng thái thiếu nhãn "Nhóm" phía trước.
- Ctrl+K: tiêu đề nhóm chưa có số đếm ("Cần bạn 2"); tab scope hiện số đếm thay ký hiệu prefix (`@ # > :`) như mockup.
- Settings next: nav có icon, chưa có tiêu đề "Cài đặt", thứ tự Workspace / Ổ đĩa ở cuối (mockup: Workspace ngay sau Giao diện); mục "Chung" có viền focus bị cắt nửa trên khi mở dialog.
- Terminal well đặc (mockup vẽ "Terminal trong suốt" bật); khớp mặc định `clearTerm` off.
- History: nhãn "Thứ 2..." của heatmap xuống 2 dòng ở 1600 (chưa đối chiếu classic).
- Board Monitor: nút "Mở trong Workspace" mờ vì session fixture không có pane (disabled 0.45, đúng hành vi cũ).

## Việc cho bước sau
- Ảnh v100 / v101 trong `docs/qa/v100/WP-0d`, `docs/qa/v101/QA`, `docs/qa/v101/WP-1c` bị E2E ghi đè khi chạy (test tự lưu ảnh vào repo); đã ghi lại nội dung HEAD cho các file đó, working tree không đổi ở các ảnh này.
- Nối E2E next vào `scripts/qa-fast.mjs` (ví dụ cờ `--ui=next`) nếu muốn gate tự động.
- Tài liệu: `CLAUDE.md` (What this is + ngoại lệ blur / gradient v102), `.planning/COLOR-SYSTEM.md` (mục v102), `.code_index/web.md` + `_meta.md`, `.planning/RELEASE-4.0.6.md` (mục v102 vi + en, 1 dòng Changelog chờ; không bump version, không sửa `changelog.ts`). `docs/` không có mục UI riêng nên không sửa.

## Sửa lệch mockup

- RailNeeds: header 2 dòng (tiêu đề + số / nút "Mở kế tiếp" + phím full width), không cắt chữ ở 248 và 264.
- LiveRail: thêm nhãn "Nhóm" (key `mist.rail.group.name`) trước segmented.
- CommandPalette next: tiêu đề nhóm có số đếm (`data-palette-group-count`).
- SettingsMenu next: tiêu đề "Cài đặt" đầu nav; `tokens.css` `.mist-settings nav` padding 2px cho viền focus.
- History next: `th[scope=row]` heatmap `white-space: nowrap` (tokens.css, chỉ next).
- Rail row dòng 2: `detailOf` (lib/next/railLive.ts) đổi " · " thành ", "; không còn ' · ' nào khác trong components/next, lib/next.
- Kiểm: vue-tsc pass, vitest components + lib/next 299 pass.
