## WP-113h1 — DONE (Hot file cần thêm để chạy được; xem dưới)

### Đã đổi
- core-desk: `run/types.rs` (`ApkMeta`, `SessionPatch.apk`), `run/sessions.rs` (ghi `apk {path,size,applicationId,variant}` vào session), `run/android/runner.rs` (patch lúc `installing`, sau `find_apk`), `runner_tests.rs` (assert patch, chỉ chạy trên unix).
- shell `remote/`: MỚI `offer_apk.rs` (+`offer_apk_tests.rs`), `run_log.rs` (+`run_log_tests.rs`). Sửa `push.rs` (kind `apk`, text vi/en, tag, nạp module `offer`, phone đang kết nối vẫn nhận), `push_extra.rs` (`Extra.apk`, apk vào quota info, mang `w`), `viewer_map.rs` (op `run.log`, `run.watch`, `run.unwatch`, `SPECS` 34 -> 37, `Watch::RunAdd/RunDrop`), `viewer.rs`, `viewer_events.rs` (nạp `run_log`, ghi ring buffer ở `Hub::push_run`, `drop_run`).
- Ngoài danh sách nhưng cần (mỗi chỗ 2-3 dòng): `push_seal.rs::sealed_plain` và `push_web.rs::web_payload` thêm `apk`; `push_content.rs::notice` cho `apk` dùng text cố định.
- web: MỚI `features/remote/lib/remoteOffer.ts` (+test), export trong `features/remote/index.ts`; `features/run/lib/runApk.ts`, `lib/apkOffer.ts`, `components/RunApkOffer.vue` (+tests), sửa `RunPane.vue` (nút + công tắc), `stores/run.ts` (hook auto 3 dòng), `i18n/run-v113.ts` (khoá `runApk` en+vi).

### Tiêu chí
- [x] cargo: offer-apk từ chối không apk / ngoài build / không phải apk / file đã xoá; ticket ApkOffer sống cạnh ApkPage; push `apk` không có path; ring buffer max 2000, `since`, mask; viewer op map.
- [x] core-desk: patch session (test ApkMeta chạy trên Windows; test runner chỉ unix, CHƯA CHẠY trên Windows).
- [x] Vitest: nút gửi/công tắc tự gửi (đúng 1 lần, chỉ khi bật), toast lỗi. Chưa có test mount `RunApkOffer.vue`, test logic ở lib.
- [x] Không comment, i18n en + vi. `run-v113.ts`, `viewer_map.rs` < 400 dòng; `runner.rs` đã 440 từ trước.

### Audit
- Web không gửi path: kênh nhận `sessionId`, desk lấy `apk.path` từ run session, canonicalize cả path lẫn `cwd`, bắt buộc nằm trong `cwd` và có đoạn `build/outputs`, rồi `is_apk` (đuôi, file, size 1..1GiB), SHA-256 tính trên file thật.
- Payload `apk {url, urls<=3, size, sha256, app, variant, exp}`: chỉ trong seal (FCM) hoặc Web Push (E2E), chỉ phone bật content và (seal hoặc web). Plain FCM không bao giờ có url/sha. URL chỉ IP private v4 của LAN.
- `apk` dùng quota info (20/giờ/phone), không bị tạm dừng (pauseUntil) vì không thuộc nhóm group.

### Review (edge case)
- Session không apk: `noApk`. File bị xoá sau build: `gone` (và ticket sau đó trả 404 vì `resolve` kiểm lại `is_apk`). Symlink ra ngoài: canonicalize nên `outside` (test chỉ unix). `..` trong path: `outside`.
- Phone không bật push / không content / không seal: không vào danh sách; không ai đủ điều kiện -> `none`. Ngoài LAN / không có IP private: `lan`. Quá quota: `limit`.
- Hạn chế: phone FCM chưa seal sẽ không nhận được (cố ý, tránh gửi ticket URL dạng plaintext qua Google). Ring buffer chỉ ghi khi Remote đang chạy (ctx tồn tại); không có phone nào cần cũng vẫn ghi.
- `apk.path` tuyệt đối nằm trong run session mà web renderer đã nhận qua `ccm:run-event` (theo spec); UI chỉ dùng sự có mặt của `apk`.
- `SPECS` thêm 3 op nên snapshot/tài liệu op list có thể cần cập nhật.

### Test
- `cd apps/core; cargo test --locked -j 4 -p core-desk --lib run` -> 103 passed.
- `cd apps/shell; cargo test --locked -j 4 remote` -> 341 passed, 0 failed (lần đầu có 2 lỗi `phone_ws::clip` của WP-113j, lần sau tự hết).
- `npm run typecheck` -> sạch.
- `npm test -w @ccm/web -- src/features/run/lib src/features/remote/lib` -> xanh (apkOffer 4, runApk, remoteOffer). Chạy rộng hơn từng thấy lỗi `useRemotePhone.test.ts > rename and move` (ws.move, ngoài phạm vi, không do gói này) và timeout do máy tải; `src/i18n/keys.snapshot` lệch vì khoá mới (main cập nhật, em không chạy `-u`).

### Nhìn
CHƯA NHÌN (nút ở Run pane, không chạy app). Nút "Gửi lên điện thoại" chỉ hiện khi có phone push và session có apk; công tắc "Tự gửi khi build xong" luôn hiện khi có phone push.

### Hot file cần thêm
- `apps/shell/src/remote/mod.rs`: thêm `"ccm:remote-offer-apk",` vào `CHANNELS` và đổi `[&str; 26]` thành `[&str; 27]` (nếu 113j đã đổi thì cộng thêm 1); thêm dòng `"ccm:remote-offer-apk" => Ok(self.offer_apk(arg)),` vào `call_with`.
- `apps/shell/shim/preload.ts` (cạnh dòng 383): `remoteOfferApk: (sessionId: string): Promise<unknown> => typeof sessionId === 'string' && sessionId.length > 0 && sessionId.length <= 64 ? ipcRenderer.invoke('ccm:remote-offer-apk', sessionId) : Promise.resolve({ ok: false, error: 'invalid' }),` (rồi `node scripts/gen-desk-shim.mjs` sinh lại `shim.js`).
- `apps/web/src/core/lib/desktop.ts` (cạnh dòng 191): `remoteOfferApk?: (sessionId: string) => Promise<unknown>`.
- i18n en/vi: khoá `runApk` đã nằm trong `features/run/i18n/run-v113.ts` (đã nạp sẵn); chỉ cần cập nhật snapshot `src/i18n/__snapshots__`.
- `.code_index`: `offer_apk.rs` (gửi APK qua push), `run_log.rs` (ring buffer run log + op `run.log/run.watch/run.unwatch`), kind push `apk`; thêm kind `apk` vào `.planning/wp/WP-113g-contract.md`: `k="apk"`, data ngoài seal `m,k,w`; trong seal/Web Push `apk{url,urls,size,sha256,app,variant,exp}`; quota info, không bị pause, gửi cả khi phone đang kết nối, FCM notification `tag "<m>:apk"` (CHƯA ghi vào file contract do đang chờ main).
- `phone_ws.rs`/hook forward `ccm:run-event`: đã có sẵn đường `desk.rs -> remote::host::run_event -> Hub::push_run`, ring buffer ghi ở đó, không cần thêm.

### Ngoài phạm vi
Không đụng pair*/lan/rebind/loopback*/connectApi/connect/*, phone_ws/clip, mod.rs/ctx.rs/preload/desktop/i18n en,vi, mobile. Phía phone (tải, kiểm sha, cài, trang Run log) thuộc WP-113h2.

### Commit message đề xuất
feat(remote): offer a finished APK build to paired phones and serve Run logs to the phone

Record the built APK on the run session, add the apk push kind with a LAN download ticket, a per-project auto-send switch and send button in the Run pane, and a 2000-line run log ring buffer behind new run.log, run.watch and run.unwatch viewer ops.
