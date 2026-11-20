# WP-104d - Cảnh báo đụng file (báo cáo)

Switch chung `clashWarn` (`ccm.notify.clash`), **mặc định tắt**. Bật: chip trên dòng rail next + chip header pane (next), toast trong app 1 lần / cặp / lượt, `Notification` web khi cửa sổ không focus. Chỉ cảnh báo, không chặn, không gửi gì vào pane.

## File + symbol
| File | Thay đổi |
|---|---|
| `apps/web/src/lib/clash.ts` (mới) | `Clash`, `clashList`, `clashesByTerminal`, `clashKey`, `pairKeys`, `newClashes`, `othersOf`, `relPath`, `isUnder`, `summarize`, `addIgnored` / `removeIgnored` / `pruneIgnored` / `parseIgnored`, `MAX_IGNORED = 200` |
| `apps/web/src/lib/agentTouches.ts` | `FileTouchInfo.path` (path gốc, để hiện tên file tương đối) |
| `apps/web/src/stores/agentTouches.ts` | `clashes` (rỗng khi switch tắt, không tính `index`), `clashesOf`, `ignored` (`ccm.clash.ignored`), `ignoreFile`, `unignoreFile`, `takeNew` (Set `reported` khoá cặp + `turnStart`), prune ignore theo `terminals.live` |
| `apps/web/src/stores/settings.ts` | `clashWarn` |
| `apps/web/src/components/settings/index.ts` | entry `{ page: 'notify', id: 'clash', needs: 'terminals' }` |
| `apps/web/src/components/settings/SettingsNotify.vue` | `ToggleSwitch` `[data-setting-row="clash"]` cuối section Sự kiện |
| `apps/web/src/lib/next/railLive.ts` | `RowClash`, `LiveExtras.clash`, `LiveRow.clash` |
| `apps/web/src/components/next/rail/LiveRail.vue` | `clashOf` (tooltip max 3 file + "+n") |
| `apps/web/src/components/next/rail/RailRowItem.vue` | chip `[data-clash-chip]` (icon `FileWarning` + "Đụng file" / "Đụng {n} file", token `warning`), tooltip + aria-label của nút phủ dòng |
| `apps/web/src/components/workspace/ClashChip.vue` (mới) | chip header + popover `[data-clash-pop]`: mỗi file "Xem diff", "Bỏ qua file này"; "Tách worktree" mở `WorktreeMenu` có sẵn; container < 520px chỉ còn icon + số |
| `apps/web/src/components/workspace/TermPane.vue` | `<ClashChip v-if="isNext && pane.terminalId">` sau `RemotePhoneChip` |
| `apps/web/src/composables/useClashActions.ts` (mới) | `viewDiff` (`openChanges(cwd chứa file, { select: path tuyệt đối })`, không có cwd nào chứa file thì `openFile`), `ignore` (+ toast Undo 5 s `[data-clash-undo]`), `fileName`, `clashToastId` |
| `apps/web/src/composables/useNotifications.ts` | `NotifyKind` `'clash'`, `enabled('clash')`, watch `touches.clashes` → `takeNew()` → toast `warning` `[data-clash-toast]` + `notify(null, 'clash', …)` |
| `apps/web/src/i18n/v104-ux.ts` | `v104.clash.*` en + vi (thêm `more`, `listTitle`, `toastDetail`, `undo`, `worktreeHint` ngoài bảng SPEC 6.6) |
| `apps/desktop/e2e/v104-clash.cjs` (mới), `run.cjs`, `scripts/qa-fast.mjs::E2E_MAP` | E2E cổng 4481 / 9481, `CCM_DATA_DIR` tạm |
| `.code_index/web.md`, `.code_index/_meta.md` | mục WP-104d |

## Test
- Unit: `npm test -w @ccm/web -- src/lib/clash.test.ts src/lib/agentTouches.test.ts src/stores/agentTouches.test.ts src/components/settings src/lib/next/railLive.test.ts`: 11 file, 100 test pass. Toàn bộ web: 317 file, 2735 test pass.
- `npm run typecheck -w @ccm/web`: sạch.
- E2E `node apps/desktop/e2e/v104-clash.cjs` (Tauri dev, UI next, vi): 22/22 PASS, chạy 2 lần liền đều sạch. Kịch bản: switch tắt không chip / toast (Explorer vẫn `conflict`); bật qua Settings; chip trên 2 dòng rail + 2 header; toast đúng 1; cùng lượt không toast lại; lượt mới của 1 bên toast thêm 1; "Bỏ qua file này" tắt chip + toast, lưu `ccm.clash.ignored`, Undo trả lại; popover header có danh sách + "Tách worktree", nằm trong cửa sổ; "Xem diff" mở Changes chọn đúng `src/a.ts`; tắt switch ẩn hết.

## Ảnh (`docs/qa/v104/WP-104d/`)
`v104d-01-chips-{dark,light}.png` (rail + header + toast), `v104d-02-popover-{dark,light}.png`, `v104d-03-diff-{dark,light}.png`.

## Lệch SPEC / quyết định nhỏ
- Toast trong app chỉ tôn trọng giờ yên lặng, không phụ thuộc `notifyEnabled` (như toast `context-warn`); `Notification` web khi không focus vẫn qua `notify()` nên cần `notifyEnabled`. Lý do: `notifyEnabled` cần quyền Notification của OS, gắn vào thì switch bật mà không thấy gì.
- Không thêm `ToastKind` OS / core (Q1 đóng). Ở vỏ Tauri, `Notification` của webview khi app ở nền chưa đo.
- `clashesByTerminal` nhận danh sách `Clash[]` (đã lọc ignore) thay vì `TouchIndex`.
- Nhóm Inbox "Đụng file" chưa làm: thuộc WP-104e (`InboxInput.clashes` đọc `useAgentTouchesStore().clashes`, đã rỗng khi switch tắt).
- Switch hiện cả ở classic (trang Thông báo dùng chung, `rows.test.ts` kiểm cả 2); classic chỉ có toast, không chip.

## Rủi ro còn lại
- Báo nhầm khi 2 agent sửa 2 đoạn khác nhau hợp lệ (chỉ cảnh báo, mặc định tắt).
- Terminal đã `working` từ trước khi store khởi tạo có `turnStart` rỗng: cặp đó chỉ báo lại khi 1 bên qua lượt mới.
- "Xem diff" với file không thuộc git nhưng nằm trong cwd của session: mở Changes (báo lỗi git) thay vì mở file.
- `reported` chỉ trong memory: reload app thì cặp đang đụng báo lại 1 lần.

## QA tay Windows
Chạy qua E2E Tauri dev (ảnh ở trên). Chưa thử với 2 session Claude thật cùng sửa 1 file.

## Test macOS
- [ ] Settings → Thông báo: dòng "Cảnh báo 2 agent sửa cùng file" mặc định tắt.
- [ ] Bật, 2 session Claude cùng project sửa 1 file: chip "Đụng file" trên 2 dòng rail + 2 header pane, toast 1 lần.
- [ ] Đưa Orrery xuống nền rồi tái hiện: có `Notification` của webview Tauri không (Q1); không có thì ghi lại.
- [ ] Popover header: "Xem diff" mở Changes đúng file; "Bỏ qua file này" tắt chip, Undo trả lại; "Tách worktree" mở menu worktree.
- [ ] Đường dẫn macOS (case-sensitive / `/Users/...`): tên file tương đối đúng trong toast và tooltip.
