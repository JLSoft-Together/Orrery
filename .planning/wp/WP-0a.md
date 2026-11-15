## WP-0a: spike mẫu menu Claude / Codex + parser

- Mục tiêu: có parser thuần TS nhận diện khối menu chọn đang hiện trên màn live của terminal (Claude Code 2.1.289, Codex 0.162.0) và trả về danh sách đáp án + dòng màn hình + đáp án đang highlight + loại menu; có bộ mẫu màn thật làm fixture. Đây là nền cho phase 0b-0e của `.planning/PLAN-v100-click-menu.md` (đọc plan trước).
- Phạm vi file (tạo mới): `apps/web/src/lib/termMenu.ts`, `apps/web/src/lib/termMenu.test.ts`, `apps/web/src/lib/__fixtures__/term-menu/*` (mẫu màn dạng text, mỗi file 1 màn, giữ nguyên ký tự `❯`, `●`, `☐`, `☒`, khoảng trắng), `scripts/capture-term-menu.mjs` (script chụp mẫu: spawn CLI qua node-pty có sẵn trong repo + `@xterm/headless` nếu đã có trong deps, dump màn live ra text).
- Cấm đụng: mọi file khác, đặc biệt `TermPane.vue`, `useTelegramInbound.ts`, i18n, package.json (thiếu dep → ghi vào báo cáo, không tự cài).
- Phụ thuộc: không. Song song với WP-1a (Rust, không chung file).
- Model: sonnet.

## Việc

1. Chụp mẫu màn thật cho: AskUserQuestion single, multiSelect (có ô tick + tab Submit), dòng `Other` / "Type something", hỏi quyền tool (Yes / Yes, and don't ask again / No), duyệt plan (ExitPlanMode), `/model`, `/resume`, `/config` hoặc menu chọn khác, và menu Codex (approval, `/model`). Menu `/model`, `/resume` không tốn token. Cần model trả lời (AskUserQuestion, hỏi quyền) thì được chạy tối đa 4 prompt ngắn với `claude --model haiku` trong thư mục tạm của scratchpad (không chạy trong repo), không chạy gì khác. Không chụp được loại nào → ghi rõ, dựng mẫu tay từ hiểu biết nhưng đánh dấu `synthetic` trong tên file.
2. Kiểm và ghi kết quả: menu Claude có nhận phím số `1`-`9` để chọn thẳng không (chọn luôn hay chỉ chuyển highlight)? multiSelect: phím nào tick (Space?), sang Submit bằng gì? Codex thì sao? Mouse tracking: Claude/Codex có bật xterm mouse mode (DECSET 1000/1002/1006) không - có thì click của xterm đã bị gửi vào app.
3. Tra (không sửa) nguồn data từ hook / transcript cho câu hỏi đang hiện: `apps/collector/src/transcript.ts`, `apps/core/crates/core-transcript/src/origin.rs`, `apps/web/src/composables/useTelegramInbound.ts` (`keyOf`, `keysReady`, cách gửi phím vào pane). Ghi `file:line` để phase 0b dùng: lấy câu hỏi + đáp án ở đâu, gửi phím vào pane bằng hàm nào.
4. Viết `termMenu.ts`: input = mảng dòng màn live (string[]) + vị trí cursor; output `{ kind: 'ask'|'ask-multi'|'permission'|'plan'|'picker'|'codex-approval'|..., items: [{ index, label, row, selected, checked?, isOther?, isSubmit? }], highlighted, confident: boolean }` hoặc `null`. `confident=false` khi khớp không chắc. Không phụ thuộc DOM/xterm để test thuần.
5. Test: mỗi fixture có expected; thêm case âm (màn code, diff, list markdown đánh số, output `ls`) phải ra `null`.

## Tiêu chí nghiệm thu

- [ ] ≥ 8 fixture, ghi rõ cái nào chụp thật, cái nào synthetic.
- [ ] Parser đúng 100% fixture dương, 0 false positive trên ≥ 5 fixture âm.
- [ ] Báo cáo có kết luận phím số / Space / Submit / mouse mode cho Claude và Codex.
- [ ] Báo cáo có `file:line` nguồn data hook + hàm gửi phím, đủ để phase 0b làm thanh nút.
- [ ] `npm test -w @ccm/web -- src/lib/termMenu.test.ts` pass; `npm run typecheck -w @ccm/web` sạch.

- Lệnh test: như trên.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-0a-report.md`.

## Nhật ký
- 17:00 script chụp scripts/capture-term-menu.mjs xong (cần @xterm/headless, cài tạm ở scratchpad); chụp được Claude + Codex menu thật
- 17:03 chụp AskUserQuestion single/multi/Other/Submit thật; Codex /model, /permissions thật; hỏi quyền, plan, Codex approval dựng synthetic
- 17:05 termMenu.ts + termMenu.test.ts: 27 test pass; typecheck sạch
- 17:10 viết báo cáo WP-0a-report.md
