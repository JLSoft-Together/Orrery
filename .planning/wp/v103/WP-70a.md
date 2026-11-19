## WP-70a: UI next — rail / panel trái: nút Explorer dễ phân biệt, bỏ hộp "việc cần", session lùi vào trong project, collapse / expand panel trái, thanh hạn mức xuống đáy

- Plan + luật chung: `.planning/PLAN-v103-next-polish.md` (đọc mục "Luật chung", bắt buộc). Góp ý #1, #3, #6, #7, #10.
- Đọc trước: `CLAUDE.md` (đoạn v102 + rule màu / glass), `.planning/SPEC-v102-mist-ui.md`, `.code_index/web.md`, `apps/web/src/components/next/**` (NextShell, rail, explorer toggle), `apps/web/src/lib/next/**`, chỗ hiện hạn mức (usage) trong NextShell / footer.
- Model: sonnet. Song song với WP-72 (git viewer) và WP-73 (Disk) — không đụng file của họ. WP-70b làm sau, cũng đụng NextShell: giữ thay đổi gọn, tách component mới thay vì phình NextShell.

### Việc
1. #1 Nút mở Explorer trên rail / header hiện trông giống nút ở thanh dưới header session → đổi icon / vị trí / nhãn để không nhầm (tooltip + aria rõ "Explorer (Ctrl+Shift+E)"); ghi lý do chọn.
2. #3 Bỏ hộp "việc cần" ở góc dưới trái (rail needs list hoặc khối tương tự — xác định đúng phần tử bằng ảnh chụp UI next, đối chiếu i18n). Thông tin "đang chờ trả lời" vẫn còn ở chip "Chờ trả lời" / badge session.
3. #7 Session thuộc 1 project hiển thị lùi vào dưới dòng project (indent + đường dẫn hướng nhẹ), không ngang hàng; rail chế độ icon giữ gọn.
4. #10 Panel trái (rail) có nút collapse / expand (thu về dải icon 56 px hoặc ẩn hẳn, mở lại bằng nút + phím tắt), nhớ trạng thái (`localStorage` key `ccm.next.*` theo quy ước), có motion; vẫn kéo chỉnh size như cũ.
5. #6 Thanh hạn mức chuyển xuống đáy cửa sổ (footer next), 1 dòng gọn dễ đọc: mỗi agent có dữ liệu thật một cụm "tên + % + reset lúc"; Claude Code + Codex nếu có nguồn số thật (tra `stores/usage*`, collector usage index); không có số thì không hiện cụm đó. Bỏ thanh cũ ở vị trí trước.
6. Test vitest + E2E next (ca: nút Explorer, rail collapse / expand + nhớ, indent session, footer hạn mức), ảnh sáng / tối `docs/qa/v103/WP-70a/`.

### Báo cáo
`.planning/wp/v103/WP-70a-report.md`.
