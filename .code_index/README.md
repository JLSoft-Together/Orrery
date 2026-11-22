# .code_index

Index kiến trúc gọn cho agent. Cách dùng: đọc `lookup.md` (việc -> `path::Symbol`), rồi 1-2 file module (mỗi file tối đa 8 KB). Không đọc `archive/` khi làm task (lịch sử, đường dẫn có thể đã chết).

- Module: `rust-core.md` + `core-*.md` (Rust `apps/core`), `shell*.md` (Tauri `apps/shell`), `web-*.md` (`apps/web`), `pwa.md`, `relay.md`, `protocol.md`, `office.md`. App điện thoại: `apps/mobile/.code_index/mobile.md`.
- Tính năng: `features.md` (1 dòng mỗi tính năng). Thay đổi gần đây: `_meta.md`.
- Mẫu 1 file module: tiêu đề, "Đường vào", bảng "File chính", "Bất biến" (chỉ điều không được phá), "Liên kết" (plan, E2E, file index kế). Không kể lịch sử v-số (chỉ link plan), mỗi dòng có `path` hoặc `path::Symbol`.
- Sau mỗi task: sửa file module bị ảnh hưởng + thêm 1 dòng `_meta.md`; "tại sao" ghi ở đây hoặc `.planning/`, không ghi vào source. Chạy `node scripts/index-check.mjs` (0 đường chết, file module tối đa 8 KB).
