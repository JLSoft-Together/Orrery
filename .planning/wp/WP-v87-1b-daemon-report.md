# WP-v87-1b-daemon — báo cáo

Trạng thái: DONE (trừ mục CHƯA KIỂM bên dưới)

## Đã đổi
- `apps/desktop/src/layout/download.ts`: `KitSpec`, `isAllowedKitUrl` (github.com + CDN), `kitDirFor`, `followRedirects` (mỗi hop kiểm host, max 5), `HttpBody.location`, `downloadArtifact` nhận allow + giữ DownloadError; `LayoutlibBundle` có dep `kit`: `kitPending/hosts/downloadBytes/isReady` tính cả kit, `fetchKit` (tmp -> extract -> kiểm jar + kit.json -> marker -> rename nguyên tử, xoá bản khác version + partial), `fetchLayoutlib` tách riêng (người đã có layoutlib chỉ tải kit). Lỗi kit mang tiền tố `daemon-` (vd `daemon-sha256-mismatch`, `daemon-http-404`).
- `apps/desktop/src/layout/kit.ts` (mới): `embeddedKit()` đọc `__DAEMON_KIT__` (esbuild define), URL `https://github.com/JLSoft-Together/Orrery/releases/download/v<ver>/layout-daemon-<ver>.zip`.
- `src/layout/ipc.ts`: dep `kit`, `jarExists` = jar có hoặc kit còn chờ tải (để UI hiện nút tải thay vì "daemon chưa build"), consent host = `bundle.hosts`.
- `src/layout/wire.ts`: truyền `kit`, chữ consent en/vi nhắc render daemon.
- `src/main.ts` (import + khối wireLayout): packaged + có kit -> jar / loose-kit trỏ `<DATA_DIR>/layoutlib/daemon/<ver>/`; dev (không packaged) giữ `apps/layout-daemon/build`, không tải.
- `apps/desktop/daemon-kit.mjs` (mới): zip writer (zlib, không dependency mới, deterministic), sha256.
- `apps/desktop/build.mjs`: daemon block lên trước; `--dist` đóng `apps/layout-daemon/build/dist/layout-daemon-<ver>.zip`, nhúng `{version,size,sha256}` qua esbuild `define`; `--daemon-zip-only` (CI job daemon, esbuild import động nên không cần npm ci); dùng lại zip có sẵn khi `CI=true` hoặc `CCM_REUSE_DAEMON_ZIP=1` (local mặc định build lại, tránh zip cũ).
- `electron-builder.yml`: bỏ 2 extraResources jar + loose-kit (giữ gradle init script, afterPack, filter pdb).
- `.github/workflows/desktop.yml`: job `daemon` (ubuntu, JDK 21) -> artifact `layout-daemon`; build Win/mac `needs: daemon`, tải artifact vào `apps/layout-daemon/build/dist`, bỏ setup-java. `release.yml`: thêm `dist/*.zip`.
- `apps/layout-daemon/README.md` (mục Distribution), `.code_index/desktop.md`, test mới `apps/desktop/test/layout-kit.test.ts`.

## Tiêu chí 5/6 (1 CHƯA KIỂM)
- [x] extraResources hết jar/kit; zip + sha256 tạo đúng (test buildZip -> extractZip, sha ổn định; chạy `CI=true node build.mjs --daemon-zip-only` với zip giả -> in size/sha, exit 0, đã xoá zip giả).
- [x] Tải, kiểm checksum, giải nén nguyên tử, không tải lại, lỗi mạng / checksum / http / zip thiếu file / zip-slip / thư mục không ghi được (test).
- [x] Dev không tải (kit = null khi không packaged; test "without a kit spec").
- [x] CI một zip dùng chung, release đăng zip.
- [x] Test + typecheck pass.
- [ ] Đo Setup size + chạy XML Preview thật: CHƯA KIỂM (không dist, release chưa có asset).

## Audit / Review / Test
- Edge: mất mạng giữa chừng -> `daemon-network`, tmp bị xoá; sai sha -> `daemon-sha256-mismatch`; 2 lần bật song song -> dùng chung 1 promise (1 fetch); version đổi -> thư mục version cũ + partial bị xoá sau khi bản mới xong; thư mục thiếu `.ccm-ready` coi như chưa có; redirect sang host lạ bị chặn; dev không đụng.
- Giới hạn: chưa có khoá giữa 2 process app (đã có single-instance lock). Người cập nhật từ bản cũ đã có layoutlib sẽ chỉ tải kit (~vài chục MB).
- `npm test -w @ccm/desktop -- --maxWorkers=6` -> 120 files / 1429 tests pass. `npm run typecheck -w @ccm/desktop` -> sạch.
- Nhìn: CHƯA NHÌN (không chạy app).

## Main cần làm
1. Release phải có asset `layout-daemon-<version>.zip` (chạy workflow desktop rồi release.yml, hoặc `gh release upload`). Zip local do `npm run dist:win` tạo ở `apps/layout-daemon/build/dist/` — nếu đăng zip local thì phải là đúng zip đã nhúng sha (CI Win + mac dùng chung 1 zip).
2. `npm run dist:win`, đo Setup size (mục tiêu <= 115 MB), rồi test XML Preview packaged: tải lần đầu có tiến độ (dev-only override: bản local chưa có release thì sẽ ra `daemon-http-404`, đúng thiết kế).
3. Lưu ý: CRLF working copy (autocrlf) — file đã sửa giữ LF khi ghi.

## Commit message đề xuất
📦 Download the layout daemon jar + loose kit on first XML Preview use instead of bundling them (sha256-pinned release asset, CI builds one zip)

## Vòng 2
- 1: `apps/layout-daemon/build.gradle.kts` archive reproducible; `daemon-kit.mjs` zip timestamp cố định + sort tên; local `--dist` (không CI): `fetchReleaseZip` thử tải `layout-daemon-<ver>.zip` từ release, có thì dùng + pin sha của nó, không có thì build local + cảnh báo "upload đúng file này" (`chooseKitSource` thuần, có test). Giữ nguyên 2 sửa của main trong build.mjs.
- 2: desktop.yml job daemon fail khi tag != `v<apps/desktop version>`; release.yml kiểm zip khớp `layout-daemon-${TAG#v}.zip`. 3: release.yml fail nếu không đúng 1 zip.
- 4: `renameWithRetry` (EPERM/EBUSY/EACCES, backoff 100·2^i, 5 lần) cho kit + layoutlib. 5: `kitsToPrune` chỉ xoá bản cũ hơn, giữ 1 bản khác; không xoá bản mới hơn. 6: chỉ xoá `<kit.version>.partial-*` hoặc partial quá 10 phút. 7: main.ts log cảnh báo (console + layout.log) khi packaged mà không có kit. 8: consent host "github.com (GitHub release assets)". 9: `downloadArtifact` thành overload (Artifact: url tuỳ chọn; Pinned: url + allow bắt buộc), bỏ cast.
- Test: `npm test -w @ccm/desktop` toàn bộ pass (xem số ở trả lời), typecheck sạch. CHƯA KIỂM: gradle reproducible thật (không chạy gradle), workflow YAML (không chạy CI).
