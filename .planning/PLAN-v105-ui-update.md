# PLAN v105: update UI theo nghiên cứu `/ui/` (U1 đến U5)

Nguồn: `.planning/site/public/data/ui.json` (live https://orrery.jinvu.vn/ui/). Chủ nhân chốt 2026-11-20: "triển khai những cái đã nghiên cứu UI, sau đó nghiên cứu UI tiếp".

| WP | Việc | Model | File chính |
|---|---|---|---|
| U1 | Dòng session trên rail: tên pane / agent + số, chip trạng thái chữ, tool đang chạy, thời gian gắn trạng thái, hover focus / đóng | sonnet | `components/next/rail/RailRowItem.vue` |
| U2 | Thanh quota: icon + % (chủ nhân 2026-11-20: ít chữ, không hiện reset), cửa sổ ≥ 70% nổi bật, bấm mở popover đủ cửa sổ, reset tương đối + tuyệt đối, mức bằng chữ trong popover | sonnet | `components/next/NextQuotaBar.vue`, `lib/next/quotaBar.ts` |
| U3 | Chân rail 3 nút có chữ khi rail rộng, còn lại vào menu Thêm; tab Explorer có chữ khi panel ≥ 360px (container query); root cắt giữa; toast tối đa 3 dòng | sonnet | `components/next/NextShell.vue`, `components/workspace/ExplorerPanel.vue` |
| U4 | PWA Workspace: lưới 1 cột dưới 600px, preview 3 dòng, thẻ Chờ bạn có nút đáp án, gửi sau Hoàn tác 3s | opus | `apps/pwa/src/components/PaneGrid.vue`, `PaneCard.vue` |
| U5 | PWA terminal: chế độ Đọc (ngắt dòng, 13px, chỉ xem), thẻ câu hỏi thu 1 dòng khi cuộn, dải phím có mép mờ | sonnet | `apps/pwa/src/components/TerminalSurface.vue` |

Luật chung: không comment trong code, i18n en + vi, màu qua token, không cuộn ngang, ngoại lệ 6 / 7 (mọi phím sau 1 click của chủ nhân, pane chỉ submit khi idle). Subagent chỉ chạy vitest + typecheck; main build web + chạy E2E sau khi gộp.

Sau khi nghiệm thu: đợt nghiên cứu UI kế tiếp, cập nhật `/ui/`.
