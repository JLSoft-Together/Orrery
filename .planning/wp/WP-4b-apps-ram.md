## WP-4b-apps-ram: danh sách app + size + gỡ cài đặt + RAM

- Mục tiêu (PLAN v101 Q6, Q12, Q18): 2 tab mới trong view Ổ đĩa.
  1. **App** (`group` riêng, không phải provider diskClean):
     - Windows: đọc registry `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`, `HKLM\SOFTWARE\WOW6432Node\...\Uninstall`, `HKCU\...\Uninstall` (dùng `reg query` qua `execFile` mảng tham số, hoặc crate / API có sẵn trong repo; không PowerShell ghép chuỗi): tên, publisher, version, ngày cài, `EstimatedSize`, `InstallLocation`, `UninstallString`, `QuietUninstallString`, `SystemComponent` (ẩn), `ParentKeyName` (ẩn bản cập nhật). Size thật: nếu `InstallLocation` có trong index `core-disk` dùng size index (op `disk.sizes` đã có), không thì `EstimatedSize`. Store app (MSIX): liệt kê nếu rẻ (`Get-AppxPackage` chỉ khi cần, ghi CHƯA KIỂM nếu bỏ), gỡ qua mở Settings > Apps (`ms-settings:appsfeatures`).
     - macOS: `/Applications` + `~/Applications` `.app` (tên, bundle id từ `Info.plist`, version, size từ index hoặc du nhanh, ngày dùng cuối `kMDItemLastUsedDate` qua `mdls` nếu rẻ).
     - Nút **Gỡ**: Windows chạy `UninstallString` của app đó sau dialog native ghi rõ lệnh sẽ chạy (parse an toàn: tách exe + args, chỉ chạy exe tồn tại, `MsiExec.exe /X{GUID}` cho phép; không chạy qua `cmd /c` chuỗi tự do; chạy không admin, trình gỡ tự xin UAC nếu cần); macOS chuyển `.app` vào Trash (`shell.trashItem`) sau confirm, chặn app hệ thống (`/System/Applications`, app Apple ký trong `/Applications` như Safari) và chính Orrery.
     - **Dọn sót** sau khi gỡ (hoặc nút riêng): liệt kê thư mục có tên khớp publisher / tên app / bundle id trong `%APPDATA%`, `%LOCALAPPDATA%`, `%PROGRAMDATA%` (Windows) và `~/Library/Application Support`, `Caches`, `Preferences/<bundle>.plist`, `Containers/<bundle>`, `Saved Application State` (macOS); **không tick sẵn**, khớp lỏng thì gắn nhãn "có thể", xoá qua đường xoá có guard canonical (diskMap / diskClean `runDelete`).
  2. **RAM**: dùng bảng process sẵn có (orrery-core procs / `resources` sampler, xem `.code_index/desktop.md` mục resources + `resKill` trong `main.ts`): process và nhóm theo app tốn RAM nhất (working set / RSS, % RAM máy), CPU %, đánh dấu process của agent / dev (claude, codex, node, java/gradle daemon, adb, emulator, docker) và process của chính Orrery (không cho kill Orrery chính / collector / core). Nút **Kết thúc** qua dialog native (dùng lại `resKill` / confirm đang có nếu khớp), không can thiệp bộ nhớ hệ thống. Cập nhật mỗi 2s chỉ khi tab đang mở.
- Bảo mật: IPC chỉ nhận id app (key registry / bundle path đã liệt kê ở main) hoặc pid đã liệt kê; main tra lại trước khi chạy. Env E2E chỉ khi `!app.isPackaged`.
- Phạm vi file: mới `apps/desktop/src/diskApps/**`, `apps/desktop/src/diskRam/**` (hoặc dùng lại `resources/`), khối riêng `main.ts` + `preload.ts`, mới `packages/shared/src/diskApps.ts` (+ export index; tên type tránh trùng: kiểm `rg` trước khi đặt tên), web `components/disk/apps/**`, `components/disk/ram/**`, `stores/diskApps.ts`, `stores/diskRam.ts`, `lib/diskApps.ts`, i18n mới `apps/web/src/i18n/disk-apps-v101.ts` (main gộp import en/vi), gắn 2 tab ở `DiskView.vue` (đọc lại trước Edit), test + E2E `apps/desktop/e2e/v101-apps-ram.cjs`.
- Cấm đụng: `apps/desktop/src/diskClean/**`, `diskMove/**`, `diskMap/guard.ts`, `packages/shared/src/{diskClean,diskMove}.ts`, `components/disk/clean/**`, `components/disk/move/**`, `stores/diskMove.ts`, i18n `disk-clean-v101.ts` / `disk-move-v101.ts` (WP-4a đang làm); `apps/core/**`; `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`, `apps/shell/**`.
- Model: opus (chạy trình gỡ, kill process).

## Tiêu chí nghiệm thu

- [ ] Test desktop: parse registry mẫu (fixture output `reg query`) gồm ẩn SystemComponent / bản cập nhật, parse UninstallString (exe có dấu cách, có nháy, MsiExec GUID, chuỗi độc hại `& del` bị từ chối), macOS Info.plist mẫu, dọn sót khớp đúng / không khớp nhầm thư mục chung (`Microsoft`, `Google` cha), kill bị chặn với process của Orrery.
- [ ] `npm test -w @ccm/desktop -- diskApps diskRam`, web test (`--maxWorkers=6`) pass; typecheck sạch.
- [ ] E2E trên **bản dev tách biệt** (cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): danh sách app thật chỉ đọc; **không gỡ app thật**: gỡ thử với app giả (fixture registry + exe giả qua env chỉ bản dev); RAM: kill một process con giả do E2E tự spawn (vd `node -e setInterval`), không kill gì khác. Chụp `docs/qa/v101/WP-4b/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-4b-apps-ram-report.md`.

## Nhật ký
- 15/11: desktop diskApps (reg export UTF-16, parse UninstallString, leftovers, mac plist) + diskRam (sampler, kill có kiểm start + lock self/parent/system), main/preload khối riêng; typecheck desktop sạch.
- 15/11: test desktop diskApps + diskRam 32/32 pass.
- 15/11: web lib/stores/components apps + ram, i18n disk-apps-v101 (tự merge qua components/disk/apps/messages.ts), tab ở DiskView; typecheck web sạch, test web 40/40.
- 15/11: E2E v101-apps-ram.cjs (4491/9491) 26/26 PASS, ảnh ở docs/qa/v101/WP-4b/.
- 15/11: audit: thêm chặn exe không phải .exe (tránh .bat qua Start-Process), nhóm RAM có self/parent không kết thúc cả nhóm, lịch sử dọn làm mới sau xoá sót; E2E chạy lại 26/26, ảnh cuối ở docs/qa/v101/WP-4b; cập nhật .code_index desktop.md + web.md.
