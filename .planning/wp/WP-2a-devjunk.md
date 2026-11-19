## WP-2a-devjunk: tab Dọn dẹp + nhóm Rác dev (gộp Dọn đồ build v77) + bảo vệ project

- Mục tiêu: view Ổ đĩa có tab mới **Dọn dẹp** (cạnh Duyệt / Tìm kiếm / File lớn / File cũ). Tab liệt kê **nhóm** dọn, mỗi nhóm có danh sách mục (path, project chủ, size, tuổi = ngày project sửa lần cuối, lệnh khôi phục), tick sẵn theo rule, tổng size đã chọn, xoá qua `diskMapDelete` có sẵn (Thùng rác / Xoá hẳn, dialog native do main hiện). WP này dựng **khung nhóm dùng chung** + các nhóm **Rác dev**; nhóm Rác hệ thống + Dọn 1 chạm là WP-2b sau, cắm vào cùng khung.
- Khung (desktop): `apps/desktop/src/diskClean/` registry các provider `{ id, group: 'dev' | 'system', scan(ctx) → CleanItem[] }`, IPC `ccm:diskclean-categories` (danh sách nhóm + mô tả i18n key), `ccm:diskclean-scan { ids, roots }` (stream progress theo nhóm, trả item), cancel. Type ở `packages/shared/src/diskClean.ts` (`CleanItem { path, size, project?, lastTouched?, restore?: string, group, category, locked?: string }`). Preload method `diskClean*`.
- Nhóm Rác dev (provider):
  1. `node_modules` (cũng `.pnpm-store` trong project), 2. build output của project (dùng lại logic `apps/desktop/src/buildClean/scan.ts` / Rust `core-desk` `build_clean` đang có: Gradle `build/`, `.gradle/` project, Flutter `build/` + `.dart_tool/`, `target/` cargo, `dist/`, `.next/`, `out/`... theo rule đang có trong buildClean, chỉ tính khi thư mục cha có file đánh dấu project: `package.json`, `build.gradle(.kts)`, `pubspec.yaml`, `Cargo.toml`...), 3. Xcode `DerivedData` + `Archives` cũ (macOS), 4. cache toàn cục: npm cache, yarn cache, pnpm store, pub cache, Gradle `caches/` (lưu ý Gradle home có thể là link sang ổ khác, ví dụ máy này `~/.gradle` → `K:\dev\.gradle`: tính size thật, xoá trong thư mục thật, không xoá link), cargo `registry/cache`, pip cache, CocoaPods cache. Cache toàn cục **không tick sẵn**, mỗi mục ghi rõ hệ quả (lần build sau tải lại).
  - Nguồn tìm: index `core-disk` nếu đã quét (dùng op search theo tên, rất nhanh), không có thì walk dưới các root project (project Orrery đang biết + thư mục người dùng thêm), dừng khi gặp thư mục junk (không đi vào node_modules).
  - Tuổi: max mtime của file đánh dấu project + `.git/HEAD` / `.git/index`; tick sẵn khi > 30 ngày (setting đổi được `ccm.diskCleanDays`).
  - Lệnh khôi phục: `npm install` / `pnpm install` / `yarn` (theo lockfile), `flutter pub get`, `./gradlew build`, `cargo build`...
- Bảo vệ (Q26): project có session agent / pane Workspace / Run đang mở → mục bị khoá (`locked` = lý do, không tick được, không gửi xoá). Web biết pane / session (stores workspace, terminals, run): tính danh sách root đang mở, truyền vào scan và kiểm lại ngay trước khi gửi xoá.
- Gộp v77: chỗ mở "Dọn đồ build của project" cũ (tìm `BuildCleanDialog` + nơi mở nó) chuyển sang mở view Ổ đĩa tab Dọn dẹp lọc sẵn project đó. Giữ code cũ chạy được nếu cần cho test cũ; ghi rõ trong báo cáo cái gì còn dùng, cái gì có thể xoá sau.
- UI: `apps/web/src/components/disk/clean/**` (DiskClean, CleanGroup, CleanRow...), store `apps/web/src/stores/diskClean.ts` (+test), lib `apps/web/src/lib/diskClean.ts` (+test), i18n file riêng `apps/web/src/i18n/disk-clean-v101.ts` (main sẽ thêm import vào en/vi; ghi vào "Hot file cần thêm"). Gắn tab vào `DiskView.vue` (1 chỗ). Chỉ token + utility có sẵn (COORD).
- Cấm đụng: `DiskScanBar.vue`, `lib/diskMap.ts`, `stores/diskMap.ts`, `i18n/disk-v101.ts`, `apps/desktop/src/diskMap/**`, `apps/core/**`, `preload.ts` phần `diskMap*` (WP-1d-mft đang làm); `components/workspace/**`, `stores/menuQueue.ts`, `lib/shortcuts*`, `lib/workspaceKeys*`, `i18n/click-menu-v100.ts` (WP-0d đang làm); `style.css`, `themes.css`, `i18n/en.ts` / `vi.ts`. `preload.ts` + `main.ts`: chỉ thêm khối `diskClean*` riêng, không sửa dòng của WP khác (WP-1d cũng thêm 1 method vào preload: nếu Edit báo file đổi thì đọc lại rồi thêm).
- Model: opus (chọn thư mục để xoá trên dữ liệu người dùng).

## Tiêu chí nghiệm thu

- [ ] Test desktop: mỗi provider trên cây thư mục tạm (nhận đúng project, không nhận `build/` không có file đánh dấu, không đi vào node_modules lồng, cache qua link tính đúng thư mục thật), tuổi project, lệnh khôi phục theo lockfile.
- [ ] Test web: khoá project đang mở (không tick, không gửi xoá, kiểm lại trước khi gửi), tick sẵn > 30 ngày, cache toàn cục không tick.
- [ ] `npm test -w @ccm/desktop -- diskClean` + `npm test -w @ccm/web -- --maxWorkers=6` pass; typecheck desktop + shared + web sạch.
- [ ] E2E `apps/desktop/e2e/v101-diskclean.cjs` trên **bản dev tách biệt** (`apps/desktop/e2e/cdp.cjs`, env `CCM_E2E_DISKMAP_CONFIRM=1`, cổng riêng, không kill Orrery/Electron đang chạy, không dùng 4317 / 5173): dựng 3 project giả trong thư mục tạm (1 cũ 60 ngày, 1 mới, 1 đang mở pane), tab Dọn dẹp hiện đúng, project đang mở bị khoá, xoá mục đã tick vào Thùng rác, lịch sử có dòng. Không quét / xoá gì ngoài thư mục tạm; cache toàn cục thật chỉ được **đọc size**, không xoá. Dọn Thùng rác sau E2E. Chụp `docs/qa/v101/WP-2a/`.
- [ ] Không comment trong code. Text vi không "trần", em dash, "·".

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/WP-2a-devjunk-report.md`.

## Nhật ký
- 15/11 subagent WP-2a: đọc brief + code buildClean/diskMap; who_is_editing không chạy (Orrery window not ready), các hot file chưa bị sửa trong git status. Lệch: diskMapDelete chỉ nhận path trong root đã index nên diskClean có kênh xoá riêng dùng lại runDelete + dialog + history của diskMap.
- 15/11: xong desktop diskClean (projects/caches/providers/service/ipc) + main/preload/shared; vitest diskClean 15/15 pass.
- 15/11: web lib/store/components/i18n + DiskView tab + App.vue chuyển ui.buildClean sang tab Dọn dẹp; unit web disk 33/33.
- 15/11: E2E v101-diskclean 22/22 (bản dev, 4473/9473); cache toàn cục đổi sang đo khi bấm (Gradle caches máy này 20 GB / 396k file ~8 phút). Cập nhật .code_index desktop/web.
- 15/11: DONE, báo cáo .planning/wp/WP-2a-devjunk-report.md.
