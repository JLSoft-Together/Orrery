## WP-v87-2-B (PLAN-v87 bước 2, core-pty)
- Spec: `.planning/PLAN-v87-step2-core-pty.md` — đọc toàn bộ; WP B ở §5 (bảng WP, file scope độc quyền, phụ thuộc) và §5 "Test từng WP" (lệnh + tiêu chí xong). Protocol §2, chia trách nhiệm §3, thiết kế Rust §4.
- Quyết định main chốt: crate ở `apps/core/`; `wt.*`, validate cwd, theo dõi branch ở TS main qua control channel; cờ `CCM_CORE_PTY` mặc định `node`. Câu O1 (fallback khi Rust lỗi) để tới WP-F, chưa làm.
- Model: sonnet.
- Báo cáo: `.planning/wp/WP-v87-2-B-report.md`

## Nhật ký
- build.mjs: buildCore + chmod/copy staging, electron-builder win/mac extraResources core/, .gitignore, desktop.yml/test.yml rust steps, apps/core scaffold (cargo test CHUA CHAY: may khong co MSVC link.exe)
