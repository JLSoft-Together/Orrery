## WP-114c — DONE (phần desktop web + shell; Dart/PWA không thuộc gói)

### Đã đổi
- `packages/shared/src/remote.ts`: lệnh `ws.snooze {paneKey, minutes: 15|60|null}`, `ws.reply {paneKey, text}` + parse; `RemotePane.question?`, `snoozed?`; `REMOTE_QUESTION_MAX=300`, `REMOTE_SNOOZE_MINUTES`; mã lỗi `notWaiting`, `useMenu`.
- `packages/shared/src/orreryApi.ts` (ngoài danh sách phạm vi, bắt buộc để đăng ký op): `ws.snooze`, `ws.reply` = fromPhone.
- `features/next/lib/railSnooze.ts`: lưu `sessionStorage` `ccm.railSnooze` (until + since), `setSnoozeSinceResolver`, `releaseChanged` (tự bỏ hoãn khi since đổi), `clearSnoozed`; API cũ giữ nguyên chữ ký. Mức 60 phút đã có sẵn trong `railLive.ts` (error vẫn max 15) nên không sửa railLive.
- `features/click-menu/stores/inbox.ts`: `snoozePane(paneKey, minutes)`, `snooze(id, minutes)`, resolver since theo `statusSince` của terminal (waiting/error), watch gọi `releaseChanged`.
- `features/remote/lib/remoteSummary.ts` + `useRemoteSummary.ts`: pane có `question` (masked, max 300, chỉ khi rail asked, không menu, nguồn = `waitingFor` của session) và `snoozed: true`.
- `features/remote/composables/useRemotePhone.ts`: handler `ws.snooze` (hệ UI next theo paneKey + core `useSnoozeStore` nếu có terminalId; null = bỏ hoãn cả hai) và `ws.reply` (prefill + `prefillSubmit` + `prefillWaiting`, chờ tối đa 3 s `REPLY_WAIT_MS` cho TermPane gõ; hết hạn thì xóa prefill).
- `TermPane.vue` (+2 dòng) và `workspace.ts` (+1 dòng): `prefillWaiting` cho `prefillReady()` chỉ khi status `waiting`.
- Shell: `phone_ws.rs` (2 op mới: rate, `host.control()` -> `denied`, danger check áp cho cả `ws.reply`, `authed` bị strip; CODES thêm `stale`, `notWaiting`, `useMenu`), `workspace.rs` (WS_COMMANDS 11, WS_GUARDED, WS_TEXT), `chat_src.rs` (mask `question` bằng `mask_text`, `snoozed` chỉ nhận `true`), `pane_link.rs::pane_snoozed`, `push_extra.rs::queue_pane` bỏ push (mọi kind trừ `open`) cho pane `snoozed`.

### Tiêu chí
- ws.snooze 2 hệ, null = undo, sessionStorage, tự bỏ hoãn khi since đổi: đạt (vitest).
- ws.reply gõ vào pane waiting, từ chối khi có menu / không waiting / busy / pane hết chờ giữa chừng: đạt.
- summary có question masked: đạt (web + Rust).
- cargo: danger, view-only, push bỏ pane hoãn: đạt.
- E2E: CHƯA KIỂM (không chạy theo luật).

### Audit
- Edge: câu hỏi mới khi đang hoãn -> since đổi -> bỏ hoãn; pane sang working -> bỏ hoãn (resolver trả null). Terminal chưa load sau reload (undefined) -> giữ hoãn. Phone cũ không gửi op mới, summary field thêm bị bỏ qua. View-only -> `denied`. Rid trùng -> không gõ 2 lần.
- Rủi ro còn: nếu working kéo dài < ~300 ms rồi hỏi lại, push có thể bị chặn nhầm do summary `snoozed` chưa kịp cập nhật (rất hiếm).
- `question` hiện là `waitingFor` (lý do chờ của registry), không phải chữ câu hỏi đầy đủ: chưa có nguồn chữ tin cậy khác ở web (đúng nhánh dự phòng của spec).
- Phát hiện: `error_code` cũ biến `stale` thành `internal`, đã thêm `stale` vào CODES (phone vốn đã chờ `stale` ở `menu_action.dart`).

### Review
- Không comment, file mới ≤ 400 dòng; `useRemotePhone.test.ts` thêm 90 dòng (không phải file nguồn).
- Mask: web `maskSecrets` + Rust `mask_text` hai lớp. Không tự gửi gì: ws.reply chỉ chạy khi phone gọi (ngoại lệ 7).

### Test
- `cargo test --locked -j 4 danger` -> 6 passed (có `ws_reply_follows_the_same_danger_check_and_view_only_refuses_snooze_and_reply`).
- `cargo test snoozed` -> 2 passed (`a_snoozed_pane_gets_no_push...`); `pane_question...`, `menus_are_masked...`, `renderer_results...` -> passed.
- `npm test -w @ccm/web -- src/features/remote/composables/useRemotePhone.test.ts` -> 22 passed (lần đầu 'rename and move' flake khi cargo chạy song song, chạy lại sạch).
- `npm run qa -- --no-e2e --jobs=4`: PASS typecheck shared, vitest related shared, cargo core-desk, cargo shell. FAIL typecheck web: 4 lỗi ở `palette/CommandPalette.vue`, `prompts/useInsertPrompt.ts`, `useSavePrompt.ts` (WP-114f). FAIL vitest related web: `i18n/keys.snapshot.test.ts` và `settings/components/index.test.ts` (key `savedPrompts.*` của WP-114f / snapshot do main cập nhật). Không có lỗi thuộc file của gói này.

### Hợp đồng cho phone (WP-114d)
- Cap `queue` do main thêm vào `DESKTOP_CAPS`; phone chỉ gửi 2 op mới khi có cap.
- `ws.snooze {t, paneKey, minutes: 15|60|null}` -> `{ok:true, minutes, until(ms)}`; `minutes:null` -> `{ok:true, minutes:null}`. Lỗi: `notFound`, `notWaiting` (pane không chờ), `denied` (view-only), `rate`, `invalid`. Error-kind bị kẹp 15 phút (trả `minutes` thật).
- `ws.reply {t, paneKey, text(<=2000), authed?}` -> `{ok:true, submitted:true}` (chỉ trả sau khi desktop đã gõ). Lỗi: `needsAuth` (danger, gửi lại với `authed:true`), `useMenu`, `notWaiting`, `busy` (còn prefill / hết 3 s mà pane vẫn waiting), `notFound`, `invalid`, `denied`.
- Summary pane mới: `question?: string` (<=300, đã mask, chỉ pane waiting không menu), `snoozed?: true`. Pane đang hoãn biến khỏi `inbox` của pane đó, nên phone phải dùng `snoozed` để phân biệt với `gone`.
- `ws.menu` stale giờ tới phone đúng là `stale` (trước là `internal`).

### Hot file cần thêm
- `apps/shell/src/remote/ctx.rs`: thêm cap `queue` vào `DESKTOP_CAPS`.
- i18n en/vi (tuỳ chọn): `remoteV97.log.snooze` nếu muốn ghi log phone khi hoãn (hiện không ghi).
- i18n snapshot / docs: `docs/claude-code-integration.md` (hoặc remote-integration) ghi 2 op, 2 field, 3 mã lỗi; `.code_index` module remote + `_meta.md` 1 dòng.

### Ngoài phạm vi
- Dart/PWA (WP-114d), E2E, i18n, lỗi typecheck và snapshot của WP-114f.

### Commit message đề xuất
feat(remote): add ws.snooze and ws.reply, persist rail snooze per wait, publish pane question and snoozed, skip push for snoozed panes
