# web-small: feature nhỏ (click-menu, palette, resources, chat, shortcuts...)

| Feature | Đường vào | File chính | Ghi chú |
|---|---|---|---|
| click-menu | `apps/web/src/features/click-menu/index.ts` | `lib/termMenu.ts::parseTermMenu`, `composables/useTermMenu.ts`, `stores/menuQueue.ts`, `stores/inbox.ts` (đều dưới `apps/web/src/features/click-menu/`) | trả lời menu Claude / Codex bằng chuột + Inbox "Chờ trả lời" (v100) |
| palette | `apps/web/src/features/palette/index.ts` | `components/CommandPalette.vue`, `lib/paletteSearch.ts`, `lib/paletteKey.ts` | Ctrl+K, tiền tố `>` `@` `#` `:` |
| resources | `apps/web/src/features/resources/index.ts` | `stores/resources.ts`, `lib/resources.ts`, `lib/devTools.ts` | CPU / RAM / port theo session, DevTools (JWT, base64...), Diagnostics |
| chat | `apps/web/src/features/chat/ChatView.vue` | `feed.ts`, `model.ts`, `markdown.ts` | chat view của session (v109) |
| shortcuts | `apps/web/src/features/shortcuts/useShortcuts.ts` | `bindings.ts`, `defaults.ts`, `MruSwitcher.vue` | phím tắt tuỳ biến, MRU |
| secretGuard | `apps/web/src/features/secretGuard/secretGuard.ts` | `SecretGuardDialog.vue` | chặn commit / push có token (v109 C1) |
| shellDone | `apps/web/src/features/shellDone/shellDone.ts` | `useShellDone.ts` | toast lệnh shell xong (OSC 633) |
| termSearch | `apps/web/src/features/termSearch/search.ts` | `TermSearchPanel.vue`, `registry.ts` | tìm trong terminal |
| diffComments | `apps/web/src/features/diffComments/diffComments.ts` | `store.ts`, `DiffCommentBar.vue` | bình luận trên diff |
| shareMode | `apps/web/src/features/shareMode/useShareModeKey.ts` | `ShareBar.vue`, `ShareModeChip.vue` | che nội dung khi chia sẻ màn hình |
| footerSys | `apps/web/src/features/footerSys/store.ts` | `FooterSysChip.vue`, `format.ts` | chip CPU / RAM ở footer |
| notify | `apps/web/src/features/notify/badge.ts` | `grouping.ts`, `useTaskbarBadge.ts` | badge taskbar, gộp thông báo |
| deepseek | `apps/web/src/features/deepseek/index.ts` | `store.ts`, `gate.ts`, `SettingsDeepSeek.vue` | tạm dừng, bật bằng `CCM_DEEPSEEK_FEATURE=1` |

## Bất biến

- click-menu: app không tự trả lời menu; mọi phím gõ vào pane đi sau một thao tác của chủ nhân (docs §11as). Fixture `apps/web/src/features/click-menu/lib/__fixtures__/term-menu` do E2E `v100-menu-*.cjs` đọc.
- secretGuard / shareMode: che trước khi hiển thị, không lưu giá trị gốc.
- Feature nhỏ không import feature khác trừ qua `index.ts`.

## Liên kết

- Phía core: `core-toast-bridge.md` (toast menu), `core-desk-preview.md` (secrets). E2E: `v100-menu-*`, `v109-*`.
