# PLAN v95 — Theme "Ngân hà" + Quỹ đạo bản đồ sao + Giám sát (Bảng) mới

Chủ nhân 2026-11-11: "muốn kiểu ngân hà 1 chút, đen sâu và chấm chấm sao", sau đó "ở workspace vẫn hơi xanh" → đổi sang đen
trung tính, rồi "ok em chốt làm nhé, làm đi". Canvas: https://claude.ai/artifact/8Bw9wH1fNr3ukNexFFsaj3
(`V95Orbit.dc.html`, `V95Board.dc.html`, `V95Workspace.dc.html`; v94 nền: `V94Monitor.dc.html`).

## Chốt
- Theme mới **Ngân hà** (id `galaxy`, chỉ tối), đứng cạnh Orrery / Console trong Settings. Không đổi mặc định (Orrery vẫn mặc định).
- Nền ngoài đen trung tính `#040405` + sao chấm CSS tĩnh 2 lớp (radial-gradient lặp, trắng / xanh nhạt / vàng nhạt, alpha thấp).
  Sao CHỈ ở nền ngoài: khe giữa panel, header, footer, bản đồ Quỹ đạo. Panel, thẻ, terminal, popover, dialog: đặc, không sao.
- Bề mặt trung tính, không ánh xanh: panel `#0b0b0d`, thẻ `#0e0e11`, raised `#18181c`, line `#1d1d21` / `#232328`, terminal
  `#060607`, ink `#ececf1`, muted `#a6a6b0`, faint `#85858f`. Accent cam như Orrery dark; màu agent như Orrery dark.
- Công tắc "Hiện sao" trong Settings › Giao diện (mặc định bật, chỉ có tác dụng ở theme có sao), key `ccm.stars`.
- Ngoại lệ luật "no gradients/decoration" của CLAUDE.md: sao của theme Ngân hà (chủ nhân duyệt 2026-11-11). Không blur,
  không animation cho sao.
- Quỹ đạo (Monitor › Quỹ đạo) thành bản đồ sao: mỗi project = hệ mặt trời; agent chính = mặt trời màu agent; subagent = hành tinh
  trên vòng, quay chậm khi chạy (tắt khi reduced-motion); session nhắn nhau (peer links) = đường đứt màu accent + nhãn số tin;
  chờ chủ nhân = mặt trời vàng + nút "Mở pane"; rảnh = mờ. Bảng chú giải "Cách đọc bản đồ". Chạy ở mọi theme (nền sao chỉ khi
  theme có sao).
- Giám sát › Bảng (v94 A, B, D): A cột rỗng thu thành dải dọc 40px, "Cần chủ nhân" chỉ tô khi > 0; B thẻ session giàu hơn (ô chữ
  cái project, tool đang chạy + thời gian, nhánh, quỹ đạo subagent nhỏ, thước context có số, nút "Mở trong Workspace" / "Xem diff");
  D "Xong hôm nay" thành dòng thời gian (giờ, tên, project, số file nếu có). Tiêu đề tóm tắt: "N agent đang chạy, M chờ chủ nhân".
- Sửa lỗi v94 H: mục "Chung" trong Settings có gạch cam thừa dưới đáy.
- Chưa làm: v94 C (Nhóm theo), E (thanh đáy chung), F/G (thẻ theme preview) — chờ chủ nhân.

## Luật chung
Không comment trong code. Chuỗi UI qua vue-i18n (en + vi). Màu qua token, không if theo theme id. Không blur. Không cuộn ngang.
Trạng thái = icon + chữ + màu. Không bịa số. Không hiện nội dung session / tham số tool (thẻ không hiện câu prompt trừ khi app đã có
trường tóm tắt đã sanitize đang hiện ở chỗ khác).

## Gói việc
| WP | Phạm vi | File độc quyền |
|---|---|---|
| A | Theme Ngân hà + sao + công tắc + lỗi H | `style.css`, `themes.css`, `lib/themes.ts` + test, `index.html`, `App.vue` (lớp nền sao), ThemePicker / Settings Giao diện, settings store, nav Settings (lỗi H), i18n `theme-v95.ts`, CLAUDE.md + `.planning/COLOR-SYSTEM.md` |
| B | Giám sát › Bảng (A, B, D) | Monitor board components (SessionBoard, BoardCard, cột, header Monitor), helper `lib/` + test, i18n `board-v95.ts` |
| C | Quỹ đạo bản đồ sao | component Quỹ đạo (Monitor orbit view) + helper layout `lib/` + test, i18n `orbit-v95.ts` |
