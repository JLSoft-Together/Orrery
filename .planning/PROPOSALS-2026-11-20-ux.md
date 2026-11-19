# PROPOSALS 2026-11-20: UI/UX, clean, tính năng

Nghiên cứu chỉ đọc trên nhánh `feat/v98-pwa` (sau v103, UI next mặc định). Mỗi đề xuất có bằng chứng `file:line`. Không đụng Office (đang dừng). Ràng buộc giữ nguyên: chỉ quan sát, không gọi API LLM, không blur ở classic, mọi chuỗi qua i18n.

Ký hiệu nhóm: (a) UI/UX, (b) clean, (c) tính năng.

---

## 1. Gộp các lối "cần bạn" thành 1 Inbox (a)

**Vấn đề.** Cùng một câu hỏi "agent nào đang chờ mình" có 5 lối vào, 2 phím tắt khác nhau:
- Chip "Cần bạn" + popover, phím Ctrl+Shift+A: `apps/web/src/components/workspace/WorkspaceView.vue:2043-2056`, `AttentionPopover.vue:41`.
- Ngay cạnh đó chip "Chờ trả lời" (menu v100), phím Ctrl+Alt+Q: `WorkspaceView.vue:2058` (`MenuQueueChip`), `stores/menuQueue.ts`, `lib/shortcuts.test.ts:51`.
- Badge chuông trên cụm project ở rail: `components/workspace/rail/RailClusterView.vue:212-215`, `WorkspaceView.vue:1817`.
- Trạng thái chờ trên từng dòng rail next: `components/next/rail/LiveRail.vue:66-71` + hoãn (`lib/next/railSnooze`).
- Cột `needs` trên Monitor board: `lib/boardV95.ts:10`, `monitor/ModeBar.vue:23`; nhóm `needs` trong Ctrl+K: `CommandPalette.vue:663`.
Hai chip đứng sát nhau nhưng đếm 2 tập khác nhau (permission / câu hỏi / lỗi vs menu chọn), user phải tự hiểu sự khác biệt. v103 #3 chủ nhân đã bỏ hộp "việc cần" góc trái, tức là đã thấy thừa.

**Đề xuất.** Một chip "Inbox (N)" duy nhất trên thanh tab, popover chia nhóm: Chờ trả lời menu, Xin quyền, Câu hỏi, Lỗi, Xong chưa xem. Một phím tắt (giữ Ctrl+Alt+Q = nhảy tới mục kế, Ctrl+Shift+A = mở danh sách), `Enter` mở pane, `S` hoãn (dùng lại `railSnooze`). Rail và board chỉ hiện chấm trạng thái, không đếm riêng. Giống Linear Inbox / GitHub notifications: một hàng đợi, có thứ tự theo thời gian chờ.

**Lợi ích.** Người chạy 6-10 agent chỉ nhìn một con số; bớt 1 chip trên thanh tab vốn đã chật.
**Công sức.** M (gộp `needs` + `menuQueue` thành 1 selector, 1 component popover, sửa E2E `v100-menu-queue.cjs`).
**Rủi ro.** Đụng luồng trả lời menu v100 (ngoại lệ thứ 6 trong CLAUDE.md): phải giữ nguyên "mọi phím đều sau một click của chủ nhân". Thói quen phím cũ đổi nghĩa.

---

## 2. Settings: sửa search trỏ vào dòng bị ẩn ở UI next, bỏ setting trùng (a)

**Vấn đề.**
- Registry search khai báo `fileIcons`, `dust`, `starsPanels`, `starsFx` thuộc trang `appearance` (`components/settings/index.ts:100-101, 106-107`), nhưng ở UI next các section đó bị ẩn (`SettingsAppearance.vue:84` và `:141` `v-if="... !uiGen.isNext"`). Gõ "file icons" ra kết quả, bấm vào thì `flash()` không tìm thấy dòng và im lặng return (`components/SettingsMenu.vue:187-188`).
- `fileIcons` và `dust` có 2 bản: `SettingsAppearance.vue:142-145` và `SettingsWorkspace.vue:56-58`. Trang `workspace` có trong `SETTINGS_PAGES` (`index.ts:3`) nhưng không có entry nào trong registry, nên search không bao giờ dẫn tới đó.
- 15 trang settings (`index.ts:3`), 158 khoá `ccm.*` trong web. Trang `reminders` (stuck, contextWarn, budget) chồng ý với `notify` (event `stuck`, `context`, `idle`) (`index.ts:118-149`); entry `about/bugReportNudge` nằm lẫn giữa các dòng `reminders` (`index.ts:148`).

**Đề xuất.** (1) Registry khai báo theo thế hệ UI (`ui?: 'next' | 'classic'`) và test: mọi entry hiển thị phải có `[data-setting-row]` thật trên trang đích (unit test mount từng trang). (2) `fileIcons` / `dust` chỉ ở trang Workspace. (3) Gộp `reminders` vào `notify` thành nhóm "Nhắc nhở"; trang DeepSeek ẩn hẳn khi tắt feature (đã gated qua `needs: 'deepseek'`, giữ). Mục tiêu 12 trang. Giống VS Code: search settings luôn mở đúng dòng.

**Lợi ích.** Hết lỗi "bấm kết quả không có gì xảy ra"; ít chỗ để tìm một switch.
**Công sức.** S (sửa registry + test) / M nếu gộp trang.
**Rủi ro.** Thấp; khoá `ccm.settings.page` cũ trỏ tới `reminders` cần map sang `notify`.

---

## 3. Lộ trình gỡ UI classic, bắt đầu từ E2E (b)

**Vấn đề.**
- App mặc định next từ v103 nhưng E2E mặc định vẫn classic: `apps/desktop/e2e/ui-gen.cjs:18` (`env.CCM_E2E_UI || 'classic'`). Chỉ 5 / ~90 file E2E chạy `ui: 'next'` (vd `v103-polish.cjs:13`). Suite đang canh giữ giao diện sắp bỏ.
- E2E hỏng vì Mission Control đã bỏ: `v51-compact.cjs:51-55`, `v25.cjs:207-209` (chờ `[data-mission]`), handoff 2026-11-20 mục 3.5 đã ghi.
- Nhánh classic/next rải khắp: `next/tokens.css` 214 chỗ, `WorkspaceView.vue` 43 (vd `:139`, `:1769`, `:1796`, `:2316`, `:2327`, `:2351`), `CommandPalette.vue` 23, `NextShell.vue` 22, `DeviceUiTestDialog.vue` 18, `TermPane.vue` 13; 2 component Appearance song song (`SettingsAppearance.vue:72-84`). `WorkspaceView.vue` 2440 dòng, `stores/workspace.ts` 1994 dòng.

**Đề xuất.** 3 bước:
1. Ngay: đổi default `ui-gen.cjs` sang `next`, chạy cả suite, sửa / xoá `v51-compact.cjs`, `v25.cjs`; E2E nào chỉ có ý nghĩa với classic thì gắn `--ui=classic` rõ ràng.
2. Bản kế: classic đóng băng (chỉ fix vỡ), thêm dòng changelog báo trước.
3. Sau 1-2 bản (chủ nhân đã chốt): xoá `isNext` branch, `uiGen` chỉ còn palette / motion, gộp 2 Appearance, tách `WorkspaceView.vue` (tab strip, divider, explorer cũ, drop overlay cũ) xuống dưới ~1200 dòng.

**Lợi ích.** Mỗi tính năng UI chỉ làm 1 lần; E2E bắt lỗi trên UI người dùng thật thấy.
**Công sức.** Bước 1: S. Bước 3: L.
**Rủi ro.** Người dùng còn ở classic mất giao diện quen; một số tính năng chỉ có ở classic (chuông `otherNeeds` `WorkspaceView.vue:1817`, explorer cũ `:2327`) phải kiểm đủ tương đương ở next trước khi xoá.

---

## 4. Gỡ vỏ Electron và các fallback Node (b)

**Vấn đề.**
- DECISIONS 2026-11-12: "Vỏ chỉ còn Tauri (4.0.0 bỏ Electron)" (`.planning/DECISIONS.md:41`); `npm run dist:win|mac` đã chạy `scripts/dist-tauri.mjs` (`package.json`). Nhưng `apps/desktop/src` (~54.8k dòng TS) vẫn được sửa song song: v103 sửa `apps/desktop/src/preview/gitImage.ts`, `preview/ipc.ts` cùng bản Rust (commit `365bbc21`); 33 file test `*.twin.test.ts` giữ 2 bản khớp nhau.
- E2E mặc định launch Electron, Tauri chỉ khi `CCM_E2E_SHELL=tauri` (`apps/desktop/e2e/cdp.cjs:14, 82`).
- Fallback Node cho từng lõi: `CCM_CORE` (`apps/desktop/src/core-serve-host.ts:41`), `CCM_CORE_PTY` (`main.ts:189`), `deskMode` (`main.ts:865, 930`, `desk/forward.ts:22, 65`), kéo theo collector Node (~14.7k dòng) phải giữ.
- `CLAUDE.md` (Architecture, Commands) vẫn mô tả Electron là shell chính, dễ làm agent sửa nhầm chỗ.

**Đề xuất.** Kiểm Tauri đã đủ tính năng (bảng feature Electron vs `apps/shell` từ `.code_index/desktop.md` + `shell.md`), rồi: E2E default `tauri`; xoá `apps/desktop/src` trừ `preload.ts` (vẫn là contract cho `scripts/gen-desk-shim.mjs`) và phần TS mà Rust còn gọi (vd `collector/src/toast.ts` qua `serve-toast.ts`); bỏ biến `CCM_CORE_*=node`; viết lại phần Electron trong CLAUDE.md. Giữ tag cuối có Electron để rollback.

**Lợi ích.** Bớt khoảng 50-70k dòng bảo trì kép, bớt 33 twin test, build / qa nhanh hơn, mọi sửa đổi desktop chỉ làm 1 lần.
**Công sức.** L (chia 2-3 gói: kiểm khoảng trống, chuyển E2E, xoá).
**Rủi ro.** Cao: macOS chưa test kỹ Tauri (checklist `.planning/QA-mac-checklist.md` còn treo); mất đường thoát khi lõi Rust lỗi trên máy lạ. Nên làm sau khi có một bản Tauri chạy ổn trên Mac.

---

## 5. Dọn code chết, DeepSeek đang dừng, và `.code_index` phình to (b)

**Vấn đề.**
- 10 component không còn ai import (1502 dòng): `ActivityFeed.vue`, `ToolTimeline.vue`, `MetricsStrip.vue`, `WaitingQueue.vue`, `SessionList.vue`, `SessionSummary.vue`, `FlipClock.vue`, `monitor/OrbitRings.vue`, `next/ui/GlassDialog.vue`, `workspace/DevServerChip.vue` (tất cả trong `apps/web/src/components/`). `.code_index/web.md:31` ghi chúng "stay unused" từ v79.
- Comment còn nhắc Mission Control đã bỏ: `components/workspace/DropTerminal.vue:21`, `lib/paneSnapshots.ts:1`.
- DeepSeek dừng từ 2026-11-03 (`CCM_DEEPSEEK_FEATURE` mặc định tắt) nhưng còn 55 file web tham chiếu, `SettingsDeepSeek.vue` 18.5 KB, `apps/desktop/src/deepseek/` ~1.1k dòng, import ở `LiveRail.vue:16`.
- `.code_index/` tổng 761 KB (`web.md` 172 KB, `_meta.md` 137 KB, `desktop.md` 134 KB) trong khi CLAUDE.md quy định `_meta.md` "1-2 dòng mỗi thay đổi, không changelog dài". Agent đọc index trước mỗi task nên đây là chi phí token thật.

**Đề xuất.** (1) Xoá 10 component + `ui` prefs đi kèm (`stores/ui.ts:78, 342` `timelineWindow` nếu chỉ `ToolTimeline` dùng). (2) DeepSeek: chọn 1 trong 2, hoặc tách thành 1 module lazy (`features/deepseek/`) cho dễ gỡ, hoặc xoá hẳn và giữ trong git tag. Cần chủ nhân chốt. (3) Index: `_meta.md` cắt phần trước v90 sang `.code_index/archive/`, mỗi file chính giữ dưới ~40 KB, tách theo khu vực (`web-workspace.md`, `web-monitor.md`, `web-settings.md`).

**Lợi ích.** Ít nhiễu khi grep, index rẻ token hơn nhiều lần.
**Công sức.** S (component chết, comment) / M (index, DeepSeek).
**Rủi ro.** Thấp với component chết (typecheck + vitest bắt). DeepSeek: mất đường bật lại nhanh nếu xoá.

---

## 6. Cảnh báo 2 agent cùng sửa 1 file, đưa lên rail và header pane (c)

**Vấn đề.** Dữ liệu đã có: `lib/agentTouches.ts:64-97` tính `state: 'conflict'` khi ≥ 2 session sửa cùng file trong lượt hiện tại (từ `tool.span` path + access, ngoại lệ đã duyệt ở CLAUDE.md). Nhưng chỉ hiện trong Explorer tree (`components/workspace/ExplorerTree.vue:120, 394, 442`), tức là phải mở Explorer đúng thư mục mới thấy. Cảnh báo ở mức checkout (`lib/conflicts.ts:13`) chỉ hiện trên `SessionCardSignals.vue:61` của Monitor, không có ở rail next. Không có notification cho tình huống này.

**Đề xuất.** Selector `conflictsByTerminal` từ `useAgentTouchesStore` → (1) chip cảnh báo trên dòng rail + header pane: "Đụng file với <session>: `a.ts`" (icon + text + màu `warning`); (2) đưa vào Inbox (đề xuất 1) nhóm "Đụng file"; (3) kind notify mới `clash`, mặc định bật, dùng toast có sẵn; (4) bấm vào mở diff file đó. Chỉ đọc dữ liệu có sẵn, không tốn token. Gợi ý trong chip: "tách worktree" (đã có `wt.add`).

**Lợi ích.** Đây là lỗi đắt nhất khi chạy nhiều agent song song trên 1 checkout: ghi đè nhau mà không biết. Zed / Cursor chưa có tín hiệu này giữa các phiên độc lập.
**Công sức.** S-M.
**Rủi ro.** Báo nhầm khi 2 agent sửa cùng file ở 2 đoạn khác nhau một cách hợp lệ: chỉ cảnh báo, không chặn; có nút "bỏ qua cho file này".

---

## 7. Changes theo session: "lượt này agent đã sửa gì" (c)

**Vấn đề.** Changes pane chỉ chia "Chưa xem" theo repo (`lib/reviewQueue.ts:15, 72`, `ChangesPane.vue:132-139`), không biết file nào do session nào sửa. Khi 3 agent cùng làm 1 repo, review sau khi một agent báo "done" phải tự lọc bằng mắt. `agentTouches` đã có `turnStart` / `idleSince` theo terminal (`stores/agentTouches.ts:12, 31-32`) và path sửa theo span.

**Đề xuất.** Trong Changes pane thêm bộ lọc "Theo session" (segmented: Tất cả, <session A>, <session B>, Không rõ), giao dữ liệu git status với tập path `edit` của session trong lượt gần nhất. Khi notify `done` (`composables/useNotifications.ts:34`) kèm nút "Xem thay đổi" mở Changes đã lọc theo session đó. Rail menu chuột phải: "Xem thay đổi của lượt này". Giống Warp blocks / Zed agent diff review, nhưng ghép từ dữ liệu quan sát.

**Lợi ích.** Review nhanh từng agent, ít bỏ sót file; nối thẳng từ "agent xong" sang "duyệt diff".
**Công sức.** M.
**Rủi ro.** File do Bash (sed, codegen, formatter) sửa không có `tool.span` path: nhóm "Không rõ" phải luôn hiện, không được giấu. Span cũ hết hạn sau `AFTER_TURN_MS`: cần giữ tập path lượt cuối tới khi user đánh dấu đã xem.

---

## Thứ tự ưu tiên

| # | Đề xuất | Nhóm | Công sức | Lý do xếp |
|---|---|---|---|---|
| 1 | 3, bước 1: E2E sang next + sửa E2E hỏng | b | S | Rẻ, mở đường cho mọi việc UI sau |
| 2 | 2: Settings search + setting trùng | a | S | Lỗi thật đang có ở UI mặc định |
| 3 | 6: Cảnh báo đụng file | c | S-M | Dữ liệu sẵn, giá trị cao nhất cho nhiều agent |
| 4 | 1: Inbox gộp | a | M | Bớt rối rõ nhất trên thanh tab |
| 5 | 5: Dọn code chết + index | b | S-M | Rẻ, giảm token mỗi phiên |
| 6 | 7: Changes theo session | c | M | Nên làm sau 6 (dùng chung selector) |
| 7 | 3, bước 3 + 4: gỡ classic, gỡ Electron / Node | b | L | Lợi lớn nhất về lâu dài nhưng rủi ro cao, chờ Tauri ổn trên Mac |

Cần chủ nhân chốt: DeepSeek xoá hẳn hay giữ lazy; mốc gỡ classic (bản nào); có làm notify kind `clash` mặc định bật không.
