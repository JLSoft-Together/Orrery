# UI inventory 04: Settings, Devices, Extensions, Disk (+ click menu v100)

Đọc working tree ngày 15/11/2026, nhánh `feat_clean_memory` (có phần chưa commit). Chỉ đọc, không sửa code.
Đường dẫn gốc: `apps/web/src/components/` (viết tắt `C/`). Label vi lấy từ `apps/web/src/i18n/*.ts` (resolve từ `vi.ts`).
Cờ: [WIP] = đang làm trong working tree (chưa commit, theo `.planning/PROGRESS-2026-11-15-click-disk.md` và `.planning/wp/WP-*.md`). [ĐÃ COMMIT] = đã vào lịch sử.

## 0. Tổng quan nhanh

| Màn | Hình thức | File gốc | Ghi chú |
|---|---|---|---|
| Settings | Dialog modal giữa màn, 1040x760 mặc định, kéo resize, min 560x420 | `C/SettingsMenu.vue:296-307`, `DEFAULT_SIZE` dòng 48 | Nút mở = icon Settings2 32px ở header (`:279`), phím Ctrl/Cmd+`,` (`:222`) |
| Devices | View toàn trang (`ui.view='devices'`) | `C/devices/DevicesView.vue:61` | max-w 1560, padding 16/24px |
| Extensions | View toàn trang | `C/extensions/ExtensionsView.vue` | cùng khung `main` |
| Disk | View toàn trang: 4 tab ĐÃ COMMIT + Dọn dẹp (một phần WIP) + File trùng [WIP] + Sắp xếp [WIP, chưa gắn] | `C/disk/DiskView.vue:73` | Nút header `diskMap.nav` = "Ổ đĩa" (`AppHeader.vue:52`) |
| QuickSetup | Dialog 520px | `C/QuickSetup.vue` | mở từ Settings, Cập nhật & trợ giúp |
| DiskBadge | Popover teleport, neo vào `HeaderStatusPopover` | `C/DiskBadge.vue` | cảnh báo ổ sắp đầy (v77) |

Khung chung của 3 view: `<main class="mx-auto w-full max-w-[1560px] flex-1 px-4 py-5 sm:px-6 sm:py-6">`, tiêu đề `h2 text-2xl font-semibold`, mô tả `text-sm text-ink-muted max-w-[80ch]` ngay dưới, nút hành động chính căn phải trên cùng hàng (`flex-wrap items-end justify-between`). Utility dùng nhiều: `ccm-group` (nhóm dòng, viền 0.5px giữa các dòng), `ccm-panel`, `ccm-btn` / `ccm-btn-primary` / `ccm-btn-danger` / `ccm-btn-plain`, `ccm-field` (input/select), `ccm-chip`, `SegmentedControl`, `ToggleSwitch`, `ccm-sheet` (dialog), `ccm-menu` (popover).

---

## 1. Settings

### 1.1 Layout (dialog)

```
 scrim (fixed inset-0, p-4)
 |  ccm-sheet 1040x760 (min(…, 100vw-32px)), bg-canvas, radius-card      |
 |  +-----------------+--------------------------------------------------+
 |  | sidebar w=224px | header 44px: h3 tên page (truncate)         [X]  |
 |  | bg-fill p=12    +--------------------------------------------------+
 |  | [Tìm cài đặt]   | content: ccm-scroll, px=20 pt=16 pb=24           |
 |  | nav 12 nút      |   SettingSection: tiêu đề 11px (px=12)           |
 |  |  icon15 + label |   ccm-group (row 36-44px, px=12 py=8)            |
 |  |  active =       |   ...                                            |
 |  |  bg-accent-soft | cuộn dọc, không cuộn ngang                       |
 |  | "Lưu trên máy   |                                                  |
 |  |  này."          |                                       [grip]     |
 |  +-----------------+--------------------------------------------------+
```

- Container query `@container`, breakpoint 720px: dưới 720 sidebar thành hàng nút wrap phía trên (`SettingsMenu.vue:310-311`), từ 720 sidebar dọc rộng 224px (`w-56`) có border phải.
- Nút nav: `rounded-control px-10 py-6 text-sm`, icon lucide 15px, active `bg-accent-soft text-accent-ink font-medium`, hover `bg-fill-strong` (`:335-336`). Icon theo page (`:113-126`).
- Grip resize 20px góc dưới phải (`:392-400`), kéo đối xứng, double click reset, phím mũi tên chỉnh cỡ. Cỡ lưu `ccm.settings.size`, page cuối lưu `ccm.settings.page`.
- Focus trap Tab trong panel (`:205-219`). Esc: xóa ô tìm nếu đang focus ô tìm, không thì đóng. Mở thì focus vào nút nav đang active.
- Chuông thông báo (`:265-277`) render cùng component này, cạnh nút Settings ở header (Bell / BellOff, Lottie `notification-bell` khi bật). Nút Settings có chấm tròn accent ở góc khi `update.available` (`:290`).

### 1.2 Tìm kiếm

- Ô `ccm-field` ở đầu sidebar, placeholder "Tìm cài đặt", icon Search 14px (`:312-326`). Enter mở kết quả đầu.
- Nguồn: bảng phẳng `entries(mac)` trong `C/settings/index.ts:81-173` (page, id, labelKey, hintKey, needs). Khớp mọi từ trong label hoặc hint. Xếp hạng: từ bắt đầu label, rồi trong label, rồi chỉ hint (`:203-205`). Gõ có dấu thì khớp đúng dấu trước, không có kết quả mới bỏ dấu (`searchSettings`, `:196-224`).
- Khi đang tìm: header đổi thành "Kết quả tìm kiếm" (`settingsV79.results`), nav mất trạng thái active, content là `ul.ccm-group`, mỗi dòng `Tên page > Tên cài đặt` (ChevronRight). Trống: "Không có cài đặt nào khớp "{q}"." (`:372`).
- Bấm kết quả: `openHit` chuyển page, scroll tới `[data-setting-row=id]`, gắn class `ccm-setting-flash` 1.4s (`:172-189`). Với `disk` và `phone` còn tự bấm mở khối "Cảnh báo ổ đĩa và điện thoại" đang thu gọn (`:174-177`).
- Hàng bị ẩn theo `SettingsEnv` (21 cờ: terminals, mac, loginItem, zoom, awake, github, run, capture, agentData, phone, telegram, disk, updater, backup, grants, statusLine, toast, notifyData, deepseek, xmlPreview, bugReport). Page ẩn bằng `PAGE_NEEDS` (editor = terminals, github, deepseek, telegram).
- `diskMap` (mục Ổ đĩa trong Dữ liệu) nằm trong `entries` nhưng không có cờ env, hiện theo `hasDiskMap` ở `SettingsData.vue:18`. `clickMenu`, `menuToast` nằm page `editor` (`index.ts:67-68`).

### 1.3 Cây section -> row

Loại row: [T] toggle (`ToggleSwitch` 38x22, label + hint 11px + icon info), [S] select, [G] segmented (`SegmentedControl`), [N] number / stepper, [B] button, [L] list, [E] embedded panel (component riêng, bọc `SettingEmbed`).
Row thường: `SettingRow` `px-3 py-2`, label trái, control phải. Biến thể `stacked`: control xuống dòng dưới, full width.

**Page "Chung"** (`settingsV79.pages.general`, `SettingsGeneral.vue`)
- (không tiêu đề) Ngôn ngữ [G] Tiếng Việt / English
- Office view [T] (kèm `OfficeNeedsNode` inline nếu thiếu node)
- Hiện gợi ý phím trên nút [T]
- Mẹo phím tắt [T] (cần terminals)
- Mở session bằng [G] Terminal / iTerm (chỉ macOS)
- Khởi động (`settings.section.startup`): Khi mở app [G stacked fill] Hỏi / Tự mở lại / Không làm gì. Chạy cùng Windows (macOS: "Chạy cùng macOS") [T]
- [E] AwakeSettings "Giữ máy tính không ngủ" [G stacked] + cảnh báo pin (BatteryWarning)

**Page "Giao diện"** (`appearance`, `SettingsAppearance.vue`)
- Giao diện (`settings.theme`): [E] `ThemePicker` = [G] chế độ (nhãn `themeV96.mode` cũng là "Giao diện") + lưới thẻ theme `grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-8`. Mỗi thẻ: preview swatch cao 80px (mini cửa sổ có header, rail, 2 pane, footer, sao nếu theme có), tên, nhãn sáng/tối, chip "Mặc định", mô tả, 3 chấm màu agent Claude / Codex / Shell. Dưới lưới: nhãn "Theme chỉ đổi màu" + hàng chấm palette (`ui/ThemePicker.vue`)
- Hiện sao [T]. Sao trong panel [T]. Hiệu ứng sao [G] Tắt / Nhấp nháy / Nhấp nháy + sao băng. Ba hàng này disabled khi theme không phải Ngân hà.
- Chữ (`fonts.section`): Font giao diện [FontPicker]. Font code [FontPicker]. Cỡ chữ code [N stepper - px +]. Thu phóng giao diện [N stepper %, 58-173%, chỉ desktop]
- (không tiêu đề, cần terminals): Icon tệp [G] Atom / Simple. Hiệu ứng tan biến khi đóng [T]

**Page "Trình soạn thảo & Terminal"** (`editor`, `SettingsEditor.vue`, một section)
- Tự động lưu [G stacked fill] Tắt / Chờ / Rời pane / Rời app, + [G] độ trễ giây (chỉ khi "Chờ")
- Minimap code [T]. Làm mờ pane không dùng [T]. Đánh dấu lệnh trong shell [T]
- Công cụ Orrery cho claude / codex gõ trong shell [T]. Compact tự gửi /compact và dòng làm tiếp [T]
- [WIP v100] Thanh nút cho menu của agent [T] (`settings.clickMenu`, `SettingsEditor.vue:58-65`). Toast có nút đáp án [T] (`settings.menuToast`, `:66-73`). Plan v102 gọi "Settings → Workspace switch" nhưng trong code hai toggle nằm cuối page này, chưa có page Workspace riêng. `SettingsEditor.vue` và `index.ts` đều đang ở trạng thái đã commit (04e390e1, ce9a9709); `index.ts` không có trong `git status` đầu phiên.

**Page "Thông báo"** (`notify`, `SettingsNotify.vue`)
- Kênh thông báo: Báo hiệu hệ thống [T] + ghi chú lỗi `health` (bg st-error/10). Hàng "Gửi thử" [B] + dòng trạng thái. Thông báo trên máy [T] (disabled / ghi chú khi trình duyệt chặn hoặc không hỗ trợ)
- Sự kiện nào báo cho chủ nhân: [L] bảng 3 cột `grid-cols-[1fr_5.5rem_5.5rem]` (Sự kiện / Cảnh báo hệ thống / Thông báo), mỗi ô là checkbox thô. 10 dòng: Session đang chờ chủ nhân, Session đã xong, Tác vụ nền bị chặn, Session lỗi liên tục, Session có vẻ bị kẹt, Context quá 95%, Sắp hết giới hạn gói hoặc ngân sách, Session và server bị bỏ quên, File nhạy cảm hoặc lệnh rủi ro, Ổ đĩa gần đầy (`NOTIFY_ROWS`, `index.ts:62-73`)
- Âm thanh và giờ yên lặng: Phát âm báo [T]. Giờ yên lặng [2 input time từ-đến + nút]. Coi là bỏ quên sau [S native select: Tắt / phút / giờ]
- Khối thu gọn "Cảnh báo ổ đĩa và điện thoại" (nút "Cài đặt…" / "Thu gọn", `aria-controls=notify-more`): [E] `DiskSettings` "Cảnh báo khi ổ sắp đầy" [G fill ngưỡng GB]. [E] `PhoneSettings` (Điện thoại: bật [T], kênh [G] ntfy / Discord, link Play Store / App Store, server + topic [input + ẩn/hiện + tạo + chép + QR], webhook, Lưu / Xóa / Gửi thử, idle [G], ghi chú riêng tư)

**Page "Telegram"** (`telegram`, `C/TelegramSettings.vue`, 1556 dòng, [E], cần `hasTelegram`)
- Thẻ tổng quan: icon Send, `@bot` hoặc "chưa có bot", dòng tóm tắt, 5 chip bước (Bot, Group, Topic, Đã ghép, Nhận lệnh; xanh khi xong, bấm nhảy tab). Banner lỗi `pollConflict` + nút "Thử lại".
- Công tắc bật + [G fill] 4 tab: Kết nối, Topic, Nhận lệnh, Báo cáo.
  - Kết nối: token bot (form + Lưu / Xóa có xác nhận inline có/không / Gửi thử), file lớn (toggle + api id/hash), danh sách group (quét, admin / không admin).
  - Topic: cây group, topic, route project (lọc, chip, kéo thả, thu gọn, tạo / sửa / xóa, link session, trạng thái gone / broken / error).
  - Nhận lệnh: ghép tài khoản, rule nhận lệnh vào pane.
  - Báo cáo [T]: Gồm câu trả lời, Báo khi agent chờ chủ nhân / xin quyền, Gom báo xong, Cảnh báo hạn mức / ổ đầy / build lỗi, Giờ yên lặng, Tóm tắt ngày, Ghim trạng thái, Icon topic.

**Page "Nhắc nhở & ngân sách"** (`reminders`)
- Nhắc nghỉ giải lao [T] (hint động có giờ nghỉ kế). Báo kẹt sau [G stacked fill, phút / Tắt]. Cảnh báo context [N input, đơn vị "k token", rộng 96px]. Ngân sách ngày [N input "$", "mỗi ngày"]

**Page "Tài khoản GitHub"** (`github`, `C/GitHubAccountsSettings.vue`, [E])
- [L] tài khoản (login, chip "Mặc định", scope, nút Đặt mặc định, nút xóa) hoặc dòng trống. Form token (input + nút Thêm primary + trạng thái đang kiểm). Link "Tạo token". Hint cuối.

**Page "Tích hợp"** (`integrations`)
- Hiện giới hạn gói trên header [E inline]: tiêu đề + trạng thái cầu nối (xanh / vàng), khối `<pre>` JSON max-h 144px + nút chép.
- Chạy `RunSdkSettings` [E]: SDK path chọn / đặt lại, Auto compile [T].
- XML Preview `XmlPreviewSettings` [E]: trạng thái, phiên bản, dung lượng, Cài / Cập nhật / Gỡ, tiến trình tải, gợi ý Java.
- Ảnh chụp và video quay `CaptureSettings` [E]: thư mục [B chọn / mở / xóa], gửi như file [T], giữ [G], cỡ tối đa [G].

**Page "DeepSeek"** (`deepseek`, `SettingsDeepSeek.vue` 370 dòng; ẩn mặc định, chỉ bật với `CCM_DEEPSEEK_FEATURE=1`)
- Giới thiệu + các bước. DeepSeek CLI (Codewhale): trạng thái cài, nút Làm mới / Kiểm cập nhật / Cập nhật / Tải / Cài. Đăng nhập. API key [input + lưu]. Đăng xuất [B]. Gửi số liệu sử dụng [T]. Chép rule, skill và MCP từ Claude [B].

**Page "Quyền & bảo mật"** (`access`)
- [E] `SensitiveSettings` "Canh nhạy cảm": [T] bật, danh sách glob [L input + Thêm + xóa], danh sách bỏ qua [L + bỏ bỏ qua].
- Quyền truy cập tệp: Mở tệp ở mọi thư mục [T]. Quên các thư mục đã cho phép [B dạng dòng, đổi thành tick xanh 4 giây].

**Page "Dữ liệu & sao lưu"** (`data`)
- [E] `AgentDataSettings` "Dữ liệu agent" (304 dòng): đo dung lượng, danh sách nhóm + dung lượng, lọc tuổi [G], Dọn, di chuyển + Undo, xóa bản gốc, dải "strip".
- [E] `BackupSettings` "Sao lưu": Export / Import [B], xem trước từng mục có checkbox, Hủy / Áp dụng primary, nhắc token GitHub, hint tự sao lưu.
- [E] `settings/SettingsDiskMap.vue` "Ổ đĩa" (v101): title, hint, nút "Mở Ổ đĩa", danh sách ổ `DiskVolumeRow` (thanh dùng 6px), dòng trạng thái "Chưa có index / Đang quét / Quét đang tạm dừng / Đã index, N mục", "Mức quét" [G fill Nhẹ / Cân bằng / Tối đa]. Nằm ở khối thứ ba của "Dữ liệu & sao lưu", không có page "Ổ đĩa" riêng như COORD mô tả. Bản đầu đã commit (42f9788d).

**Page "Cập nhật & trợ giúp"** (`about`)
- [E] `UpdateSettings`: phiên bản hiện tại, Kiểm tra / Xem bản mới [B], Tự kiểm tra [T], gợi ý dev.
- Trợ giúp: 8 dòng nút (icon + label + hint + chevron): Có gì mới, Xem hướng dẫn, Hướng dẫn Workspace, Chẩn đoán, Thiết lập nhanh (QuickSetup), Giấy phép font, Giấy phép icon tệp, Giấy phép văn phòng. Bấm thì đóng Settings rồi mở overlay tương ứng (`openFrom`).
- [E] `BugReportSettings` "Báo lỗi": danh sách log + tổng dung lượng, Gồm dump [T], Nhắc báo lỗi [T], nút Lưu báo cáo.

### 1.4 Rows redesign phải thêm

| Hàng | Vị trí đề xuất | Dữ liệu | Ghi chú |
|---|---|---|---|
| Giao diện mới (UI next gen) `ccm.uiGeneration` classic / next | Page "Giao diện", đầu page, trước ThemePicker | [G] 2 lựa chọn, hoặc nút "Thử giao diện mới (UI next gen)" / "Về giao diện Orrery cũ" (quyết định #7: đổi ngay, không restart) | Chưa có trong code. Từ "Giao diện" đã dùng 3 lần: tên page, tiêu đề section `settings.theme`, nhãn segmented `themeV96.mode`. Nên đổi tên để khỏi nhầm |
| Chọn palette UI mới `ccm.nextTheme` | Thay lưới thẻ `ThemePicker` khi `data-ui=next` | 9 palette: Sương kính (sáng + tối, mặc định), Tinh vân, Cực quang, Than chì, Bình minh sương (sáng + tối), Ngân hà (tối), Orrery (sáng + tối), Console (tối) | Giao diện cũ giữ nguyên bộ theme hiện tại; hai danh sách lưu khóa riêng |
| Giảm trong suốt | Page "Giao diện" | [T], đi cùng `prefers-reduced-transparency` | Plan nhắc ở mục Token |
| Entry mới trong `entries()` | `index.ts` | thêm id để tìm kiếm được | `index.test.ts` kiểm danh sách |

### 1.5 Pain points Settings

- 12 page, sidebar dài. Page "Dữ liệu & sao lưu" và "Tích hợp" gom nhiều panel nhúng khác phong cách (mỗi panel tự có `h3` 11px riêng, khác `SettingSection`).
- `TelegramSettings` 88.9KB, tab nội bộ lồng trong dialog, không dùng `SettingRow`.
- Khối "Cảnh báo ổ đĩa và điện thoại" phải bấm mới mở, tìm kiếm phải tự bấm hộ (`flash`).
- Hai kiểu hint: `info` (icon `i` hover) và `hint` (dòng 11px dưới label) trộn lẫn.
- Bảng sự kiện thông báo dùng checkbox thô, khác `ToggleSwitch`.
- Page DeepSeek ẩn mặc định nhưng vẫn nằm trong `SETTINGS_PAGES`.
- `clickMenu` nằm ở "Trình soạn thảo & Terminal", plan gọi "Workspace".

---

## 2. Devices (`C/devices/`)

### 2.1 Layout

```
 main max-w=1560 px=24 py=24
 +------------------------------------------------------------------+
 | Thiết bị (h2 24px)                              [+ Thêm thiết bị]|
 | Điện thoại và máy tính bảng Android mà adb kết nối được...       |
 +------------------------------------------------------------------+
 | DeviceConnectBar (hiện khi bấm Thêm; rounded-card border p=12)   |
 |  [192.168.1.5:5555____] [Ghép nối bằng mã] [mã 6 số] [Kết nối]   |
 |  [Ghép nối bằng mã QR] [Tìm thiết bị] [Kết nối thiết bị đã lưu]  |
 |  [toggle Tự kết nối khi mở app]   (QR popover top-full right-0)  |
 +------------------------------------------------------------------+
 | cảnh báo lỗi adb (warning, nút X)                                |
 | [Đã chọn 2]  [Kết nối][Ngắt kết nối][Quên (danger)] [X] accent-soft
 +------------------------------------------------------------------+
 | [ ] | Trạng thái | Tên | Serial / địa chỉ | Kết nối   (11px)     |
 | ccm-group list:                                                  |
 | [ ] Sẵn sàng  Pixel 7 Pro  2A021F…  [USB]  [Chụp][Thử UI][Dữ liệu app][Hiệu năng][File][Chuyển sang Wi-Fi]
 | [ ] Ngoại tuyến  Galaxy A33  192.168.1.14:5555  [Wi-Fi]  [Kết nối lại][Quên]
 +------------------------------------------------------------------+
 | Emulator (h3)  N                                    [Làm mới]    |
 |  Pixel_6_API_34   Đang chạy  emulator-5554   [Tắt]               |
 |  Pixel_8_API_35   Đã tắt                     [Chạy][Cold boot][Xóa dữ liệu]
 +------------------------------------------------------------------+
```

### 2.2 Component, file:line

| Phần | File | Ghi chú |
|---|---|---|
| Khung + header | `DevicesView.vue:61-79` | nút Thêm chỉ khi `store.adbFound`; primary khi chưa mở, thường khi đang mở |
| Thiếu adb | `:81-87` | banner `bg-warning-soft` + nút "Mở Cài đặt" (nhảy Settings, Tích hợp) |
| Lỗi chung | `:92-101` | `store.lastError` + detail mono 11px, 3 dòng |
| Batch toolbar | `:103-143` | role toolbar, chỉ khi có chọn; Kết nối / Ngắt / Quên theo loại thiết bị chọn |
| Danh sách | `:154-174`, `DeviceRow.vue` | lưới `ROW_GRID` 6 cột từ md (`meta.ts:23`); dưới md thành flex wrap |
| Emulator | `DeviceEmulators.vue`, mount `:176` | chỉ khi `hasEmulators` |
| 5 dialog công cụ | `:178-182` | Capture, UiTest, AppData, Perf, Files |

### 2.3 Elements, data, states

- Row (`DeviceRow.vue`): checkbox, trạng thái (icon + chữ, màu theo `STATE_META`: ready = success, unauthorized = warning, offline = muted; spinner khi `busy`), tên (double click hoặc nút bút chì để đổi alias, tối đa 64 ký tự, Esc hủy), model mờ khi có alias, serial / địa chỉ mono 11px, chip transport (USB / Wi-Fi / Máy ảo), nút công cụ (chữ từ breakpoint lg, dưới đó chỉ icon), nút hành động theo `rowActions()` (`meta.ts:19-23`): Chuyển sang Wi-Fi / Ngắt kết nối / Kết nối lại / Quên. Click cả dòng = chọn.
- Trạng thái adb (`devices.state.*`): Sẵn sàng, Chưa cho phép (hint "Bấm Cho phép ở hộp thoại gỡ lỗi USB trên điện thoại"), Ngoại tuyến, Đang xác thực, Recovery, Sideload, Bootloader, Thiếu quyền, Không rõ.
- Dữ liệu: `stores/devices` (adb SDK qua desktop bridge, `lib/devices.ts`). Thiết bị Wi-Fi đã nhớ vẫn hiện khi offline.
- States: chưa tải (không render list); rỗng (`data-dev-empty`: icon Smartphone 22px, "Cắm điện thoại qua USB hoặc nhập địa chỉ Wi-Fi của máy", nút Thêm); không adb; có lỗi; đang kết nối (spinner trong nút); `partialCode` ("Nhập đủ 6 chữ số"); sau ghép nối ("Đã ghép nối. Giờ nhập địa chỉ hiện ở mục Gỡ lỗi không dây rồi kết nối.").
- Nhiều item: không phân trang, list `ccm-group` cuộn theo trang; header cột có select all với trạng thái indeterminate.
- Dialog công cụ (đều `ccm-sheet` + scrim):
  - Chụp màn hình (`DeviceCaptureDialog.vue`, 1100 x 90vh): chụp lại / quay video, inspect UI (panel phải 288px), a11y audit (panel phải 320px), đính vào session.
  - Thử UI (`DeviceUiTestDialog.vue`, 640px): density, font scale, dark, locale, tắt animation, mở launcher / URI / package, khôi phục.
  - Dữ liệu app (`DeviceAppDataDialog.vue`, 1080 x 86vh): chọn package, danh sách file db / prefs, bảng SQLite (chọn table, query, phân trang), SharedPreferences sửa / khôi phục.
  - Hiệu năng (`DevicePerfDialog.vue`, 920px): đo cold start / frames, lịch sử, so sánh với lần trước (select), reset.
  - File (`DeviceFilesDialog.vue`, 980 x 86vh): breadcrumb mono, cột Tên / Sửa lúc / Size, push / pull, thư mục mới, xóa, thả file vào.
- Destructive + confirm: Quên thiết bị = `ccm-btn-danger` (batch) hoặc nút hàng, không confirm riêng (chỉ xóa bản nhớ). Emulator "Xóa dữ liệu" (wipe): native confirm ở main trước (`lib/devices.ts:65`, hint `emulators.action.wipeHint` ghi "có hỏi trước"). Xóa file trên máy: "Hỏi trước khi xoá" (`devFiles.deleteHint`). Sửa SharedPreferences: native confirm hiện thay đổi, giữ bản gốc đầu để restore (`lib/devices.ts:90`).

### 2.4 Pain points

- Emulator nằm dưới danh sách, không có điều hướng; một trang chứa cả kết nối, danh sách, 5 công cụ, emulator.
- Hai nơi chỉnh adb (banner + Settings, Tích hợp, Chạy).
- Header cột `devices.col.*` chỉ hiện từ md; dưới md nhãn gộp thành "Danh sách · N".

---

## 3. Extensions (`C/extensions/`)

### 3.1 Layout

```
 main
 | Tiện ích mở rộng (h2)       [Tiện ích | Hook] (SegmentedControl)  [+ Thêm nguồn]
 | Cài MCP server, plugin và skill vào Claude Code, cho mọi project hoặc một project...
 | [Cài cho: Mọi project | Một project] [project ▾] [Chọn thư mục…] path-mono   [Đã kiểm tra server 3 phút trước][Làm mới]
 |
 | MCP server (h3 16px) + hint 12px
 |  grid auto-fill minmax(260px,1fr) gap=12:
 |  +--------------------+ +--------------------+
 |  | [icon36] Figma     | | GitHub             |   ExtensionCard: rounded-card bg-surface p=16 shadow-card
 |  |  vendor 11px       | |                    |
 |  | mô tả 2 dòng       | |                    |
 |  | Đã kết nối [Toàn cục] chip scope          |
 |  | [Cài / Đăng nhập]            [⋮]          |
 |  +--------------------+ +--------------------+
 | Plugin & skill (cùng lưới)
 | Đã cài khác: grid 3fr / 2fr (lg):
 |   trái ccm-group: dòng server (icon brand, tên mono, trạng thái, scope chip, [đăng nhập][xóa]) + dòng plugin
 |   phải: nhóm skill cá nhân (~/.claude/skills) / của project; mỗi dòng sparkle + tên mono + [chép đường dẫn]
 |
 | (tab Hook) HooksPanel: ô lọc 224px + Làm mới; nhóm theo event; mỗi hook: checkbox / chip Orrery / khóa plugin,
 |  matcher mono, command mono, async, timeout, nút Test
```

### 3.2 Components, data

- `ExtensionsView.vue` (header `:3-12`, toolbar `:15-47`, no-project `:49-54`, hooks `:56`, 3 section `:65-215`), `ExtensionCard.vue`, `InstallSheet.vue` (dialog 540px), `AddSourceSheet.vue` (dialog 540px), `HooksPanel.vue`, `ScopeRadio.vue`, `meta.ts`, `brandIcons.ts`.
- Dữ liệu: `CATALOG` trong `packages/shared/src/extensions.ts` (MCP: Figma, GitHub, GitLab, Discord, Meshy AI, Unity, Blender, Godot, Godot Editor, Funplay MCP for Godot; plugin: PM skills, UI/UX Pro Max, Frontend Design, Unity plugin) + trạng thái từ `claude` CLI (`stores/extensions`). `lib/extensions.ts:48 others()` gom mục ngoài catalog.
- Trạng thái thẻ (`ext.state.*`): Đã kết nối, Cần đăng nhập, Kết nối lỗi, Chờ phê duyệt, Đã cấu hình, Chưa cài, Đã cài, Cài từ trong app, Đang kiểm tra… (spinner). Cảnh báo `prereqMissing` (TriangleAlert vàng). Kết quả hành động hiện inline dưới thẻ (xanh / vàng / mờ khi cancelled).
- Menu `⋮` (`ContextMenu`): Cài, Gỡ, Đăng nhập, Tài liệu, Quản lý, Các bước cài.
- States: chế độ "Một project" chưa chọn project (card trống + "Chọn thư mục…"); không có CLI (banner vàng "Không tìm thấy Claude Code CLI"); tải lỗi (`ext.loadFailed`); đang làm mới (`mcpLoading`); "Không có gì khác được cài ở đây."; skill rỗng.
- Destructive + confirm: Cài / Gỡ chỉ chạy qua `claude` CLI sau native confirm hiện đúng lệnh (`ext.sheet.confirmNote`: "Hộp thoại hệ thống hiện đúng lệnh trước khi chạy"). Nút Trash trên dòng "đã cài khác" cũng đi qua `ext.remove`. Hook: checkbox bật / tắt ghi `settings.json` ngay (hook plugin bị khóa); Test chạy lệnh thật và hiện hộp kết quả.

### 3.3 Pain points

- Header có 3 hàng điều khiển (Section, Scope, Project); tab Hook trộn chung trang với catalog.
- Ba nhóm lưới cùng kiểu thẻ nhưng "Đã cài khác" lại là list: hai bố cục khác nhau.
- Brand icon tự vẽ trong `brandIcons.ts`, entry không có brand rơi về Puzzle.

---

## 4. Disk (`C/disk/`)

Trạng thái theo git (từ `git status` + PROGRESS):
- ĐÃ COMMIT: Duyệt, Tìm kiếm, File lớn, File cũ, scan bar, treemap / sunburst, lịch sử dọn, dung lượng project trong Explorer / rail / Monitor (WP-2c, 0514ae7c), tab Dọn dẹp bản rác dev (59f51156).
- [WIP] sửa chưa commit: `DiskView.vue`, `clean/CleanGroup.vue`, `CleanRow.vue`, `DiskClean.vue`, `lib/diskMap.ts`, `lib/diskClean.ts`, `stores/diskClean.ts`, `i18n/disk-*.ts`. "Rác hệ thống" + "Dọn 1 chạm" (WP-2b-sysjunk, trạng thái GIAO; nhật ký WP ghi code + E2E 26/26 đã chạy nhưng chưa commit, `clean/CleanOneTap.vue` là file mới).
- [WIP] File trùng (WP-3a-dupes, GIAO): `dupes/DiskDupes.vue`, `DupeList.vue`, `model.ts`, `stores/diskDupes.ts` đều chưa track. `DiskView.vue:10-13` import dupes chen giữa dãy import; `hasDiskDupes` ở `lib/diskMap.ts:39`.
- [WIP, chưa gắn] Sắp xếp (WP-3b-organize, GIAO): `organize/OrganizeTab.vue`, `OrganizeRules.vue`, `OrganizeHistory.vue` có file nhưng `DiskView.vue` chưa import; `i18n/disk-organize-v101.ts` có (tab "Sắp xếp") nhưng chưa nối vào `vi.ts` / `en.ts`. Theo brief: nguồn mặc định Downloads, thêm Desktop / thư mục khác, chế độ Theo loại hoặc Theo tháng, Undo, công tắc Tự động (mặc định tắt), mục "Dọn thư mục rỗng và shortcut hỏng".
- Plan v102 WP10 làm Disk sau cùng; bố cục cuối cùng của Disk chưa chốt.

### 4.1 Layout tổng

```
 main
 | Ổ đĩa (h2)                                        [Treemap | Sunburst] (chỉ tab Duyệt)
 | Xem thứ gì đang chiếm chỗ trên ổ đĩa, rồi chuyển vào Thùng rác hoặc xoá hẳn...
 | section rounded-card border p=12 (DiskScanBar)
 |  [x] C: NTFS  [====----] Đã dùng 410 GB, còn trống 66 GB trên 476 GB    (grid 1/2/3 cột)
 |  [Mức quét: Nhẹ|Cân bằng|Tối đa] [Quét] [Quét siêu nhanh]  Lần quét gần nhất 14/11 22:10, 2413902 mục
 |  đang quét: [Tạm dừng][Huỷ]  hộp bg-fill: "Đang quét: 182,340 file, 41 GB, 00:42" + path mono
 | [Duyệt | Tìm kiếm | File lớn | File cũ | Dọn dẹp | File trùng]   (hiện khi đã index)
 | -- tab Duyệt --
 | [Lên]  C: / Users / dauph / ...  (breadcrumb)                 12.4 GB  [reload]
 | grid lg: [ canvas treemap / sunburst  h=24rem (lg 30rem) | cây DiskTree 22-30rem ]
 | legend: Thư mục, Media, Code và config, File nén và image, Tài liệu, App và binary, Khác
 | [Đã chọn 3 mục, 4.2 GB   Bỏ chọn [Vào Thùng rác] [Xoá hẳn]]  + report
 | Lịch sử dọn (card, list max-h 224px)
```

### 4.2 Từng phần

| Phần | File:line | Elements | Dữ liệu | States |
|---|---|---|---|---|
| Khung | `DiskView.vue:73-172` | h2 + SegmentedControl chart (`:79`), banner unsupported (`:82-85`) | `stores/diskMap` (IPC `diskMap*`) | không hỗ trợ: banner vàng "Bản này chưa đọc được ổ đĩa. Cập nhật app để dùng Ổ đĩa." |
| Scan bar | `DiskScanBar.vue` | mỗi ổ = card checkbox (`border-accent bg-accent-soft` khi chọn) + `DiskVolumeRow`; level segmented; Quét / Quét lại; Quét siêu nhanh (Windows NTFS MFT, cần UAC, WP-1d-mft ĐẠT, chờ UAC thật); Tạm dừng / Tiếp tục / Huỷ; hộp tiến trình `role=status` | `DiskVolume {mount, fs, total, free, network}`, `DiskProgress {files, bytes, elapsedMs, current}` | chưa index ("Chưa có index. Chọn ổ ở trên rồi bấm Quét."); đang quét; tạm dừng; fast lỗi (`fastMessage`, icon vàng); không ổ ("Không tìm thấy ổ đĩa nào.") |
| DiskVolumeRow | `DiskVolumeRow.vue` | icon HardDrive / Network, tên, fs, thanh `role=meter` cao 6px, "Đã dùng X, còn trống Y trên Z" | màu thanh theo `level` | dùng chung với Settings |
| Tab Duyệt | `DiskView.vue:98-162` | Lên, breadcrumb (mục cuối đậm, ngoài root bị disable), tổng size, reload; canvas; `DiskTree` ảo hóa | `DiskChildren {path, size, total, items[]}` | rỗng "Thư mục này trống (0 byte)."; chưa chọn "Chọn một thư mục trong bản đồ hoặc danh sách."; loading = reload quay |
| Treemap | `DiskTreemap.vue` | canvas, tối đa 2000 ô (`MAX_CELLS`), tô theo nhóm `GROUP_VAR` alpha 0.3 / 0.38 hover / 0.5 chọn, tooltip `DiskTip` | `items` | click thư mục = mở, click file = chọn (Shift = dải) |
| Sunburst | `DiskSunburst.vue` | canvas, tối đa 80 cung + 12 con (`MAX_ARCS`, `MAX_SUB`), tên thư mục giữa, nạp con theo cần | `sub` | |
| DiskTip | `DiskTip.vue` | `bg-raised ring-1 ring-line rounded-inner shadow-lg max-w-18rem`, tiêu đề + dòng mờ | | |
| DiskTree | `DiskTree.vue` | header (checkbox all, Tên, Dung lượng); dòng ảo: checkbox, icon Folder (`text-agent-file`) / File, tên, CloudOff (file chỉ trên cloud), thanh % 56px, %, size; chuột phải: mở trong trình quản lý file / Vào Thùng rác / Xoá hẳn | | rỗng hoặc đang tải |
| Tìm kiếm / File lớn / File cũ | `DiskSearch.vue` + `DiskResults.vue` | ô tìm, select loại (`DISK_KINDS`), "Chỉ" file / thư mục, sắp xếp Dung lượng / Tên / Ngày sửa, min / max MB + preset 100 MB / 1 GB / 10 GB, ngày sau / trước, File cũ thêm segmented tháng, checkbox phạm vi thư mục hiện tại, dòng tóm tắt "N kết quả, X ms, top N", bảng ảo hóa (Tên + path mono rút gọn, Loại, Ngày, Size) | `diskMapSearch` (index Rust; bench 2.5M mục: search 4-26ms) | chờ nhập (`idle`), đang tìm, lỗi (`searchFailed`), không kết quả |
| Thanh chọn | `DiskActions.vue` | `bg-accent-soft` role toolbar: "Đã chọn N mục, SIZE", Bỏ chọn, Vào Thùng rác (primary), Xoá hẳn (danger, icon TriangleAlert); hộp report `bg-st-done-soft` hoặc `bg-warning-soft`: "Đã giải phóng X từ N mục, M mục lỗi", Chi tiết (path: lý do, mono), Đóng | `DiskDeleteResult {done, failed, freed, permanent}` | |
| Lịch sử dọn | `DiskHistory.vue` | card `rounded-card border`, "Lịch sử dọn" + "Tổng đã giải phóng: X", list: icon Trash2 / TriangleAlert, "N mục vào Thùng rác / xoá hẳn", size, thời gian | `diskMapHistory` | rỗng "Chưa dọn lần nào." |

Confirm khi xóa (Duyệt, Tìm kiếm, File trùng): nút trong app chỉ gọi IPC; native confirm ở main process (`apps/desktop/src/diskMap/delete.ts:16-17,88`): Thùng rác một lần xác nhận, Xoá hẳn hai lần (`confirm` rồi `confirmPermanent`); guard canonical realpath + danh sách bảo vệ (WP-2sec, 07260d5d). Không có dialog web để mockup; hộp thoại hệ thống không theme được.

### 4.3 Tab Dọn dẹp (`clean/`) [bản rác dev ĐÃ COMMIT; hệ thống + 1 chạm WIP]

```
 hint 12px ......                    Tick project không sửa quá [ 30 ] ngày
                                     [+ Thêm thư mục] [Dọn 1 chạm] [Quét]
 chips: [Project: Orrery x]  hoặc [path1 x][path2 x]
 +- Dọn 1 chạm (CleanOneTap, ccm-panel) ---------------------------+
 | Dọn 1 chạm  [X]   mô tả                                         |
 | [x] Temp  [xoá hẳn] 124 mục  3.1 GB   (mỗi nhóm một dòng)        |
 |                                  [Dọn 4.6 GB] (primary)          |
 +------------------------------------------------------------------+
 Rác dev (nhãn nhóm, chỉ khi có cả hai nhóm)
 +- node_modules [ ] mô tả ..        9 mục   12.3 GB --------------+
 | [ ] F:\work\web\Orrery\node_modules  Project: Orrery  3 ngày trước  [npm install]  1.2 GB  [mở]
 +------------------------------------------------------------------+
 Rác hệ thống
 +- Thùng rác 28 mục  [Dọn sạch (danger)] ...
 sticky bottom-8: [Đã chọn 5, 8.1 GB   Bỏ chọn [Vào Thùng rác][Xoá hẳn]]
 report (xanh / vàng)
```

- `DiskClean.vue`: dòng điều khiển (hint, ngày `DAYS_MIN..DAYS_MAX` input 64px, Thêm thư mục, Dọn 1 chạm [chỉ khi có provider system], Huỷ / Quét / Quét lại), chip project / thư mục, `CleanOneTap`, nhóm, thanh chọn sticky `bottom-2`, report.
- `CleanGroup.vue`: card `rounded-card border`, header (checkbox chọn hết nhóm có `indeterminate`, tiêu đề 14px, hint 11px, spinner, "N mục", tổng size 64px), trạng thái `pending` ("Chưa đo. Đo sẽ đọc từng file, có thể mất vài phút." + nút "Đo dung lượng"), list max-h 28rem cuộn.
- `CleanRow.vue`: checkbox (disabled khi khóa), tên + path / project / ghi chú ("Lần build sau tải lại", "N file"), chip khóa (`ccm-chip-warning`, icon Lock / ShieldAlert), tuổi 96px, chip lệnh khôi phục (bấm chép), nút Dọn sạch cho Thùng rác (danger), size 64px, nút mở trong trình quản lý file.
- 14 nhóm (`diskClean.cat.*`): node_modules, Đồ build, Xcode DerivedData và Archives, Cache của tool (không tick sẵn), File tạm (Temp), Cache trình duyệt, Crash dump, Log cũ và báo cáo lỗi, Cache ảnh thu nhỏ, File cài đặt cũ trong Downloads (không tick sẵn), Thùng rác, Cache Windows Update (cần admin, chỉ hiện size), Cache của app (macOS).
- Khóa (`diskClean.locked.*`): project đang mở, đang Run, có server chạy, Orrery đang chạy từ đây, cần admin, trình duyệt đang mở, app đang chạy, không đọc được.
- Confirm: xóa qua kênh riêng `ccm:diskclean-delete`; native confirm (Thùng rác 1 lần, Xoá hẳn 2 lần; `diskClean/ipc.ts:14`, `service.ts:53,382-390`). "Dọn sạch" Thùng rác có confirm riêng. Dọn 1 chạm hiện danh sách nhóm bỏ tick được, chip "xoá hẳn" trên nhóm xóa hẳn, rồi mới chạy.
- States: chưa biết project ("Chưa biết project nào. Mở project trong Workspace hoặc thêm thư mục."), chưa quét ("Bấm Quét để đo đồ build của các project và cache của tool."), "Đang quét {name}...", nhóm trống "Không có gì để dọn ở đây.", Dọn 1 chạm: đang tìm hoặc "Hiện không có gì an toàn để dọn."
- Vào từ Explorer / rail / Monitor: `clean.openFor(project)` hiện chip "Project: X" kèm "Xem mọi project" (WP-2c).

### 4.4 Tab File trùng (`dupes/`) [WIP, WP-3a]

```
 note 12px
 Tìm trong [Thư mục người dùng v]  Size tối thiểu [ v ]  [Tìm file trùng]   (đang chạy: [Tạm dừng][Huỷ])
 progress: stage (Gom file cùng size / Hash nhanh phần đầu và cuối file / Hash đủ các file còn lại), bar 6px accent, path mono
 summary: "N nhóm trùng, giải phóng được X"
 lọc: [folder search 224px] [loại v] min MB [ ] MB   "Hiển thị N"
 ccm-panel list ảo hóa h=24rem (lg 30rem):
   > invoice_oct.pdf  (3 bản, 2.4 MB)                    giải phóng 4.8 MB
        (o) giữ  [ ] ...\Downloads\a.pdf   2025-10-12   (radio giữ + checkbox xóa)
 toolbar bg-accent-soft: "Sẽ giải phóng X từ N file"  [Đặt lại][Bỏ chọn][Vào Thùng rác][Xoá hẳn]
```
- Dữ liệu: `diskMapDupes` (Rust BLAKE3: size, 64KB đầu / cuối, hash đủ), `stores/diskDupes.ts`. Lỗi `diskDupes.error.*`: unavailable, busy, not-indexed, invalid, failed, exited. Mặc định mỗi nhóm giữ 1 bản, bản còn lại tick xóa; confirm giống mục 4.2.
- Hai ô min size khác nhau: `data-dupes-min-run` (khi chạy) và `data-dupes-min` (lọc kết quả).

### 4.5 DiskBadge + DiskSettings (v77, ĐÃ COMMIT)

- `DiskBadge.vue`: popover `ccm-menu` fixed, max-h min(70vh, 520px), header 36px "Ổ đĩa" + Làm mới + Đóng. Mỗi ổ: nhãn, mức (icon + chữ), nút mở thư mục, thanh 6px theo `level`, "free / total GB", nơi chứa (`places`). Footer: nút "Dọn build" (`hasBuildClean`, v77, WP-2a ghi sẽ thay bằng tab Dọn dẹp), nút "Dữ liệu agent", giờ đo. Header: "Ổ C sắp đầy" / "N ổ sắp đầy".
- `DiskSettings.vue`: một khối `ccm-group` [G fill] ngưỡng cảnh báo, nằm trong khối thu gọn của page Thông báo.

---

## 5. Click menu (v100) [ĐÃ COMMIT: WP-0b 04e390e1, WP-0c 45e558ee, WP-0d 6f766885, WP-0e ce9a9709]

| Phần | Render ở đâu | Elements | States |
|---|---|---|---|
| Thanh nút đáp án | `C/workspace/TermMenuBar.vue`, gắn dưới vùng xterm trong `TermPane.vue:1486` (cuối `ccm-pane-ring`) | `role=group` nhãn "Đáp án menu"; hàng wrap `flex-wrap gap-1.5 px-2 pt-1 pb-2`, cao tối đa 96px (`max-h-24`) cuộn dọc; mỗi đáp án một `ccm-btn h-6 px-2 text-xs` (primary khi `selected`); icon ChevronRight (đang highlight), Check (đã tick, multi), TextCursorInput (Other), CornerDownLeft (Submit); số thứ tự `tabular-nums` nếu menu đánh số; label `truncate`; tooltip `clickMenu.submitTitle` / `otherTitle` | `busy` = tất cả disabled; `@mousedown.prevent` giữ focus terminal; chỉ render khi parser nhận ra menu (Claude / Codex, pane `running`, `settings.clickMenu` bật, `TermPane.vue:422`) |
| Hover dòng menu trong terminal | `TermPane.vue:1482` | overlay tuyệt đối `rounded-inner bg-accent/20 pointer-events-none`, vị trí `menuMouse.hover` (left / top / width / height px) trên `host` xterm | hiện khi chuột đứng trên dòng đáp án; click gửi phím |
| Chip "Chờ trả lời N" | `C/workspace/MenuQueueChip.vue`, mount `WorkspaceView.vue:1956` cạnh chip "cần bạn" | `ccm-btn` chữ `text-st-waiting`, icon MessageCircleQuestion; popover `ccm-menu` listbox max-h min(60vh, 420px): dòng StatusIcon waiting + tiêu đề + vị trí + tuổi chờ; chọn = nhảy tới pane; phím nóng nhảy pane kế | ẩn khi 0 hàng; loại `menuQueue.kind.*`: Câu hỏi, Câu hỏi chọn nhiều, Xin quyền, Duyệt plan, Chọn, Codex approval |
| Mission Control | `workspace/mission/MissionCard.vue` | dùng dữ liệu `menuQueue` | chưa rõ giao diện thẻ |
| Toast hệ thống | `composables/useMenuToast.ts` + main process | toast OS: nút theo đáp án (tối đa 3) + nút Mở / Bỏ qua. Bật khi Orrery không focus, `clickMenu` + `menuToast` bật, kênh `waiting` bật, ngoài giờ yên lặng | không theme được bằng CSS; macOS + bấm toast thật CHƯA KIỂM (WP-0e) |
| Công tắc | Settings, Trình soạn thảo & Terminal (mục 1.3) | 2 toggle | giá trị mặc định chưa rõ |

Plan v102 WP12 phải làm bản "Sương kính" cho thanh nút nổi dưới pane + hover dòng menu.

---

## 6. QuickSetup (`C/QuickSetup.vue`)

- Dialog 520px, `items-start`, padding trên 8vh. Header: "Bật những gì bạn cần", phụ đề, nút X ("Để sau"). Body `ul.divide-y`: icon 32px (xanh khi bật), tiêu đề 14px, mô tả 12px, ghi chú 11px (vàng khi bị chặn hoặc statusLine / hooks chưa bật), bên phải nút "Bật" (primary) / chip "Đã bật" xanh / "Chọn thư mục…" / "Tắt" (riêng hook). Footer: "Để sau" + "Xong" primary.
- Dòng: Thông báo desktop, Toast hệ thống, Âm thanh, Status line bridge, Thư mục project, Hook bridge. Bật statusLine / hook ghi `settings.json` của Claude Code sau native confirm hiện nội dung.
- Mở từ Settings, Cập nhật & trợ giúp, "Thiết lập nhanh". Điều kiện tự mở lần đầu: chưa rõ.

---

## 7. Must keep

- Mọi `data-*` hook E2E (`data-settings-page`, `data-setting-row`, `data-disk-*`, `data-clean-*`, `data-dupes-*`, `data-dev-*`, `data-ext-*`, `data-term-menu-bar`...). E2E classic phải pass không sửa (bất biến plan v102).
- `SETTINGS_PAGES`, khóa lưu `ccm.settings.page`, `ccm.settings.size`; bảng tìm kiếm `entries` phải bao đủ mọi row, kể cả row mới.
- Native confirm cho mọi thao tác phá hủy: Xoá hẳn 2 lần, Thùng rác 1 lần, wipe emulator, sửa prefs, cài / gỡ extension. Không thay bằng dialog web.
- Không cuộn ngang trang (`overflow-x-hidden`, truncate path); path luôn mono, có `title` đầy đủ.
- Trạng thái = icon + chữ + màu, không dùng màu đơn độc (`STATE_META`, `LEVEL_*`).
- Canvas treemap / sunburst cần màu 7 nhóm `GROUP_VAR` đọc được trên nền kính; tooltip `DiskTip`.
- Danh sách lớn giữ ảo hóa (DiskTree, DiskResults, DupeList: ROW cố định, `absolute top`).
- Pane terminal KHÔNG blur (plan). `TermMenuBar` nằm trong `ccm-pane-ring`, không đẩy xterm gây resize giật.
- Hai ngôn ngữ vi / en. Text vi không dùng "trần", em dash, "·". Namespace i18n: `clickMenu.*` / `menuQueue.*` (v100), `diskMap.*` / `diskClean.*` / `diskDupes.*` / `diskOrganize.*` (v101), `disk.*` (v77, DiskBadge).
- Blur chỉ cho chrome + lớp nổi (rail, popover, dialog); list dài và canvas không blur.

## 8. Pain points / không nhất quán

1. Settings có 3 kiểu panel: `SettingSection` + `SettingRow`, panel nhúng tự có `h3`, Telegram tự bố cục.
2. Từ "Giao diện" lặp 3 lần trong Settings, và sắp thêm "Giao diện mới".
3. Devices, Extensions, Disk lặp một header (h2 24px + mô tả) nhưng thanh điều khiển khác nhau (Devices: nút phải; Extensions: segmented + nút + toolbar 2 hàng; Disk: segmented chart + scan bar tách card).
4. Hai ngôn ngữ card: `rounded-card border border-line` (Disk) và `bg-surface shadow-card` (Devices, Extensions).
5. Disk có 6 tab (dự kiến 7) trong một SegmentedControl dài; `tab` ghép từ 3 store (`store.mode`, `clean.active`, `dupes.active`), logic rối (`DiskView.vue:39-55`).
6. Thanh hành động chọn nhiều (accent-soft) lặp 4 lần với biến thể (Devices batch, DiskActions, Clean sticky, Dupes toolbar).
7. Empty state không thống nhất: icon + chữ (Devices), viền dashed (Disk), card shadow (Extensions, Hooks).
8. DiskBadge còn nút "Dọn build" v77 sắp bị thay bằng tab Dọn dẹp.
9. Mã v101 chưa gọn: import dupes chen giữa; `diskOrganize` chưa nối `vi.ts`; hai ô min size dupes; `DiskView.test.ts` chưa chắc phủ dupes / organize (chưa rõ).
10. Dialog thiết bị 920-1100px x 86-90vh khác kích thước Settings 1040x760, không theo thang chung.

## 9. Nội dung mẫu cho mockup

**Settings**
- Page: Chung, Giao diện, Trình soạn thảo & Terminal, Thông báo, Telegram, Nhắc nhở & ngân sách, Tài khoản GitHub, Tích hợp, Quyền & bảo mật, Dữ liệu & sao lưu, Cập nhật & trợ giúp (+ DeepSeek nếu bật).
- Theme hiện tại: Orrery sáng / tối, Ngân hà (mặc định tối), Console, Apple, Xcode, GitHub, Solarized, Monokai, Dracula, Material, One Dark, JetBrains Islands.
- Số mẫu: Cỡ chữ code 13 px, Thu phóng 100%, Ngân sách $20 mỗi ngày, Cảnh báo context 800 k token, Báo kẹt sau 10 phút, Giờ yên lặng 22:00 - 07:00.
- Telegram: `@orrery_owner_bot`, topic "orrery-main". GitHub: `jinvu` (Mặc định), scope `repo, read:org`.
- Tìm kiếm: gõ "sao" ra "Hiện sao", "Sao trong panel", "Hiệu ứng sao".

**Devices**: `Pixel 7 Pro` (USB, Sẵn sàng, `2A021FDH30XXXX`), `Galaxy A33` (Wi-Fi, `192.168.1.14:5555`, Ngoại tuyến), `Xiaomi 12` (Chưa cho phép), emulator `Pixel_6_API_34` (`emulator-5554`, Đang chạy).

**Extensions**: Figma (Đã kết nối, Toàn cục), GitHub (Cần đăng nhập), Unity (Chưa cài), Blender, Godot, PM skills (plugin), Frontend Design (Đã cài); hook `PreToolUse` matcher `Bash` + command `node ~/.claude/jin/hooks/resource-guard.mjs`, chip "Orrery" cho hook của app; scope chip "Toàn cục / Project, chia sẻ / Project, chỉ mình bạn".

**Disk**
- Ổ: `C: NTFS` đã dùng 410 GB, còn 66 GB, tổng 476 GB; `D: NTFS` 1.2 TB; `K: NTFS` (dev); `\\NAS\media` (Network).
- Duyệt: `F:\work\web\Orrery` 6.4 GB gồm `node_modules` 1.2 GB, `apps\desktop\release` 2.1 GB, `.git` 640 MB.
- File lớn: `C:\Users\dauph\Videos\screen-rec-2026-10.mp4` 5.2 GB, `...\Downloads\android-studio-setup.exe` 1.1 GB, `K:\dev\.gradle\caches` 8.4 GB.
- Dọn dẹp: node_modules 12.3 GB (9 mục), Đồ build 18.7 GB, Cache của tool 6.1 GB (không tick sẵn, "Lần build sau tải lại"), Temp 3.1 GB, Cache trình duyệt 2.4 GB (Chrome: "Trình duyệt đang mở: đóng rồi quét lại"), Thùng rác 28 mục 1.9 GB, Minidump "Cần quyền admin". Dọn 1 chạm: "Dọn 4.6 GB".
- Lệnh khôi phục: `npm install`, `flutter pub get`, `./gradlew build`.
- Report: "Đã giải phóng 4.2 GB từ 7 mục, 1 mục lỗi: đang bận".
- Trùng: `invoice_oct.pdf` (3 bản, 2.4 MB, giải phóng 4.8 MB); "Sẽ giải phóng 3.1 GB từ 212 file".
- Quét: "Đang quét: 182,340 file, 41 GB, 00:42", path `C:\Users\dauph\AppData\Local\Temp\...`; "Lần quét gần nhất 14/11/2026 22:10, 2,413,902 mục".

**Click menu**: câu hỏi agent với 3 đáp án `1 Vue`, `2 React`, `3 Svelte` (+ Other); hoặc xin quyền "Allow once", "Allow always", "Deny"; chip "Chờ trả lời 2"; hàng popover "Orrery / Workspace 1 (Claude)", tuổi "2 phút".

## 10. Chưa rõ

- Giá trị mặc định `settings.clickMenu` / `menuToast` trong `stores/settings.ts` (chưa đọc).
- Layout thẻ Mission Control cho trạng thái chờ trả lời.
- Điều kiện tự mở QuickSetup lần đầu.
- Hằng `ROW` (px) của DiskTree / DiskResults / DupeList; giá trị padding / radius của `ccm-group`, `ccm-btn` (token trong `style.css`).
- Bố cục tab Sắp xếp (component chưa gắn, chỉ có brief WP-3b và key i18n).
- Telegram tab Topic và Nhận lệnh: chỉ đọc cấu trúc, chưa mô tả từng control.
- Panel nhúng AgentData (strip) và DeepSeek: mới liệt kê, chưa dựng sơ đồ chi tiết.
