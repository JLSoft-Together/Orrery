## WP-113d — DONE
- Đã đổi: `MainActivity.kt` (extra `pairQr`, onCreate/onNewIntent, xoá extra sau đọc, bỏ qua khi savedInstanceState != null, channel `dev.ccm.orrery.remote/pair`, hàng đợi qua `ready`, `fromShell` = referrer/callingPackage `com.android.shell`); `AndroidManifest.xml` (+host `pair`); `pane_link.dart` (`parsePairLink`); mới `pair_request.dart` (PairRequest, ChannelPairSource, sameSubnet24, lanMatchesPhone, phoneIpv4Addresses bằng `NetworkInterface.list`), `pair_confirm_sheet.dart`; `deep_link_host.dart` (pairSource/decodeQr/localAddresses tuỳ chọn, giữ khi khoá, dedupe 2s, sheet thay thế khi có yêu cầu mới); ARB en+vi (7 chuỗi `pairConfirm*`, `pairInvalidCode`, `pairWrongNetwork`) + gen l10n; test mới `test/pair_link_test.dart`.
- Tiêu chí: parsePairLink (hợp lệ/thiếu q/quá dài/host lạ/scheme lạ/control char) OK; shell + web đều tới sheet, luôn hỏi trước OK; khoá giữ rồi xử lý OK; khác subnet chặn OK; link pane cũ xanh OK; chuỗi en+vi, không comment OK.
- Audit: không đụng connect_route / pubspec / gradle / Rust / web. Không package mới.
- Review: decode lỗi -> toast "Mã ghép đôi không hợp lệ"; hết hạn/lỗi pair hiện NoticeCard trên sheet; xoay màn: configChanges + xoá extra; link trùng trong 2s bỏ; yêu cầu khác khi sheet mở -> thay sheet. Chỉ so subnet với `lan` trong QR (không dùng `host` relay).
- Test: `flutter test --concurrency=2 test/pair_link_test.dart deep_link_test pane_link_test arb_no_dots_test pair_page_test lock_ui_test` -> 46 pass. `flutter analyze lib/app test/pair_link_test.dart` -> chỉ 1 info unnecessary_import, đã sửa sau đó (CHƯA chạy lại analyze). (Chạy JIN_FORCE=1 vì máy >90% do agent song song.)
- CHƯA KIỂM: Kotlin chưa compile; `referrer`/`callingPackage` thực tế khi `adb shell am start` (có thể null, khi đó sheet ghi "Mở từ trang web" và bị kiểm subnet); desktop nên truyền cả mã qua extra `pairQr`.
- Nhìn: CHƯA NHÌN. Sheet đáy: tiêu đề "Ghép đôi với <tên>?", dòng icon USB/globe + nguồn, dòng icon Wi-Fi + "Địa chỉ <ip>", NoticeCard lỗi (khác mạng / lỗi ghép), hàng tiến trình LAN/relay, 2 nút Huỷ / Ghép đôi (ẩn Ghép đôi khi khác mạng).
- Hot file cần thêm: không.
- Ngoài phạm vi cần main xử lý: nếu WP-113a/desktop đổi tên extra hoặc cách mở adb (`--es pairQr`), khớp với `pairQr` này.
- Commit đề xuất: feat(mobile): confirm and pair from pairQr intent extra and orrery-remote://pair link

### Vòng 2 (sửa theo main)
- Chỉ link URI orrery-remote://pair (PairRequest.fromLink) mới kiểm subnet; mọi yêu cầu từ extra bỏ kiểm. Nhãn: shell = "Qua cáp USB"; extra khác/null = "Qua cáp USB hoặc app trên máy" (pairConfirmSourceDevice, en+vi); link = "Mở từ trang web".
- Extra: hiện IP đầu tiên khác 127.0.0.1; chỉ có loopback = "Qua cáp USB" (pairConfirmAddressUsb).
- Test: flutter analyze lib/app test/pair_link_test.dart: No issues found. flutter test --concurrency=2 (pair_link, deep_link, pane_link, arb_no_dots, pair_page, lock_ui): 49 pass (thêm 3 test: extra không referrer bỏ subnet + nhãn, loopback-only, loopback + LAN).
