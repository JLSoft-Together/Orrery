# PLAN v93 — 7 đề xuất Workspace (wireframe #2–#8)

Chủ nhân 2026-11-11: bỏ #1 (hàng "Cần chủ nhân"), "chốt 2->8". Wireframe:
https://claude.ai/artifact/8Bw9wH1fNr3ukNexFFsaj3 (`V93Overview.dc.html`, `V93Details.dc.html`).
Làm trên nền v92 (`PLAN-v92-orrery-b-layout.md`: `--ccm-pane-accent`, `.ccm-agent-*`, `WorkspaceFooter.vue`).

## Luật chung
- Không comment trong code. Chuỗi UI qua vue-i18n (en + vi). Màu qua token. Không blur. Không cuộn ngang.
- Trạng thái = icon + chữ + màu. Không bịa số (token, diff, subagent): thiếu dữ liệu thật thì ẩn.
- Monitoring không gửi prompt vào agent. Không có nút nào tự trả lời / tự submit.
- Hoàn tác (#4) chỉ sau xác nhận native, dùng đường hoàn tác checkpoint có sẵn nếu có; không có thì chỉ "Xem diff".

## Gói việc
| WP | Mục | File độc quyền |
|---|---|---|
| 1 | #2 Quỹ đạo subagent, #3 Thước context, #4 Thẻ "Lượt vừa rồi" | `TermPane.vue`, mới `SubagentOrbit.vue`, `ContextMeter.vue`, `TurnCard.vue`, helper `lib/` + test, `i18n/pane-v93.ts` |
| 2 | #5 Tab nhóm theo project + tràn "+N", #7 Chế độ tập trung (Ctrl+.) | `WorkspaceView.vue`, `WorkspaceFooter.vue` (nút Tập trung), `stores/workspace.ts` chỉ phần cần, helper `lib/` + test, `i18n/tabs-v93.ts` |
| 3 | #8 Launcher tab mới | `LaunchPicker.vue`, helper `lib/` + test, `i18n/launcher-v93.ts` |
| 4 | #6 Explorer đánh dấu file agent đang chạm | `ExplorerTree.vue`, `ExplorerPanel.vue` nếu cần, helper `lib/agentTouches.ts` + test, `i18n/explorer-v93.ts` |

### #2 Quỹ đạo subagent
Chip trong header pane Claude: vòng tròn mảnh + chấm (tối đa 5) = subagent đang chạy của agent pane (`agents.subagents` theo
lineage/terminal), chấm màu trạng thái; quay chậm, tắt khi `prefers-reduced-motion`. Chữ "N subagent". Bấm mở popover
(opaque): tên/type, tool đang chạy, thời gian. Không có subagent → ẩn. Collapse sớm khi header hẹp.
### #3 Thước context
Vạch 3px ngay dưới header pane agent: tỉ lệ `contextTokens / settings.contextWarnTokens`; < 80% ink-faint, 80–100% warning,
> 100% error + nút "Compact…" (luồng `useCompact` v51 có sẵn). Có `role=meter` + aria-valuenow. Không có số → ẩn.
### #4 Thẻ "Lượt vừa rồi"
Khi agent pane chuyển working → idle và checkpoint có `before:`/`after:` (`stores/checkpoints.ts`, `useTurnCheckpoints`):
thẻ nhỏ cuối pane (trên terminal, không che dòng nhập): số file, +/−, 3 file đầu, "Xem diff" (mở Changes/TurnReview có sẵn),
"Hoàn tác lượt" chỉ khi đã có đường hoàn tác + confirm. Tự ẩn khi pane chuyển working lại hoặc bấm ✕.
### #5 Tab nhóm theo project
Tab cùng project (git root / cwd) đứng liền nhau (thứ tự ổn định, kéo thả vẫn chạy), mép trái 3px màu project (cùng thuật toán
màu monogram rail `lib/rail/projectRow.ts`), vạch ngăn giữa nhóm. Hết chỗ: tab tràn gom vào nút "+N" mở menu (giữ tab đang
chọn luôn hiện). Không co tên tab dưới ~80px.
### #7 Chế độ tập trung
Ctrl+. (Cmd+. trên mac) bật/tắt ẩn rail trái + explorer phải cho tab hiện tại (nhớ theo tab, lưu `ccm.*`), pane chiếm hết.
Footer có nút "Tập trung" (aria-pressed). Thêm vào palette lệnh + ShortcutsDialog.
### #8 Launcher tab mới
`LaunchPicker` thành khung chọn: 3–4 thẻ agent (Claude / Codex / Shell / DeepSeek nếu bật) màu `.ccm-agent-*`, danh sách
project gần đây (có sẵn nguồn recent), dòng "Mở lại session dở" (`resume=<uuid>` như `open_session`, chỉ session chưa xong),
bàn phím: ←/→ chọn agent, ↑/↓ project, Enter chạy. Không submit prompt.
### #6 File agent đang chạm
Từ activity / hook events có path (Edit/Write/Read của lượt hiện tại theo session): cây Explorer đánh dấu file: chấm đặc =
sửa, chấm rỗng = đọc, màu agent; tooltip agent + thời điểm. 2 agent cùng sửa 1 file → nền warning/error + chữ "2 agent cùng sửa"
(khớp logic `OverlapChip`). Xoá dấu khi lượt kết thúc + 2 phút. Path chuẩn hoá (Windows `\`, hoa thường).

## Kiểm
`npm run qa` + E2E liên quan (`p3`, `v89-pane`, `v50-groups`, `tier2`, `explorer`, `v91-themes`, `term-ime`). Thêm E2E nhỏ
cho #7 (Ctrl+. ẩn rail) và #5 (tràn "+N").
