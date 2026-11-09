## WP-v87-1b-daemon: Layout daemon tải về lần đầu bật XML Layout Preview
- Bối cảnh: PLAN-v87 bước 1b (`.planning/PLAN-v87-rust-core.md` §Bước 1b), cổng Setup ≤ 115 MB (v3.4.0: 146 MB). Bản cài đang mang `layout-daemon.jar` (31 MB) + `loose-kit` (14 MB) qua `extraResources` (`apps/desktop/electron-builder.yml`). Đây là phần lớn nhất có thể bỏ; đa số người dùng không mở XML Layout Preview.
- Hạ tầng sẵn có: `apps/desktop/src/layout/download.ts` đã tải layoutlib từ Google Maven (checksum, tiến độ `downloading pct`, thư mục tạm, `BundleStatus`), `zip.ts` giải nén, `daemon.ts` / `cds.ts` dùng `JAR_NAME`. Dev chạy jar / kit thẳng từ `apps/layout-daemon/build/` (`main.ts`, `build.mjs`).
- Mục tiêu: bản đóng gói không còn jar + loose kit. Lần đầu bật XML Layout Preview, app tải `layout-daemon-<version>.zip` (jar + loose-kit) từ GitHub release cùng version (`https://github.com/JLSoft-Together/Orrery/releases/download/v<version>/layout-daemon-<version>.zip`), kiểm sha256 đã nhúng lúc build, giải nén vào `CCM_DATA_DIR`-tương đương phía desktop (đọc cách daemon / layoutlib hiện lưu cache, dùng cùng gốc), rồi chạy như cũ. Dev (`npm run desktop`) vẫn dùng bản build local, không tải.
- Việc:
  1. `apps/desktop/build.mjs --dist`: sau khi gradle build xong, đóng gói `apps/layout-daemon/build/dist/layout-daemon-<version>.zip` (jar + loose-kit; dùng thư viện zip đã có trong repo hoặc tool có sẵn, không thêm dependency mới nếu tránh được — nếu bắt buộc, ghi lý do), tính sha256 + size, nhúng vào `main.cjs` qua esbuild `define` (hoặc file json đóng vào asar). Nếu đã có zip sẵn (do CI tải artifact về) thì dùng zip đó, không build lại.
  2. `electron-builder.yml`: bỏ 2 dòng `extraResources` jar + loose-kit (giữ `ccm-project-model.gradle`). Không đụng các dòng khác (main vừa thêm `afterPack` + filter `.pdb`).
  3. Desktop main (`layout/download.ts`, `daemon.ts`, `ipc.ts`, `main.ts` phần chọn đường dẫn jar / kit): thêm bước tải daemon kit vào cùng luồng tải layoutlib hiện có (cùng tiến độ, cùng trạng thái lỗi / thử lại, cùng UI hiện tại — không thêm UI mới nếu luồng sẵn có hiển thị được), kiểm sha256, giải nén nguyên tử (thư mục tạm → rename), bỏ bản cũ khác version. Mất mạng / sai checksum → trạng thái lỗi rõ ràng như layoutlib, không crash, thử lại được.
  4. CI `.github/workflows/desktop.yml`: job `daemon` (ubuntu, JDK 21) build zip một lần, upload artifact `layout-daemon`; job build Win / mac `needs: daemon`, tải artifact vào `apps/layout-daemon/build/dist/` trước `npm run dist:*` để cả 2 OS nhúng cùng sha256; zip cũng vào artifact để `release.yml` đăng. `release.yml`: thêm `dist/*.zip` vào `gh release create`.
  5. Test (vitest desktop): luồng tải daemon kit (fetcher giả): đúng checksum → giải nén + status ready; sai checksum → lỗi, không để file dở; đã có bản đúng version → không tải lại; bản cũ version khác bị xoá. Test build helper tính sha256 nếu tách được.
  6. Ghi vào `docs/` hoặc README layout-daemon: release phải có asset zip, nếu không XML Preview không tải được (1–3 dòng).
- Phạm vi file: `apps/desktop/build.mjs`, `apps/desktop/electron-builder.yml` (chỉ mục 2), `apps/desktop/src/layout/*`, `apps/desktop/src/main.ts` (chỉ chỗ đường dẫn daemon), test `apps/desktop/test/*` liên quan, `.github/workflows/desktop.yml`, `.github/workflows/release.yml`, `apps/layout-daemon/README.md`. Cấm đụng: `apps/web`, `apps/collector`, `packages/`, `apps/desktop/src/pty/*`, `apps/desktop/after-pack.cjs`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Bản `--dist` không còn jar / loose-kit trong extraResources; zip + sha256 tạo đúng.
  - [ ] Tải lần đầu, kiểm checksum, giải nén nguyên tử, không tải lại khi đã có, lỗi mạng / checksum xử lý được (test).
  - [ ] Dev không tải (dùng `apps/layout-daemon/build`).
  - [ ] CI: một zip dùng chung cho Win + mac, release đăng zip.
  - [ ] `npm test -w @ccm/desktop -- --maxWorkers=6` + `npm run typecheck -w @ccm/desktop` pass.
  - [ ] Đo Setup size + chạy thật XML Preview tải về: CHƯA KIỂM trong WP (main chạy `dist:win` + kiểm sau; release thật mới có asset).
- Nhìn thấy được: có (tiến độ tải lần đầu trong XML Preview) → CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-1b-daemon-report.md`

## Nhật ký
- 1 AUDIT: download.ts / ipc.ts / wire.ts / main.ts / build.mjs / yml đọc xong, thiết kế kit vào LayoutlibBundle.
- 2 CODE: download.ts (KitSpec, followRedirects, isAllowedKitUrl, fetchKit), kit.ts, ipc/wire/main, daemon-kit.mjs, build.mjs, electron-builder.yml, workflows, README, code_index.
- 3 TEST: layout-kit.test.ts 15 test; full desktop 1429 pass; typecheck sạch.
