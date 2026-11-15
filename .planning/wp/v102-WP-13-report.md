WP-13: ĐẠT
File: apps/web/src/lib/next/invite.ts, invite.test.ts (mới); apps/web/src/components/next/NextInvite.vue (mới); apps/web/src/App.vue (import + mount cạnh WhatsNewDialog); apps/web/src/i18n/mist-v102.ts (mist.invite en+vi); .planning/RELEASE-4.0.6.md (2 dòng đề xuất Có gì mới).
Typecheck: pass  Test: 13 pass / 0 fail (invite.test.ts); i18n messages.test.ts pass.
Hành vi: thẻ chỉ ở classic + desktop, chờ 1.5s, không chen setup / whatsNew / tour (open, due, shown trong phiên) / compact; Thử ngay gọi uiGen.setGeneration('next'); Để sau / ✕ / Esc ghi ccm.next.invite; không hiện nếu ccm.uiGeneration đã có giá trị.
Rủi ro: thumbnail SVG dùng class fill-line-strong / fill-accent (cần tailwind sinh được từ token màu, chưa xem bằng mắt); Esc là listener window, không lấy focus.
