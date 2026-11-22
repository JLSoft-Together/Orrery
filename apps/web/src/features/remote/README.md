# remote

Orrery Remote (app điện thoại): pair, relay, push, guard, access log, install card, Settings → Remote / Notify phone, chip điện thoại trên pane.

- Đường vào: `@features/remote` (`SettingsRemote`, `PhoneSettings`, `RemotePhoneChip`, `hasPhone`, `onPhone`, `useRemoteStore`, `installRemotePhone`, `installRemoteSummary`).
- File chính: `stores/remote.ts`, `composables/useRemotePhone.ts`, `lib/remote.ts`, `components/SettingsRemote.vue`.
- Bất biến: remote control tắt mặc định; env / secret / app key / PTY token không rời máy (docs §11au).
- i18n: `i18n/{remote-v97,remote-v98,remote-setup-v97,v106-install}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`. `mist-remote` thuộc next, ở lại.
