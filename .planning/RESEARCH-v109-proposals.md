# RESEARCH v109: 12 đề xuất (UI/UX, chức năng đã có, chức năng mới)

Ngày 22/11/2026, nhánh `feat/v98-pwa`. Trang chọn: `.planning/wireframes/v109-proposals.html` (offline, nút Làm / Để sau / Bỏ + Chép lựa chọn).
Đã loại trùng với: v104 (Inbox, Settings, đụng file, Changes theo session, DeepSeek lazy), v105-v107 (U1-U17, gọn UI 128 phần tử, chat điện thoại), v108 (màu), `proposals.json` #1-#7. Office tạm dừng: không đề xuất.

Đã kiểm là **có sẵn** nên không đề xuất: worktree (so sánh, merge ff / squash, gửi cùng prompt cho 2 worktree: `components/workspace/WorktreeMenu.vue`, `lib/preview.ts:290-306`), checkpoint + hoàn tác lượt (`TurnCard.vue:75`, `CheckpointsPanel.vue:59`, `TurnReview.vue`), layout đã lưu + mở khi khởi động (`lib/projects.ts:35`), focus mode (`lib/focusMode.ts`), quiet hours (`useNotifications.ts:116`), tìm prompt cũ (`components/history/PromptSearchCard.vue`), thời gian chờ chủ nhân trong History (`HistoryView.vue:86`), push điện thoại chỉ khi rời máy N phút (`core-desk/src/phone/mod.rs:19`), tìm trong 1 terminal (`TermFind.vue`).

Nguyên tắc: bớt hơn thêm, mượt trước; không gửi prompt, không gọi API, mọi thứ local. Mục nào điền chữ vào pane agent thì chỉ **điền sẵn**, chủ nhân tự bấm Enter (như `open_session` mặc định).

Công sức: S dưới 1 ngày, M 1-3 ngày, L trên 3 ngày.

## Nhóm A: UI/UX

### A1 Menu project gọn: gom công cụ, ẩn cái không hợp (S)
- Vấn đề: menu "..." của project có tới 7 mục công cụ ngang hàng (Run, Scripts, Plan, Env, i18n, Quyền, Build clean) cộng Layouts, Explorer, Copy path, Reveal, Move up/down, Gập, Đóng (`lib/rail/railMenu.ts:54-81`). Env / i18n / Quyền hiện cho mọi project chỉ cần bridge có hàm (`components/next/rail/LiveRail.vue:508-511`: `env: hasEnvCheck`, `i18n: hasI18nScan`, `perms: hasPermAdvisor`), project web cũng thấy công cụ Android.
- Đề xuất: 1 mục "Công cụ dự án ›" chứa các công cụ; chỉ hiện công cụ khớp loại project (Android / Flutter: i18n, Quyền, APK; Node: Scripts; có gì chạy được: Run). Menu chính còn khoảng 8 dòng.
- Rủi ro: chủ nhân quen vị trí cũ; nhận diện loại project sai thì mất lối vào. Giảm: khi không chắc thì vẫn hiện trong submenu; Ctrl+K vẫn tìm được mọi công cụ.
- File: `lib/rail/railMenu.ts` (`projectMenu`), `components/next/rail/LiveRail.vue:500-515`, `lib/rail/railMenu.test.ts`, i18n.

### A2 Gộp thông báo dồn dập thành 1 (S)
- Vấn đề: throttle theo cặp `terminal|kind` (`composables/useNotifications.ts:112-117`), nên 4 session xong gần nhau ra 4 thông báo OS + 4 toast (toast tối đa 3 cùng lúc, `stores/toasts.ts:40`). Đúng loại "ngộp" chủ nhân chê.
- Đề xuất: gom trong cửa sổ 8 giây: 1 thông báo "3 session xong, 1 chờ bạn", bấm mở Inbox. Chỉ 1 sự kiện thì giữ như cũ. Mục "chờ bạn" vẫn đứng đầu.
- Rủi ro: chậm 8 giây với thông báo đầu tiên. Giảm: thông báo đầu ra ngay, các cái sau trong 8 giây cập nhật cùng `tag` (Notification thay thế, không bật cái mới).
- File: `composables/useNotifications.ts` (`notify`), `stores/toasts.ts` (`push` thêm `group`), test `useNotifications`.

### A3 Badge số "chờ bạn" trên taskbar / Dock (S)
- Vấn đề: vỏ Tauri chưa có badge / overlay icon / nháy cửa sổ (`rg "set_overlay_icon|set_badge|request_user_attention" apps/shell/src` = 0). Hiện chỉ đổi `document.title` (`useNotifications.ts:66-77`), Windows taskbar gộp nhóm thì không thấy.
- Đề xuất: Windows overlay icon có số (1-9, 9+), macOS badge Dock; khi có mục "chờ bạn" mới mà cửa sổ không focus thì nháy taskbar 1 lần (`request_user_attention` Informational). Không cần mở app vẫn biết có việc.
- Rủi ro: overlay icon cần vẽ PNG số theo DPI; quiet hours phải tắt cả nháy.
- File: `apps/shell/src/window.rs`, `ccm.rs` (kênh `ccm:badge`), web `composables/useNotifications.ts` (đếm sẵn `n`), `stores/inbox.ts`.

### A4 Ctrl+Tab về tab vừa dùng (MRU) (S)
- Vấn đề: Ctrl+Tab đi theo thứ tự tab (`lib/workspaceKeys.ts:109` `cycleTab`, `lib/shortcuts.ts:82`), qua lại giữa 2 tab đang làm phải bấm nhiều lần khi có 6-8 tab. Store đã có `lastFocused` (`stores/workspace.ts:1644`).
- Đề xuất: Ctrl+Tab như trình duyệt / VS Code: nhả nhanh = về tab trước đó; giữ Ctrl thì hiện list nhỏ theo thứ tự dùng gần nhất. Ctrl+PageUp / PageDown giữ đi theo thứ tự.
- Rủi ro: đổi hành vi quen; terminal bắt Ctrl+Tab. Giảm: dòng "What's new"; switch ở Settings → Workspace nếu cần.
- File: `lib/workspaceKeys.ts`, `stores/workspace.ts` (lịch sử focus max 20), `components/workspace/WorkspaceView.vue` (list nhỏ), `lib/changelog.ts`.

## Nhóm B: Chức năng đã có

### B1 Quota: dự báo giờ hết (S)
- Vấn đề: thanh quota chỉ có % đã dùng + giờ reset (`lib/next/quotaBar.ts:23-35`, `QuotaWindowView { used, tone, resetsAt }`). Không biết với nhịp hiện tại có kịp tới reset không.
- Đề xuất: giữ các mẫu `usedPct` trong RAM (max 2 giờ), tính tốc độ; nếu dự báo chạm 100% trước reset thì popover ghi "Hết khoảng 15:40" và thanh chuyển `warning` sớm. Không đủ mẫu (dưới 15 phút) thì không hiện, không bịa số.
- Rủi ro: dùng không đều làm dự báo nhảy. Giảm: trung bình trượt 30 phút, chỉ hiện khi tốc độ ổn định.
- File: `lib/next/quotaBar.ts` (`forecast`), `components/next/NextQuotaBar.vue`, test `quotaBar.test.ts`.

### B2 Góp ý trên diff, điền sẵn cho agent (M)
- Vấn đề: xem diff trong Changes / Turn review xong muốn bảo agent sửa thì phải tự gõ lại tên file + dòng vào terminal. Conductor có bình luận nhiều dòng trên diff gửi lại agent ([conductor.build/docs](https://www.conductor.build/docs/index), [The New Stack](https://thenewstack.io/a-hands-on-review-of-conductor-an-ai-parallel-runner-app/)).
- Đề xuất: chọn dòng trong `DiffView` / `SplitDiff` → ô góp ý; nút "Gửi cho agent" gom mọi góp ý thành 1 đoạn (`path:line` + câu) và **điền sẵn** vào pane agent đã sửa file đó (dùng `prefill` có sẵn, `stores/workspace.ts:131`). Không submit.
- Rủi ro: chọn nhầm pane khi 2 agent sửa cùng file: hỏi chọn pane. Góp ý chỉ ở RAM, đóng diff thì hỏi giữ.
- File: `components/workspace/DiffView.vue`, `SplitDiff.vue`, `TurnReview.vue`, `stores/workspace.ts` (`prefill`), `stores/turnEdits.ts` (biết pane nào sửa), i18n.

### B3 Shell pane: báo khi lệnh dài xong + mã thoát (M)
- Vấn đề: pane shell không biết lệnh bắt đầu / kết thúc (không có OSC 133 / 633: `rg "133;|633;" apps/shell/src apps/core/crates/core-pty apps/web/src` = 0). Build 5 phút xong không ai báo; lỗi hay thành công phải nhìn chữ.
- Đề xuất: shell integration nhẹ (PowerShell `prompt`, zsh `precmd` / `preexec`) phát OSC 133 A/B/D; Orrery đọc trong stream: lệnh trên 30 giây xong mà pane không focus thì thông báo "build xong, mã 0" / "lỗi, mã 1"; tab có chấm xanh / đỏ. Kèm nhảy giữa các lệnh trong scrollback.
- Rủi ro: chèn script vào shell của chủ nhân. Giảm: chỉ trong pane do Orrery mở, qua env / tham số khởi động (như `presets.rs child_env`), không sửa profile; switch tắt được.
- File: `apps/shell/src/presets.rs`, `apps/core/crates/core-pty` (parser OSC), `components/workspace/TermPane.vue`, `composables/useNotifications.ts` (kind `job` sẵn có).

### B4 Đổi phím tắt (M)
- Vấn đề: danh sách phím chỉ để xem (`lib/shortcuts.ts`), phím cứng trong `lib/workspaceKeys.ts`. Ctrl+Shift+A, Ctrl+Alt+Q, Ctrl+Shift+B có thể đụng phím của app khác / IME tiếng Việt.
- Đề xuất: Settings → Phím tắt: bấm vào dòng, nhấn tổ hợp mới, báo trùng; Khôi phục mặc định. Lưu `ccm.keys`.
- Rủi ro: phím trong terminal (xterm) phải đi chung 1 bảng; Mac / Windows khác nhau. Giảm: chỉ cho đổi các lệnh toàn cục trước.
- File: `lib/shortcuts.ts`, `lib/workspaceKeys.ts`, `lib/paletteKey.ts`, `components/settings/*`, i18n.

## Nhóm C: Chức năng mới

### C1 Chặn lộ secret trước commit / push (S-M)
- Vấn đề: agent hay ghi key / token vào file cấu hình; `CommitBox.vue:39-43` commit thẳng `gitCommit(root, message, amend)`, không kiểm. Orrery đã có bộ che secret cho transcript (`core-desk/src/claude/mask.rs:31`).
- Đề xuất: trước commit / push quét phần staged diff bằng cùng mẫu (AWS, GitHub `ghp_`, Cloudflare `cfut_`, private key PEM, `.env`): có thì hộp xác nhận liệt kê file:dòng (đã che), nút "Vẫn commit" / "Bỏ stage file". Local, không mạng.
- Rủi ro: báo nhầm chuỗi giống key; diff lớn chậm. Giảm: chỉ dòng thêm, max 2 MB, danh sách bỏ qua theo project.
- File: `apps/core/crates/core-desk` (git commit / push + mask), `components/workspace/CommitBox.vue`, Push dialog trong `log/LogPane.vue`, i18n.

### C2 Xem dạng chat trên desktop (M)
- Vấn đề: muốn đọc lại agent đã trả lời gì thì phải cuộn terminal dài. Desktop đã có parser chat cho điện thoại (`core-transcript/src/chat/page.rs:89` `page`, v107 C3) nhưng chỉ phát qua remote.
- Đề xuất: nút "Chat" trong menu pane agent mở pane chỉ đọc: câu hỏi của chủ nhân + câu trả lời (markdown, code), tool gộp 1 dòng; bấm 1 dòng tool nhảy tới file / diff. Không có ô nhập (gõ vẫn ở terminal).
- Rủi ro: thêm 1 loại pane là thêm thứ để nhìn. Giảm: chỉ mở khi bấm, không tự hiện; đọc theo trang, không giữ cả file.
- File: `apps/shell/src/remote/chat.rs` (dùng lại op), IPC mới `ccm:chat-page`, `stores/workspace.ts` (Pane kind `chat`), component mới `workspace/ChatPane.vue`, `lib/rail/railMenu.ts`.

### C3 Chế độ chia sẻ màn hình (S-M)
- Vấn đề: khi share màn hình / quay video, rail, header, Explorer, toast hiện đường dẫn có tên user (`C:\Users\<user>`), tên project riêng, email GitHub, mã ghép điện thoại.
- Đề xuất: 1 phím (Ctrl+K "Chế độ chia sẻ màn hình") đổi home thành `~`, che email / token / mã ghép, tắt toast có nội dung, chip "Đang che" trên rail. Terminal không đụng.
- Rủi ro: sót chỗ hiện path. Giảm: hàm `displayPath` dùng chung + test quét chuỗi.
- File: `lib/paths` (hàm hiển thị path), `stores/ui.ts`, `composables/useNotifications.ts`, Settings Remote / GitHub, `CommandPalette.vue`.

### C4 Tìm trong mọi terminal đang mở (M)
- Vấn đề: `TermFind.vue` chỉ tìm trong 1 pane. Nhớ có lỗi "ECONNREFUSED" ở đâu đó trong 8 pane thì phải mở từng pane.
- Đề xuất: Ctrl+Shift+F (khi focus Workspace) tìm chữ trong scrollback của mọi pane, kết quả nhóm theo pane, bấm nhảy tới dòng. Chỉ đọc buffer xterm đang có, không đọc đĩa.
- Rủi ro: CPU khi scrollback lớn. Giảm: max 5000 dòng / pane, tìm khi ngừng gõ 200ms, dừng ở 200 kết quả.
- File: `components/workspace/TermFind.vue` (tách logic ra `lib/termSearch.ts`), `WorkspaceView.vue`, `lib/workspaceKeys.ts`.

## Thứ tự đề xuất
A2, A3, A1, B1 (S, giảm ngộp ngay) → C1 (an toàn) → A4, B2 → B3, C2 → C3, C4, B4.

Nguồn web: [Conductor docs](https://www.conductor.build/docs/index), [Conductor diff tools](https://www.conductor.build/blog/diff-tools), [Crystal README](https://raw.githubusercontent.com/stravu/crystal/v0.3.4/README.md), [VS Code: organize sessions](https://code.visualstudio.com/docs/agents/run/sessions/organize-sessions), [Warp agent notifications](https://docs.warp.dev/agents/capabilities/agent-notifications/).
