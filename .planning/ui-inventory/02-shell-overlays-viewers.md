# UI inventory 02: shell, overlays, viewers (hiện trạng, trước v102)

Phạm vi: App shell, overlay/dialog/popover, viewer trong Workspace, design primitives. Đo trên nhánh `feat_clean_memory`, đọc code, không sửa source. Đường dẫn tính từ `apps/web/src/`. "chưa rõ" = không xác minh được từ code.

Số liệu đếm (rg trên `components/**/*.vue`): `ccm-btn` 333, `ccm-btn-plain` 389, `ccm-btn-primary` 81, `ccm-btn-danger` 18, file dùng `ccm-sheet` 55, file dùng `ccm-scrim` 50, file có `role="dialog"` 66, file có `role="menu"` 13, file dùng class `ccm-menu` 41 (tổng 55 lần), file dùng `SegmentedControl` 36, `ToggleSwitch` 23, `ContextMenu` 14, `AppSelect` 16, `title=` native 1002 chỗ, không có `backdrop-filter` nào (rule "No blur" 09/11).

---

## A. Shell

### A1. App.vue (`App.vue:187-219`)
- Cấu trúc: `div.flex.min-h-dvh.flex-col.overflow-x-hidden` > skip link `#main` > `AppHeader` > lưới 1 ô `grid-cols-[minmax(0,1fr)] *:[grid-area:1/1]` chứa các view > `WorkspaceFooter` > các overlay.
- Không có vue-router. Chuyển view bằng `ui.view` (`stores/ui.ts:14` kiểu `View = monitor | workspace | usage | history | extensions | devices | disk`), lưu `sessionStorage ccm.view` (`ui.ts:18-30`). `ui.setView()` tại `ui.ts:422` và tắt compact. Chế độ compact qua `?mode=compact` (`ui.ts:87`, `:350`).
- Transition `ccm-view` = crossfade 220ms vào, 150ms ra (`style.css:1435-1445`); view cũ bị `inert` + bỏ id `main` (`App.vue:117`).
- `WorkspaceView` mount 1 lần (lazy), sau đó `v-show` để giữ xterm (`App.vue:195-199`). Các view khác `v-if`. `MonitorView`, `ExtensionsView`, `DevicesView`, `DiskView`, `WorkspaceView` là async component (`App.vue:53-57`).
- Điều kiện hiện view: `extensions` cần `hasExtensions`, `devices` cần `hasDevices`, `disk` cần `hasDiskMap` (view Disk đã có, `App.vue:192`). `workspace` cần `hasTerminals` (`App.vue:99`).
- Footer chỉ hiện ở Workspace và Monitor, không ở compact (`App.vue:100`). Chiều cao header+footer ghi vào `--ccm-header-h` bằng ResizeObserver (`App.vue:121-150`).
- Overlay mount ở root theo thứ tự: `CommandPalette` (eager, giữ Ctrl+K), `RecapDialog`, `DevToolsDialog`, `DiagnosticsDialog`, `ResourcesDialog`, `BuildCleanDialog` (ẩn khi `diskCleanOn`), `SessionToolsSheet`, `QuickSetup`, `OnboardingTour`, `WhatsNewDialog`, `UpdateDialog`, `ToastHost`, `ShortcutsDialog`, `ZoomHud` (`App.vue:204-217`). Lazy qua `lib/lazyOverlay` (`App.vue:68-72`).
- Phím: Cmd+W (mac) đóng window khi không ở workspace (`App.vue:156-160`); Ctrl+K palette (`CommandPalette.vue:690`); `?` mở shortcuts (`composables/useShortcuts.ts:16`); backtick toggle command panel ở Monitor (`useMonitorKeys.ts:12`).

### A2. AppHeader.vue (`components/AppHeader.vue`)
- `<header class="ccm-material ccm-titlebar sticky top-0 z-30 border-b ccm-chrome-edge">` (`:109`). Hàng chính `h-10` (40px), `gap-2`; padding trái `pl-[84px]` khi titleBar mac, `pl-3` còn lại (`:60`); padding phải dùng `env(titlebar-area-*)` cho overlay Windows (`:61`). Vùng kéo cửa sổ `app-region: drag`, phần tử tương tác `no-drag` (`style.css:545-553`).
- Trái sang phải: logo `ArtImage logo` 20px (`:111`) | wordmark `orrery` (chỉ hiện với theme có `--ccm-wordmark-display`) | tablist | nút `...` | spacer | update pill | `TelegramToggle` | `HeaderStatusPopover` | Inbox | Search (palette) | `SettingsMenu` (chuông + cài đặt).
- Tablist (`:114-142`): segmented tự viết dùng `ccm-seg` + `ccm-seg-thumb` trượt `translateX`, tab `h-6`, `text-xs`, `sm:min-w-[92px]`, icon 14px, nhãn ẩn dưới `sm`. Tab: Giám sát (Activity), Workspace (SquareTerminal) + 1 tab "extra" là view phụ đang mở. Phím mũi tên trái/phải xoay tab (`:99-105`).
- Nút `...` "Màn hình khác" (`:144-157`, `size-7`) mở `ContextMenu` (`:212`): Thống kê, Lịch sử, Mở rộng, Thiết bị, Disk | (dim inactive khi split tab) | Compact vào/ra | Phím tắt | Hướng dẫn Workspace (chỉ ở workspace) (`:71-80`).
- Update pill (`:162-174`): `h-6 rounded-full bg-fill`, icon + "Có bản mới {version}" (key `update.pill`), mở `UpdateDialog`.
- Inbox button (`:177-196`): icon Inbox 15px, badge tròn `h-3.5 min-w-3.5 bg-st-waiting` đếm agent cần chủ nhân (tối đa "99+"), mở `InboxPanel` (popover, `inbox/InboxPanel.vue:96`, width `min(400, vw-24)` `:39`, cao `max-h-[min(70vh,520px)]`, rows `min-h-9`, hint "J/K, Enter, E đã xem" key `inbox.keys`).
- Search button (`:197-207`): aria-keyshortcuts Ctrl/Meta+K, tooltip "Bảng lệnh (Ctrl+K)".
- State: banner stale `connection.stale` ("mất kết nối") trượt dưới header khi `connection.state !== 'connected' && hasData` (`:213-221`), nền `bg-warning-soft`.
- Compact: ẩn tablist, update pill.

### A3. HeaderStatusPopover (`components/HeaderStatusPopover.vue`)
- Nút (`:122-154`) `ccm-btn-plain gap-1.5 text-xs`: icon Gauge / CircleAlert / OctagonAlert theo mức (14px), nhãn "Hạn mức" (ẩn dưới `sm`), số % thấp nhất ("còn {pct}%"). Offline: WifiOff + chữ trạng thái. Chấm đĩa đầy (góc trên phải, `size-2`, đỏ/vàng) và chấm risk forecast (góc dưới phải). Có `data-awake`, `data-disk-badge`.
- Popover (`:157-198`): `ccm-menu fixed z-50 p-3 max-h-[min(80vh,560px)]`, width cố định `WIDTH = 340` (`:77`), neo phải theo nút, `top = bottom + 6`.
- Nội dung: tiêu đề "Hạn mức và trạng thái" > `PlanLimits list` cho Claude và Codex (nếu có số 5h hoặc 7d) > câu rỗng "Chưa có số liệu hạn mức..." > hàng `KeepAwakeBadge row` ("Đang giữ máy thức") > hàng "Ổ đĩa sắp đầy / Xem các ổ" (chỉ khi `diskLow`) > `ResourcesButton row` > dòng "Bộ theo dõi" + trạng thái kết nối + lần cập nhật cuối.
- Nguồn: `extras.limits`, `extras.codexLimits`, `awake.state`, `disk.worst`, `connection` (store).
- Kèm `DiskBadge` (`:199`) là popover riêng neo vào nút.

### A4. PlanLimits (`components/PlanLimits.vue`)
- 4 biến thể trong 1 file: `list` (`:134-154`, trong popover: lưới `grid-cols-[3.5rem_1fr_auto]`, thanh `h-1.5`), `compact` (`:155-181`, nút `h-7`, `5h 42% 7d 18%`), mặc định (`:182-235`, nút có lưới 3-4 cột, thanh 44-56px `h-1.5`, cột reset ẩn dưới `sm`), và popover chi tiết (`:237-281`, `sm:w-[340px]`, `p-4!`, số lớn `text-2xl`, thanh `h-2` có vạch 20%).
- Mỗi window (5h, 7d): % đã dùng/còn, giờ reset ("resetAt (resetsIn)"), rate, forecast "Hết lúc {at} nếu dùng như hiện tại", mức `ok|warn|critical` (icon `LEVEL[...]`, `ccm-pulse` khi critical).
- Animation: `ccm-quota-flash`, `ccm-quota-delta`, `ccm-quota-shine`, `ccm-tick` khi số đổi.
- Màu: Claude `text-agent-claude-ink`, Codex `text-agent-codex` (icon Asterisk / Hexagon). Stale hiện note "limits.staleNote".

### A5. MetricsStrip (`components/MetricsStrip.vue`)
- 2 chế độ (`:76-113`): `compact` = `dl` cuộn ngang 1 hàng (nhãn `text-xs` + số `text-sm font-semibold` + thanh `h-1 w-10`); đầy đủ = lưới `grid-cols-2 sm:grid-cols-4 xl:grid-cols-8`, mỗi thẻ `rounded-card bg-surface shadow-card px-4 py-3`, số `text-xl`, thanh `h-1`.
- Nguồn: `metrics` (script), dùng `StatusIcon`, `TokenCount`, `CountNum`. Số liệu token chỉ hiện khi có data thật (rule PLAN).
- Ai dùng: chưa rõ từ file này (Monitor/Compact). Không nằm trong header hiện tại.

### A6. ToastHost (`components/ToastHost.vue`)
- Cố định `fixed right-4 bottom-4 z-50`, rộng `min(360px, 100vw-2rem)`, `aria-live=polite`, xếp dọc `gap-2` (`:33-37`). Tối đa `MAX_VISIBLE = 3` (`stores/toasts.ts:40`).
- Mỗi toast `ccm-sheet p-3`. Kind: `info | warning | undo | break | phoneFail` (`stores/toasts.ts:6`). Phần tử: art (`w-16`) hoặc icon 16px, `h2` title, text `text-sm`, detail `text-2xs`, nút đóng `ccm-btn-plain`, action `ccm-btn` (primary tuỳ chọn, có `kbd` phím).
- Layout inline (action nằm cùng hàng, text truncate) hoặc stacked (action xuống hàng dưới, meta bên trái).
- Tương tác: hover/focus tạm dừng timer (`hold`), Esc đóng (`@keydown.esc`). Animation vào `translate-y-3` + opacity.
- Nguồn tạo toast: `composables/useAppToasts.ts`, `useBugReportPrompts.ts`, `useBreakReminder`.
- Lưu ý: DoneTray không phải toast (xem A7).

### A7. DoneTray (`components/DoneTray.vue`)
- Không thuộc shell: là node Vue Flow của Agent Map (Monitor). Pill `rounded-full border-dashed border-line-strong bg-surface`, kích thước `TRAY_WIDTH x TRAY_HEIGHT` (từ `lib/layout`), các chấm `size-2` (`bg-st-done` hoặc `bg-st-stale` nếu cancelled), "+N", nhãn "{n} đã xong" (key `mapV53.tray.count`), chevron. `role=button`, `aria-expanded`.

### A8. SettingsMenu (`components/SettingsMenu.vue`)
- Header có 2 nút `size-8`: chuông thông báo (`:265-277`, `aria-pressed`, disabled khi trình duyệt không hỗ trợ, LottieArt khi đang rung) và Cài đặt (`:279-291`, chấm `bg-accent ring-canvas` khi có update).
- Dialog (`:293-411`): `Teleport body`, scrim `ccm-scrim p-4`, panel `ccm-sheet bg-canvas`, kích thước mặc định `1040x760`, min `560x420`, lề `EDGE = 16`, lưu `ccm.settings.size`, có tay kéo resize góc dưới phải + phím mũi tên `+-32px` + dblclick reset (`:40-120`). Container query `@[720px]`: sidebar trái `w-56` hoặc xếp trên.
- Sidebar: ô tìm `ccm-field` (placeholder `settingsV79.searchPlaceholder`) + 12 trang: general, appearance, editor, notify, telegram, reminders, github, integrations, deepseek, access, data, about (`settings/index.ts:3`), ẩn theo `pageVisible(env)`. Mục chọn `bg-accent-soft text-accent-ink`.
- Nội dung: header trang `h3 text-base`, vùng cuộn `px-5 pt-4 pb-6`, kết quả tìm = `ccm-group` danh sách "Trang > nhãn" (`:358-371`), nhảy tới dòng + `ccm-setting-flash` 1.4s.
- Phím: Esc đóng (`onKey`), Mod+, mở (khai báo `lib/shortcuts.ts:28`, nơi gắn: chưa rõ, có thể ở main process menu).
- Primitive trong trang: `SettingRow`, `SettingSection`, `ToggleSwitch` (`h-[22px] w-[38px]`, knob `size-[18px]`, `ToggleSwitch.vue:29-35`), `ThemePicker`, `FontPicker`, `SegmentedControl`.

### A9. WorkspaceFooter (`components/workspace/WorkspaceFooter.vue`)
- `footer.ccm-footer` cao `--ccm-footer-h` (26px), font 11.5px, nền `--ccm-chrome-fill`, viền trên `--ccm-chrome-line` (`style.css:899-906`). `role=contentinfo`.
- Mục: "{n} chạy", "{n} chờ" (nút, focus agent đầu hoặc mở Bảng), Port dev server, branch + "{n} thay đổi" (mở Changes), Context token (cảnh báo ở limit), "Hôm nay {n} token" (mở Usage), nút Focus mode (hiện khi `!onBoard`). Nguồn: workspace store, terminals, usage.
- Là footer duy nhất của app. Header cũng có footer ở Monitor.

---

## B. Overlays

Pattern chung (lặp lại từng file): `Teleport to=body` > `Transition name="ccm-dialog"` + `@before-leave="retireOverlay(...)"` > scrim `fixed inset-0 z-50 ccm-scrim` (nền `--ccm-scrim` = `rgb(12 13 17 / .45)` sáng, `style.css:593-596`) > `ccm-sheet`. Click scrim đóng bằng `@mousedown.self`. Animation: scrim opacity 220ms, sheet `scale .96 -> 1` (`style.css:1584-1606`). Focus trap tự viết `function trap(e)` trong 25 file (rg `function trap`), không có composable chung.

### B1. CommandPalette (`components/CommandPalette.vue`, 36.5K)
- Vị trí `items-start`, `pt-[8vh] sm:pt-[12vh]`, sheet `max-w-[640px] max-h-[min(78vh,620px)]` (`:745-751`). Ô nhập `ccm-bare-input h-14 text-base`, icon Search 19px, nút xoá hoặc kbd "Esc".
- Prefix (`lib/paletteSearch.ts:6`): `>` Hành động, `@` Session, `#` File, `:` Cài đặt. Gõ prefix khoá scope. Footer có 4 nút chip chọn prefix (`:894-902`).
- Scope tab (radiogroup, gạch chân `border-b-2 border-accent`, đếm số, `h-9`): Tất cả, Session, Dự án, Lệnh, File, Cài đặt (`SCOPES`, `:146`). Tab xoay scope (`:654`).
- Nhóm theo `GROUP_ORDER` (`:145`): sessions, projects, favorites, files, nav ("Đi tới"), actions ("Thao tác"), prefs ("Tuỳ chọn"), help ("Trợ giúp"), settings. Mỗi nhóm cap `GROUP_CAP = 5` (`paletteSearch.ts:8`) rồi hàng "Xem thêm {n}"; file tối đa `FILE_LIMIT = 50`.
- Lệnh (id `c:*`, `:249-495`): Tới Giám sát, Thống kê, Lịch sử, Mở rộng, Thiết bị, Disk (`c:disk`, `:264`), Git Log, Chuyển dự án (Ctrl+Alt+P), Thêm nhóm (Ctrl+Alt+G), Focus mode (Ctrl+.), Prompts (Alt+Shift+P), Plan board, Env check, Strings, Perms, Context, Compact, Inbox, Recap, Xoá session đã kết thúc, toggle (chuông, âm thanh, theme tối, font code +/-, gợi ý phím, ẩn tool, ngôn ngữ), Cài đặt, Có gì mới, Phím tắt, Hướng dẫn, Chẩn đoán, Thiết lập nhanh.
- Hàng: `min-h-11 rounded-inner px-3 py-1.5`, icon `size-7`, StatusIcon cho session, highlight chữ khớp bằng `<mark>`, hàng đang chọn `bg-accent text-on-accent` (accent đặc, không phải accent-soft), toggle hiện chấm + "Bật/Tắt", hàng chọn hiện `Shift+Enter` (mở phụ) và icon Enter.
- Phím: Mũi tên chọn, Enter mở, Shift+Enter thao tác phụ (mở cửa sổ mới, tab mới), Tab đổi scope, Esc đóng/xoá ô (`:668-690`).
- Rỗng: ArtImage `empty-project-search` + "Không có gì khớp với "{q}"" + gợi ý + nút "Xem tất cả {n}". File: các note `noProject`, `loading`, `failed`, `truncated` (`paletteV80.files.*`).

### B2. QuickOpen (`components/workspace/QuickOpen.vue`)
- Cùng khung palette: `max-w-[600px] max-h-[min(72vh,560px)]` (`:175-176`). Phím Ctrl+P. Placeholder "Tìm tệp theo tên, thêm :số-dòng để nhảy tới". Hint footer: Di chuyển, Mở, Mở bên cạnh (Shift+Enter), Đóng; prefix `>` chạy script (`workspace.scripts.quickHint`) nếu có scripts. Rỗng "Không có tệp nào khớp".

### B3. ProjectSwitcher (`components/workspace/ProjectSwitcher.vue`)
- `max-w-[640px] max-h-[min(76vh,620px)]` (`:189-190`), `data-project-switcher`. Ctrl+Alt+P (giữ phím để xoay). Combobox + listbox. Hàng: tên dự án (highlight), sao favorite, chip "đang mở", branch, `RunBadge`, "cần bạn / lỗi {n}", số tab hoặc "đã đóng", thời điểm dùng. Footer: hint (tiếp, di chuyển, mở, tab mới, xoá, đóng) + nút "Mở thư mục..." (`canPick`). Rỗng: "Chưa có dự án nào..." hoặc "Không có dự án nào khớp".

### B4. ContextMenu (`components/ui/ContextMenu.vue`)
- Một panel hoặc nhiều (submenu) `ccm-menu ccm-scroll fixed z-50 p-1 max-h-[calc(100vh-16px)]` (`:186`). Entry: `{id,label,icon,hint,wrapHint,shortcut,disabled,children}` hoặc separator `mx-2 my-1 border-t`. Item 14px icon, nhãn truncate, hint `text-2xs`, `kbd` shortcut, chevron cho submenu. `role=menuitem`, mũi tên điều hướng, hover mở submenu. Type `MenuEntry` ở `lib/menu`. Test: `ContextMenu.test.ts`.
- 14 file dùng component này, nhưng 41 file khác tự viết menu bằng `ccm-menu` (xem phần Pain points).

### B5. SegmentedControl (`components/ui/SegmentedControl.vue`)
- Props: `options (value,label,icon,iconClass)`, `label`, `fill`, `iconOnly`, `disabled` (`:13`). Thumb trượt đo bằng ResizeObserver (`:31`, `:65-66`), nút `h-6 rounded-inner px-2.5 text-xs`, track `ccm-seg`. Dùng ở 36 file. 10 file khác tự viết lại (liệt kê ở Pain points).

### B6. ThemePicker (`components/ui/ThemePicker.vue`)
- `SegmentedControl` chế độ (sáng/tối/hệ thống) + radiogroup lưới `grid-cols-[repeat(auto-fill,minmax(11rem,1fr))]` thẻ theme: swatch mini `h-20` vẽ bằng `th.swatch[0..3]` + màu agent (inline style), sao (galaxy), tên, nhãn sáng/tối, badge "mặc định", mô tả `descKey`, 3 chấm Claude/Codex/Shell. Dưới là hàng "Palette" các theme chỉ-màu (`:108-117`). Phím mũi tên trong radiogroup.
- Dữ liệu: `lib/themes.ts` (danh sách theme, `startTheme`).

### B7. FontPicker (`components/ui/FontPicker.vue`)
- Dòng label + hint + nút `ccm-btn max-w-[55%]` hiện tên font; mở listbox `max-h-72 rounded-inner border bg-surface p-1` (`:100-127`), mỗi option tên + mẫu `sample` bằng chính font đó, "chưa cài" cho font hệ thống thiếu, Check cho đang chọn. `kind = code | ui`. Font bundle offline từ `fonts.css`.

### B8. Dialog (bảng)
| Dialog | File:dòng | Kích thước | Nội dung chính |
|---|---|---|---|
| UpdateDialog | `UpdateDialog.vue:102-112` | `max-w-[520px] max-h-[84vh]`, `py-[8vh]` | "Có bản cập nhật": version, size, ngày publish, release notes, nút Tải và cài / Để sau / Bỏ qua bản này, tiến độ "Đang tải {pct}%", "Khởi động lại để cập nhật", Mở trang / Hiện file |
| WhatsNewDialog | `WhatsNewDialog.vue:109-119` | `max-w-[560px] max-h-[88vh]`, `py-[6vh]` | "Có gì mới" + `Orrery {version}`, mở/thu gọn tất cả, danh sách thay đổi theo version; tự mở 1 lần mỗi version sau QuickSetup và tour |
| ShortcutsDialog | `ShortcutsDialog.vue:71-78` | `max-w-[520px]`, tự cuộn | "Phím tắt": nhóm panels, map, workspace, view, help (`:17-26`), nguồn `lib/shortcuts.ts` (~40 phím workspace), kèm `ToggleSwitch` "Hiện gợi ý phím" |
| RecapDialog | `RecapDialog.vue:181-188` | `max-w-[680px]` | "Tổng kết ngày/tuần": span, token, cost, số session, theo project/model/ngày, vs kỳ trước, chép Markdown, tải .md, CSV session |
| ResourcesDialog | `ResourcesDialog.vue:211-219` | `max-w-[880px] max-h-[90vh]`, `py-[5vh]` | Tài nguyên máy: `SegmentedControl` tab (`:249`), tiến trình, port, v.v. (chưa rõ chi tiết các tab) |
| DevToolsDialog | `DevToolsDialog.vue:107-113` | `w-[min(94vw,720px)] max-h-[min(88vh,760px)]`, `grid place-items-center` | Tab (tablist có gạch chân), tiện ích (UUID, v.v.), `pre` kết quả `max-h-64 bg-fill`; không có Transition `ccm-dialog` |
| BuildCleanDialog | `BuildCleanDialog.vue:173-179` | `h-[min(84vh,720px)] w-[min(94vw,760px)]` | Dọn thư mục build; ẩn khi Disk Clean bật (`App.vue:208`); đóng bị khoá khi `running`; không có Transition `ccm-dialog` |
| DiagnosticsDialog | `DiagnosticsDialog.vue:91-98` | `max-w-[600px]` | "Chẩn đoán": Làm mới, Chép log, Lưu log, trạng thái offline |
| OnboardingTour | `OnboardingTour.vue:198-222` | `z-[60]`, card `w-[360px]` neo vào target hoặc giữa `w-[420px]` | Tour `monitor` (welcome, tabs, palette) và `workspace` (new, split, keys) (`lib/tour.ts:12-21`), lỗ spotlight `box-shadow: 0 0 0 9999px var(--ccm-scrim)`, nút Tiếp/Quay lại/Bỏ qua, "Bước n/total", `ArtImage tour-welcome` |
| QuickSetup | `QuickSetup.vue:212-221` | `max-w-[520px]`, `py-[8vh]` | "Bật những gì bạn cần": thông báo, status line, hook bridge, quyền thư mục; chạy lần đầu (`ui.setupOpen`) |
| SessionToolsSheet | `workspace/SessionToolsSheet.vue` | chưa rõ | Công cụ session theo dự án/pane (`ui.sessionTools`) |

Workspace dialog (pattern lặp lại): `CreateEntryDialog`, `DiscardConfirm`, `LayoutsDialog`, `RunConfigsDialog`, `ApkSizeDialog`, `StringsDialog`, `LibUpdatesDialog`, `ReleaseCheckDialog`, `PermAdvisorDialog`, `EnvCheckDialog`, `ContextSourcesDialog`, `TelegramLinkDialog`, `LocalHistoryDialog`, `TextCompareDialog`, `log/PushDialog`, `log/NewBranchDialog`, `view/CloseConfirmDialog`, `view/RunCloseDialog`, `devices/Device*Dialog`, `extensions/AddSourceSheet`, `InstallSheet`. Cờ mở nằm trong `stores/ui.ts:195-285` (`planBoard`, `devTools`, `envCheck`, `permAdvisor`, `projectSwitcher`, `buildClean`, `sessionTools`, `apkSize`, `stringsTable`, `libUpdates`, `releaseCheck`, `contextSources`, `telegramLink`, `tourOpen`, `setupOpen`, `whatsNewOpen`, `settingsOpen`...).

### B9. Popover
- `HeaderStatusPopover` (340px, A3), `InboxPanel` (400px max), `ConsolePopover` (`workspace/ConsolePopover.vue:11` `WIDTH = 380`, `ccm-menu ccm-pop`, tiêu đề "Lỗi và cảnh báo console", mỗi lỗi có nút "Chèn", footer "Xoá danh sách" + "Chèn tất cả"), `AttentionPopover` (`workspace/AttentionPopover.vue:15` `WIDTH = 288`, "Cần bạn", listbox hàng `rounded-inner`, footer note), `DeviceMenu` (`WIDTH = 236`), `RunBar` menu config `w-64` và device `w-72`, `LogPane` context menu `w-52`.
- Mỗi popover tự tính `left = clamp(anchor.right - W, 12, vw - W - 12)`, `top = anchor.bottom + 6`, `--ccm-origin` cho animation (`ccm-menu` transition `style.css:1564-1580`). 15 file khai báo hằng `WIDTH` riêng.
- Toast: xem A6. Tooltip: dùng `title=` native (1002 chỗ), riêng `workspace/ShortcutTip.vue` và `KeyHint.vue` (nhãn phím nhỏ cạnh nút, bật bằng `settings.keyHints`, phím H).

---

## C. Viewer trong Workspace

Tất cả viewer là pane: `rounded-card bg-surface shadow-card` (Changes, Run, Browser) hoặc `bg-well` (FilePane: `FilePane.vue:1362`). Header 36px (`h-9`) `ccm-material` có `data-pane-head`, hàng phụ `h-8`. Dùng container query `@container` + `@max-[Npx]` để ẩn dần nút, gom vào `PaneMore` (`workspace/PaneMore.vue`).

### C1. FilePane + CodeEditor (`workspace/FilePane.vue` 1890 dòng, `CodeEditor.vue` 446 dòng)
- Toolbar (`FilePane.vue:1363-1446`): icon FileText (màu `agent-file`), tên file, chấm dirty "●" (`text-warning`, "Có thay đổi chưa lưu"), thư mục `dir=rtl` (ẩn dưới 420px), "{n} dòng" (từ `@2xl`), nút diff `+N -M` (`GitCompare`, `@max-[480px]:hidden`), `SegmentedControl` Mã/Chia đôi/Thiết kế cho XML layout (`isXmlPreview`), save/edit/done (Lưu, Sửa, Xong; `@max-[360px]:hidden`), trạng thái "Đang lưu..." / "Đã lưu", `PaneMore`.
- Menu `headMore` (`:1299`): Diff, layout mode, Flutter preview (`fwp`), HTTP panel, md split, Xem trước/Mã nguồn, Xem dạng cây (JSON/YAML...), Ngắt dòng, Tìm (Ctrl+F)...
- Dải thông báo xếp dọc dưới toolbar: tìm (`role=search`, `h-9`), `notice` (`bg-st-waiting-soft`), "Hiển thị không tô màu (tệp lớn)", `editNotice` (error/info), xung đột đĩa `data-file-conflict` (`bg-warning-soft`, nút so sánh), so sánh `data-file-compare`, `OutlinePicker`, `ProblemBar`, live preview note (`:1473-1503`).
- Vùng thân (`:1507+`): `LogFileView` (khi file log), editor CodeMirror 6 (`CodeEditor.vue:4-30`: autocomplete, lint gutter, merge `unifiedMergeView`, search, bracket match; theme `lib/editor/cmTheme`), ảnh (`max-h-full object-contain`), binary/too-large ("Mở bằng app" `ccm-btn`), denied (`AccessPrompt`), `CodeMinimap` (chỉ khi rộng >= `MINIMAP_MIN_WIDTH = 420`, `:91`), `DrawablePreview`, `LayoutPreview`, md split preview, `HttpPanel`, `FlutterPreviewPanel`, `HunkPopup`, `LocalHistoryDialog`, `DiscardConfirm`.
- State: loading ("Đang tải..."), denied, binary, too-large, conflict, formatting overlay, saving, auto save (`PLAN-v62`).
- Phím: Mod+S lưu, Ctrl+H (Win) hoặc Mod+Alt+F (mac) thay thế (`CodeEditor.vue:144-145`); Ctrl/Cmd+F tìm (`FilePane.vue:994`), F12 và Mod+F12 outline/usages, F11 bookmark, Ctrl+E recent, F8 problem kế tiếp, F7 change kế tiếp (`:964-1021`), F4 nhảy source.
- Màu: nền `well` tối cố định cả theme sáng, token `--ccm-syn-*`, `--ccm-selection`.

### C2. DiffView + SplitDiff + HunkPopup (`DiffView.vue` 492, `SplitDiff.vue` 565, `HunkPopup.vue` 186)
- `DiffView` render HTML chuỗi (`v-html`) dạng lưới `max-content max-content 1.5em 1.25em 1fr` (số cũ, số mới, dấu, nút revert, nội dung) (`:371-374`), font `var(--ccm-code-size) - .5px`, `line-height: 1.5`, `tab-size: 4`. Nền tint dùng `--ccm-diff-add-well`/`-del-well` 14%/16%, word diff 34%/38%. Dòng fold dashed, nút "N dòng ẩn". Dòng hiện tại có thanh focus 2px `--ccm-focus`.
- 2 layout: inline hoặc `SplitDiff` hai cột (đồng bộ cuộn `sync`, căn dòng `align`).
- Revert block: nút `.rv` 1.35em; disabled có tooltip `revertBlocked`.

### C3. ChangesPane (`workspace/ChangesPane.vue` 1101 dòng) + CommitBox + ConflictView
- Header (`:708-848`): icon GitCompare, tiêu đề, badge tổng file (`rounded-full bg-fill`), badge chưa xem (`bg-accent-soft`), branch (ẩn dưới `@md`), radiogroup tự viết "Gộp chung / Hai cột" (`:718-731`, `rounded-[5px]`), nút tuỳ chọn mở menu tự viết (`role=menu`, `:748-845`): lệnh (Shift+F7, F7, mũi tên, F4), thu gọn, context 0/3/..., whitespace, highlight, split-view option; `PaneMore`.
- Banner: notice, merge (`bg-st-waiting-soft`, nút Huỷ merge), review bar ("{n} chưa xem" + "Đánh dấu đã xem").
- State rỗng: không phải git repo hoặc lỗi status (icon 26px), "Đang tải...", "Không có thay đổi / Thư mục làm việc khớp với commit gần nhất".
- Thân: danh sách trái (listbox hoặc tree theo `treeMode`, hàng `h-7 rounded-inner text-xs`, `+N` xanh `-N` đỏ) và panel phải `bg-well` có hàng `h-8` đường dẫn, vị trí "change i/n" (`data-diff-position`), nút mở file, stage block, revert; `ImageCompare` cho ảnh, `ConflictView` cho file conflict.
- Phím: mũi tên trái/phải cây, Enter/Space mở dir, ArrowUp/Down danh sách (`:400-433`, `:609-620`).
- `CommitBox` (`CommitBox.vue:64-104`): textarea `min-h-12 max-h-40 rounded-inner border bg-control`, nút `ccm-btn ccm-btn-primary h-6 text-2xs` "Commit {n} file" hoặc "Sửa commit cuối", checkbox native "Amend" (`accent-(--ccm-accent)`), "Huỷ" khi busy, note kết quả có nút đóng `size 11`. Phím Ctrl/Cmd+Enter (`:57`).
- `ConflictView` (`ConflictView.vue:74-127`): header `h-8` ("còn {n} xung đột" hoặc "Không còn dấu xung đột"), nút "Đánh dấu đã giải"; mỗi block có 4 nút `well-btn` (Lấy của mình, Lấy của nhánh, Lấy cả hai, Sửa), 2 hoặc 3 cột (ours, base, theirs; `@xl` và `@3xl`).

### C4. Git Log pane (`workspace/log/*`, tổng 1863 dòng)
- `LogPane.vue:413`: `section.bg-surface` header `h-9` "Log" + `LogRefsToolbar` (menu refs: New branch, So sánh, Favorite, Navigate, Xoá, Mở/thu gọn tất cả, nhóm, tags; row `h-7`), 3 cột: `LogRefsTree` (width `refsW` 140-480, kéo bằng separator `w-1 cursor-col-resize`), giữa là `LogFilterBar` + `LogTable`, phải `LogCommitFiles` + `LogCommitDetails` (width 200-720, chia dọc `split` 0.2-0.8) (`LogPane.vue:64-93`).
- `LogTable`: ảo hoá, `ROW_H = 24` (`LogTable.vue:30`), `LogGraphCell`, phím mũi tên/PageUp/Down/Enter, Ctrl+C chép hash.
- `LogFilterBar` (`h-6`): text, regex, match case, branch, user, date from/to, paths; nút xoá bộ lọc.
- Diff 1 file mở thay thế bảng (`data-log-diff`): nút Back (Esc), đường dẫn + short hash, F7 block kế, `[` `]` file trước/sau (`LogPane.vue:199-209`), nền `bg-well`.
- Banner: so sánh nhánh (`bg-accent-soft`), `branchNotice` (`bg-st-error-soft` nếu fail). Dialog: `PushDialog` (16.3K), `NewBranchDialog`. Fatal: `AccessPrompt` hoặc lỗi.
- Context menu riêng `ccm-menu w-52` (`LogPane.vue:545`, hàng `h-7`), không qua `ContextMenu`.

### C5. Run pane + RunBar + RunLog (`RunPane.vue` 332, `RunBar.vue` 341, `RunLog.vue` 303)
- `RunPane.vue:191`: header `h-9` (cao thêm khi hẹp, `@max-[480px]:flex-wrap`): icon CirclePlay, `RunBar`, version Flutter + chip FVM, hàng trạng thái `h-8` (`data-run-state`: busy/running/error/stopped + progress text), select session khi có >1 phiên live (`ccm-field h-6`), nút capture Android, thanh tiến độ `h-0.5` (determinate theo `step.n/total` hoặc `ccm-shimmer`).
- State: "Đang tìm project có thể chạy...", không chạy được (icon CircleSlash + danh sách project tìm thấy, `data-run-found-project`).
- `RunBar` (`:173`): menu config `w-64`, device `w-72` (`absolute`, `ccm-menu max-h-80`), nút Chạy/Chạy lại, Dừng (`enabled:text-st-error`), Hot reload, Hot restart (disabled "chỉ Flutter"), DevTools, auto reload (menuitemcheckbox), Sửa config, Toolchain, Release check. Hàng menu `MENU_ROW` `h-8 text-sm`.
- `RunLog`: tab Log/Build (tablist tự viết `rounded-[5px]`, đếm lỗi), chip crash (`bg-st-error-soft rounded-full`), select mức log, ô tìm, nút follow (`aria-pressed`) và xoá; panel crash `max-h-80` (exception, caused by, frames, nút "Điền vào agent").

### C6. BrowserPane + BrowserToolbar (`BrowserPane.vue` 505, `BrowserToolbar.vue` 269)
- Toolbar `h-9`: Back, Forward, Stop/Reload, ô địa chỉ `ccm-field h-7` (form `role=search`, favicon 14px hoặc Globe `agent-browser`, thanh load `h-0.5 w-6`), nút device frame, nút handoff gửi console/element picked cho agent (badge số console), `PaneMore`. Esc thoát chế độ pick (`BrowserPane.vue:340`, `BrowserToolbar.vue:90`).
- Stage (`:461`): nền `bg-canvas` khi có device frame (`rounded-control shadow-pop`) hoặc `bg-surface`; `.browser-frame bg-page` (`--ccm-page`).
- State: không có preview "Trình duyệt chỉ có trong ứng dụng desktop" (`:462`), `denied` (Lock 26px + `AccessPrompt`), chưa có URL (Globe 28px + gợi ý), `failure` (overlay `role=alert`, CircleAlert 24px), picking (`bg-accent-soft`), handoffNote (`bg-raised`).
- Chú ý cho thiết kế: nội dung trang là view native của Electron đặt đè lên stage; overlay HTML của renderer không phủ lên trên được và không blur được qua nó (cách app xử lý khi mở overlay: chưa rõ, cần xem `lib/preview.ts` và main process).

### C7. LayoutPreview + FlutterPreviewPanel + ImageCompare + JsonTree + LogFileView
- `LayoutPreview.vue:632` (988 dòng): `bg-well text-well-ink`. Toolbar wrap `min-h-9`: `AppSelect` device (icon Smartphone), nút xoay, `SegmentedControl` theme sáng/tối (icon), `AppSelect` locale (+ input custom `w-24`), theme app, density (+ input `w-16`), nhãn API level, chọn app, chip model (`role=status`, refresh), zoom, blueprint, compare. Banner note model/codeChanged (`role=note`). Setup: checking, thiếu jar, tải bundle (progressbar `h-1.5`, nút `ccm-btn-primary`), thiếu JDK. Canvas `ccm-scroll`; thanh busy `h-0.5` + pill chờ; compare strip nhiều cấu hình; stage khung ảnh PNG.
- `FlutterPreviewPanel.vue:172` (260 dòng): toolbar wrap `min-h-9`: `AppSelect` device, flavor, nút Chạy/Chạy lại/Dừng (`ccm-btn`), trạng thái. Lỗi `role=alert` (`bg-st-error-soft`), không có device (hướng dẫn + làm mới), danh sách widget theo nhóm "ở file này / khác" (icon Eye/Circle/CircleX theo trạng thái, `skipped`), hint.
- `ImageCompare.vue:88` (182 dòng): hàng `h-8`: nhóm chế độ tự viết (side/slider/onion `rounded-[5px]`), range input native `accent-(--ccm-accent)` (slider hoặc opacity `w-28`), kích thước HEAD và "Bản đang sửa"; thân: 2 cột `gap-px bg-well-line` hoặc 1 khung chồng, nền caro `.checker`. Rỗng/lỗi: `ImageOff`.
- `JsonTree.vue:98` (173 dòng): hàng `h-8` ô lọc `ccm-field h-6` + mở/thu gọn tất cả; `ul role=tree` hàng `min-h-6`, font mono, class `tk-*` theo kiểu, nút chép path (Check/Copy 12px), `ROW_LIMIT` cắt ("truncated"), "Đang đọc...", "Không khớp". Phím: Enter/Space, mũi tên, Ctrl+C.
- `LogFileView.vue:231` (318 dòng): toolbar `h-9`: select mức log native `ccm-field h-7`, ô tìm `max-w-72`, spinner, "Nạp phần trước", "{size} mới" (refresh khi không follow), "Theo dõi". Thân `role=log` mono, `ccm-scroll`; rỗng "Log trống."; lỗi + `AccessPrompt`. Esc xoá ô lọc (`:196`).

---

## D. Design primitives hiện có (`style.css` 2085 dòng, `themes.css` 1289 dòng)

### D1. Tailwind `@theme` (`style.css:6-107`)
- Màu (map sang `--ccm-*`): `canvas surface raised line line-strong ink ink-muted ink-faint ink-disabled accent accent-ink accent-soft on-accent link focus selection fill fill-strong control well well-ink well-line`; trạng thái `st-working/waiting/idle/done/error/stale` (+ `-soft` cho working, waiting, error, done); `success warning warning-soft favorite diff-add/del/mod (+ -soft)`; agent `agent-claude agent-claude-ink agent-codex agent-shell agent-shell-mark agent-deepseek agent-browser agent-file`; `peer peer-soft page`; chart `series-input/output/write/read`, `model-1..8`.
- Type: `text-2xs` 12px, `text-xs` 13px, `text-sm` 14px, `text-base` 15px (không có size lớn hơn ngoài `text-xl`, `text-2xl` mặc định Tailwind). Breakpoint thêm `3xl = 110rem`.
- Radius: `--radius-card 12px`, `--radius-control 8px`, `--radius-inner 6px`. Shadow: `shadow-card`, `shadow-control`, `shadow-pop`. Easing: `ease-out-quint`, `ease-spring`, `ease-exit`.
- Font: `--font-sans` (SF Pro / Segoe UI Variable), `--font-mono`.

### D2. Utility `ccm-*` (1 dòng mô tả)
Vỏ và nền:
- `.ccm-material` (`:533`) nền chrome mờ-đặc cho toolbar/sidebar (hiện đặc, `--ccm-material`). `.ccm-titlebar` (`:537`) nền `--ccm-chrome-fill` + vùng drag. `.ccm-material-pop` (`:557`) nền popup. `.ccm-sky` (`:541`) nền canvas có sao.
- `.ccm-scrim` (`:594`) nền phủ modal. `.ccm-sheet` (`:598`) dialog/toast: radius card, nền surface, `shadow-pop`. `.ccm-menu` (`:604`) menu/popover: radius card, nền `--ccm-material-pop`, padding 5px, `shadow-pop`.
- `.ccm-panel` (`:754`) panel nội dung, nền `--ccm-panel-bg`, viền `--ccm-panel-edge`, lớp sao/meteor. `.ccm-sky-well` (`:764`) giếng terminal có sao. `.ccm-chrome-edge` (`:808`) viền chrome `--ccm-chrome-line`. `.ccm-group` (`:915`) nhóm hàng settings kiểu System Settings (radius control, viền 0.5px giữa hàng).
Control:
- `.ccm-btn` (`:611`) nút 28px, radius inner, nền `--ccm-control`, `shadow-control`; `.ccm-btn-primary` (`:645`) nền accent; `.ccm-btn-danger` (`:657`) chữ `--ccm-error`; `.ccm-btn-plain` (`:662`) nút không viền min 28x28, nền `fill` khi hover/pressed. `.ccm-field` (`:731`) input 28px radius inner, focus ring 2px + halo 4.5px. `.ccm-slider` (`:697`) slider macOS. `.ccm-bare-input` (`:930`) input không ring (palette). `.ccm-press` (`:1513`) scale nhấn.
- `.ccm-seg` / `.ccm-seg-thumb` / `.ccm-seg-on` (`:866-875`) segmented: track, thumb, chữ chọn. `.ccm-tab-on` (`:812`) và `.ccm-tab-rail` (`:819`) tab đang chọn và gạch đáy. `.ccm-chip` + `-working/-waiting/-error/-warning/-quiet` (`:823-840`) chip trạng thái, độ đậm theo `--ccm-chip-fill`.
- `.ccm-pane-ring`, `.ccm-pane-active`, `.ccm-pane-head-on`, `.ccm-pane-idle` (`:842-864`) viền và header pane active, mờ pane không active (`--ccm-dim-inactive`). `.ccm-agent-claude/codex/shell/deepseek` (`:877-888`) đặt `--ccm-pane-accent`.
- `.ccm-wordmark` (`:890`), `.ccm-footer` (`:899`), `.ccm-label` (`:908`) nhãn nhỏ (font/case/tracking theo theme).
Khác:
- `.ccm-code` (`:568`) và `.ccm-no-liga` (`:563`) code text không ligature. `.ccm-scroll` (`:1826`) scrollbar mảnh. `.ccm-tabstrip` (`:1798`) dải tab có mờ đầu/cuối. `.ccm-rail-*` (`:1904-2003`) rail sidebar gấp.
- Animation: `ccm-spin`, `ccm-spin-once`, `ccm-enter`, `ccm-pop`, `ccm-flash`, `ccm-attention`, `ccm-node-in`, `ccm-pill-in`, `ccm-glyph-pop`, `ccm-swap-*`, `ccm-dot-*`, `ccm-peer-*`, `ccm-grow(-x)`, `ccm-highlight`, `ccm-shimmer`, `ccm-tick`, `ccm-pulse`, `ccm-quota-*`, `ccm-token-*`, `ccm-list-*`, `ccm-boot-*`.
- Transition Vue: `ccm-view`, `ccm-fade`, `ccm-panel-left/right`, `ccm-sheet-up`, `ccm-kbd`, `ccm-menu` (`:1564`), `ccm-dialog` (`:1584`), `ccm-scrim` (`:1609`), `ccm-sheet` (`:1619`), `ccm-tab`, `ccm-flip`, `ccm-rail-word`.

### D3. Token `--ccm-*` (`style.css:109-330`, mặc định Apple light, có block `dark`)
- Nền và chữ: `canvas surface raised material material-pop well well-ink well-line control`; `ink ink-muted ink-faint ink-disabled`; `line line-strong`; `fill fill-strong`.
- Accent và focus: `accent accent-ink accent-soft on-accent link focus selection`; `mono-ink mono-mix`.
- Elevation: `shadow shadow-control shadow-pop shadow-accent`, `scrim`, `card-shadow`.
- Motion: `dur-press 100ms dur-fast 150ms dur-base 220ms dur-slow 300ms dur-emph 520ms dur-signal 900ms stagger 28ms`.
- Trạng thái: `working waiting idle done error stale` + `-soft`; `success warning warning-soft favorite`; diff `diff-add/del/mod` (+ `-soft`, `-well`).
- Agent và peer: `agent-claude agent-claude-ink agent-codex agent-shell agent-shell-mark agent-deepseek agent-browser agent-file peer peer-soft page`.
- Dữ liệu: `cat-1..8`, `model-1..8`, `grid flow-dot edge`.
- Terminal và code: `ansi-0..15`, `well-selection`, `syn-keyword/string/number/comment/type/func/var`.
- Shape v91 (có default Apple, theme ghi đè): `panel-bg panel-edge chrome-line chrome-fill tab-on-bg tab-on-shadow tab-on-bar tab-rail chip-fill pane-accent pane-ring pane-ring-w pane-ring-mix pane-bar pane-head-tint dim-inactive seg-track seg-thumb seg-thumb-shadow seg-on-ink label-font label-case label-tracking gap-shell footer-h wordmark-font wordmark-display`.
- Sao Ngân hà: `sky sky-size sky-display sky-pane sky-pane-display sky-twinkle sky-twinkle-size sky-twinkle-pane sky-pos sky-rep sky-fill meteor-pane`.

### D4. `themes.css` (16 block, dòng 51-1289)
Theme id: `xcode-light xcode-dark monokai dracula solarized-light solarized-dark github-light github-dark material one-dark islands-light islands-dark orrery-dark orrery-light console galaxy`. Mỗi theme palette đủ 16 lần: `canvas surface raised ink ink-muted ink-faint accent accent-ink on-accent focus selection control well well-ink well-selection working waiting idle done error stale warning favorite peer diff-add/del/mod cat-1..8 ansi-0..15 syn-* agent-*`. Chỉ 4-5 theme (orrery-*, console, galaxy) ghi đè token shape (`panel-*`, `tab-*`, `seg-*`, `pane-*`, `chip-fill`, `card-shadow`, `chrome-line`, `label-*`, `gap-shell`, `wordmark-*`, `dim-inactive`), 5 theme ghi `material`, `line`, `line-strong`. Còn lại dùng default style.css. `lib/themes.test.ts` kiểm contrast.

---

## Must keep (không được mất khi đổi giao diện)

1. 3 bất biến PLAN: không scroll ngang cấp app (`overflow-x-hidden`), mọi chuỗi qua vue-i18n (en, vi), mọi màu qua token theme; trạng thái = icon + chữ + màu, không chỉ màu.
2. Modal a11y: `role=dialog` + `aria-modal`, focus trap, trả focus về nút gọi, Esc đóng, click scrim đóng (trừ BuildClean khi `running`), `retireOverlay` khi leave (để overlay cũ không nhận click).
3. Palette: 4 prefix `> @ # :`, 6 scope, Tab đổi scope, Shift+Enter thao tác phụ, cap 5 mỗi nhóm + "Xem thêm", tìm bỏ dấu tiếng Việt (`fold`), highlight `<mark>`.
4. Header: tablist có mũi tên trái/phải, `data-tour` (`tabs`, `compact`, `palette`, `limits`, `settings`) vì tour định vị bằng các attribute này; `data-*` mà E2E dùng (`data-status-button`, `data-inbox-button`, `data-settings-dialog`, `data-toast-host`...).
5. Compact mode (`?mode=compact`) và titlebar overlay (padding `env(titlebar-area-*)`, `app-region`).
6. Settings: 12 trang + tìm và flash dòng, resize có nhớ kích thước, `ccm.settings.size`.
7. Toast: hold khi hover/focus, Esc, tối đa 3, action có phím.
8. Viewer: code, diff, log, terminal luôn trên giếng tối `well` có token `syn-*`, `ansi-*`, `diff-*-well` (đọc code, tương phản); `WorkspaceView` giữ mounted (xterm state); Browser pane view native Electron; container query ẩn dần nút theo bề rộng pane.
9. Phím viewer: Mod+S, Ctrl/Cmd+F, F7/Shift+F7, F4, F8, F11, F12 và Mod+F12, Ctrl+Enter commit, `[` `]` file diff, Esc đóng diff/pick, Ctrl+P, Ctrl+Alt+P.
10. Reduce Motion: mọi animation compositor-only tắt khi `prefers-reduced-motion`.
11. Rule hiện hành "No blur" 09/11 sẽ bị thay bởi quyết định 3 của v102; bản classic phải giữ nguyên (không có `backdrop-filter` nào hôm nay).

---

## Pain points / inconsistencies

1. **Dialog scaffold lặp 55 lần**: scrim, `Transition ccm-dialog`, `retireOverlay`, `trap()` viết tay trong 25 file; không có component `Dialog`. Lệch nhau: `DevToolsDialog` và `BuildCleanDialog` dùng `grid place-items-center` + `@pointerdown.self` và không có Transition `ccm-dialog`; các dialog khác `flex items-start` + `@mousedown.self`. Lề dọc `py-[5vh]`, `[6vh]`, `[8vh]`, `pt-[12vh]` tuỳ file.
2. **Kích thước dialog tuỳ hứng**: 520, 560, 600, 640, 680, 720, 760, 880, 1040px; chiều cao `84vh`, `88vh`, `90vh`, `min(84vh,720px)`...; chưa có thang size (sm/md/lg).
3. **Popover/menu tự viết**: 41 file dùng `ccm-menu` (55 lần), chỉ 14 file dùng `ContextMenu`. 15 file khai báo hằng `WIDTH` riêng (236, 288, 340, 380, 400...) và tự tính `left/top` giống nhau. Hàng menu có ít nhất 3 phiên bản: `ContextMenu` `ITEM`, `RunBar MENU_ROW` (`h-8 text-sm`), `LogRefsToolbar ROW` và `LogPane` (`h-7 text-xs`), `ChangesPane MENU_ROW`.
4. **Segmented lặp**: 36 file dùng `SegmentedControl` nhưng 10 file tự viết lại với `rounded-[5px] bg-fill p-0.5` (ChangesPane, RunLog, ImageCompare, ExplorerPanel, HttpPanel, NotesPanel, DevToolsDialog, DeviceFilesDialog, ModeBar, SettingsAppearance). Header tab (AppHeader) dùng `ccm-seg` trực tiếp. Tab gạch chân thứ ba ở palette và DevTools.
5. **Radius**: thang 12/8/6 đã có nhưng còn 69 chỗ `rounded-[Npx]` (4px x24, 5px x15, 2px x10, 3px x8, 6px x7, 7px x3, 1px x2) và `.ccm-btn-plain` hard-code `7px`; `rounded-md/lg/xl/sm` còn 21 chỗ. Nút `ccm-btn` radius inner (6) trong khi `ccm-btn-plain` 7 và field 6.
6. **Chiều cao control rời rạc**: `ccm-btn` mặc định 28px nhưng bị đè bằng `h-5/6/7/8` nhiều nơi (h-6 x41, h-7 x39, size-6 x12, size-7 x9...). Header pane 36px (`h-9`), hàng phụ 32px (`h-8`), header app 40px, footer 26px: đúng ý đồ nhưng không có token.
7. **Cỡ chữ ngoài thang**: `text-[11px]` x11, `[9px] [10px] [13px] [15px]`; footer 11.5px, wordmark 13px hard-code.
8. **z-index**: `z-50` x74 (mọi overlay cùng tầng, thứ tự xếp chồng phụ thuộc thứ tự DOM), `z-[60]` tour, `z-10/20/30/40`, lẻ `z-11/12/13`. Chưa có thang.
9. **Màu hard-code**: ít (hex chỉ làm fallback trong `var(--x, #hex)` ở FilePane, DiffView, ChangesPane: `#7f8c98`, `#3f638b`, `#6bdfff`, `#ff6961`); nhưng `bg-on-accent/20` (kbd trên hàng accent), `bg-st-error/25`, `bg-canvas/45` là alpha ngoài token; swatch ThemePicker dùng inline style từ data; `well-btn` định nghĩa scoped trong ChangesPane/ConflictView thay vì token chung.
10. **Hàng đang chọn dùng accent đặc** (`bg-accent text-on-accent`) ở palette, QuickOpen, ProjectSwitcher, ContextMenu (focus) nhưng `bg-accent-soft text-accent-ink` ở Settings nav, ChangesPane, Inbox: hai ngôn ngữ chọn.
11. **Tooltip native**: 1002 `title=`, chỉ `ShortcutTip` và `KeyHint` có UI riêng; không có Tooltip component, không hiện phím tắt thống nhất.
12. **Toggle hai kiểu**: `ToggleSwitch` (23 file) và checkbox native `accent-(--ccm-accent)` (34 file), ví dụ CommitBox amend. Range native ở ImageCompare. Select native (`<select class="ccm-field">`) ở RunLog, LogFileView, RunPane trong khi có `AppSelect` (16 file).
13. **Trạng thái rỗng/lỗi/loading không thống nhất**: chữ trơn `text-xs` (loading), icon 22/24/26/28px + chữ (rỗng/lỗi), `ArtImage` (palette), `LoaderCircle` có lúc `ccm-spin` có lúc `animate-spin`. Banner phụ trong pane dùng 5 màu nền khác nhau (`st-waiting-soft`, `warning-soft`, `st-error-soft`, `accent-soft`, `raised`/`fill`) không có component `Banner`.
14. **Header pane lặp**: FilePane, ChangesPane, RunPane, BrowserToolbar, LogPane, LayoutPreview, FlutterPreview tự dựng thanh `h-9 ccm-material border-b` (hoặc `min-h-9 flex-wrap`), `FilePane` nền `well` còn Changes/Run/Browser nền `surface` + `shadow-card`: không có `PaneHeader` component (có `PanelHeader.vue` nhưng không dùng trong các viewer này; chưa rõ nơi dùng).
15. **PlanLimits 4 biến thể trong 1 file**, popover 340px tự cố định, nhiều animation riêng (`ccm-quota-*`); `MetricsStrip` còn 2 layout.
16. **Chuỗi i18n còn em dash và chấm giữa** trong vi.ts (vd `workspace.quickOpen.placeholder`, `inbox.keys`, `wsFooter.branchChanges`): ngược với GLOSSARY mới (13/11), cần quét lại khi làm bản mới.
17. **Browser pane native view**: không thể blur hay phủ overlay HTML lên trên; ảnh hưởng thiết kế glass (cần quy ước ẩn view hoặc chụp ảnh tĩnh khi mở overlay; cách hiện tại chưa rõ).
18. **Sao và meteor nền**: `.ccm-panel` có `::before` + `background-attachment: fixed` và `isolation: isolate` chỉ cho galaxy (`style.css:776-806`); chồng với backdrop-filter sẽ tốn GPU và đổi stacking context.

---

## Primitive set mà design system mới cần (số biến thể hiện có)

| Primitive | Hiện có | Cần cho bản mới |
|---|---|---|
| Button | 4 class (`ccm-btn`, `-primary`, `-danger`, `-plain`) + cỡ lệch h-5..h-8, `size-5..8`; không có nút `ghost` có viền, `subtle`, `link` | `primary`, `secondary`, `ghost` (plain), `danger`, `icon`; 3 cỡ (sm 24, md 28, lg 32); trạng thái pressed/loading/disabled thống nhất; biến thể trên nền `well` (thay `well-btn`) |
| Input text/search | `ccm-field` (28px) + `ccm-bare-input` + textarea tự viết (CommitBox) + search trong palette | `Input`, `SearchInput`, `Textarea`, cùng focus ring; cỡ sm/md |
| Select | `AppSelect` (16 file), `<select>` native (>= 4 chỗ), FontPicker listbox, ThemePicker | 1 `Select` + `Combobox`, cỡ sm/md |
| Menu / popover | `ContextMenu` + 41 file tự viết, 5 chiều rộng khác nhau, 4 kiểu hàng | 1 `Popover` (anchor, placement, clamp, `--ccm-origin`), 1 `Menu` (item, checkbox, radio, submenu, kbd, separator), thang width (sm 236, md 288, lg 340, xl 400) |
| Dialog | `ccm-sheet` x55, 9 chiều rộng, 2 kiểu căn dọc | 1 `Dialog` (size sm 420, md 520, lg 680, xl 880, full cho Settings), có `Header`, `Body`, `Footer`, focus trap chung, `ConfirmDialog` |
| Command overlay | Palette, QuickOpen, ProjectSwitcher (cùng khung 600-640px) | 1 `CommandSurface` + `ResultRow` (icon, nhãn highlight, sub, trailing, kbd), `ScopeTabs`, `FooterHints` |
| Chip / badge | `ccm-chip` + 5 tone, badge `rounded-full bg-fill`, `bg-accent-soft`, badge số trên icon | `Chip` (tone: neutral, accent, working, waiting, error, warning, success), `Badge` (count, dot) |
| Tabs | 3 kiểu: segmented header, gạch chân (palette, DevTools), tab strip workspace (`ccm-tabstrip`) | `Tabs` underline, `Segmented` (đã có, 10 bản copy cần thay), `TabStrip` |
| Segmented | `SegmentedControl` + `ccm-seg` + 10 bản tự viết | 1 `Segmented` (text, icon-only, fill), dùng cả cho header |
| Toggle | `ToggleSwitch` 38x22 + checkbox native x34 | `Switch`, `Checkbox`, `Radio` cùng style, `Slider` (đã có `ccm-slider`) |
| Tooltip | `title=` native x1002, `ShortcutTip`, `KeyHint` | `Tooltip` (delay, kbd tự hiện từ `lib/shortcuts`), giữ `KeyHint` |
| Toast | `ToastHost` (5 kind, 3 layout) | `Toast` (info, warning, undo, success, error) + action, countdown; vị trí bottom-right giữ |
| Banner / notice | 5 màu nền ad-hoc trong pane | `Banner` (info, warning, error, accent) dạng strip trong pane |
| Pane header / toolbar | 7 bản tự viết `h-9` + container query + `PaneMore` | `PaneHeader` (leading icon, title, meta, actions, overflow), `Toolbar`, `Divider` |
| List row | `h-7 rounded-inner` (Changes, Log), `min-h-9` (Inbox), `min-h-11` (palette) | `ListRow` 3 mật độ (compact 24-28, normal 36, comfy 44), trạng thái selected/active/disabled |
| Empty / loading / error | rải rác, icon 22-28px | `EmptyState`, `Spinner`, `Skeleton`, `InlineError` |
| Group / form row | `ccm-group`, `SettingRow`, `SettingSection` | giữ, đổi nền kính |
| Code well | `bg-well`, `.ccm-code`, diff tint | giữ giếng tối cố định, token riêng cho kính (viền, bo) |
| Surfaces | `ccm-material`, `ccm-sheet`, `ccm-menu`, `ccm-panel`, `ccm-sky-well` | 3 bề mặt: `glass-chrome` (header, footer, rail), `glass-overlay` (menu, dialog, toast, palette, có blur), `solid` (nội dung và code); token `blur`, `saturate`, `border-glass`, `shadow-glass` |
| Tokens | màu x~110, radius x3, shadow x3, motion x7, shape v91 x~35 | thêm thang z-index, size dialog, size control (24/28/32), cỡ chữ (11, 12, 13, 14, 15, 20, 24), mật độ |
