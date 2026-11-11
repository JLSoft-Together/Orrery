# PLAN v92 — Orrery theo đúng bảng B (bố cục) + màu cam / màu theo agent

Chủ nhân 2026-11-11 (sau khi test 4.0.2 trên Windows): "theme nó không được như anh chốt". 4.0.2 (v91) mới làm
palette + token hình dáng của bảng B; phần bố cục của bảng B chưa làm. Canvas:
https://claude.ai/artifact/8Bw9wH1fNr3ukNexFFsaj3 (`StyleB.dc.html` = token, `Workspace.dc.html` = bố cục chung).

## Chốt
- Làm cả 4 phần còn lệch: thanh trạng thái đáy, rail project kiểu B, header pane + tab kiểu B, chữ "orrery" ở header.
- Màu: "dạng màu là đen dark + cam cam 1 chút chứ không để tím, hoặc claude panel thì cam, codex thì tím, shell thì khác".
  Chốt cả hai: theme Orrery accent chung đổi từ tím sang **cam dịu**; còn chỗ đánh dấu pane / tab theo loại agent:
  **Claude cam**, **Codex tím**, **shell xanh ngọc (teal)**, DeepSeek xanh dương. Nền dark giữ nguyên `#131315`.
- Chỉ đổi theme Orrery (dark + light). Apple, Console, theme cũ giữ nguyên hình (bố cục mới vẫn hiện ở mọi theme vì là
  component, nhưng màu/viền lấy từ token nên theme khác vẫn đúng chất của nó).
- Không comment trong code. Mọi chuỗi UI qua vue-i18n (en + vi). Không blur. Không cuộn ngang. Trạng thái = icon + chữ + màu.

## Hợp đồng chung (mọi WP dùng)
- Token mới (WP-A định nghĩa trong `style.css` `:root` + override trong `themes.css`):
  - `--ccm-pane-accent`: màu đánh dấu của pane/tab. Mặc định `var(--ccm-accent)`.
  - Class `ccm-agent-claude | ccm-agent-codex | ccm-agent-shell | ccm-agent-deepseek` đặt `--ccm-pane-accent` = màu agent
    tương ứng (`--ccm-agent-claude`, `--ccm-agent-codex`, `--ccm-agent-shell-mark` mới = teal, `--ccm-agent-deepseek` mới).
  - `ccm-pane-active` (thanh trái / ring) và `ccm-tab-on` (gạch dưới) dùng `var(--ccm-pane-accent)` thay vì accent cứng.
  - `--ccm-footer-h` (26px), utility `ccm-footer` (nền chrome, viền trên `--ccm-chrome-line`, font `--ccm-label-font`).
  - `--ccm-wordmark-font` (Orrery/Console = mono, Apple = sans), `--ccm-wordmark-display` (Apple = `none`, Orrery/Console = `inline`).
- WP-D đặt class `ccm-agent-<preset>` lên root pane (`TermPane.vue`) và tab (`WorkspaceView.vue`); WP-C dùng nó cho chấm/ô của dòng rail nếu cần.

## Gói việc
| WP | Phạm vi | File (độc quyền) |
|---|---|---|
| A | Token cam + màu agent + wordmark header | `style.css`, `themes.css`, `lib/themes.ts`, `lib/themes.test.ts`, `AppHeader.vue`, `index.html` (nếu cần) |
| B | Thanh trạng thái đáy (Workspace) | `components/workspace/WorkspaceFooter.vue` (mới), `lib/footerStatus.ts` (+test, mới), mount trong `App.vue`, i18n mới `i18n/footer-v92.ts` |
| C | Rail project kiểu B | `WorkspaceSidebar.vue`, `components/workspace/rail/*`, `lib/rail/*` (nếu cần), i18n `i18n/rail-v92.ts` |
| D | Header pane + tab kiểu B | `TermPane.vue`, `WorkspaceView.vue`, `AgentStatusChip.vue`, i18n `i18n/pane-v92.ts` |

### A — chi tiết
- Orrery dark: accent `#e8835d`-ish cam dịu (accent-ink đủ 4.5:1 trên surface), on-accent tối nếu cần đạt tương phản;
  Orrery light: cam đậm hơn (vd `#c4572f`). Qua `themes.test.ts` (4.5:1 chữ, 3:1 glyph).
- Màu agent cho Orrery: claude `#d97757`, codex `#9d8cf8`, shell teal `#4cc2b8` (dark) / tương ứng light; DeepSeek `#4c9bf5`.
- Header: chữ "orrery" (thường, mono, 600–700, letter-spacing 0.02em) cạnh logo, theo token; Apple ẩn.
### B — chi tiết
- Footer 26px dưới Workspace (chỉ khi view = workspace): trái `● N chạy` (màu working) · `✋ N chờ` (màu waiting) ·
  `Port a · b` (dev server đang chạy của workspace); phải: nhánh + số thay đổi của pane đang chọn, `context / limit`
  token của agent đang chọn (chỉ khi có số thật, không bịa). Bấm phần chạy/chờ → focus pane đầu tiên có trạng thái đó.
- Font `--ccm-label-font`, chữ 11.5px, màu muted; mỗi phần là button có aria-label.
### C — chi tiết
- Nhãn section mono IN HOA nhỏ (`ccm-label`): "ĐANG MỞ" / "GẦN ĐÂY" (đúng các section đã có: agents/opened/others).
- Dòng project: ô chữ cái 22px bo `--radius-inner` màu theo project (avatar có sẵn `lib/rail/avatars.ts`), tên, thời gian
  hoạt động gần nhất; dòng phụ `● N chạy  ✋ N chờ  412K` (chỉ phần có số). Project đang chọn: nền `selBg`, không viền.
- Bỏ khung viền bao quanh cụm cây (B không có viền; nổi bằng nền). Session con thụt vào, chấm trạng thái 6px + tool đang chạy.
### D — chi tiết
- Tab: chấm trạng thái 7px + tên + "N pane" (khi ≥2) chữ faint; chip "chờ" khi cần user; gạch dưới màu `--ccm-pane-accent`.
- Header pane: ô màu agent 18px, tên, chip trạng thái `● Đang chạy · <tool> · 12s` (đã có dữ liệu tool + thời gian từ v52),
  khoảng trống, token context (mono, muted, chỉ khi có), chip nhánh `master ↑2` (mono, nền field), menu ⋯ / split.
- Pane đang chọn: thanh trái 2px `--ccm-pane-accent`; pane không chọn mờ `--ccm-dim-inactive`.

## Kiểm
- `npm run qa` (typecheck + vitest related + E2E map; thêm `v91-themes`, `term-ime`, `p3`, `v89-pane`).
- Chụp E2E Orrery dark/light, Console, Apple dark để so với canvas.
