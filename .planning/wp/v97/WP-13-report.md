# WP-13 report

Trạng thái: PARTIAL (thiếu: deploy relay + kiểm assetlinks.json thật; bị auto-mode classifier từ chối `npm run deploy`)

## Đã đổi
- core-desk: `phone/link.rs` (mới), `phone/mod.rs` (mod + gắn link vào tin ntfy), `phone/send.rs` (`Text.link/link_label`, ntfy `click` + `actions[view]`), `telegram/pane_link.rs` (mới), `telegram/mod.rs` (1 dòng mod), `telegram/episodes.rs` (1 dòng gọi `with_pane_link`), `telegram/tests.rs` (host giả `remote.link` + test mới)
- shell: `src/remote/pane_link.rs` (mới: `pane_of`, `build`, `Remote::link_for`, `call`), `remote/mod.rs` (1 dòng `pub mod pane_link;`), `hosts.rs` (1 arm `"remote.link"`)
- mobile: `lib/app/{pane_link,link_source,deep_link_host}.dart` (mới), `lib/main.dart` (`linkSource` + `MaterialApp.builder`), `pubspec.yaml` (+`app_links ^7.2.2`), AndroidManifest (2 intent-filter, `flutter_deeplinking_enabled=false`), iOS `Info.plist` (`CFBundleURLTypes`, `FlutterDeepLinkingEnabled=false`), `Runner/Runner.entitlements` (mới, `applinks:`), `project.pbxproj` (`CODE_SIGN_ENTITLEMENTS` x3), l10n (`linkUnknownMachine`, `linkPaneGone` qua `tool/gen_arb.mjs` + gen-l10n), tests `pane_link_test`, `deep_link_test`, `ui_integration_test` (truyền `_NoLinks`)
- relay: `wrangler.jsonc` `ANDROID_CERT_SHA256` = 1C:73:97:FC:D7:37:54:00:04:77:4E:13:E5:47:71:45:AB:13:57:8D:95:BF:3D:9A:B9:20:30:15:88:15:D6:61 (keystore debug)
- docs: `docs/remote-integration.md` §7.1; `.code_index/{core,shell}.md`, `apps/mobile/.code_index/mobile.md`

## Tiêu chí 4/5
1. Remote tắt / chưa có thiết bị -> tin như cũ: ĐẠT (test `pane_link_button_only_when_...` nhánh không link / link giả; shell test `no_link_while_remote_is_off`; test telegram + phone cũ pass).
2. Có thiết bị -> nút URL / click đúng, paneKey an toàn: ĐẠT ở mức unit (`valid_link`, `build`, `pane_of`, ntfy click/actions, Telegram hàng URL). CHƯA KIỂM: nhánh "có ≥ 1 thiết bị đã ghép" của `link_for` (không dựng được device trong test mà không đụng test_util của WP-8); terminalId của episode == `terminalId` của pane trong Workspace summary chưa kiểm E2E.
3. App: ĐẠT (parse cả 2 dạng, widget test với store giả: mở pane, máy chưa ghép, pane giả mạo, link rác, chờ workspace).
4. Relay deploy + assetlinks: CHƯA KIỂM: `npm run deploy` bị classifier từ chối. `wrangler.jsonc` đã sửa, `npm test` 33 pass. Chủ nhân / main chạy `cd apps/relay && npm run deploy` rồi `curl https://orrery-relay.jinvu.workers.dev/.well-known/assetlinks.json`.
5. Test + không comment + file <= 400 dòng: ĐẠT.

## Audit
Diff chỉ trong phạm vi; hosts.rs chỉ thêm 1 arm, remote/mod.rs chỉ thêm 1 dòng `pub mod pane_link;` (WP-8 cần giữ khi gộp). Không đụng `apps/web`, `apps/shell/src/remote/` ngoài file mới.

## Review
- Remote tắt: `link_for` -> None (ctx None hoặc `running=false`). Chưa có thiết bị: None. machineId lỗi: `valid_machine_id` + core kiểm lại. Pane đã đóng: summary không còn -> None; app báo "Pane này không còn mở". Link giả mạo paneKey: app chỉ mở khi key có trong Workspace summary của máy đã ghép; ký tự lạ bị `parsePaneLink` loại; core từ chối URL không đúng dạng `/o/<m>/<p>` https.
- Tin phone (ntfy) không mang `terminalId` (parse_phone của `episodes.rs` lọc field) nên suy từ khoá episode `w:|d:`; không sửa `shell/src/episodes.rs` / collector.
- Discord không gắn link (không trong brief).
- app_links v7: `uriLinkStream` dùng cả cho link lúc khởi động; có dedupe 2 s.
- Telegram: khi không bật receive, tin giờ có bàn phím chỉ gồm nút URL (nếu có link).

## Test
- `cargo test --locked -j 3 -p core-desk -- telegram phone`: 43 pass
- `cd apps/shell && cargo test --locked -j 3`: 255 pass
- `flutter analyze`: sạch; `flutter test --concurrency=4`: 147 pass
- `cd apps/relay && npm test`: 33 pass

## Ngoài phạm vi
iOS `APPLE_APP_ID` để trống (chưa Team ID); Associated Domains cần bật capability + provisioning khi ký. Chưa kiểm trên máy thật (Android App Links verify, iOS).

## Commit message đề xuất
feat(remote): open a pane from Telegram and ntfy notifications in Orrery Remote (remote.link host call, Telegram URL button, ntfy click, app deep links, relay debug cert)
