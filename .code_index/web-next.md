# web-next: UI next gen "Sương kính" (v102+)

Đường vào: `apps/web/src/App.vue` nạp `NextShell`, `ExplorerDrawer`, `NextInvite` trực tiếp; `apps/web/src/features/next/index.ts` chỉ export lib bảng màu. Root `data-ui="next"`, key `ccm.uiGeneration`; mặc định từ v103, classic đóng băng từ 4.0.8 và bỏ ở 4.1.0.

## File chính

| File | Vai trò |
|---|---|
| `apps/web/src/features/next/components/NextShell.vue` | khung: rail, canvas, header |
| `apps/web/src/features/next/components/rail/LiveRail.vue` | rail sống (264 / 248 / 56 px hoặc overlay), danh sách "cần chủ nhân" |
| `apps/web/src/features/next/components/canvas/useNextLayout.ts` | layout menu, drop compass, split hint |
| `apps/web/src/features/next/lib/railLive.ts` | model rail theo trạng thái session |
| `apps/web/src/features/next/lib/customTheme.ts` | bảng màu Tự chọn (8 card) |
| `apps/web/src/features/next/tokens.css` | token glass (import ở `main.ts`) |
| `apps/web/src/features/next/stores/uiGen.ts` | store thế hệ UI, đổi live không remount pane |
| `packages/mist-tokens` | token màu dùng chung với PWA |

## Bất biến

- Blur chỉ dưới `:root[data-ui='next']` và chỉ trên chrome / lớp nổi (`.glass`, `.glass-pop`, `.glass-side`); pane terminal, list dài, nội dung không blur; không animate blur / layout. Từ 23/11 luôn panel đặc: `uiGen` đặt `data-glass='reduced'` cố định, `startNext` / `pickNext` (index.html) trả `glass: 'reduced'`; bỏ công tắc Giảm trong suốt + key `ccm.next.reduceGlass` (resize nhiều lần làm cửa sổ phủ mờ, đứng). Bỏ luôn slider Độ trong kính (`clarity`) ở theme tự tạo; field `clarity` vẫn giữ trong `CustomThemeConfig` để đọc dữ liệu cũ.
- 5 palette + Ngẫu nhiên + Tự chọn, light / dark, motion full / reduced / off.
- Phụ thuộc vòng: next -> workspace, next -> rail; workspace / rail import ngược `lib/next/*`.
- Test đọc file ngoài feature bằng đường tương đối (`apps/web/src/features/next/palettes.test.ts`, `apps/web/src/features/next/lib/mau.test.ts`): dời cây thì đếm lại `../`.

## Liên kết

- i18n: `apps/web/src/features/next/i18n/`. Plan: `.planning/PLAN-v102-mist-redesign.md`, `SPEC-v102-mist-ui.md`, `DECISION-v108-colors.md`. E2E: `v102-next`, `ui-gen`, `v109-themes-all`. Style: `docs/visual-style.md`.
