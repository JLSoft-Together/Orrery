# telegram

Telegram bot: gửi report, nhận lệnh từ tài khoản đã pair vào session, topic theo project, nút trả lời, link pane ↔ topic.

- Đường vào: `@features/telegram` (`TelegramToggle`, `TelegramSettings`, `hasTelegram`, `onTelegram`, `topicUrl`, `useTelegramStore`, `useTelegramLinks`, `openTelegramGlyph`, `installTelegramInbound`).
- `TelegramLinkDialog` nạp lazy trực tiếp trong WorkspaceView.vue.
- File chính: `components/TelegramSettings.vue` (lớn), `stores/telegram.ts`, `composables/useTelegramInbound.ts`.
- Bất biến: tin nhắn từ tài khoản đã pair mới được submit vào pane (docs §11an).
- i18n: `i18n/telegram-v82.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
