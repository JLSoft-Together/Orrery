# QA v100 + v101 (2026-11-15)

Bản dev tách biệt (`apps/desktop/e2e/cdp.cjs`: user-data-dir, cổng, data dir, APPDATA tạm), mỗi script một cổng riêng, tối đa 3 song song. `orrery-core` build vào target-dir scratchpad, trỏ `CCM_CORE_BIN`.

## E2E

| Script | Kết quả | Số check |
|---|---|---|
| v100-menu-bar | PASS | 15 |
| v100-menu-click | PASS | 25 (lần 1 fail 2 check mouse mode do chuột thật di qua cửa sổ, chạy lại sạch) |
| v100-menu-queue | PASS | 17 |
| v100-menu-toast | PASS | 14 |
| v101-diskmap-host | PASS | 18 |
| v101-diskmap-ui | PASS | 30 |
| v101-diskmap-fast | PASS | 13 |
| v101-projsize | PASS | 17 |
| v101-diskclean | PASS | 22 |
| v101-diskclean-system | PASS | 26 |
| v101-dupes | PASS | 20 |
| v101-organize | PASS | 27 |
| v101-sdk-move | PASS | 28 |
| v101-apps-ram | PASS | 26 |
| v101-snap | PASS | 36 |
| v101-mcp-disk | PASS | 26 |
| v101-qa (mới) | PASS | 19 |

## Lỗi tìm thấy và cách sửa

- Mode bar 9 tab bị cắt ở 760px: thanh xuống dòng (`flex-wrap`, `max-w-full`), giữ radiogroup và phím mũi tên. Ảnh: `QA/mode-bar-760.png`, `QA/mode-bar-1400.png`.
- E2E chọn tab theo vị trí: `SegmentedControl` thêm `data-value`, các script chọn theo `[data-value]`.
- Ô ngày `mm/dd/yyyy` theo OS: input text theo locale app (vi `dd/mm/yyyy`, en `yyyy-mm-dd`), nhận cả ISO, ngày sai thì trả về giá trị cũ.
- Cleanable cộng cả mục bị khoá: `junkOf` bỏ mục `locked` hoặc nằm trong project đang mở.
- Chuyển sang ổ khác: thêm vào menu chuột phải tab Duyệt (thư mục) và Explorer (thư mục, không phải root); lịch sử có action `move` riêng (source, target, dung lượng), không tính vào "Tổng đã giải phóng".
- `v101-projsize` sửa kỳ vọng: menu có thêm "Move to another drive", tooltip rail không còn "Cleanable" khi project đang mở.

## Có từ trước

- `desk-agent-data.twin.test.ts`: `EPERM symlink` do Windows tài khoản này không tạo được symlink (kiểm bằng `fs.symlinkSync` trong thư mục tạm cũng `EPERM`), không liên quan code.
- `desk-mcp.twin.test.ts` fail với binary cũ, xanh sau khi rebuild `orrery-core`.
