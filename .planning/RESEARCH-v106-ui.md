# RESEARCH v106: UI/UX đợt 2

Ngày 20/11/2026. Đợt 1 (U1 đến U5) đã làm, đánh dấu `status: done` trong `.planning/site/public/data/ui.json`.
Trang: `.planning/site/public/ui/` (chưa deploy). Không trùng v104 (Inbox, Settings 14 trang, cảnh báo đụng file, Changes theo session).

Nguồn ảnh: E2E Tauri hôm nay `%TEMP%/ccm-qa-fast/`, PWA `apps/pwa/e2e/.out/` (WP-68b3, shots-55a), `docs/qa/v104/`.

## Xếp hạng (lợi ích trên công sức)

| # | Id | Nơi | Công sức | Việc |
|---|----|-----|----------|------|
| 1 | U6 | desktop | S | Rail: pane file / diff / Changes không giả làm session agent |
| 2 | U7 | phone | S | Inbox sheet nền đặc, vuốt Đã xem / Hoãn |
| 3 | U8 | desktop | S | Rail 264px tiếng Việt hết chữ bị cắt |
| 4 | U9 | phone | S | Giám sát: project 1 session gộp 1 dòng, thời gian đọc được |
| 5 | U10 | desktop | M | Explorer drawer đẩy pane khi đủ chỗ, không đè diff |

## U6 Rail pane công cụ
- Bằng chứng: `ccm-qa-fast/explorer/explorer-03-changes.png`, `ccm-qa-fast/v104-turn-changes/v104h-03-narrow-light.png`, `apps/web/src/components/next/rail/RailRowItem.vue:96` (chip trạng thái luôn render), `apps/web/src/lib/next/railLive.ts:36`.
- Vấn đề: main.ts, README.md, "Thay đổi (3)" có chip Idle / Rảnh; project đếm file như agent; không thấy file chưa lưu.
- Đề xuất: icon theo loại + thông tin riêng (Chưa lưu, +1 -1, 3 file mới); gom "Công cụ N" thu gọn; chấm "1 chưa lưu" khi thu gọn.

## U7 Inbox sheet điện thoại
- Bằng chứng: `docs/qa/v104/WP-104e/pwa-inbox-light.png`, `apps/pwa/src/components/BottomSheet.vue:28-36`, `apps/pwa/src/styles/base.css:105-107` (glass-pop), `apps/pwa/src/components/InboxSheet.vue:45-51`.
- Vấn đề: chữ list phía sau hiện xuyên qua sheet; không có Đã xem / Hoãn trên điện thoại; "<1p" giống mũi tên.
- Đề xuất: nền đặc + scrim đậm; vuốt trái Đã xem, phải Hoãn 15 phút, Hoàn tác 3s, nút "..." cho VoiceOver; thời gian bằng chữ.
- Rủi ro: vuốt không được đụng back của iOS (bắt đầu cách mép trái từ 24px).

## U8 Chữ bị cắt trên rail 264px
- Bằng chứng: `ccm-qa-fast/v104-inbox/desktop-inbox-light.png` ("Worksp...", "Trạng th...", "Thố... Li... Cà..."), `ccm-qa-fast/rail/rail-03-needs.png` ("Waiting Ne..."), `apps/web/src/components/next/NextShell.vue:259, 292, 340-352`, `RailRowItem.vue:94-100`.
- Đề xuất: chân rail 3 chế độ đo theo container (chữ cạnh icon, chữ dưới icon 11px, chỉ icon + tooltip), không cắt giữa từ; segment có icon; thời gian lên dòng 1 để tool đủ chỗ.
- Ghi chú: U3 đợt 1 thêm nhãn nhưng chưa tính độ dài tiếng Việt.

## U9 Giám sát điện thoại gọn hơn
- Bằng chứng: `apps/pwa/e2e/.out/shots-55a/monitor-a-phone-light-webkit.png`, `apps/pwa/src/views/MonitorView.vue:122-134`, `apps/pwa/src/lib/monitorModel.ts:164-170`, `apps/pwa/src/i18n/vi.ts:86` (`days: '{n}n'`).
- Đề xuất: project 1 session thành 1 dòng; thời gian ghi đủ, quá 7 ngày ghi ngày; Chờ bạn / Lỗi nổi lên đầu với nút Trả lời mở thẻ U4 (không gửi phím).

## U10 Explorer drawer
- Bằng chứng: `ccm-qa-fast/diff/diff-02-viewer.png` (che Inline / Split và "Show 10 unchanged lines"), `apps/web/src/components/next/explorer/ExplorerDrawer.vue:300-302`.
- Đề xuất: vùng pane còn từ 640px thì đẩy pane; không đủ thì phủ với scrim tối + bóng, Esc / bấm ngoài đóng.
- Rủi ro: terminal resize mỗi lần mở drawer, cần debounce.

## Để sau (chưa chọn)
- iPad: cột trái và cột chính cùng hiện 1 list session (`shots-55a/monitor-b-ipad-light-chromium.png`); cột chính nên là chi tiết session đang chọn.
- Chi tiết session điện thoại: tên "docs" lặp 4 lần, 2 nút cùng mở terminal (`WP-68b3/detail-phone-light-webkit.png`).
- Tab strip lặp tên project ("alpha", "alpha", "alpha", `ccm-qa-fast/rail/rail-03-needs.png`).
