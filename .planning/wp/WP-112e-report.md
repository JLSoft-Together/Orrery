## WP-112e — PARTIAL (code xong; chờ main gộp patch preload.ts / desktop.ts + gen shim)
- Đã đổi:
  - Shell: `apps/shell/src/tray_waiting.rs` (mới: parse, max 8 dòng, cắt 60 ký tự, nhãn, `entries`, `set`, `open`), `tray.rs` (menu dựng dòng chờ + header + separator trước menu cũ, tooltip, macOS title số, click `wait:<terminal>`), `ccm.rs` (kênh `ccm:tray-waiting`, LOCAL 45 -> 46), `main.rs` (mod), `strings.rs` (`tray_waiting`, `tray_tooltip` en / vi).
  - Web: `features/notify/projectLevel.ts` (mức all / waiting / off, khớp root dài nhất), `stores/projectNotify.ts` (map `ccm.notify.projects`), `trayWaiting.ts` (lọc inbox cần trả lời, che secret, cắt 60), `useTrayWaiting.ts` (chỉ gửi khi list đổi), `i18n.ts` (`projectNotify.*`), `App.vue` (gọi `useTrayWaiting`).
  - `app/composables/useNotifications.ts`: `notify` + doneToast + context toast + clash toast lọc theo mức; badge `useTaskbarBadge` giữ nguyên (Tắt vẫn đếm).
  - Rail: `railMenu.ts` (submenu `notify` + `ctx.notify`), `WorkspaceSidebar.vue` (ctx + pick `notify:<level>` set cho head, root, mọi checkout), `RailClusterView.vue` (icon BellOff + sr-only text khi Chỉ chờ / Tắt), `workspace/components/ProjectMenu.vue` (nút ⋯: 3 radio).
  - Test / E2E: `features/notify/projectNotify.test.ts`, `railMenu.test.ts` (+1 case), `apps/shell/e2e/wp112e-notify.cjs`. Snapshot `keys.en/vi.txt` thêm tay 6 key `projectNotify.*`. `.code_index/` shell.md, web-small.md, _meta.md.
- Tiêu chí: 3/4 đạt đủ, 1 PARTIAL.
  - (1) Rust test dựng menu (max 8, cắt chữ, ký tự không phải byte): đạt, 9 test `tray_waiting`.
  - (2) vitest 3 mức + store + tray rows + railMenu: đạt, 41 test.
  - (3) E2E: IPC `ccm:tray-waiting` nhận OK, menu project + icon rail đạt. CHƯA KIỂM: bấm dòng native trong tray menu (CDP không lái được), icon tray Windows có số (chỉ làm tooltip + macOS title).
  - (4) typecheck pass, cargo test: 616 pass, 1 fail `ccm::tests::the_local_channels_exist_in_the_generated_shim` vì shim chưa có `ccm:tray-waiting` (cần patch preload bên dưới rồi `node scripts/gen-desk-shim.mjs`).
- Audit: bấm dòng tray dùng đúng đường toast: show_main rồi ask `ccm:focus` (App.vue `reveal.show`), không trả lời thì emit `ccm:select`. Không submit gì vào pane. Câu hỏi che bằng `maskText` + regex secret (sk-, AKIA, xox, Bearer, JWT, token=...), cắt 60 ký tự cả web lẫn Rust.
- Review: (a) quyết định của con: project Tắt bị loại khỏi list tray và số tooltip, badge taskbar giữ nguyên; muốn tray vẫn hiện thì bỏ `allows` trong `useTrayWaiting.ts`. (b) Menu ⋯ (`ProjectMenu.vue`) chỉ set theo root của head; menu chuột phải set cả worktree checkout. Terminal ở worktree nằm ngoài root vẫn là mức "Tất cả" nếu chọn từ ⋯. (c) Toast menu v100 OS (`menu_toast.rs`) chưa lọc theo mức. (d) Khớp root không phân biệt hoa thường (cả Linux). (e) Mức "Chỉ chờ" cho qua kind waiting, reminder, job; chặn done, stuck, context, loop, clash.
- Test: `npm test -w @ccm/web -- src/features/notify src/features/rail/lib/railMenu` -> 41 pass; `npm run typecheck` -> pass; `npm test -w @ccm/web -- src/i18n src/importRules` -> importRules pass, snapshot fail chỉ vì key `v111pr.*` của WP khác (không có key projectNotify trong diff); `cargo test --locked -j 4` (apps/shell, target riêng) -> 616 pass, 1 fail như trên; `node apps/shell/e2e/wp112e-notify.cjs` (web build tạm + CCM_WEB_DIST, exe target riêng, port 4391 / 9391) -> 12 PASS, 0 FAIL.
- Nhìn: `docs/qa/v111/WP-112e/wp112e-01-project-menu.png`, `wp112e-02-rail-off.png`, `wp112e-03-project-menu-waiting.png` (chụp sau lần sửa code cuối; menu có mục "Notifications for this project" + 3 radio, icon chuông gạch cạnh project).
- Hot file cần thêm (main gộp):
  1. `apps/shell/shim/preload.ts`, sau dòng `taskbarBadge`: `trayWaiting: (payload: unknown): Promise<unknown> => ipcRenderer.invoke('ccm:tray-waiting', payload),` rồi chạy `node scripts/gen-desk-shim.mjs` (lúc con chạy script `--help` nó đã ghi lại `apps/shell/src/shim.js` từ preload hiện tại của 111d, nội dung không đổi ngoài việc đó).
  2. `apps/web/src/core/lib/desktop.ts`, cạnh `taskbarBadge?`: `trayWaiting?: (payload: unknown) => Promise<unknown>`; sau đó có thể bỏ cast `TrayBridge` trong `features/notify/useTrayWaiting.ts`.
  - Không cần sửa `en.ts` / `vi.ts` (dùng module `features/notify/i18n.ts` đã đăng ký sẵn).
- Ngoài phạm vi cần main xử lý: snapshot i18n đang đỏ vì key `v111pr.*` (WP 111a2); E2E `wp112e-notify.cjs` có thể thêm vào danh sách `npm run e2e -w @ccm/shell` nếu muốn; không có ảnh docs/qa cũ bị ghi đè.
- Commit message đề xuất: feat(notify): tray list of sessions waiting for you and per-project notification level
- Dòng changelog (en / vi, ≤ 100 ký tự): Tray lists sessions waiting for you; per-project notification level | Tray hiện session chờ chủ nhân; mức thông báo theo project

## Nhật ký
- Audit tray.rs, ccm.rs, useNotifications, railMenu, inbox; viết web + Rust; test; build web + shell vào thư mục tạm; E2E; ảnh; code_index; xoá target tạm.
