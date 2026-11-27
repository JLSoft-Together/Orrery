## WP-113m — DONE (chưa nhìn app thật)

### Đã đổi
- Rust: `apps/core/crates/core-desk/src/devices/variants.rs` (mới, 293 dòng): `Variants` apply/restore/restore_ready/pending, trait `Shell` (impl cho `Adb`), kênh `ccm:dev-variant {op: apply|restore|sweep, id, night?, fontScale?}` và `ccm:dev-built-apk (cwd, module, variant)` (tìm APK vừa build qua `find_apk`). File khôi phục `<data>/run/variants/<tên mã hoá serial>.json` (ghi atomic, TRƯỚC khi đổi).
- Rust hook: `manager.rs` (field `variants` + `restore_pending(&rows)` ở `snapshot` và watcher on_change), `mod.rs` (2 dòng mod), `install.rs::register` (1 dòng). Test: `variants_tests.rs` (mới).
- Shell: `apps/shell/shim/preload.ts` (+2 hàm), `apps/shell/src/shim.js` sinh lại bằng `node scripts/gen-desk-shim.mjs`.
- Web: `devices/lib/devices.ts` (wrapper `devVariant`, `devBuiltApk`, thêm), `run/lib/multiRun.ts` (+test, logic thuần), `run/lib/multiRunDeps.ts` (+test), `run/lib/multiRunUi.ts`, `run/stores/multiRun.ts` (store riêng `startMany`/`cancelMany`, tách khỏi `run.ts` đã 641 dòng), `run/composables/useShotsHandoff.ts`, `run/components/RunShotsDialog.vue` (+test), `RunBar.vue` (mục menu "Chạy trên nhiều máy..." cho config android, emit `multiRun`), `RunPane.vue` (mount dialog), `run.ts` (2 dòng: `sweepVariants()` khi `ensure(true)`), `run/i18n/run-v113.ts` (en+vi, namespace `runMulti`).

### Tiêu chí
- [x] multiRun.test: chia lượt max 3, build 1 lần, huỷ giữa chừng, lỗi 1 máy không chặn máy khác, restore khi lỗi/throw/huỷ.
- [x] cargo variants: apply ghi file trước khi đổi, restore trả đúng giá trị cũ (font chưa đặt thì `settings delete`, night custom thì auto), restore lúc khởi động chỉ máy ready và không động máy đang dùng trong process, giá trị lạ bị từ chối.
- [x] Dialog: lưới, gửi agent (chỉ attach, không submit, báo "chưa gửi"), đóng dialog lúc đang chạy -> `cancelMany` (store restore trong finally từng máy).
- [x] typecheck sạch; i18n en+vi; không comment; file < 400 dòng.

### Audit
Diff chỉ trong phạm vi; không đụng remote/mobile/shared/en.ts/vi.ts/.code_index/docs.

### Review
- Edge: máy rớt giữa chừng, restore lỗi -> giữ file, báo `restoreFailed` trên dialog, tự restore lần máy ready sau (snapshot/watcher/sweep). Process crash -> process mới không có "active" nên restore ngay khi máy ready. Máy "đang dùng" trong process hết hạn sau 10 phút (ACTIVE_MAX) để renderer reload không kẹt mãi.
- apply lỗi giữa chừng tự rollback ngay. Apply lần 2 giữ giá trị gốc lần 1.
- Build lỗi: không dùng APK cũ; chỉ khi máy đầu lỗi ở bước install (không phải apk-missing) mới dùng APK vừa build cho máy khác.
- Máy đầu chạy bằng session Run thường (có confirm reinstall khi signature); máy khác chỉ `adb install -r`, lỗi signature báo rõ, không tự gỡ app.
- Áp biến thể máy đầu sau khi app đã chạy (activity tạo lại), máy khác trước khi mở app.

### Test
- `cargo test --locked -j 4 -p core-desk variants` -> 12 passed (core-desk lib có 22 warning cũ, không rà từng cái).
- `npm test -w @ccm/web -- src/features/run` -> 21 files, 174 passed; `src/features/devices` -> 74 passed.
- `npm run typecheck` -> sạch.
- CHƯA KIỂM: chạy thật trên máy (adb, 2-3 máy), E2E, Mac.

### Nhìn
CHƯA NHÌN: không chạy app. Mô tả: menu thiết bị của Run bar (config Android) có mục "Chạy trên nhiều máy..." mở dialog: chip chọn máy (max 3), nhóm Sáng/Tối/Chữ 1.3x, nút Chạy và chụp, danh sách tiến độ (icon + chữ + màu), lưới ảnh theo tỉ lệ thật với nhãn máy + biến thể, nút gửi/lưu từng ảnh và "Gửi ảnh cho agent".

### Hot file cần thêm
- `apps/web/src/i18n/en.ts`: `import runV113 from '../features/run/i18n/run-v113'` và `...runV113.en,` (vi.ts tương tự `...runV113.vi,`). Chưa thêm thì dialog hiện key thô.
- `.code_index`: module run (stores/multiRun.ts, lib/multiRun*.ts, RunShotsDialog.vue), devices (variants.rs, kênh `ccm:dev-variant`, `ccm:dev-built-apk`); 1 dòng `_meta.md`.
- docs: kênh mới; file khôi phục `<data>/run/variants/`; QA Mac checklist (2 máy, 3 biến thể, rút cáp giữa chừng rồi cắm lại).

### Ngoài phạm vi
- `ccm:dev-built-apk` và store `multiRun.ts` là thêm ngoài danh sách file ban đầu (cần để lấy APK và tránh phình `run.ts`).
- Chỉ config Android; Flutter chưa hỗ trợ nhiều máy.
- "Lưu ảnh" dùng `<a download>` (như recap CSV), chưa kiểm trong webview Tauri.

### Commit message đề xuất
feat(run): run on several Android devices with variants and a screenshot grid
