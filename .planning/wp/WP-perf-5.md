## WP-perf-5: Electron main — lazy import module ít dùng lúc boot
- Mục tiêu: giảm RAM main (~92 MB private) và thời gian từ `whenReady` tới window, PLAN-v86 Gói 2.3 + 4 (main).
- Hiện trạng: `apps/desktop/src/main.ts` có ~105 import tĩnh; preview (84 IPC), devices (48), run (23), telegram, layout, github, flutterPreview, apk, phone, releaseCheck… đều load + khởi tạo lúc boot (`ready()` ~`main.ts:1369`, `await preview.start()` ~1555, `await runManager.loadSettings()` ~1731).
- Việc:
  1. Lập bảng module trong main: load lúc boot hay chỉ khi IPC đầu tiên / khi user mở tính năng. Module nào chỉ cần khi gọi → `await import()` lười trong handler (đăng ký `ipcMain.handle` vẫn lúc boot, thân handler import lười, cache promise). Không đổi tên channel/IPC contract.
  2. Việc khởi tạo không cần trước khi có window (`preview.start()`, `runManager.loadSettings()`, telegram, adb auto-connect, update check…) → chạy sau khi window đã `ready-to-show` hoặc khi lần đầu cần. GIỮ đúng thứ tự cho những gì window/dashboard cần ngay (agentData, collector host, bridge, single-instance, protocol handler).
  3. Esbuild bundle (`apps/desktop/build.mjs`): kiểm dynamic import có được tách chunk hoặc ít nhất trì hoãn evaluate; nếu bundle 1 file thì `import()` vẫn trì hoãn top-level evaluate — ghi rõ trong report.
  4. Collector spawn sớm hơn: nếu an toàn, gọi `host.start()` trước `createMainWindow` thay vì trong `boot()` (`main.ts:~732`), song song đăng ký IPC.
- Phạm vi file: `apps/desktop/src/main.ts`, các module trong `apps/desktop/src/*` chỉ khi cần tách hàm khởi tạo, `apps/desktop/build.mjs` (chỉ nếu cần cho tách chunk), test desktop tương ứng.
  Cấm đụng: `apps/web`, `apps/collector`, `apps/desktop/src/pty/host.ts`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Bảng module (trước/sau: boot vs lười) trong report.
  - [ ] Không đổi IPC channel; mọi handler cũ vẫn đăng ký trước khi renderer gọi (renderer có thể gọi ngay khi load).
  - [ ] `npm test -w @ccm/desktop` + `npm run typecheck -w @ccm/desktop` pass.
  - [ ] Đo (nếu làm được không cần build/chạy app): thời gian require + evaluate `stage/main.cjs` không có — ghi CHƯA KIỂM nếu cần build.
- KHÔNG chạy `npm run desktop` / build / app (single-instance lock của app chủ nhân đang chạy).
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-perf-5-report.md`

## Nhật ký
- AUDIT main.ts: ready() chạy tuần tự setup* + register*Ipc, createMainWindow ở cuối, host.start trong boot(); import ngoài node/electron gần như không có (sql.js đã lười).
- Sửa main.ts: createMainWindow + loading page sớm, host.start sớm (earlyStart), boot dùng lại; session-end host?.stop().
- Test: npm test -w @ccm/desktop 119 file / 1414 pass; typecheck chỉ lỗi cũ thiếu package `telegram` (gramjs/mtproto/userScan).
