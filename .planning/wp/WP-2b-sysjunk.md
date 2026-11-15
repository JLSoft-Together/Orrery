## WP-2b-sysjunk: nhóm Rác hệ thống + Dọn 1 chạm

- Mục tiêu: cắm thêm provider `group: 'system'` vào khung `apps/desktop/src/diskClean/` (WP-2a, commit 59f51156; đọc `.planning/wp/WP-2a-devjunk-report.md`, `.code_index/desktop.md` mục diskClean) và nút **Dọn 1 chạm** trong tab Dọn dẹp.
- Rác hệ thống (Q15, **chỉ danh sách vị trí biết trước**, mỗi nhóm có mô tả "xoá xong ảnh hưởng gì" + có cần đóng app nào không):
  - Windows: `%TEMP%` + `C:\Windows\Temp` (file cũ hơn 24 giờ, bỏ file đang bị khoá), Recycle Bin (hiện size; "dọn" = empty qua API shell `SHEmptyRecycleBinW` sau confirm, không tính vào Thùng rác), cache trình duyệt Chrome / Edge / Brave / Firefox (chỉ thư mục `Cache`, `Code Cache`, `GPUCache`; không đụng profile, cookie, history; trình duyệt đang chạy → khoá mục, lý do "đang mở"), crash dump (`%LOCALAPPDATA%\CrashDumps`, `C:\Windows\Minidump` cần admin → chỉ hiện size, ghi cần admin), log cũ Windows Error Reporting (`%ProgramData%\Microsoft\Windows\WER\ReportArchive`, `ReportQueue`), Delivery Optimization cache + Windows Update `SoftwareDistribution\Download` (cần admin → chỉ hiện size + hướng dẫn, không tự xin quyền ở WP này), thumbnail cache (`thumbcache_*.db`, Explorer giữ khoá → bỏ qua file khoá), installer cũ trong Downloads (`.exe` / `.msi` / `.dmg` / `.pkg` / `.zip` cài đặt cũ hơn 30 ngày: **không tick sẵn**).
  - macOS: `~/Library/Caches` (theo từng app, bỏ qua app đang chạy), `~/Library/Logs`, `/private/var/folders/.../C` của user (TMPDIR cũ hơn 24 giờ), Trash (`~/.Trash` hiện size, empty qua Finder AppleScript sau confirm), cache trình duyệt như trên, crash report `~/Library/Logs/DiagnosticReports`, installer `.dmg` / `.pkg` cũ trong Downloads (không tick sẵn).
  - **Không bao giờ**: `C:\Windows` ngoài các thư mục trên, `System32`, `Program Files`, `/System`, `/Library` (gốc), `/usr`, Documents, Desktop, Pictures, profile trình duyệt, keychain, `.ssh`, `.config`.
  - Vị trí cần admin: hiện size + nhãn "cần quyền admin", không có nút xoá ở WP này.
- **Dọn 1 chạm**: nút trên đầu tab Dọn dẹp, chạy quét mọi nhóm an toàn (Temp, cache trình duyệt của trình duyệt đang đóng, crash dump user, log cũ, rác dev cũ hơn N ngày của project không mở), hiện tổng + danh sách nhóm sẽ dọn (bỏ tick được), một nút "Dọn X GB" → cùng đường xoá `ccm:diskclean-delete` (dialog native, Thùng rác mặc định; Temp / cache trình duyệt cho phép xoá hẳn mặc định vì vào Thùng rác vô nghĩa: hỏi rõ trong dialog). Cache toàn cục (npm, Gradle...) và installer **không** nằm trong Dọn 1 chạm.
- Phạm vi file: `apps/desktop/src/diskClean/**` (provider mới `system.ts` / `winSystem.ts` / `macSystem.ts`...), `packages/shared/src/diskClean.ts`, `apps/web/src/components/disk/clean/**`, `apps/web/src/stores/diskClean.ts`, `apps/web/src/lib/diskClean.ts`, `apps/web/src/i18n/disk-clean-v101.ts`, test tương ứng, E2E `apps/desktop/e2e/v101-diskclean-system.cjs`. `preload.ts` / `main.ts` chỉ khi thật cần (thêm method empty bin): chỉ thêm khối riêng.
- Cấm đụng: `apps/web/src/components/workspace/**`, `components/explorer*`, Monitor (WP-2c đang làm), `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `apps/core/**`, `apps/shell/**`.
- Model: opus (chọn thư mục xoá trên dữ liệu người dùng, Windows/macOS hệ thống).

## Tiêu chí nghiệm thu

- [ ] Test: mỗi provider với root giả (env override như `CCM_E2E_*` hoặc tham số), bỏ file đang khoá, Temp chỉ lấy file cũ hơn 24 giờ, trình duyệt đang chạy → khoá, vị trí cần admin không có nút xoá, danh sách cấm không bao giờ lọt (test riêng với path cấm).
- [ ] Dọn 1 chạm: chọn đúng nhóm an toàn, không gồm cache toàn cục / installer / project đang mở.
- [ ] `npm test -w @ccm/desktop -- diskClean`, web test liên quan pass (`--maxWorkers=6`); typecheck sạch.
- [ ] E2E trên **bản dev tách biệt** (cổng riêng, `CCM_E2E_DISKMAP_CONFIRM=1`, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): **mọi root hệ thống trỏ vào thư mục tạm giả** qua env override; tuyệt đối không xoá Temp / cache / Thùng rác thật của máy. Chụp `docs/qa/v101/WP-2b/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-2b-sysjunk-report.md`.

## Nhật ký
- 15/11 WP-2b: đọc brief + WP-2a + review bổ sung (6 mục bảo mật); thiết kế provider system + Dọn 1 chạm.
- 15/11 WP-2b: desktop xong (system.ts, dialog.ts, service precheck/oneTap/emptyBin, projects tracked/electron-builder, caches guard, main/preload thêm khối riêng); typecheck desktop sạch.
- 15/11 WP-2b: test desktop diskClean-system (22) + diskClean cũ pass.
- 15/11 WP-2b: web xong (lib pickable/oneTapItems, store oneTap/emptyBin, CleanOneTap.vue, CleanRow/Group/DiskClean, i18n); test web 30 pass, typecheck sạch.
- 15/11 WP-2b: E2E v101-diskclean-system 26/26 PASS (4476/9476, fake roots), real bin 28 -> 28.
- 15/11 WP-2b: tối ưu quét Temp (45 s -> 6 s), E2E 2a 22/22 + 2b 26/26, code_index, báo cáo WP-2b-sysjunk-report.md. DONE.
