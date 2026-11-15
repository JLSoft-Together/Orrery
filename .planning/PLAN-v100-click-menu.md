# PLAN v100: click chuột vào menu chọn của Claude / Codex trong pane

Nhánh `feat_clean_memory`. Đợt 0 của chuỗi (v101 Ổ đĩa làm sau). Grill 15/11/2026.

## Mục tiêu

TUI của Claude Code / Codex chỉ nhận phím (Up / Down / Enter / Space). Cho phép dùng chuột trên pane Workspace.

## Đã chốt

| # | Quyết định |
|---|---|
| Q16 | Có cả 2 cách: (a) click thẳng dòng đáp án trong terminal; (b) thanh nút nổi dưới pane khi agent đang hỏi |
| (a) | Hover làm sáng dòng. Click = chuyển highlight tới dòng đó (gửi Up/Down). Double click = chọn (thêm Enter). multiSelect thì click = tick / bỏ tick (Space) |
| (b) | Đáp án lấy từ hook / transcript (data giống nút Telegram), click là chọn |
| Q20 | Áp cho mọi menu chọn của Claude + Codex: AskUserQuestion, hỏi quyền (Yes / Yes always / No), duyệt plan, `/model`, `/resume`... Nhận diện theo mẫu dòng trên màn hình |
| Q27 | Dòng `Other`: chuyển highlight tới đó rồi để con trỏ cho người dùng gõ. multiSelect: thanh nút có nút Submit. Chỉ nhận click trên màn live, không nhận khi đang cuộn lên scrollback |
| Q23 | Công tắc trong Settings > Workspace, mặc định bật. Không có menu đang hỏi thì click / double click giữ hành vi cũ (bôi chọn chữ của xterm) |

## Bổ sung sau review (15/11)

- Chọn bằng **phím số `1`-`9`** (menu Claude nhận phím số chọn thẳng) thay cho đếm Up/Down rồi Enter; Up/Down chỉ dùng khi click một lần để chuyển highlight hoặc khi menu không nhận phím số. Kiểm hành vi thật trên Claude 2.1.289 + Codex 0.162.0 ở phase 0a.
- Parser không chắc chắn (không khớp mẫu đã biết) → chỉ hiện thanh nút từ hook, không bắt click trong terminal.
- Thanh nút nổi hiện số 1-9 trên mỗi nút, hover xem mô tả đầy đủ của đáp án.
- **Trả lời từ Mission Control**: card của pane đang hỏi hiện câu hỏi + nút đáp án.
- **Hàng chờ "Chờ trả lời"**: danh sách mọi pane đang hỏi (nhiều project), phím tắt nhảy tới câu kế tiếp.
- **Toast OS có nút**: notification Windows/macOS kèm max 2-3 nút đáp án, bấm là chọn khi app ở nền; câu hỏi nhiều đáp án hơn thì toast chỉ mở pane.

## Phase

| Phase | Nội dung | Kiểm |
|---|---|---|
| 0a | Spike: chụp mẫu menu Claude + Codex (AskUserQuestion, hỏi quyền, plan, `/model`, `/resume`, multiSelect, Other), viết parser + unit test, kiểm phím số | Parser nhận đúng 100% mẫu |
| 0b | Thanh nút nổi (data từ hook, có số 1-9) + công tắc Settings | Dùng được ngay |
| 0c | Click thẳng dòng trong terminal: hover, click, double click, Other, multiSelect + Submit | E2E AskUserQuestion |
| 0d | Trả lời từ Mission Control + hàng chờ "Chờ trả lời" | |
| 0e | Toast OS có nút | Windows + macOS |

0b, 0c chạy song song được với v101 phase 1a.

## Dùng lại

- `apps/web/src/composables/useTelegramInbound.ts` (`keyOf`, `keysReady`): đã gõ phím chọn đáp án vào pane khi status `asking`. Click chuột đi cùng đường gửi phím này.
- Nguyên tắc monitoring không đổi: phím chỉ gửi khi người dùng click, giống paste. Không tự submit.

## Cần làm rõ khi code (tra code, không hỏi chủ nhân)

- Parser buffer xterm: nhận dạng khối menu (dấu `❯`, số thứ tự `1.`, ô tick của multiSelect) cho cả Claude 2.1.289 lẫn Codex 0.162.0; lấy mẫu thật trước khi viết regex.
- Tính số bước Up/Down từ dòng đang highlight tới dòng được click; đọc lại buffer sau khi gửi để kiểm.
- Double click của xterm (chọn từ) phải bị chặn chỉ khi click rơi vào dòng menu.

## Tiêu chí xong

- Click / double click chọn đúng đáp án trên Windows + macOS, Claude + Codex, single + multiSelect + Other.
- Tắt công tắc thì hành vi y như cũ.
- Unit test parser với mẫu màn thật; E2E 1 case AskUserQuestion.

## Điều phối với luồng song song

Xem `.planning/COORD-2026-11-15-v100-v102.md` (thứ tự merge, namespace i18n, hot file).
