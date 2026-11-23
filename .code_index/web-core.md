# web-core: apps/web tầng core, app, i18n, entry

Đường vào: `apps/web/src/main.ts` (Pinia + vue-i18n, mount `App.vue`), `apps/web/src/App.vue` (lazy nạp view theo tab). Alias `@core`, `@features` (`apps/web/vite.config.ts`).

## Luật phân tầng (`apps/web/src/importRules.test.ts`)

- `apps/web/src/core/`: dùng chung, không import `features/**` và `apps/web/src/app/`.
- `apps/web/src/app/`: ghép nhiều feature; được import core + features.
- `src/features/<feature>/`: có `index.ts` (cửa ra) + `README.md`; feature khác import qua `@features/<feature>`. Component nặng nạp lazy trực tiếp (không qua index) để giữ chunk.
- Feature mới nghe event của connection qua `apps/web/src/core/stores/connectionEvents.ts`, không sửa `connection.ts`.
- Dời file: `scripts/web-move.mjs` (git mv + sửa import).

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/app/stores/connection.ts::useConnectionStore` | WebSocket client: snapshot, event, resync, reconnect, đánh dấu stale |
| `apps/web/src/core/stores/connectionEvents.ts` | registry handler event cho feature |
| `apps/web/src/core/lib/desktop.ts` | client IPC `window.ccmDesktop` (kênh `ccm:*`) |
| `apps/web/src/core/lib/patch.ts` | áp ops của snapshot / patch |
| `apps/web/src/core/stores/settings.ts::useSettingsStore` | setting lưu `ccm.*` (localStorage) |
| `apps/web/src/core/stores/ui.ts` | trạng thái UI chung (tab, dialog) |
| `apps/web/src/core/stores/terminals.ts`, `agents.ts`, `activity.ts`, `peers.ts`, `toolSpans.ts` | state quan sát (mirror `packages/shared`) |
| `apps/web/src/core/lib/themes.ts::startTheme` | theme lúc mở app (khớp `apps/web/index.html` `pick`) |
| `apps/web/src/core/lib/fonts.ts` | font code / giao diện (bundled, không tải mạng) |
| `apps/web/src/core/lib/shareMode.ts`, `mask.ts` | chế độ chia sẻ màn hình + che nội dung |
| `apps/web/src/core/components/ui/` | `ContextMenu`, `SegmentedControl`, `ThemePicker`, `FontPicker` |
| `apps/web/src/core/components/EmptyState.vue` | empty state chung: `title`, `hint?`, `icon?`, `compact?`, slot `actions` (dùng ở InboxPanel, SessionBoard) |
| `apps/web/src/app/components/AppHeader.vue` | header, tab chính |
| `apps/web/src/app/composables/useNotifications.ts`, `useAppToasts.ts`, `useMcpCalls.ts` | thông báo, toast, lệnh MCP đa feature |
| `apps/web/src/i18n/index.ts` | vue-i18n; `en.ts` / `vi.ts` gộp i18n các feature, `keys.snapshot.test.ts` khoá danh sách key |
| `apps/web/src/style.css`, `apps/web/src/themes.css`, `apps/web/src/fonts.css` | token màu / shape, theme, font |

## Bất biến

- Mọi chuỗi UI qua vue-i18n (en + vi); màu qua token (`.planning/COLOR-SYSTEM.md`), không nhánh theo id theme trong component.
- Không cuộn ngang trang (`overflow-x-hidden` ở app).
- Mất kết nối: giữ state cuối và đánh dấu stale; tự reconnect.
- `ArtImage` / `LottieArt` dùng `import.meta.glob('../../assets/art/*')`: dời file thì sửa glob.

## Liên kết

- Giao thức: `protocol.md`. Plan: `.planning/PLAN-v110-modules.md`, `.planning/AUDIT-v110-web-features.md`. Style: `docs/visual-style.md`.
