WP-10 + WP-12: ĐẠT (code + CSS; chưa chụp ảnh, chưa chạy E2E)

File mới: components/settings/{SettingsMistAppearance,PaletteMini,SettingsWorkspace,SettingsDisk}.vue
File sửa: components/SettingsMenu.vue (nav theo isNext, dialog `mist-settings`, 2 page mới), settings/index.ts (SETTINGS_PAGES + workspace, disk; NEXT_ONLY_PAGES, pageAvailable, nextEntries), settings/{SettingsAppearance,SettingsEditor,SettingsData}.vue (ẩn phần đã chuyển ở next), settings/index.test.ts (+2 test), i18n/mist-v102.ts (khối mist.settings en + vi), i18n/ui-v79-settings.ts (tên page workspace, disk), next/tokens.css (khối settings + devices/extensions/disk cuối file).

Đã làm:
- G1: trang Giao diện ở next: Kiểu giao diện (Orrery cũ / Sương kính), 9 thẻ palette với mini Workspace HTML/CSS (click = setNextTheme, mist sáng/tối đặt luôn mode), Sáng / tối (Theo hệ thống / Sáng / Tối, Sáng disabled với palette chỉ tối), Giảm trong suốt, Terminal trong suốt, Chuyển động 3 mức, Sao Ngân hà (chỉ bật khi palette Galaxy), Phông chữ + cỡ chữ + zoom giữ nhúng. Classic trang Appearance giữ nguyên.
- G2: next có page Workspace (clickMenu, menuToast, dim, compactAuto, agentShim, dust, fileIcons) và Ổ đĩa (SettingsDiskMap gồm DiskSettings). Classic nav 12 page cũ, vị trí rows cũ. Tìm kiếm settings dùng nextEntries ở next. G3 tên "Giao diện" / "Sáng / tối". G4 dùng khoá sẵn của store, không đụng theme classic.
- WP10/12 restyle chỉ bằng CSS dưới data-ui=next: Devices, Extensions, Disk (thẻ/nhóm/panel thành kính không blur, tab bar 9 tab + chọn biểu đồ kiểu segment kính, tiêu đề 20px), Office chỉ bo góc khung. Dialog chuyển ổ dùng `.ccm-sheet` đã kính từ WP6. Không đổi dữ liệu / hành vi / xác nhận xoá.
Chưa làm: Mission card trả lời menu (WP7); Office chỉ khung theo brief.

Typecheck: pass. Test: 106 pass / 1 fail ngoài phạm vi (npx vitest run --root apps/web src/components/settings src/components/disk src/components/devices src/i18n src/next src/stores/uiGen.test.ts).
Fail: next/palettes.test.ts "mọi selector next phải dưới :root[data-ui='next']" do `.ccm-next-kbd` (tokens.css dòng ~449, khối của WP khác, không phải của WP10); cần gói đó bọc lại.
Rủi ro: CSS `.mist-settings .bg-fill.p-3` bám class sidebar SettingsMenu; selector devices/disk bám `.rounded-card` + `.ccm-group`, cần chụp ảnh ở WP11 (đặc biệt DiskTree list dài, DiskClean). Số nút nav Settings ở next là 14 (E2E đếm nav chỉ cần kiểm ở next).
