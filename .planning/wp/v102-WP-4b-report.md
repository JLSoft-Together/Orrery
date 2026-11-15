WP-4b: ĐẠT (cần xem bằng mắt + chạy E2E drag ở next)

File:
- Mới: `lib/next/dropCompass.ts`, `lib/layoutSnap.ts`, `lib/layoutPresets.ts`, `lib/layoutUndo.ts`, `lib/layoutSplitHint.ts`, `lib/layoutPaneMenu.ts` (+ 6 file `*.test.ts`), `stores/workspaceLayout.test.ts`, `components/next/canvas/{DropOverlay,DragGhost,LayoutMenu,SplitHint,FileDropLayer}.vue`, `components/next/canvas/useNextLayout.ts`.
- Sửa: `components/workspace/WorkspaceView.vue` (kéo thả, divider, nút Bố cục, banner, phím `detachTab`), `composables/usePaneDrag.ts` (option `compass` / `minPane` / `spring`, nguồn pane `rail`, `RAIL_PANE_DRAG`), `stores/workspace.ts` (`setLayout`, `restoreLayout`), `lib/workspaceKeys.ts` (+1 action), `lib/shortcuts.ts` (`detachKeys`, `detachShortcutText`), `components/workspace/TermPane.vue` (3 dòng: inject `PANE_LAYOUT_MENU` vào menu "..."), `i18n/mist-v102.ts` (khối `mist.drag` en + vi), `next/tokens.css` (khối drag cuối file), `.code_index/web.md` + `_meta.md`.

SPEC ID đã làm:
- D1 la bàn 5 ô 44px giữa pane đích (ô đang chọn accent, ngoài ô thì theo cạnh gần nhất như cũ), nhãn "Chia phải, 50%".
- D2 nút Bố cục ở tab strip: 1 pane (= zoom pane focus), 2 cột, 3 cột, 2 x 2, 1 lớn + cột, Trên + 2 dưới, Chia đều; chỉ đổi cây (`setLayout` kiểm cùng tập pane); tab có group project thì preset tắt kèm lý do, preset làm pane < `MIN_PANE` thì tắt.
- D3 kéo pane lên tab pill khác giữ 600ms (thanh dưới pill, scaleX) thì chuyển tab, đo lại rect, thả tiếp bằng la bàn.
- D4 file từ Explorer: ô la bàn nhận drop, mở viewer đúng cạnh (giữa = tab mới; file đã mở thì chuyển pane đó tới); phần còn lại của terminal vẫn đính kèm file như cũ. Row rail: hook `RAIL_PANE_DRAG` sẵn, chưa nối trong rail (xem dưới).
- D5 chia (accent 12% + viền 2px), đổi chỗ (viền nét đứt quanh 2 pane), quá hẹp (đỏ, "Quá hẹp: còn Npx, tối thiểu 220px", action null nên thả không đổi layout).
- D6 ghost pill kính 40px (agent + tên + Chip trạng thái), pane gốc phủ canvas 62% + "Đang kéo pane này".
- D7 divider hit 12px (`::before`), hover / kéo vạch accent 2px + grip 3 chấm. D8 nhãn "62 / 38" khi kéo, hít 1/3 1/2 2/3 trong 12px, Alt bỏ hít, dblclick 50/50, phím mũi tên + giới hạn 15-85% giữ nguyên.
- D9 toast "Đã chia pane" / "Đã đổi chỗ 2 pane" / ... + Hoàn tác 5s sau thả, preset, chia đều, tách; không gắn Ctrl+Z; `registerLayoutUndo` cho Ctrl+K. Undo = snapshot cây tab, chỉ chạy khi layout chưa đổi thêm và tập pane giữ nguyên, pane không remount.
- D10 banner tách tab (>= 5 pane, pane < 360 rộng hoặc < 180 cao), Tách / Bỏ qua, nhớ theo tab ở `ccm.next.splitHintSkip`.
- D11 menu "..." của terminal liệt kê Chia phải / dưới, Sang pane, Đổi chỗ, Phóng to, Tách sang tab mới, Đóng kèm chuỗi phím từ `lib/shortcuts.ts` theo mac/win. Phím mới Ctrl+Alt+T / ⌥⌘T (bỏ qua AltGr), không trùng phím cũ; chạy cả classic, không thêm vào danh sách help nên UI classic không đổi.
- M7 drop zone / la bàn / ghost chỉ transform + opacity, theo `data-motion` và `prefers-reduced-motion`.

Chưa làm / lệch:
- D5 dùng 220px cho cả chiều ngang và dọc đúng chữ SPEC / mockup (classic split bằng phím vẫn 140 cho chiều dọc).
- Thả file tạo pane mới nên không có toast hoàn tác (undo chỉ cho thao tác xếp lại).
- Menu "..." của 7 viewer (WP8) chưa có D11: WP8 `inject(PANE_LAYOUT_MENU)` y như TermPane (`entries(key)` nối vào cuối, `pick(key, id)` trả true là đã xử lý).
- Row rail kéo ra canvas: rail (WP3) cần gọi `inject(RAIL_PANE_DRAG)?.(e, paneKey, el)` ở pointerdown của row khi next; nguồn chỉ bắt đầu khi con trỏ vào canvas, nhưng `useRailDrag` cũng đang theo dõi cùng lần kéo, main cần quyết khi nào rail dừng theo dõi (ví dụ rời rail thì `useRailDrag` huỷ).

Typecheck: pass (`npm run typecheck -w @ccm/web`).
Test: 24 file / 263 pass / 0 fail (`npx vitest run --root apps/web src/lib/next src/lib/layout*.test.ts src/lib/workspaceKeys.test.ts src/lib/shortcuts.test.ts src/lib/splitLayout.test.ts src/components/workspace src/components/next src/stores/workspace.test.ts src/stores/workspaceLayout.test.ts src/i18n`). Không chạy E2E.

Rủi ro / việc cho gói sau:
- E2E classic: selector `[data-drop-preview]` (+ `data-edge`), `[data-drag-ghost]`, divider, caret không đổi. Ở next, `pane-drag.cjs` check "ghost names the pane and the action" (regex có "·") sẽ fail vì ghost next không ghi hành động (nhãn nằm trên vùng preview); các check còn lại dùng được ở next.
- `usePaneDrag` spring chỉ bật khi `compass()` true (next) và nguồn là pane; classic không gọi `elementFromPoint`.
- Chưa chụp ảnh preview chia / swap / quá hẹp / divider / menu Bố cục / banner.
