# core

Dùng chung cho mọi feature. Luật (`src/importRules.test.ts`): không import `features/**`, không import `src/app/**`. Alias `@core/...`.

- `components/`: ui kit (`AppSelect`, `ToggleSwitch`, `InfoHint`, `KeyHint`, `Panel*`, `Status*`, `ToastHost`, `ArtImage`, `LottieArt`, `ui/*` gồm `ContextMenu`, `SegmentedControl`, `ThemePicker`, `FontPicker`).
- `stores/`: `terminals`, `agents`, `activity`, `extras`, `favorites`, `peers`, `toolSpans`, `toasts`, `settings`, `ui`, `connectionEvents` (registry cho feature nghe event của connection).
- `lib/`: `desktop` (IPC client), `preview`, `format`, `themes`, `fonts`, `motion`, `menu`, `status`, `patch` (áp ops snapshot), `shareMode` + `mask`, `autoSave`.
- Type / hằng mà core cần từ feature nằm ở `lib/previewTypes.ts` (layout / drawable / `DirEntry`), `lib/uiTypes.ts` (`Point`, `NodeSize`, `TourId`, `SETTINGS_PAGES`, `DEV_TOOLS`, `BoardGroupBy`), `lib/settingLimits.ts`, `lib/dustFlag.ts`; file feature gốc export lại, tên symbol không đổi.
- i18n gốc vẫn ở `src/i18n/` (không ở đây): `en.ts` / `vi.ts` gộp i18n của feature nên không thể vào core.
- `ArtImage` / `LottieArt` dùng `import.meta.glob('../../assets/art/*')`: dời file thì sửa glob.
