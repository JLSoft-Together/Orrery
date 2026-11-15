WP-14: ĐẠT
File: apps/web/src/components/ui/ContextMenu.vue (WIDTH next 284 -> 304)
Typecheck: pass  Test: 14 pass / 0 fail (vitest layoutPaneMenu + components/ui), E2E v102-next 23/23 PASS

| Vùng | Lỗi thấy | Sửa | Ảnh kiểm |
|---|---|---|---|
| 1 Menu chuột phải rail (row) | OK, không cụt chữ, không mất viền | - | C:/Users/dauph/AppData/Local/Temp/claude/0f0ce96d9ef846ae/4a489006-5409-426b-86ee-7edb337e302f/scratchpad/wp14/shots/rail-ctx-row.png |
| 2 Flyout rail 56px / overlay | OK (ring row trong flyout đủ, menu chuột phải trên flyout đủ chữ, overlay 900px đủ) | - | C:/Users/dauph/AppData/Local/Temp/claude/0f0ce96d9ef846ae/4a489006-5409-426b-86ee-7edb337e302f/scratchpad/wp14/shots/flyout-1100.png, flyout-ctx-1100.png, overlay-open.png |
| 3 Explorer ghim cột | OK (ring hàng đầu, header, không cụt chữ) | - | C:/Users/dauph/AppData/Local/Temp/claude/0f0ce96d9ef846ae/4a489006-5409-426b-86ee-7edb337e302f/scratchpad/wp14/shots/explorer-pinned.png |
| 4 Ổ đĩa, 9 tab (Duyệt, Tìm kiếm, File lớn, File cũ, Dọn dẹp, File trùng, Sắp xếp, App, RAM) | OK, không ring bị cắt ngang | - | C:/Users/dauph/AppData/Local/Temp/claude/0f0ce96d9ef846ae/4a489006-5409-426b-86ee-7edb337e302f/scratchpad/wp14/shots/disk-tab-0.png ... disk-tab-8.png |
| 5 Palette nebula, aurora, graphite, dawn, orrery (sáng + tối), galaxy, console | OK: chữ, viền pane active, mục chọn, toggle đọc rõ ở Workspace 2 pane + Settings | - | C:/Users/dauph/AppData/Local/Temp/claude/0f0ce96d9ef846ae/4a489006-5409-426b-86ee-7edb337e302f/scratchpad/wp14/shots/pal-ws-*.png, pal-set-*.png |
| 6 Menu pane "Đổi chỗ với pane bên cạnh" | cụt (155 > 143 vi, 147 > 143 en) do kbd dài | next rộng 304; đo lại vi + en scrollWidth = clientWidth cho cả 10 dòng; classic giữ 252 | C:/Users/dauph/AppData/Local/Temp/claude/0f0ce96d9ef846ae/4a489006-5409-426b-86ee-7edb337e302f/scratchpad/wp14/shots/pane-menu-vi.png, pane-menu-en.png |
