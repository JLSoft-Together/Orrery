Trả lời tiếng Việt. KHÔNG sửa code. Chỉ ghi đúng 1 file: `.planning/wp/v102-ACCEPTANCE.md`.

Nhiệm vụ: viết checklist nghiệm thu cho từng gói việc v102 (UI next gen Sương kính của app Orrery).
Đọc: `.planning/SPEC-v102-mist-ui.md` (ID S/R/T/D/E/K/G/M), `.planning/PLAN-v102-mist-redesign.md` (bảng WP, map SPEC → WP),
`.planning/ui-inventory/00-SUMMARY.md`, `01-workspace.md` (mục Must keep), mockup `.planning/mockups/v102/src/*.dc.html` (số đo px, màu, chữ).

Mỗi gói (WP0+1, WP2, WP3, WP4a, WP4b, WP5, WP6, WP7, WP8, WP9, WP10+12, WP11) một mục:
- Các mục kiểm dạng `- [ ] <SPEC ID> <điều kiện kiểm được, có số đo px / ms / màu / phím cụ thể lấy từ mockup hoặc spec>`.
- Mục "Không được phá" (bất biến classic, kéo thả, phím có sẵn) riêng cho gói đó.
- Mục "Cách kiểm" ngắn: kiểm bằng đọc code (file / symbol nào), Vitest, hay ảnh chụp app.
Không dùng em dash và dấu chấm giữa làm ngăn cách. Ngắn, mỗi dòng một điều kiện. Tối đa ~250 dòng.
