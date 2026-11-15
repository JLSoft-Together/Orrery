# Checklist nghiệm thu v102 Sương kính

Mỗi mục chỉ nghiệm thu trong `data-ui="next"`, trừ bất biến classic được kiểm ở cả hai chế độ. Số đo căn theo mockup `.planning/mockups/v102/src/*.dc.html` và SPEC.

## WP0+1: Cờ giao diện và Design System

- [ ] S4 Mặc định `ccm.uiGeneration` là `classic`; `data-ui` được đặt trước khi mount nên tải lại không chớp giao diện.
- [ ] S4 Chuyển từ classic bằng nhãn “Thử giao diện mới (UI next gen)”, chuyển về bằng “Về giao diện Orrery cũ”; đổi ngay không restart.
- [ ] S4 Palette next lưu riêng `ccm.nextTheme`; đổi qua lại không làm đổi `ccm.theme` của classic.
- [ ] M2 Token có radius window 24px, card 18px, control 12px, inner 8px; kính dùng blur 20px và saturate 1.5.
- [ ] M2 Blur chỉ có trên chrome và lớp nổi; terminal pane không blur.
- [ ] M2 Terminal well mặc định đặc; khi bật trong suốt đạt 92% ở sáng và 55% ở tối.
- [ ] M2 Có đủ 9 palette next: Sương kính, Tinh vân, Cực quang, Than chì, Bình minh sương sáng/tối, Ngân hà, Orrery sáng/tối, Console.
- [ ] M2 Chữ trên surface đã ghép đạt WCAG AA 4.5:1 ở mọi palette và mode sáng/tối.
- [ ] M2 Font mặc định Manrope, font mono JetBrains Mono; cả hai dùng được offline.
- [ ] M2 `GlassDialog` có biến thể 480px, 640px, 880px và focus trap chung; `GlassMenu` rộng 220-280px, row 32px.
- [ ] M2 `GlassTooltip` hiện ngay khi focus và sau 450ms khi hover; `SegmentedControl`, chip cao 20px, `PaneHeader` và context gauge dùng primitive chung.
- [ ] G4 Các khóa `ccm.uiGeneration`, `ccm.nextTheme`, `ccm.next.railGroup`, `ccm.next.explorerPinned`, `ccm.next.reduceGlass`, `ccm.next.clearTerm`, `ccm.next.motion` đọc/ghi ổn định sau reload.
- [ ] Bất biến: E2E classic hiện có giữ nguyên kết quả; logic store, rail, split, pane info, geometry và workspace keys không bị nhân bản.
- [ ] Cách kiểm: đọc `stores/settings.ts`, `index.html`, `next/tokens.css`, `next/palettes.css`, component primitive; chạy contrast test và E2E helper ở classic/next; chụp palette sáng/tối.

## WP2: Shell

- [ ] S1 Đầu rail có logo và segmented Workspace/Giám sát; chân rail có hạn mức 5h/tuần, Inbox badge, Usage, History và Settings.
- [ ] S1 Shell next không render AppHeader và footer cũ.
- [ ] S2 Nền có 3 radial-gradient tĩnh theo palette; palette Ngân hà giữ sao nền.
- [ ] S3 Ở viewport 1440px rail rộng 264px; 1200px rộng 248px; 1024px thu còn 56px; dưới 1024px thành overlay rộng 248px.
- [ ] S3 Khi đổi kích thước qua breakpoint, hysteresis 24px ngăn rail bật/tắt liên tục.
- [ ] S4 Nút chuyển giao diện hiện trong Settings và Ctrl+K; trạng thái còn nguyên sau reload.
- [ ] M1 Chọn Đầy đủ/Giảm/Tắt cập nhật `data-motion`; mặc định theo `prefers-reduced-motion`.
- [ ] M6 Ẩn hoặc thu nhỏ cửa sổ thì mọi animation lặp dừng.
- [ ] Bất biến: classic vẫn đúng giao diện 4.0.5; Workspace/Giám sát vẫn dùng routing và store hiện có.
- [ ] Cách kiểm: đọc `App.vue`, `next/NextShell.vue`, router và settings; chụp ở 1600px, 1200px, 1024px, 1000px; kiểm chuyển mode không reload.

## WP3: Rail sống

- [ ] R1 Row session cao 46-56px, có 2 dòng; dòng 1 có icon trạng thái, tên và thời gian khi đang chạy.
- [ ] R1 Dòng 2 có agent, tên model và đúng một chi tiết ưu tiên; gauge context có kích thước 28x4px và chuyển vàng từ 65%.
- [ ] R1 Thiếu model thì để trống, không tự suy đoán; Shell không hiện model.
- [ ] R2 Segmented Project/Trạng thái đổi cách nhóm và lưu ở `ccm.next.railGroup`.
- [ ] R2 Nhóm trạng thái gồm Lỗi, Đang chạy, Xong chưa xem, Rảnh; Cần bạn ở khối riêng.
- [ ] R3 Khối Cần bạn sticky ở cả hai cách nhóm, ẩn khi rỗng; item có icon, tên, lý do, thời gian chờ và nút Mở.
- [ ] R3 “Mở kế tiếp” gọi luồng `wsAttention` bằng Ctrl+Shift+A.
- [ ] R4 Cần trả lời dùng ?, Cần duyệt quyền dùng khóa, Lỗi dùng x ở nhóm riêng; Xong chưa xem không nằm trong Cần bạn.
- [ ] R4 Thứ tự ưu tiên là duyệt quyền/câu hỏi, lỗi, xong chưa xem, rồi chờ lâu nhất; mọi trạng thái có icon, chữ và màu.
- [ ] R5 Menu hoãn có 5, 15, 60 phút; lỗi không thể hoãn quá 15 phút; xử lý item có toast hoàn tác 5s.
- [ ] R6 Khi con trỏ ở rail, đổi trạng thái không làm row đổi chỗ; thứ tự cập nhật khi con trỏ rời rail; row ghim giữ vị trí.
- [ ] R7 Rail thu gọn rộng 56px, monogram project 28px, chấm trạng thái từng session và badge Cần bạn/Lỗi; bấm project mở flyout 280px.
- [ ] R7 Focus bàn phím trên rail thu gọn hiện nhãn ngay.
- [ ] R8 Model lấy qua `agents.mainOf(id).model` và `modelLabel`; thiếu dữ liệu không hiện nhãn giả.
- [ ] M3 Chuyển sang cần bạn hoặc lỗi phát viền sáng 450ms đúng một lần, không pulse lặp.
- [ ] Bất biến: giữ đủ 9 giá trị `RailState`; Cần bạn lỗi trước asked, cũ nhất trước; toast OS có nút đáp án; mở session không đổi split và không cướp focus khi item mới xuất hiện.
- [ ] Cách kiểm: đọc `LiveRail.vue`, `usePaneInfo`, `attentionList.ts`, model helpers; Vitest cho thứ tự/taxonomy và timer; chụp rail đủ trạng thái, thu gọn và flyout.

## WP4a: Tab, pane card và well

- [ ] T1 Tab strip cao 36px, pill cao 28px; pill có monogram tone project và một chip ưu tiên; phần phụ còn lại hiện khi hover.
- [ ] T1 Khi tràn tab, phần cuối hiển thị `+N`.
- [ ] T2 Chip “Chờ trả lời N” giữ Ctrl+Alt+Q và dùng kiểu chip mới.
- [ ] T3 Pane card bo 18px, header cao 34px; pane active có viền accent 2px và header tint accent 6%.
- [ ] T3 Một `PaneHeader` dùng chung cho terminal và 7 viewer.
- [ ] T4 Thanh click menu nằm dưới pane, nền vàng nhạt, nút có số phím và nút chính đặc accent.
- [ ] T5 Terminal mặc định đặc theo palette; bật trong suốt đạt 92% sáng, 55% tối; Ngân hà cho sao xuyên qua.
- [ ] M4 Hiệu ứng tan bụi kết thúc trong 700ms.
- [ ] M7 Đổi tab pill và trạng thái head bằng transform/opacity, không animate width, left hoặc grid-template-rows.
- [ ] Bất biến: pane không remount khi đổi tab, split, zoom; menu click giữ numbered/selected/checked/Other/Submit, disabled lúc busy và hover dòng.
- [ ] Cách kiểm: đọc `TabStrip.vue`, `PaneHeader.vue`, `TermPane.vue`, viewer headers, click menu; Vitest/E2E workspace; ảnh tab, pane active, well sáng/tối.

## WP4b: Kéo thả và split

- [ ] D1 La bàn có 5 ô 44x44px: trái, phải, trên, dưới, giữa đổi chỗ; preview ghi hành động và tỉ lệ, ví dụ “Chia phải, 50%”.
- [ ] D2 Menu Bố cục có 1 pane, 2 cột, 3 cột, 2x2, 1 lớn + cột, Trên + 2 dưới, Chia đều; chỉ sắp xếp pane đang có.
- [ ] D3 Kéo lên tab pill đủ 600ms mới chuyển tab; thanh tiến trình nằm dưới pill.
- [ ] D4 Thả file Explorer mở viewer ở vị trí thả; row rail được kéo như pane hiện có.
- [ ] D5 Preview chia có nền accent 12% và viền 2px; đổi chỗ có viền nét đứt quanh cả hai pane.
- [ ] D5 Pane dưới 220px báo “Quá hẹp: còn Npx, tối thiểu 220px” màu đỏ; thả không làm thay đổi layout.
- [ ] D6 Ghost là pill kính cao 40px có agent, tên và chip trạng thái; pane gốc phủ lớp canvas 62% cùng nhãn “Đang kéo pane này”.
- [ ] D7 Divider có hit area 12px; hover/kéo hiện vạch accent 2px và grip 3 chấm.
- [ ] D8 Khi kéo divider hiện tỉ lệ; snap 1/3, 1/2, 2/3 trong 12px; giữ Alt bỏ snap; double click về 50/50.
- [ ] D9 Sau thả, preset hoặc Chia đều hiện toast và nút Hoàn tác trong 5s; Ctrl+Z không bị chiếm.
- [ ] D10 Từ 5 pane, nếu pane nhỏ hơn 360px rộng hoặc 180px cao, hiện gợi ý tách; lựa chọn Bỏ qua được nhớ theo tab.
- [ ] D11 Menu pane liệt kê phím chia phải/dưới, chuyển pane, đổi chỗ, zoom, đóng và Tách sang tab mới Ctrl+Alt+T; macOS dùng Alt+Cmd+T.
- [ ] M7 Drop zone đổi bằng transform/opacity, không animate layout.
- [ ] Bất biến: giữ kéo pane split/dock/swap, kéo tab đổi chỗ/kéo khỏi strip, kéo row rail giữa tab, kéo project đổi thứ tự; divider hỗ trợ phím, tỉ lệ giới hạn 15-85%, zoom, mở lại pane đóng và pane không remount.
- [ ] Bất biến: dùng chung `splitLayout`, `useRailDrag`, `view/geometry.ts`, `lib/workspaceKeys.ts`; Ctrl+Z của terminal/editor không đổi.
- [ ] Cách kiểm: đọc drag handlers, `splitLayout`, `useRailDrag`, geometry và workspace keys; E2E drag chạy classic/next; chụp preview chia, swap, pane hẹp và divider.

## WP5: Explorer

- [ ] E1 Explorer mở thành drawer 288px trượt đè từ phải bằng Ctrl+Shift+E; canvas có scrim 10%.
- [ ] E1 Esc đóng drawer và trả focus về phần tử mở.
- [ ] E2 Ghim lưu `ccm.next.explorerPinned`; chỉ ghim khi viewport >=1440px và canvas còn >=720px.
- [ ] E2 Nếu không đủ chỗ, Explorer về drawer và hiện toast giải thích.
- [ ] E3 Files, Changes, Search, Checks dùng được; TODO nằm trong menu; Changes giữ badge số.
- [ ] Bất biến: giữ đủ 5 view cũ, status line, git marks, resize và xoá an toàn qua trash.
- [ ] Cách kiểm: đọc `ExplorerDrawer.vue`, `ExplorerPanel.vue`, shortcut registry và settings; Vitest điều kiện pin; ảnh drawer mở/ghim và viewport hẹp.

## WP6: Lớp nổi và Ctrl+K

- [ ] K1 Khi ô tìm rỗng, Ctrl+K xếp Cần bạn, Đang chạy, Xong chưa xem, Gần đây rồi Lệnh.
- [ ] K1 Row kết quả cao 52px, có icon trạng thái, tên, lý do/tool, project, branch, model và thời gian.
- [ ] K2 Tìm kiếm khớp model, branch và repo.
- [ ] K3 Giữ các phạm vi Tất cả, Session (@), File (#), Lệnh (>), Project (:); Enter mở tại chỗ giữ split, Shift+Enter mở pane mới.
- [ ] K3 Phím cũ không đổi; phím mới duy nhất là Ctrl+Alt+T, hoặc Alt+Cmd+T trên macOS.
- [ ] Bất biến: QuickOpen, ProjectSwitcher, dialog, menu, toast và popover giữ hành vi/phím sẵn có; focus trap và trả focus hoạt động.
- [ ] Cách kiểm: đọc command palette/Ctrl+K, QuickOpen, ProjectSwitcher và primitive overlay; Vitest filter/rank; ảnh rỗng, có kết quả, không kết quả và dialog.

## WP7: Mission Control

- [ ] Mission Control là chế độ phụ trong shell next; card và tile dùng bề mặt kính.
- [ ] Thẻ trả lời menu v100 hiện đủ câu hỏi và hành động cần trả lời.
- [ ] Bất biến: phím 1-9, mũi tên roving, Esc, lọc, click vào tab/pane, kéo card/tile, snapshot terminal và ask card hoạt động như trước.
- [ ] Bất biến: click vào card/pane không làm mất pane state hoặc thay đổi split layout.
- [ ] Cách kiểm: đọc `MissionControl.vue`, `mission/*`, `useMissionDrag.ts` và ask card; Vitest MissionCard/queue; ảnh 8 session, card hỏi và bộ lọc.

## WP8: Viewer

- [ ] T3 File, Changes, Log, Run, Browser, XML và Flutter preview nằm trong card với header chung cao 34px.
- [ ] Viewer File giữ editor, minimap/ruler, outline, trạng thái chưa lưu và nút lưu.
- [ ] Viewer Changes giữ Diff/SplitDiff, CommitBox, ConflictView và HunkPopup.
- [ ] Viewer Log giữ bảng commit, đồ thị, refs, bộ lọc và chi tiết file.
- [ ] Viewer Run giữ Run/Stop, trạng thái Building/Running/Stopped/Lỗi và progress.
- [ ] Viewer Browser giữ back/forward/stop/reload, address bar cao 28px, device và console controls.
- [ ] Bất biến: XML/Flutter preview vẫn nằm đúng trong File pane; viewer loading, lỗi và dim có nhãn đọc được.
- [ ] Bất biến: giữ chỉnh sửa/lưu file, diff, run, webview và các shortcut hiện có; pane không remount khi đổi layout.
- [ ] Cách kiểm: đọc từng `*Pane.vue`, `PaneHeader.vue`, viewer controls; Vitest viewer liên quan; ảnh mỗi loại viewer ở trạng thái thường/loading/lỗi.

## WP9: Monitor, Usage, History, Inbox

- [ ] Các view Monitor/Quỹ đạo, Usage, History và Inbox được render trong shell next, không có AppHeader cũ.
- [ ] Monitor giữ card session, activity và nhãn model; Quỹ đạo giữ thông tin agent/subagent.
- [ ] Bất biến: dữ liệu, điều hướng và thao tác hiện có không đổi; trạng thái có icon, chữ và màu.
- [ ] Cách kiểm: đọc view components, route và store tương ứng; Vitest dữ liệu/state; chụp từng view trong shell next.

## WP10+12: Settings, Devices, Extensions, Disk và phần v100/v101

- [ ] G1 Settings là dialog kính 1040x760px với nav trái 220px; trang Giao diện có kiểu giao diện, 9 palette, sáng/tối, Giảm trong suốt, Terminal trong suốt, Chuyển động, Sao Ngân hà và Phông chữ.
- [ ] G1 Palette preview dùng mini Workspace; chọn palette áp dụng ngay.
- [ ] G2 Workspace và Ổ đĩa là các trang riêng; Devices và Extensions dùng shell next; Office chỉ có khung.
- [ ] G3 “Giao diện” là tên trang; lựa chọn mode có tên “Sáng / tối”.
- [ ] G4 Settings persist đủ các khóa next và không ghi đè theme classic.
- [ ] WP12 Click menu dưới pane và hover dòng giữ đủ trạng thái v100; Mission card có thể trả lời menu.
- [ ] WP12 Settings Workspace switch, Settings Ổ đĩa, view Ổ đĩa và 9 tab Duyệt, Tìm kiếm, File lớn, File cũ, Dọn dẹp, File trùng, Sắp xếp, App, RAM dùng được.
- [ ] WP12 Explorer menu “Chuyển sang ổ khác” và dialog chuyển ổ hoạt động.
- [ ] Bất biến: giữ hành vi v100/v101, xác nhận an toàn khi thao tác phá huỷ, phím cũ và dữ liệu store; classic giữ nguyên giao diện cũ.
- [ ] Cách kiểm: đọc Settings, Devices, Extensions, Disk, click-menu và Explorer disk actions; Vitest store/components; ảnh Settings, từng tab Disk và menu chuyển ổ.

## WP11: QA tích hợp

- [ ] M5 `ccm-meteor-pane` không chạy trong next; sao băng nền vẫn chạy ở palette Ngân hà.
- [ ] Classic render trùng ảnh baseline 4.0.5; E2E hiện có chạy pass không sửa.
- [ ] E2E chính và kéo thả chạy ở cả classic lẫn next.
- [ ] Ảnh bao phủ mọi màn, palette và mode sáng/tối; không có cuộn ngang trang.
- [ ] Đo 6 pane khi resize; ghi FPS và GPU; không có blur trên terminal pane hoặc list dài.
- [ ] Tất cả chuỗi next đi qua vue-i18n namespace `mist.*` cho en và vi.
- [ ] Bất biến: giảm motion tắt loop, trạng thái không chỉ phân biệt bằng màu, WCAG AA 4.5:1; không animate blur/layout.
- [ ] Cách kiểm: chạy E2E classic/next, Vitest liên quan và contrast tests; chụp ma trận palette/sáng-tối; đọc `07-motion.md`, i18n catalog và báo cáo đo GPU/FPS.
