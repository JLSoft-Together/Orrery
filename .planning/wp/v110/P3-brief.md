# Brief chung P3: dời apps/web theo feature

Repo `F:\work\web\Orrery` (Windows). Báo cáo tiếng Việt.

## Đọc trước
- `.planning/AUDIT-v110-web-features.md`: mục 1 (cây), 2 (feature → file), 3 (hub về core), 4 (core import feature), 5 (đợt), 6 (rủi ro).
- Mẫu đã làm: `apps/web/src/features/office/` (đợt P2, có `index.ts`, `.code_index/office.md`), các feature v109 trong `apps/web/src/features/`.
- Công cụ: `node scripts/web-move.mjs [--dry] [--map file.json] "cũ -> mới"` (đường dẫn tính từ `apps/web/src`, git mv + sửa import). Đọc đầu file để biết giới hạn: KHÔNG sửa `import.meta.glob`, test đọc source bằng `readFileSync` / `?raw` theo đường dẫn tương đối ngoài import, `scripts/qa-fast.mjs` E2E_MAP, CSS `@import`/`url()`, `src=` trong template. Những chỗ đó sửa tay.

## Cách làm 1 feature
1. Liệt kê file của feature theo AUDIT mục 2 (kiểm lại bằng `rg`, AUDIT có thể lệch vì v109 thêm file).
2. Đích: `apps/web/src/features/<feature>/{components,lib,stores,composables,i18n}` (chỉ tạo thư mục con cần). Đã có thư mục feature (vd `features/shortcuts`, `features/notify`) thì gộp vào, không tạo trùng.
3. `--dry` trước, rồi chạy thật. Sửa tay chỗ codemod bỏ qua.
4. `features/<feature>/index.ts`: export những gì feature khác / app cần (component chính, store, hàm). Không bắt buộc đổi mọi import bên ngoài sang index ngay; ưu tiên đổi import từ feature KHÁC sang `@features/<feature>`.
5. i18n của feature: dời file i18n riêng (vd `i18n/disk-*.ts`) vào `features/<feature>/i18n/`, `i18n/en.ts` + `vi.ts` vẫn import (không lazy, trừ khi feature đã lazy). Snapshot key `apps/web/src/i18n/keys.snapshot.test.ts` KHÔNG được đổi (dời file không đổi key). Đổi là có lỗi.
6. `features/<feature>/README.md` ≤ 15 dòng: đường vào, file chính, bất biến (nếu có).
7. Không đổi hành vi, không sửa logic, không đổi tên symbol.

## CHẾ ĐỘ NHẸ (chủ nhân đang dùng máy, từ 2026-11-22 lô D): đè các mục kiểm bên dưới
- KHÔNG `cargo`, KHÔNG E2E, KHÔNG `npm run build`, KHÔNG chạy cả bộ vitest.
- Sau mỗi feature: `JIN_RAW=1 npm run typecheck -w @ccm/web` (1 lần) + `cd apps/web && JIN_RAW=1 npx vitest run --maxWorkers=2 <thư mục feature vừa dời> src/importRules.test.ts src/i18n/keys.snapshot.test.ts` + `npx vitest related --run --maxWorkers=2 <vài file ngoài feature vừa sửa import>`.
- Mỗi lúc chỉ 1 lệnh nặng. Build, E2E, vitest full để main chạy lúc máy rảnh.

## Kiểm sau MỖI feature (hoặc mỗi cặp feature nhỏ)
- vue-tsc web PASS: `JIN_RAW=1 npm run typecheck -w @ccm/web` (hook bắt `JIN_RAW=1`).
- `JIN_RAW=1 npm test -w @ccm/web` (max 8 worker) PASS, gồm `importRules.test.ts` và `keys.snapshot.test.ts`.
FAIL thì sửa trước khi sang feature kế. Cuối lô: build web (`npm run build -w @ccm/web`) PASS.
E2E (P1 đã xong, E2E ở `apps/shell/e2e/`, chạy qua `run.cjs` / `qa-fast.mjs`; build trước: web, `node scripts/gen-desk-shim.mjs`, `cd apps/shell && cargo build -j 8`): cuối lô chạy các E2E trong `E2E_MAP` khớp file đã dời (xem `node scripts/qa-fast.mjs --dry`). Chi tiết cũ: nếu feature có file trong `E2E_MAP` của `scripts/qa-fast.mjs` thì sửa map theo đường dẫn mới; chạy E2E chỉ khi gói P1 đã xong (thư mục `apps/desktop` không còn và `apps/shell/e2e/run.cjs` tồn tại); khi chạy dùng port trống, không 9333, không đụng app cài sẵn port 4317.

## Luật
- KHÔNG viết comment trong code (mọi loại, kể cả code mới, README là markdown thì được). Comment cũ: để nguyên.
- KHÔNG `git commit` / `push` / `stash` / `checkout` / `reset`. `git mv` qua web-move được.
- Tìm kiếm bằng `rg` / Grep, không `grep -r`. Sửa file bằng Edit/Write.
- Chỉ sửa `apps/web/**`, `scripts/qa-fast.mjs` (E2E_MAP), `.code_index/features.md` (đường dẫn), và đường dẫn file web mà E2E `apps/shell/e2e/*.cjs` đọc bằng `fs` (chỉ đổi chuỗi đường dẫn, không đổi logic test). KHÔNG sửa root `package.json`, `apps/shell`, `apps/core`.
- Gói khác có thể đang chạy song song ở ngoài `apps/web` (P1 xoá Electron, có thể `npm install`): vitest lỗi do node_modules đang đổi thì chờ 1-2 phút chạy lại.
- Không chữ "trần" (dùng "max"), "watchdog"; không "—" / "·" trong chữ UI.

## Báo cáo cuối (≤ 30 dòng)
Mỗi feature: số file dời, đích, số import sửa, chỗ sửa tay. Kết quả kiểm (PASS/FAIL + số test). Phụ thuộc chéo còn lại đáng chú ý (feature A import sâu feature B). Việc còn treo.
