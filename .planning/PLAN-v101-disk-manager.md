# PLAN v101: Ổ đĩa - quét, phân tích, dọn, sắp xếp (Windows + macOS)

Nhánh `feat_clean_memory`. Grill 15/11/2026 (Q1-Q31). Click chuột menu là đợt 0, ở `PLAN-v100-click-menu.md`.

## Định hướng

- Đối tượng: **dùng chung** (Q2), rác dev chỉ là một nhóm.
- Vị trí: mục **Ổ đĩa** trong Settings gồm tóm tắt + nút mở **view lớn toàn màn hình**, ẩn trong menu `⋯` như Devices, không chiếm tab chính (Q4, Q9).
- Yêu cầu của chủ nhân: tìm kiếm, filter, xoá **nhanh nhất có thể** (index, thuật toán search) nhưng **không được làm đơ máy**.

## Phạm vi

| Nhóm | Nội dung |
|---|---|
| Bản đồ dung lượng | Treemap squarified + sunburst, **chuyển qua lại được** (Q22); vẽ canvas, cây thư mục sắp theo size có thanh %, click để đi vào thư mục, màu theo loại file |
| Tìm kiếm | Kiểu Everything: tìm tên file cả ổ gần như tức thì, filter theo size / loại / ngày (Q13) |
| File lớn / cũ | Top file lớn nhất, file không mở quá N tháng, lọc theo loại (Q5) |
| So sánh lần quét | Snapshot size theo thư mục; quét nền 1 lần/ngày lúc máy rảnh + cắm sạc, giữ 30 snapshot, chỉnh được tần suất hoặc tắt (Q5, Q28) |
| Cảnh báo ổ sắp đầy | Gộp tính năng v77 có sẵn (Q10) |
| Thư mục rỗng / shortcut hỏng | Tìm rồi dọn (Q5) |
| Rác dev | node_modules, build/, .gradle, DerivedData, target/, cache npm/pub/gradle/cargo; gộp `Dọn đồ build` v77 (Q10) |
| Rác hệ thống | **Chỉ danh sách vị trí biết trước** theo OS (Temp, cache trình duyệt, crash dump, log cũ, Windows Update cache cần admin...). Không bao giờ đụng Windows/System32, /System, Documents. Mỗi nhóm ghi rõ xoá xong ảnh hưởng gì (Q15) |
| SDK / emulator cũ | Android SDK platform/build-tools cũ, AVD, iOS Simulator, Xcode archives, version Flutter/Gradle cũ (Q6) |
| Docker | Image, container đã dừng, volume không còn dùng, build cache qua `docker` CLI (Q6) |
| App + size | Windows chạy `UninstallString` sau confirm; macOS chuyển .app vào Trash; liệt kê file sót (AppData, ~/Library) để tick rồi xoá (Q6, Q18) |
| RAM | Process / nhóm app tốn RAM nhất (bảng process có sẵn của orrery-core), đánh dấu process của agent/dev, kill qua dialog native; không can thiệp bộ nhớ hệ thống (Q6, Q12) |
| File trùng | Mặc định file ≥ 1 MB trong thư mục user (bỏ node_modules, .git, thư mục hệ thống/app). Nhóm theo size, hash 64KB đầu/cuối, rồi hash đủ BLAKE3. Tick sẵn bản thừa (giữ bản cũ nhất / đường dẫn ngắn nhất), người dùng sửa rồi xoá vào Thùng rác (Q8, Q30) |
| Sắp xếp | Hai chế độ **theo loại** (Ảnh / Video / Tài liệu / Nén / Cài đặt / Khác) và **theo tháng**, chọn như filter. Xem trước rồi bấm + công tắc tự động. Thêm được thư mục và rule (đuôi, tên, size). Mặc định Downloads. Bỏ qua `.crdownload` / `.part` và file mới dưới 2 phút. Log mọi lần di chuyển để Undo (Q7, Q17) |
| Tự xếp | Chạy khi app đang chạy (kể cả ở tray); mở app lại thì xử lý luôn file vào trong lúc app tắt; không cài tiến trình chạy cùng lúc khởi động máy (Q29) |

`Dữ liệu agent` (v76) **giữ nguyên chỗ cũ** (Q10).

## Xoá (Q3)

- Mặc định chuyển vào Recycle Bin / Trash, dialog native hiện tổng size + số mục.
- Có tuỳ chọn **xoá hẳn** (cho node_modules lớn) và phải confirm lần 2.
- File đang bị khoá / không đủ quyền: bỏ qua và liệt kê trong báo cáo.

## Bảo vệ rác dev (Q26)

- Project có session agent / pane / Run đang mở → **không cho xoá**, hiện lý do.
- Chỉ tick sẵn thư mục của project không sửa trong 30 ngày (đổi được).
- Hiện lệnh khôi phục (`npm install`, `flutter pub get`...).

## Engine quét + index

- Tiến trình riêng trong `orrery-core` (Rust), không chạy trong main / renderer.
- Mặc định (Q11): walk song song, Windows dùng `FindFirstFileEx` + `FIND_FIRST_EX_LARGE_FETCH`, macOS dùng `getattrlistbulk`; không cần admin.
- Nút **Quét siêu nhanh** (Windows NTFS, Q24): mỗi lần bấm thì bật UAC chạy `orrery-core` bản admin, đọc MFT, gửi kết quả qua pipe rồi thoát. Không cài service, app chính vẫn chạy quyền thường.
- Index (Q13, Q19): lưu tên + size + mtime mỗi ổ, đã nén, trong `CCM_DATA_DIR`. Chưa bấm quét lần nào thì không tốn gì. Người dùng chọn ổ / thư mục, có danh sách loại trừ. macOS thì hướng dẫn cấp Full Disk Access.
- Cập nhật tăng dần khi chạy quyền thường: watcher `ReadDirectoryChangesW` (Windows) / FSEvents (macOS) trên các root đã index, cộng với walk lại định kỳ. Đọc USN journal cũng cần admin nên chỉ dùng trong lần quét siêu nhanh.
- Size hiển thị là size trên đĩa (allocated); hardlink / APFS clone chỉ tính 1 lần. Mặc định bỏ qua ổ mạng.
- Tìm kiếm chạy trên index trong RAM của core; UI nhận kết quả theo batch, list ảo.

## Không làm đơ máy (Q14)

- IO / CPU priority thấp, có Pause / Cancel; quét nền tự dừng khi chạy pin.
- Settings có mức **Nhẹ / Cân bằng / Tối đa** (số luồng, priority). Mặc định Cân bằng = max nửa số nhân.

## MCP `orrery` (Q25)

- `disk_search` (tìm theo tên / filter trong index), `disk_usage` (size thư mục).
- `disk_propose_delete`: agent gửi danh sách đề xuất xoá, người dùng duyệt qua dialog native. Agent **không tự xoá / di chuyển**.

## Telegram (Q31)

Cảnh báo ổ đầy / quét xong gửi số GB **kèm tên top 5 thư mục lớn nhất**. Không gửi tên file. Index chỉ nằm trên máy.

## File trên cloud (bổ sung sau review)

File OneDrive / iCloud / Dropbox chỉ là placeholder (Windows attribute `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` / `RECALL_ON_OPEN`, macOS `SF_DATALESS`): **không bao giờ đọc nội dung** (hash file trùng, đọc metadata sâu) vì OS sẽ tải file về. Quét chỉ đọc attribute; hiện nhãn "trên cloud", size trên đĩa = 0; file trùng bỏ qua.

## Chức năng thêm sau review (15/11)

| Chức năng | Nội dung |
|---|---|
| Chuyển sang ổ khác + link | Chuyển thư mục lớn (.gradle, pub cache, AVD, Docker data...) sang ổ khác, để lại junction (Windows) / symlink (macOS); kiểm ổ đích đủ chỗ, copy xong verify rồi mới xoá bản gốc; có nút chuyển về lại; không chuyển khi tool liên quan đang chạy |
| Lịch sử dọn | Log mỗi lần dọn / xếp / chuyển: xoá gì, bao nhiêu GB, vào Thùng rác hay xoá hẳn; tổng đã giải phóng; Undo sắp xếp mở từ đây |
| Dọn 1 chạm | Tự tick các nhóm an toàn (Temp, cache, rác dev cũ hơn 30 ngày của project không mở), xem tổng rồi bấm một lần; vẫn qua dialog native |
| Size project trong Workspace | Explorer / rail hiện size thư mục từ index; chuột phải "Mở trong Ổ đĩa"; card project trên Monitor hiện size + rác dọn được |

## Phase (Q21 + review, chế độ manager)

| Phase | Nội dung | Kiểm |
|---|---|---|
| 0a-0e | Click chuột menu (`PLAN-v100-click-menu.md`) | |
| 1a | Rust engine: walk song song + index + ops `disk.*`, nhận diện file cloud-only, chưa có UI | Benchmark: 1 triệu file; tìm trên index < 50ms |
| 1b | View lớn + treemap / sunburst + **luồng xoá** (Thùng rác / xoá hẳn, dialog native) + lịch sử dọn + mục Settings | Quét, xem, xoá được |
| 1c | Tìm kiếm + filter + mức Nhẹ / Cân bằng / Tối đa + Pause / Cancel + watcher cập nhật index | |
| 1d | Quét siêu nhanh MFT (UAC, helper tự thoát) | Riêng vì rủi ro nhất |
| 2a | Rác dev (gộp Dọn đồ build) + bảo vệ project + size project trong Workspace | |
| 2b | Rác hệ thống + file lớn / cũ + Dọn 1 chạm | |
| 3a | File trùng (bỏ qua cloud-only) | |
| 3b | Sắp xếp (xem trước + tự động + Undo) + thư mục rỗng / shortcut hỏng | |
| 4a | SDK / emulator + Docker + chuyển sang ổ khác + link | |
| 4b | App + size + gỡ cài đặt + RAM | |
| 4c | Snapshot + quét nền + gộp cảnh báo ổ đầy + MCP + Telegram | |

Chạy song song được: 0b-0c cùng 1a; 2a cùng 2b. Mỗi phase dùng được ngay. Mỗi đợt cập nhật `.code_index/`, docs §mới, i18n en/vi, theme token, E2E.

## Code có sẵn để dùng lại

- `apps/desktop/src/buildClean/` + `apps/core/crates/core-desk/src/build_clean/` + `BuildCleanDialog.vue`: Dọn đồ build v77.
- `apps/desktop/src/disk/` + `DiskBadge.vue`, `DiskSettings.vue`: cảnh báo ổ đầy v77.
- `apps/desktop/src/preview/explorer.ts` `trashEntry` (`shell.trashItem`); `main.ts` `confirmBox`.
- IPC: `preload.ts` `ccmDesktop`, mẫu `devices/ipc.ts`; route sang Rust bằng `desk.route` (`desk/forward.ts`).
- Đăng ký view kiểu Devices: `stores/ui.ts` `View`, `App.vue`, `AppHeader.vue` (menu `⋯`), `CommandPalette.vue`.
- Rust chưa có `jwalk` / `ignore` / `trash` / `blake3`: thêm crate ở đợt 1–3.

## Điều phối với luồng song song

Xem `.planning/COORD-2026-11-15-v100-v102.md` (thứ tự merge, namespace i18n, hot file).
