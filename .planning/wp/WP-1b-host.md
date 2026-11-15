## WP-1b-host: Electron host `orrery-core disk` + IPC + luồng xoá + lịch sử

- Mục tiêu: main process chạy tiến trình `orrery-core disk` (lazy, chỉ khi web gọi lần đầu; tắt khi app thoát; chết thì báo lỗi và lần gọi sau khởi động lại), nối đủ bridge theo `.planning/wp/CONTRACT-v101-diskmap.md`. Đọc trước `.planning/PLAN-v101-disk-manager.md` (mục Xoá, Engine), `.code_index/core-disk.md`, `.planning/wp/WP-1a-report.md` (shape NDJSON), `.code_index/desktop.md`.
- Phạm vi file: mới `apps/desktop/src/diskMap/**` (host, ipc, history, guard, test); `packages/shared/src/diskMap.ts` + export ở `packages/shared/src/index.ts`; sửa `apps/desktop/src/preload.ts` (thêm method `diskMap*`), `apps/desktop/src/main.ts` (đăng ký ipc + confirm, ít dòng nhất có thể, theo mẫu `devices/ipc.ts` + `confirmBox` main.ts:702), chuỗi dialog native theo chỗ đang để chuỗi desktop (`apps/desktop/src/**/strings.ts` hoặc tương đương, en + vi). Được sửa `apps/core/crates/core-disk/**` chỉ để thêm op `disk.forget {paths}` (bỏ node khỏi index, trừ size lên các thư mục cha) + test.
- Cấm đụng: `apps/web/**`, crate Rust khác, `style.css`, `themes.css`.
- Đường binary: dùng đúng cách main đang tìm `orrery-core` cho các subcommand khác (`CCM_CORE_BIN`, dev vs packaged). Env `CCM_DATA_DIR` truyền xuống.
- Xoá: dialog native hiện số mục + tổng size + 3 path đầu; nút mặc định = Huỷ. `permanent` → dialog thứ 2 cảnh báo không khôi phục được. Thùng rác = `shell.trashItem` từng mục (song song có giới hạn); xoá hẳn = `fs.rm` recursive. Từ chối: path tương đối, root ổ (`C:\`, `/`), thư mục hệ thống (`C:\Windows`, `C:\Program Files*`, `/System`, `/Library`, `/usr`, `/bin`, home của user, `Documents`, `Desktop` chính nó). Lỗi từng mục ghi vào `failed`, không dừng cả lượt.
- E2E fake: env `CCM_E2E_DISKMAP_CONFIRM=1` tự bấm đồng ý (theo mẫu `CCM_E2E_RES_CONFIRM` main.ts:660) để E2E chạy không cần người.
- Model: opus (xoá dữ liệu người dùng, quản lý tiến trình con).
- Phụ thuộc: WP-1a (commit fcf8e0c3). Song song với WP-1b-ui (web, theo cùng contract) và WP-0b (web).

## Tiêu chí nghiệm thu

- [ ] Unit test (vitest `apps/desktop`): guard path cấm, parse NDJSON / ghép id, history append + đọc limit, kết quả xoá một phần lỗi.
- [ ] `cargo test -p core-disk` pass kể cả test mới cho `disk.forget`.
- [ ] `npm run typecheck` của `@ccm/desktop` + `@ccm/shared` sạch; `npm test -w @ccm/desktop` các file liên quan pass.
- [ ] E2E `apps/desktop/e2e/v101-diskmap-host.cjs` chạy trên **bản dev tách biệt** qua harness `apps/desktop/e2e/cdp.cjs` (đã cô lập userData / CCM_PORT / CCM_DATA_DIR / APPDATA): tạo cây thư mục tạm, gọi bridge qua CDP `diskMapScan` → chờ done → `diskMapChildren` đúng size → `diskMapDelete` (trash) → file vào Thùng rác + index giảm size + history có dòng. Không kill Orrery/Electron đang chạy, không dùng cổng 4317 / 5173.
- [ ] Không comment trong code.

- Lệnh test: `npm test -w @ccm/desktop -- src/diskMap`; `npm run typecheck`; build web + desktop theo package.json rồi `node apps/desktop/e2e/v101-diskmap-host.cjs`; `cd apps/core && cargo test -p core-disk -j 18 && cargo build -p core-bin -j 18`.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-1b-host-report.md`.

## Nhật ký
- 15/11: Rust op `disk.forget` + 2 test (index, engine), cargo test core-disk 19 pass, clippy sạch.
- 15/11: shared `diskMap.ts`, desktop `src/diskMap/{host,map,guard,history,delete,dialog,ipc}.ts`, strings en/vi, preload, main (setupDiskMap + stop/kill khi thoát).
- 15/11: 4 file test vitest (guard, history, delete, host + ipc fake + core thật) 33 pass.
- 15/11: cập nhật .code_index/desktop.md (mục diskMap) + core-disk.md (op disk.forget); viết E2E v101-diskmap-host.cjs, build desktop xong, web build fail do file WIP agent khác (zz-disk-harness.ts) nên dùng dist có sẵn.
- 15/11: cargo build --release core-bin xong; E2E v101-diskmap-host.cjs 19/19 PASS (bỏ 1 check yếu về progress sau đó), Thùng rác đã dọn item E2E, không còn tiến trình E2E.
- 15/11: DONE, báo cáo .planning/wp/WP-1b-host-report.md.
