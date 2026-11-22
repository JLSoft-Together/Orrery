# Office (Habblaud pixel-art), module lazy

Đọc file này chỉ khi làm Office. Office đang tạm dừng phát triển (không thêm tính năng). Plan: `.planning/PLAN-v110-modules.md` P2, `.planning/AUDIT-v110-web-features.md` mục 7.

## Đường vào
- `apps/web/src/features/office/index.ts` là cửa duy nhất ra ngoài (`@features/office`): `OfficeEntry` (async, chứa `OfficeView.vue`), `OfficeToolbar` (async, hàng nút zoom / sound / timelapse / options / full screen), `loadOfficeI18n()`, `registerOfficeFeed(pinia)`.
- `main.ts` gọi `registerOfficeFeed(pinia)` sau `app.use(pinia)`: đăng ký handler `office.snapshot` vào `stores/connectionEvents.ts` (`registerEventHandler` / `dispatchEvent`, `connection.ts` gọi ở nhánh `default` của `applyEvents`), và khi `settings.officeEnabled` bật thì nạp i18n.
- `AgentMap.vue` chỉ dùng `OfficeEntry`, `OfficeToolbar`; state full screen đi qua event `update:fullscreen` (`officeFullscreen` ref ở AgentMap).
- Server Office đã bỏ cùng collector (v110 P1). Tauri luôn trả `coreServe=rust` nên `officeNeedsNode` (`apps/web/src/core/lib/desktop.ts`) luôn đúng: bật Office thì `apps/web/src/features/map/components/AgentMap.vue` và `apps/web/src/features/settings/components/SettingsGeneral.vue` hiện dòng i18n `settings.officeNeedsNode` ("Office is not available in this build" / "Văn phòng chưa có trên bản này").

## Cây file (`features/office/`)
- `components/`: `OfficeView.vue` (world Habblaud), `OfficeTimelapse.vue`, `OfficeOptions.vue`, `OfficeToolbar.vue`, `useOfficeControls.ts` (zoom, volume, Shift+F), test.
- `stores/office.ts` (state view, sound, options, style, timelapse), test sound.
- `lib/`: `feed.ts` (`officeFeed`, snapshot localize), `localize.ts`, `i18n.ts` (từ điển Habblaud theo locale), `packs.ts`, `sound.ts`, `strings.ts`, `switching.ts`, `timelapse.ts`, `iso/` (`IsoRenderer.ts`, `isoArt.ts`, art 3D).
- `i18n/office-v60|62|63.ts`: không nằm trong `i18n/en.ts` / `vi.ts`; `loadOfficeI18n` merge bằng `i18n.global.mergeLocaleMessage`. 84 key (en và vi), nên `i18n/__snapshots__/keys.*.txt` không còn `officeV6x.*`. Test cần `await loadOfficeI18n()`.

## Bất biến
- Bundle chính (`index-*.js` lớn nhất) không chứa `IsoRenderer`, `officeV60`, store office; chỉ còn message `office.enable` / `office.subscribe` / handler `office.snapshot` trong `connection.ts` và `main.ts`.
- `importRules.test.ts`: ngoài feature chỉ import qua `@features/office`; `core` không import feature.
- Test đọc file thật bằng đường dẫn tương đối (`isoArt.shipped.test.ts`, `packs.test.ts` đọc `apps/web/public/assets/...`): dời cây thì đếm lại số `../`.
- Office tắt (mặc định, `ccm.office` = `0`): không nạp, không vẽ, không ghi gì.

## packages/habblaud
Vendored MIT (`@ccm/habblaud`), alias `./locales/all` → `locales/none.ts` trong `apps/web/vite.config.ts` (chỉ nạp từ điển đang dùng). `stores/office.ts` chỉ import `world/api` (nhẹ); `world` entry nạp lười trong `OfficeView.vue`. Giấy phép `apps/web/public/habblaud-LICENSE.txt`.
