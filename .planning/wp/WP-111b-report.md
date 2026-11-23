## WP-111b: DONE
- Đã đổi:
  - `apps/core/crates/core-transcript/src/chat/export.rs` (mới): `export()` đi hết các trang chat (lùi từ mới nhất, dừng khi vượt 2 MB hoặc 400 trang), ra Markdown dạng `full` (tiêu đề agent, project, nhánh, thời gian UTC; `### Bạn` / `### Claude|Codex`; tool gộp 1 dòng `> Tool: Edit 3 file, Bash 2 lần`; cuối `## File đã sửa`) và `short` (`## Yêu cầu` mỗi prompt max 300 ký tự + file đã sửa). Cuối cùng chạy `mask_secrets` (secret + home path), cắt đầu file ở ranh giới `### ` khi quá 2 MB kèm ghi chú. `file_name()`, `label()`, `Form`, `Meta`.
  - `core-transcript/src/chat/mod.rs`: `pub mod export;`.
  - `apps/shell/src/chat_ipc.rs`: op `export {target, form, dest: text|save|telegram}`; nhánh đọc từ `.git/HEAD` (hỗ trợ worktree `gitdir:`); `dest=save` mở save dialog (mẫu files_ipc), `dest=telegram` gọi `desk:tg-doc`. `dispatch` nhận thêm `&AppHandle`.
  - `apps/shell/src/ccm.rs:303`: truyền `app` vào `chat_ipc::dispatch` (1 chỗ, ngoài phạm vi brief nhưng bắt buộc cho save dialog).
  - `apps/core/crates/core-desk/src/telegram/doc.rs` (mới) + `mod.rs` (`pub mod doc`, kênh `desk:tg-doc`, CHANNELS +1) + `tests.rs` (1 test): `Tg::send_doc` gửi `.md` bằng `sendDocument`.
  - Web `apps/web/src/features/chat/`: `export.ts` (`exportChat(target, form)`, `runExport`, `paneChatTarget`), `ChatExportMenu.vue` (nút Xuất + menu Chép Markdown / Lưu file .md / Gửi Telegram, ẩn Telegram khi chưa ghép), `ChatView.vue` (thanh header mỏng chứa nút), `i18n.ts` (`chatExport.*` en + vi), `index.ts` (mới), `export.test.ts` (mới), `chat.test.ts` (thêm pinia cho mount).
  - `apps/web/src/features/palette/components/CommandPalette.vue`: 2-3 lệnh Ctrl+K "Xuất session: chép Markdown / lưu file .md / gửi Telegram" cho pane Claude / Codex đang chọn, toast kết quả.
  - `apps/web/src/i18n/__snapshots__/keys.{en,vi}.txt`: chỉ thêm key `chatExport.*` (đã gỡ key `v106rail.filter.*` của 112f do `-u` kéo vào).
  - `apps/shell/e2e/v111-export.cjs` (mới). `.code_index/{lookup,core-desk-telegram,_meta}.md`.
- Tiêu chí: 6/6 đạt.
  1. Rust test fixture Claude + Codex: thứ tự, tool gộp, che `sk-ant-` / `ghp_` / home path, cắt 2 MB: đạt.
  2. Dạng short: đạt.
  3. Chép / Lưu / Telegram: Chép đạt E2E thật; Telegram đạt test giả (`an_exported_markdown_goes_out_as_one_document`); Lưu: logic ghi file chạy, save dialog native CHƯA bấm thử (không điều khiển được dialog hệ điều hành qua CDP).
  4. vitest menu: đạt (`export.test.ts`).
  5. E2E `v111-export.cjs`: 32 PASS (classic + next), kiểm nội dung .md.
  6. typecheck + cargo test: đạt.
- Audit: diff chỉ chạm file trong phạm vi + 3 chỗ ngoài (ccm.rs 1 chỗ, telegram/{mod,tests}.rs, CommandPalette.vue, snapshot i18n). Không sửa hot file. Không comment trong code; không dùng em dash / chấm giữa trong chuỗi UI. Token / secret không ra frontend: text xuất đã qua `ShellClean` + `mask_secrets`.
- Review: (a) `mask_secrets` che run chữ >= 32 ký tự có đủ chữ hoa thường, có thể che nhầm 1 dòng dài vô hại trong file xuất (chấp nhận, an toàn hơn). (b) Prompt/AI text bị `ShellClean` cắt 4000 / 8000 ký tự mỗi lượt như chat view (đã có cờ `cut` ghi "đã rút gọn"). (c) Session rất lớn (hàng trăm MB): đọc tối đa 400 trang x 4 MB scan, chặn ở 2 MB text; chạy trong `blocking`, UI hiện "Đang xuất". (d) Telegram: cần route topic của project, nếu không có trả `no-topic` (UI báo "Không xuất được"). (e) Menu Lưu khi chủ nhân bấm Hủy trả `cancelled`, không toast lỗi. (f) Header mới trong ChatView thêm 32px chiều cao.
- Test:
  - `cargo test --locked -j 4 -p core-transcript chat::export` → 6 passed.
  - `cargo test --locked -j 4 -p core-desk telegram::tests` → 10 passed (3 lần liền). Lần chạy cả crate có 1 fail `episodes_go_to_the_topic_with_buttons_only_when_receiving` (flaky do poll thread, không liên quan, chạy lại pass).
  - `cargo test --locked -j 4 chat_ipc` (apps/shell) → 5 passed. `cargo clippy` không cảnh báo ở export.rs / doc.rs.
  - `npm test -w @ccm/web -- src/features/chat` → 17 passed; `src/features/palette` → 24 passed; `src/i18n src/importRules` pass sau `-u`.
  - `npm run typecheck -w @ccm/web` → sạch.
  - `node apps/shell/e2e/v111-export.cjs` (build web + shim + shell debug trước) → 32 PASS, 0 FAIL.
  - `node scripts/index-check.mjs` → ok 39 file, 0 lỗi.
- Nhìn: `docs/qa/v111/WP-111b/export-menu-classic.png`, `docs/qa/v111/WP-111b/export-menu-next.png` (chụp sau lần sửa code cuối; menu Xuất mở, 2 mục vì chưa ghép Telegram). Chưa nhìn: menu có mục Telegram (cần bot ghép thật).
- Hot file cần thêm: không. (Nút "Xuất" trong menu pane `TermPane.vue` để lượt sau, như brief.)
- Ngoài phạm vi cần main xử lý: (1) `apps/shell/src/ccm.rs` đổi 1 dòng gọi `chat_ipc::dispatch(&handle, &args)`. (2) `CommandPalette.vue` (feature palette) thêm lệnh Ctrl+K. (3) `core-desk/src/telegram/{mod,doc,tests}.rs` thêm kênh `desk:tg-doc`. (4) WP-111f dùng `exportChat(target, 'short')` từ `@features/chat`. (5) Snapshot i18n: 112f cần chạy `-u` lại cho key `v106rail.filter.*` của họ.
- Commit message đề xuất: feat(chat): export a session as Markdown (copy, save .md, send to Telegram)
- Dòng changelog (en / vi, ≤ 100 ký tự): Export a chat session as Markdown: copy, save a .md file or send it to Telegram / Xuất session thành Markdown: chép, lưu file .md hoặc gửi Telegram
