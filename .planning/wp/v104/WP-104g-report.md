# WP-104g DeepSeek lazy module

## File đổi
- Dời (git mv): `stores/deepseek.ts` -> `features/deepseek/store.ts`, `stores/deepseek.test.ts` -> `features/deepseek/store.test.ts`, `lib/deepseekGate.ts` -> `features/deepseek/gate.ts`, `components/settings/SettingsDeepSeek.vue` -> `features/deepseek/SettingsDeepSeek.vue`, `i18n/deepseek-v81.ts` -> `features/deepseek/i18n.ts` (chỉ key dùng trong Settings DeepSeek).
- Mới: `features/deepseek/index.ts` (`loadDeepSeek`, `deepseekStore`, `gateDeepSeek`, `gateDeepSeekAgent`, `warmDeepSeek`, `loadSettingsDeepSeek`), `features/deepseek/index.test.ts`, `i18n/deepseek-launch.ts` (key dùng ngoài module: `launchNote.*`, `openSetup`, `agentNotice`, và các key mà `components/settings/index.ts` dùng cho tìm kiếm Settings: `intro`, `status`, `apiKey`, `apiKeyHint`, `logout`, `logoutHint`, `telemetry`, `telemetryHint`, `sync`, `syncHint`, `update.check`), `apps/desktop/e2e/v104-deepseek-lazy.cjs`.
- Sửa tối thiểu: `App.vue`, `stores/workspace.ts` (3 chỗ `gateDeepSeek`), `components/workspace/LaunchPicker.vue`, `components/SettingsMenu.vue` (chỉ import + `defineAsyncComponent`), `i18n/en.ts`, `i18n/vi.ts` (đổi import), `apps/web/vite.config.ts` (`chunkFileNames` đặt tên `deepseek-*`), `apps/desktop/e2e/run.cjs`, `scripts/qa-fast.mjs`.
- `LiveRail.vue`, `lib/desktop.ts::hasDeepSeek`, `lib/provider.ts::isDeepseek`: giữ nguyên (chỉ là cờ / so sánh chuỗi, không kéo module).

## Hành vi
- `hasDeepSeek` false: không import module, không request chunk, trang Settings DeepSeek ẩn như cũ.
- `hasDeepSeek` true: `App.vue` gọi `warmDeepSeek` ngay sau khi setup (chunk store + i18n), refresh status sau 4s như cũ (cửa sổ compact chỉ tải, không refresh). Chunk Settings chỉ request khi mở trang Settings DeepSeek.
- Cửa sổ ngắn trước khi module tải xong: `gateDeepSeek` trả `true` (không mở pane), module tải xong thì chạy gate (mở Settings nếu thiếu cài / login), user bấm lại là mở. `open_session` từ agent đợi module (`await loadDeepSeek()`).

## Test
- `JIN_RAW=1 npx vitest run` ở apps/web: 317 file, 2735 test pass (gồm `stores/handBack.test.ts`, `stores/workspace.test.ts`, `lib/launcher.test.ts`, `lib/rail/railMenu.test.ts`, `components/settings/*`, `features/deepseek/*` 8 test).
- `npm run typecheck -w @ccm/web` sạch. `npm run build -w @ccm/web` sạch.
- E2E `v104-deepseek-lazy.cjs` (Tauri, cổng 4491 / 9491, data dir tạm): 15 PASS, 0 FAIL. Có cờ: bridge có, chunk store + i18n tải lúc khởi động, chunk Settings chỉ khi mở Settings, trang render, không key thô, không lỗi console. Không cờ: không bridge, không request chunk `deepseek-*` sau 6s và sau khi mở Settings, không có trang DeepSeek.

## Kích thước chunk (build web)
- Trước (dist 03:23 cùng ngày): `index-*.js` main 2,093,251 B.
- Sau: main 2,078,729 B (-14,522 B). Chunk mới: `deepseek-store` 1.64 kB, `deepseek-i18n` 6.18 kB, `deepseek-SettingsDeepSeek` 15.80 kB (gzip 4.72 kB).

## Rủi ro
- Key i18n: đã giữ key dùng ngoài module ở bundle chính; key mới thêm vào `SettingsDeepSeek.vue` về sau phải vào `features/deepseek/i18n.ts`, còn nếu dùng ở launcher / rail thì vào `i18n/deepseek-launch.ts`.
- `manualChunks` thử và bỏ: kéo cả đồ thị phụ thuộc của store (i18n chính, stores) vào chunk 979 kB; dùng `chunkFileNames` chỉ để đặt tên.
- Test `store.test.ts` phải `await loadDeepSeek()` trước khi gọi `ws.launch('deepseek')`.

## Test macOS
- Chạy `CCM_DEEPSEEK_FEATURE=1`: Settings -> DeepSeek mở không lỗi; không cờ thì không có trang DeepSeek.
