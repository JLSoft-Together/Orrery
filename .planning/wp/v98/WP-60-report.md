# WP-60 báo cáo

## Đơn vị
- Nguồn: `docs/design/v97-mobile-ui/src/screens.js` (màn mon-a, thẻ session): "2p 14g", "6p", "41p", "1g". Vi: `p` phút, `g` giờ, `n` ngày (mockup không có ngày, chọn `n`); giây `s`. En: `m` / `h` / `d` / `s`. "<1p" / "<1m" qua i18n.
- PWA: `apps/pwa/src/lib/monitorModel.ts` `formatElapsed(ms, t)`, khoá `duration.*` trong `i18n/{vi,en}.ts`, gọi từ `SessionCard.vue`, `SessionRow.vue`; test cập nhật cả hai ngôn ngữ.
- Android: khoá `duration*` trong `app_{en,vi}.arb` (sửa tay; `tool/gen_arb.mjs` đã cũ so với ARB, chạy lại sẽ làm mất khoá), `shortElapsed` / `elapsedText` ở `ui/widgets/common.dart`, `relativeTime(l, seconds)` ở `log_page.dart`, `SinceText`, `pane_header`, `commit_page` đều dùng; test mới `test/duration_text_test.dart`.

## Tách file (trước -> sau)
- `remote/connection.dart` 434 -> 400 (+ `remote/connect_route.dart` mới, `markRevoked` vào `link_rules.dart`)
- `ui/viewers/changes_page.dart` 431 -> 222 (+ `changes_widgets.dart` 219)
- `ui/viewers/explorer_page.dart` 431 -> 299 (+ `explorer_row.dart`)
- `ui/viewers/run_page.dart` 431 -> 245 (+ `run_widgets.dart`)
- `ui/monitor/session_detail_page.dart` 406 -> 106 (+ `session_detail_parts.dart`)

## Test
- `npm test -w @ccm/pwa`: 156 pass, 0 fail; `npm run typecheck -w @ccm/pwa` sạch.
- `node apps/mobile/scripts/check.mjs`: analyze sạch; flutter test 278 pass, 63 skip, 0 fail (trước 276, thêm 2); "files > 400 lines" trống.
