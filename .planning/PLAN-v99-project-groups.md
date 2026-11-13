# v99 — Project groups trong một tab (kiểu Chrome tab group)

Chủ nhân 2026-11-13: màn 34" muốn một tab chứa nhiều **nhóm project** cạnh nhau (A bên trái chia 1–2–3 session, B bên phải), thay cho cách v97 rải pane project khác vào chung một split tree.

## Data model

- `lib/splitLayout.ts`: `LayoutNode` thêm node `GroupNode = { group: <project path>, node: LayoutNode, name?, tone?, folded? }`.
- Bất biến (giữ bởi `settleGroups`, store gọi `settle(tab)` sau mọi thay đổi):
  1. Tab một project = cây thường, **không có group node** (không có header, y như trước).
  2. Có ≥ 2 project → mọi leaf nằm trong đúng một group; group không lồng nhau; mỗi project đúng một group (trùng → gộp).
  3. Còn 1 group → bỏ group node, `tab.project` = project đó.
- Group là nguồn sự thật; `Pane.project` vẫn giữ (đồng bộ từ group, = group khi khác `tab.project`) để Explorer / MCP / rail / `paneProjectOf` dùng như cũ.
- Mọi op cây cũ (`leaves`, `splitLeaf`, `removeLeaf`, `dockTree`, `setRatio`, `boxes`, `dividers`, `neighbor`) đi xuyên group node; `dividers` gắn `group` cho divider trong group.
- Op mới: `dockGroup`, `moveGroup`, `takeGroup`, `groupsOf`, `groupOf`, `groupBoxes`, `setGroupMeta`, `neighborGroup`, `foldLayout`, `foldedKeys`, `cleanGroups`, `ungrouped`, `mapLeaves`.

## Quyết định

- Split "Other project…" / kéo project từ rail / Group nhiều project → **tạo group mới** cạnh group đích (hoặc nhập vào group sẵn có của project đó).
- Split / "+" / session mới trong group → luôn thuộc project của group (`split` trong group, `joinProject` → `dockNear` vào group khớp cwd).
- Kéo session project A sang vùng group B: **không bao giờ vào B**; group A một pane → cả group chuyển sang cạnh B (phía thả); A còn pane khác → nhập lại group A. Swap giữa 2 group bị chặn.
- Rail: tab nhiều group = **một** entry dưới project chính (`tab.project`), tiêu đề `A + B` (hoặc tên tab) + chấm màu, bên trong là subgroup từng project. Không lặp ở cluster từng project (đóng / fold / rename tab vẫn một chỗ).
- Màu group: tự động `projectTone(cluster key)` = màu avatar project trên rail; đổi được (8 token, Automatic). Tên hiển thị đổi được (≤ 40 ký tự).
- Collapse = dải dọc 36px (mono + tên dọc + số pane), click để mở; group còn lại chiếm chỗ; không collapse group mở cuối cùng; focus vào pane trong group đã collapse → tự mở.
- Phím: Ctrl+Alt+←/→ focus group bên cạnh (chỉ khi tab có ≥ 2 group; tab một project vẫn là back/forward vị trí), Ctrl+Alt+Shift+←/→ chuyển group, Ctrl+Alt+G (⌃⌥G) thêm project thành group (palette "Add another project as a group").
- Saved layout: cây lưu dạng đã bỏ group node (validator Rust/Node cũ không đổi), group dựng lại từ cwd khi mở; tên / màu / collapse lưu sidecar `localStorage['ccm.layoutGroups'][layoutId]`. Layout cũ mở như trước. Tab nhiều group được chọn khi lưu từ bất kỳ project nào của nó.
- Persist sessionStorage / `ccm.workspace.last` / closed stack giữ group node; dữ liệu v97 (pane.project rải rác, không group) tự gom thành group khi load.

## Còn lại

- Ctrl+Alt+←/→ trong tab nhiều group thay back/forward vị trí file (Windows); muốn đổi phím khác thì sửa `workspaceKeys.ts`.
- Màu chọn tay chỉ áp cho group trong tab đó (không đồng bộ sang avatar rail của project).
