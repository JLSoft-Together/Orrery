## WP-2sec - DONE

| # | Mục | Kết quả |
|---|---|---|
| 1 | HIGH path Windows lọt guard (`.` / space cuối, 8.3, `:`) | Đã sửa: lexical win32 chặn segment kết thúc `.` / space, chứa `: * ? " < > \|`; canonical `fs.realpath` expand 8.3 rồi kiểm lại; còn segment 8.3 sau resolve -> `invalid`; `\\host\C$\...` so như `C:\...` |
| 2 | MEDIUM junction / symlink ở giữa | Đã sửa: realpath cả path (target là link thì realpath cha + tên link), canonical kiểm root / system / protected; diskMap bắt buộc path là node trong index (`known` qua `disk.children`) -> `outside` |
| 3 | MEDIUM danh sách bảo vệ thiếu | Đã sửa trong `guard.ts` (áp cho cả diskMap và diskClean, không cần sửa keep ở main): self `AppData`, `AppData\Roaming|Local|LocalLow`, `.config`, macOS `Library`, linux `.local`, `.local/share`, mọi con trực tiếp của `X:\Users` / `/Users` / `/Volumes` / `/home` / `/mnt` / `/media`, `OneDrive*` dưới home, `ctx.volumes` (diskMap lấy từ `disk.volumes`); cả cây `.ssh`, `.gnupg`, `Library/Mobile Documents`, `Library/CloudStorage`, `Library/Keychains`, `ctx.keepTree`. `~/Library/Caches/*` vẫn xoá được (diskClean cần) |
| 4 | MEDIUM `autoConfirm` ở bản đóng gói | Đã sửa: `e2eAutoConfirm(process.env, app.isPackaged)` ở `setupDiskMap` + `setupDiskClean` |
| 5 | LOW `rm` dừng giữa chừng | Đã sửa: xoá hẳn lỗi mã khác EACCES / EPERM / ENOENT -> vẫn `disk.forget`, `measureRest` đo phần còn lại, cộng phần đã xoá vào `freed` + `paths` lịch sử |
| 6 | LOW TOCTOU sau dialog | Đã sửa: sau confirm resolve lại từng target, khác trước (path / link / missing) -> `changed`, lý do guard mới -> lý do đó; xoá đúng path canonical đã kiểm |

## Hàm export cho WP-2b (`apps/desktop/src/diskMap/guard.ts`)

- `checkDeletableCanonical(p: unknown, ctx: GuardContext, io?: CanonicalIo): Promise<GuardReason | null>`
- `resolveDeletable(p: unknown, ctx: GuardContext, io?: CanonicalIo): Promise<{ reason: GuardReason | null; canonical?: Canonical }>`
- `canonicalOf(p: string, platform, io?): Promise<Canonical | null>`, `sameCanonical(a, b, platform)`, `realIo`
- `GuardContext` thêm tuỳ chọn `keepTree?: string[]` (bảo vệ cả cây), `volumes?: string[]` (mount point); `GuardReason` thêm `'changed'`.
- `apps/desktop/src/diskMap/delete.ts`: `runDelete` đã tự gọi `resolveDeletable` trước và sau confirm (diskClean dùng `runDelete` nên được hưởng luôn). `DeleteDeps` thêm `known?`, `measure?`, `io?`. Export `measureRest(p)` (nên truyền `measure: measureRest` ở diskClean), `e2eAutoConfirm(env, packaged)`.
- WP-2b lưu ý: `checkDeletable` (sync) giờ cần `ctx.keep` có thể thiếu (đã `?? []`), nhưng `providers.ts:103` còn `ctx.keep.some` nên test `diskClean.test.ts` "Gradle home that is a link" fail vì ctx test không có `keep`; test ipc fail vì 5 kênh (code WIP của WP-2b). macOS: TMPDIR `/private/var/folders/...` và `/tmp` resolve ra `/private/...` = `system`.

## Test

- `npx vitest run diskMap --maxWorkers=6` (apps/desktop): 7 file, 65 pass, 0 fail.
- Fail trước: chạy `checkDeletable` của HEAD với `C:\Users\dev.`, `C:\Users\dev `, `C:\Users.`, `C:\PROGRA~1`, `C:\Users\DEV~1`, `D:\work\a:stream`, `AppData\Roaming`, `.ssh`, `Users\Public`, `Users\other`, `OneDrive`, mac `~/Library`, `Mobile Documents/x`, `/Volumes/Ext` -> toàn `null` (cho xoá). File tạm đã xoá.
- Junction thật trong `%TEMP%` (`scan\lnk` -> home giả, `scan\users` -> `C:\Users`): lexical `null`, canonical `protected`; `C:\PROGRA~1` -> `system`; tên 8.3 thật của thư mục trong tmp -> `protected`.
- `npm run typecheck -w @ccm/desktop`: sạch.
- `node apps/desktop/e2e/v101-diskmap-host.cjs` (4471 / 9471): 18/18 PASS; `node apps/desktop/e2e/v101-diskmap-ui.cjs` (4472 / 9472): 30/30 PASS (desktop build mới, web dist có sẵn).
- `npx vitest run diskClean`: 2 fail thuộc code WIP của WP-2b (nêu trên), không do guard.

## CHƯA KIỂM

- macOS / linux trên máy thật (chỉ unit theo platform); NFD trên APFS.
- Dialog native bấm tay; `rm` dừng giữa chừng thật (chỉ unit với lỗi giả).
- `CCM_E2E_MENU_TOAST_LOG` (`main.ts` `setupMenuToast`, ngoài phạm vi file): bản đóng gói vẫn đọc env, ghi nội dung toast (câu hỏi + đáp án menu) ra file tuỳ ý; không bỏ qua confirm. Đề xuất gate `!app.isPackaged` ở WP sau. `CCM_E2E_RES_CONFIRM` chỉ có tác dụng khi `CCM_RES_FIXTURE` (kill giả) nên không nguy hiểm.

## Commit message đề xuất

fix(desktop): canonical delete guard with realpath, Windows name folding, wider protected list, post-confirm recheck and packaged-safe auto confirm
