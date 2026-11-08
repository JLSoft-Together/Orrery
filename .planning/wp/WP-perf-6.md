## WP-perf-6: Office (Habblaud) tắt mặc định, bật trong Settings
- Quyết định chủ nhân 2026-11-09: Office hầu như không dùng → mặc định TẮT, ai cần thì bật trong Settings.
- Mục tiêu: khi tắt, không tốn CPU/RAM/disk cho Office: collector không tạo `OfficeHub` / không chạy recorder timelapse (không ghi `timeline/<ngày>.jsonl`), web không load OfficeView/Habblaud, không có nút chuyển Map ↔ Office. Bật lên thì hoạt động như hiện tại (timelapse bắt đầu ghi từ lúc bật), tắt lại thì dừng hub + recorder ngay, không cần restart app.
- Hiện trạng: `apps/collector/src/index.ts:163,181,225,320,356-367` luôn tạo + start `OfficeHub` sau `monitor.start()`; recorder timeline tạo ở index.ts (tìm `recorder`); web `stores/ui.ts:29-42` `MapView = 'map' | 'office'` (persist), toggle ở `components/AgentMap.vue:~1266`, `OfficeView` async ở `AgentMap.vue:~1287`; store `stores/office.ts`.
- Cách làm gợi ý (tự chọn cách gọn nhất, khớp kiến trúc có sẵn):
  - Setting ở web (`stores/settings.ts` hoặc store settings hiện có, persist như các setting khác), mặc định `false`. UI: một switch trong Settings (mục Appearance/Monitor hoặc nơi hợp lý nhất), có animation như switch khác, text qua vue-i18n `en` + `vi`.
  - Báo collector: message client → collector (thêm vào protocol `packages/shared` nếu cần, vd `office.enable { on }`), web gửi khi connect và khi đổi setting. Collector bật/tắt hub + recorder theo message; nhiều client thì bật nếu bất kỳ client nào bật. Không có client nào bật → tắt. Hoặc persist cờ ở `CCM_DATA_DIR` nếu gọn hơn — miễn là mặc định tắt và boot không tạo hub khi tắt.
  - `ui.mapView === 'office'` khi setting tắt → hiển thị `map` (không mất giá trị đã lưu, bật lại thì về office nếu trước đó chọn office là tuỳ, chọn cách đơn giản).
  - `timeline.get` khi tắt: trả rỗng/hiện có, không lỗi.
- Phạm vi file: `apps/collector/src/index.ts` (vùng office), `apps/collector/src/office/*`, `packages/shared` (protocol office), `apps/web/src/stores/ui.ts`, `stores/office.ts`, store settings, component Settings liên quan, `components/AgentMap.vue`, locale `en`/`vi`, test tương ứng.
  Cấm đụng: `TermPane.vue`, `WorkspaceView.vue`, `stores/workspace.ts`, `stores/attachments.ts`, `usePaneAttachments.ts`, `devServer.ts` (WP-perf-2 đang sửa); `apps/collector/src/gitstat.ts` (WP-perf-1 đang sửa).
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Mặc định (storage trống): collector boot không tạo OfficeHub, không ghi file timeline; test collector chứng minh.
  - [ ] Bật → hub start, subscribe chạy như cũ; tắt → hub stop, recorder dừng; test.
  - [ ] Web: setting mặc định false; tắt → không render OfficeView, không có nút Office ở toolbar map; test store/component nếu có pattern test sẵn.
  - [ ] i18n `en` + `vi` đủ key; không hardcode text.
  - [ ] `npm test -w @ccm/collector`, `npm test -w @ccm/shared` (nếu đổi), `npm run typecheck` các workspace đã đổi pass. Test web chạy theo file đã đổi (full web test có thể fail ở `stores/workspace*` do WP-perf-2 đang dở — ghi lại, không sửa).
- Nhìn thấy được: có → không được chạy app, ghi CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-perf-6-report.md`

## Nhật ký
- audit index.ts/hub/web wiring xong; runtime.ts + index.ts + server.ts + shared types; web settings/connection/AgentMap/Settings UI/i18n; test collector 417 pass, web 105+31 pass, typecheck chỉ lỗi TermPane (WP-perf-4)
