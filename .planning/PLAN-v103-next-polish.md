# PLAN v103 — UI next gen: 12 góp ý của chủ nhân + UI next mặc định (2026-11-19)

Nhánh: `feat/v98-pwa` (đã merge master 4.0.7). Chủ nhân: "bản mới mặc định UI next, 1–2 bản nữa bỏ UI cũ"; "cứ làm và test liên tục". UI classic chỉ cần không vỡ.

## Góp ý → gói
| # | Góp ý chủ nhân | Gói |
|---|---|---|
| 1 | Nút Explorer giống nút dưới header session, gây nhầm | WP-70a |
| 2 | Menu chức năng góc phải không click outside để tắt | WP-70b |
| 3 | Bỏ hộp "việc cần" góc dưới trái | WP-70a |
| 4 | Vào chế độ gọn thì không về chế độ đầy đủ được | WP-70b |
| 5 | Quét ổ đĩa: chọn item không đổi trạng thái; tích mỗi ổ C vẫn quét cả D | WP-73 |
| 6 | Thanh hạn mức xuống hẳn bottom, đơn giản dễ nhìn (hiện chỉ Claude Code) | WP-70a |
| 7 | Session trong 1 project phải lùi vào (đang ngang hàng) | WP-70a |
| 8 | Xem diff nhanh (git log) khó dùng | WP-72 |
| 9 | Thay đổi của commit: bấm ảnh (webp, png) báo nhị phân, không cho xem | WP-72 |
| 10 | Panel trái chưa có collapse / expand (chỉ kéo size) | WP-70a |
| 11 | Explorer: chưa kéo chỉnh width bằng chuột, chưa cuộn ngang với tree sâu (tên bị …); thêm chế độ ghim cố định bên phải kéo width được (chủ nhân thêm 2026-11-19) | WP-71 |
| 12 | Bỏ Mission Control | WP-70b |
| — | UI next mặc định (cài mới + migration 1 lần cho máy đang dùng classic, vẫn chọn lại được classic) | WP-70b |

## Quyết định main (chủ nhân bảo làm liên tục, không hỏi)
- #6: thanh hạn mức đặt ở đáy cửa sổ (footer next), một dòng gọn: mỗi agent có dữ liệu thật (Claude, Codex nếu có số đáng tin) một cụm "tên + % + thời điểm reset"; không có số thật thì không hiện (rule: không bịa số).
- #12: bỏ hẳn Mission Control khỏi UI next (lệnh, phím tắt, Ctrl+K, card, nút); câu trả lời menu v100 từ Mission Control chuyển sang chỗ còn lại (chip "Chờ trả lời", toast) — không mất tính năng trả lời.
- UI next mặc định: migration 1 lần `ccm.uiGeneration` → `next` (khoá migration riêng như `4.0.2-galaxy`), `index.html` boot khớp; classic vẫn bật lại được trong Appearance.
- #8: gói opus tự soát cách dùng (so với IntelliJ / VS Code / Fork), sửa những điểm khó dùng rõ ràng, ghi lại phần cần chủ nhân chốt.

## Thứ tự (≤ 3 subagent, máy hay tải nặng)
- Lượt 1: WP-70a (rail / panel trái / thanh hạn mức) ‖ WP-72 (git diff + ảnh) ‖ WP-73 (Disk).
- Lượt 2: WP-70b (menu, chế độ gọn, bỏ Mission Control, mặc định next) ‖ WP-71 (Explorer) ‖ WP-68a (mockup web app).
- Lượt 3: WP-67 (remote trên UI next, sau khi rail ổn định).
- Mỗi gói về: main chạy lại test liên quan, xem ảnh, commit + push.

## Luật chung cho mọi brief
- Không comment trong code (mọi loại, kể cả TODO). Chuỗi qua vue-i18n en + vi (không em dash, không chấm giữa). Màu qua token; glass / blur chỉ dưới `:root[data-ui='next']` theo CLAUDE.md. Motion cho mọi thay đổi (thêm / bớt / đổi trạng thái).
- Không `git commit` / `push`. Không build release. Không tắt app thật của chủ nhân; E2E dùng dữ liệu riêng.
- `cargo -j 4`; hook resource-guard chặn khi tải đến từ app khác → thêm `JIN_FORCE=1` đầu lệnh. Test chập chờn khi tải nặng (`usage.test.ts`, `disk_organize`) → chạy riêng xác nhận.
- File mới ≤ ~400 dòng. Tìm bằng tool Grep hoặc `rtk proxy rg`. Tắt tiến trình nền khi xong.
- Test: vitest file liên quan + `npm run typecheck -w <ws>`; E2E desktop chế độ next (`apps/desktop/e2e`, `launch({ ui: 'next' })`, `v102-next.cjs`) cho ca UI; ảnh sáng / tối vào `docs/qa/v103/<WP>/`.
- Báo cáo `.planning/wp/v103/<WP>-report.md`: file + symbol, test (lệnh + số), ảnh, bước QA tay Windows + mục riêng "Test macOS" (main chép vào `.planning/QA-mac-checklist.md`), việc dở.
