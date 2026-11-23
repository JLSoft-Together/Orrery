## WP-112i — DONE
- Đã đổi:
  - `apps/core/crates/core-desk/src/run/history.rs` (mới): `RunHistory` ghi nối dần `runs/<sha256(root)[..8]>/<id>.log` (JSON mỗi dòng, BufWriter, không giữ log trong RAM) + `index.json`; che secret bằng `mask_secrets` từng dòng; giữ 20 lần chưa ghim / project, tổng max 50 MB (xoá cũ nhất chưa ghim), max 25 MB / lần (đánh dấu `truncated`), lần ghim không bị xoá; `read` đọc theo trang (max 2000 dòng); lần chạy dở dang sau khi app tắt được đánh dấu `interrupted`.
  - `run/mod.rs` (+`pub mod history`), `run/manager.rs` (`Inner.history`, dùng chung `mask`), `run/sessions.rs` (`begin` ở `launch`, `append` ở `Hooks::log`, `finish` khi session kết thúc, exit code suy từ `failure`), `run/ipc.rs::on_history` (kênh `ccm:run-history-list/read/pin/delete`, qua gate root).
  - `packages/shared/src/run.ts` (+`RunHistoryEntry`, `RunHistoryPage`), `apps/web/src/core/lib/desktop.ts` (+4 method), `apps/shell/shim/preload.ts` (+4 method), `apps/shell/src/shim.js` (sinh lại bằng `gen-desk-shim.mjs`, kéo theo cả thay đổi preload của WP khác).
  - Web: `features/run/lib/runHistory.ts` (+test), `components/RunHistoryDialog.vue` (+test), `RunHistoryLog.vue` (log ảo hoá), nút "Lịch sử" ở `RunPane.vue`, `i18n/run-v112.ts` + đăng ký ở `src/i18n/en.ts`, `vi.ts`.
  - `apps/shell/e2e/run-pane.cjs`: thêm đoạn E2E History (list, ghim, so sánh, mở nguyên văn, Escape).
  - `.code_index/` (`core-desk-run.md`, `web-run.md`, `lookup.md`, `_meta.md`).
- Tiêu chí: 6/6 đạt.
  1. Rust test lưu / xoay vòng / ghim / max 50 MB / che secret: `history.rs` 8 test (thêm giới hạn 25 MB / lần, xoá, interrupted, exit code).
  2. Lib so sánh + test chuẩn hoá (timestamp, pid, đường dẫn temp, hex dài, thời lượng): `runHistory.test.ts` 8 test, multiset diff, cắt 500 dòng nhưng giữ tổng.
  3. Log 100k dòng: test Rust `page_big_log_without_loading_all` đọc trang cuối của 100k dòng; UI chỉ render cửa sổ cuộn (ảo hoá, 18px / dòng, tải theo trang 1000).
  4. Vitest UI list: `RunHistoryDialog.test.ts` 4 test (rỗng, list + chip exit, ghim / xoá, so sánh).
  5. E2E: mở rộng `run-pane` (3 run thật với fake tool).
  6. typecheck + cargo test pass, ảnh có.
- Audit: log Run trước đây chỉ ở RAM (5000 dòng, `run.ts`) và main không giữ lịch sử; hook điểm duy nhất là `SessionHooks::log` (đã mask) và `Inner::update` (khi kết thúc).
- Review: exit code suy ra (không có field exit trong session): stopped không lỗi = 0, `failure.code=exit` lấy detail, lỗi khác = 1; session bị kill lúc đóng app thành `interrupted` ở lần list sau. Ghim không tính vào mốc 20 (chỉ bị giới hạn bởi 50 MB tổng, và lần ghim không bị xoá kể cả khi vượt). Đọc trang quét từ đầu file (offset lớn = quét tuyến tính, 100k dòng vẫn nhanh). So sánh đọc tối đa 200k dòng mỗi bên qua IPC theo trang. Chưa phủ: so sánh khi lần mới còn đang chạy (nút bị loại vì chỉ chọn lần đã xong); id lạ bị `safe_id` chặn.
- Test:
  - `cargo test --locked -j 4 -p core-desk run::` → 87 passed.
  - `npm test -w @ccm/web -- src/features/run src/i18n src/importRules` → 22 files, 156 passed.
  - `npm run typecheck` → pass.
  - `node apps/shell/e2e/run-pane.cjs` (CCM_WEB_DIST = build web ra thư mục tạm, cổng 4461 / 9461, CCM_E2E_OUT tạm) → tất cả PASS, gồm 7 check History mới.
  - `node scripts/index-check.mjs` → ok 39 file, 0 lỗi.
- Nhìn: `docs/qa/v111/WP-112i/run-06-history-list.png`, `run-07-history-compare.png`, `run-08-history-raw.png` (list có chip exit, mốc ghim; so sánh exit 0 sang 1; log nguyên văn).
- Hot file cần thêm: không.
- Ngoài phạm vi cần main xử lý:
  - Snapshot i18n `keys.en.txt` / `keys.vi.txt`: chỉ thêm 37 key `runHistory.*` bằng tay, không chạy `-u` (các key `editor.send.*` của WP khác đã có sẵn).
  - E2E không ghi đè ảnh docs/qa cũ đã track (OUT đặt ở thư mục tạm).
  - `apps/shell/src/shim.js` sinh lại, có thể lệch với WP khác đang sửa `preload.ts`; main nên sinh lại 1 lần cuối trước khi commit.
  - Lần build shell đầu bị lỗi "Access is denied" vì `orrery-shell.exe` debug của WP khác đang chạy E2E; chờ nó thoát rồi build lại.
- Commit message đề xuất: feat(run): keep a log history per run with pin, paged reading and run comparison
- Dòng changelog (en / vi, ≤ 100 ký tự): Run history: reopen old logs, pin a baseline, compare two runs / Lịch sử Run: mở lại log cũ, ghim mốc, so sánh 2 lần chạy
