WP-16: ĐẠT
File: components/AppSelect.vue (thêm prop `disabled`, `title` từ attrs thắng title mặc định), AppSelect.test.ts (mới, 4 test), next/tokens.css (cuối file), 18 file thay `<select>`: disk/{DiskSearch,DiskTree,apps/DiskApps,dupes/DiskDupes,organize/OrganizeRules}.vue, devices/{DeviceAppData,DeviceFiles,DeviceUiTest,DevicePerf}Dialog.vue, office/OfficeTimelapse.vue, settings/SettingsNotify.vue, workspace/{HttpPanel,LogFileView,ReleaseCheckDialog,RunConfigsDialog,RunLog,RunPane,StringsDialog}.vue. Test sửa: office/officeTimelapse.test.ts, disk/DiskTree.snap.test.ts.
Việc 1: còn 0 `<select>` trong apps/web/src (rg). 24 chỗ đã chuyển. Không có `<optgroup>`. `disabled` thêm vào AppSelect (button disabled, không mở, keydown bỏ qua, đóng list nếu đang mở khi chuyển sang disabled).
Việc 2: 4 rule dưới `:root[data-ui='next']` ở cuối `next/tokens.css`: option AppSelect đang trỏ = nền `--ccm-accent-soft`, chữ `--ccm-ink`, hint `--ccm-ink-faint`; item ContextMenu focus giống. Không có box-shadow mới. ContextMenu không có mục danger (không có field danger trong lib/menu.ts) nên không cần giữ màu lỗi.
Typecheck: pass (npm run typecheck -w @ccm/web). Test: toàn bộ apps/web 2593 pass / 0 fail (vitest run, reporter json).
Thay đổi hiển thị: dấu "·" trong label option đổi thành " - " ở 6 chỗ (RunPane, StringsDialog, RunLog, LogFileView, RunConfigs device offline, AppData table rows, DevicePerf); chuỗi i18n sẵn có (vd "Today · ..." của timelapse) giữ nguyên. Tooltip `title` từng option (DeviceUiTest roots) bỏ. Trigger size sm (h-7 text-xs) cho mọi chỗ cũ h-6/h-7/text-2xs, md cho SettingsNotify và RunConfigs; text-2xs cũ thành text-xs.
Rủi ro: DiskTree label "Compare" từ `<label for>` thành `<span>` + aria-label trên trigger (id `disk-compare-base` giữ trên trigger button).

Cách E2E chọn giá trị mới: click trigger `[data-xxx]` (button role=combobox), rồi click `.ccm-menu [role=option]` theo text hoặc index (list teleport ra body, `data-index`, `aria-selected`). Đọc giá trị hiện tại: text của trigger hoặc option có `aria-selected=true`. Không còn `.value`, `option`, `selectedOptions`, `dispatchEvent('change')`.

| Chỗ cũ (file:line mới) | Hook test | Ghi chú |
|---|---|---|
| disk/DiskSearch.vue:82,90,99 | `data-disk-kind`, `data-disk-only`, `data-disk-sort`; aria-label diskMap.search.kind/only/sort | value '' = tất cả |
| disk/DiskTree.vue:133 | `#disk-compare-base`, `data-disk-compare-base` | option 0 = "No comparison", sau đó theo thứ tự snapshots |
| disk/dupes/DiskDupes.vue:54,58,101 | `data-dupes-scope`, `data-dupes-min-run`, `data-dupes-kind` | disabled khi store.running |
| disk/apps/DiskApps.vue:40 | `data-app-sort` | size/name/date |
| disk/organize/OrganizeRules.vue:53 | không hook; aria-label diskOrganize.rules.kind.ext, nằm trong `[data-organize-rule]` | |
| devices/DeviceUiTestDialog.vue:265,322 | `data-uitest-root`, `data-uitest-locale` | root: option 0 = noProject; locale disabled khi blocked/busy |
| devices/DeviceFilesDialog.vue:336 | `data-files-root` | |
| devices/DeviceAppDataDialog.vue:367,450 | `data-appdata-table`, `data-appdata-draft` (chỉ khi pref boolean, option 'true'/'false') | |
| devices/DevicePerfDialog.vue:278 | `data-perf-base` | |
| office/OfficeTimelapse.vue:368 | không hook; aria-label officeV60.timelapse.day, `[role=combobox]` trong thanh timelapse | disabled khi chưa có ngày |
| settings/SettingsNotify.vue:214 | `data-idle-select` | giá trị number |
| workspace/HttpPanel.vue:147 | `data-http-env` | option '' = noEnv |
| workspace/LogFileView.vue:238 | `data-log-level` | value -1..5, option 0 = allLevels |
| workspace/ReleaseCheckDialog.vue:179 | `data-release-app` | chỉ hiện khi apps > 1 |
| workspace/RunConfigsDialog.vue:447,451,456,466 | `#rc-cwd`, `#rc-entry`, `#rc-module`, `#rc-device` | id nay nằm trên trigger button |
| workspace/RunLog.vue:205 | không hook; aria-label run.log.level | |
| workspace/RunPane.vue:260 | `data-run-session` | |
| workspace/StringsDialog.vue:249 | `data-strings-set` | value = index set |

E2E trong apps/desktop/e2e đang đụng select cũ (chỉ liệt kê, KHÔNG sửa):
- v101-snap.cjs:118-122 (`[data-disk-compare-base] option`, `.value=`, change) - dùng `#disk-compare-base`
- v101-diskmap-ui.cjs:147,155 (`setField('[data-disk-kind]'|'[data-disk-only]', ..., 'change')`)
- v70-tools.cjs:171 (`[data-log-level]` `.value='4'` + change), 175 (setInput '-1')
- v72.cjs:257 (`[data-uitest-root]` `.value === DIR`)
- v73.cjs:272 (`[data-appdata-table] option`)
- v74.cjs:423,426,430 (`[data-strings-set]` options, selectedOptions, `.value` + change)
- v76.cjs:409-412 (`[data-idle-select] option`, selectedIndex) và các dòng dùng group này sau đó
- Đã kiểm, không đụng select của gói này: pane-drag.cjs:32, v102-next.cjs:47 (chỉ loại trừ tag `select` trong closest(), vô hại).
