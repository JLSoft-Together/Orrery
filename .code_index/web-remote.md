# web-remote: Orrery Remote + Telegram (phía trang desktop)

Đường vào: `apps/web/src/features/remote/index.ts` (`SettingsRemote`, `PhoneSettings`, `RemotePhoneChip`, `hasPhone`, `onPhone`, `useRemoteStore`, `installRemotePhone`), `apps/web/src/features/telegram/index.ts` (`TelegramToggle`, `TelegramSettings`, `hasTelegram`, `useTelegramStore`, `useTelegramLinks`, `installTelegramInbound`).

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/remote/stores/remote.ts` | state Remote (pair, thiết bị, relay, push, guard) |
| `apps/web/src/features/remote/composables/useRemotePhone.ts` | chip điện thoại trên pane |
| `apps/web/src/features/remote/lib/remote.ts` | bridge `ccm:remote-*` |
| `apps/web/src/features/remote/components/SettingsRemote.vue` | Settings Remote |
| `apps/web/src/features/telegram/stores/telegram.ts` | state Telegram |
| `apps/web/src/features/telegram/composables/useTelegramInbound.ts` | tin vào -> pane (hành động mở pane) |
| `apps/web/src/features/telegram/components/TelegramSettings.vue` | Settings Telegram (lớn) |

## Bất biến

- Remote tắt mặc định; env, secret, app key, PTY token không rời máy (docs §11au).
- Chỉ tin từ tài khoản Telegram đã pair mới được submit vào pane (docs §11an).
- `mist-remote` i18n thuộc `features/next`, ở lại đó.

## Liên kết

- Phía desktop: `shell-remote.md`, `core-desk-telegram.md`. Khách: `pwa.md`, `mobile.md`, `relay.md`. Kiểu: `packages/shared/src/remote.ts`, `telegram.ts`, `phone.ts`. Plan: `.planning/PLAN-v97-mobile.md`, `PLAN-v82-*.md`. E2E: `v97-remote`, `v98-remote-next`, `push`.
