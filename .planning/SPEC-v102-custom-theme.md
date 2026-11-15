# SPEC v102 bổ sung: Theme tự chọn (color picker) cho giao diện mới

Chủ nhân chốt 16/11/2026: theme builder đầy đủ, picker kính tự vẽ, mỗi màu chọn 1 lần và app tự sinh bản sáng + tối. Chỉ áp giao diện mới (`data-ui="next"`). Giao diện cũ không đổi.

## C1 Thẻ "Tuỳ chỉnh"
- Thẻ thứ 10 trong lưới bảng màu (Cài đặt > Giao diện, `SettingsMistAppearance.vue`), id `custom`, PaletteMini vẽ bằng màu hiện tại của theme tự chọn.
- Chọn thẻ thì mở khối builder ngay dưới lưới (không dialog mới). Lần đầu: hạt giống lấy từ bảng màu đang dùng trước đó.
- Theo chế độ sáng / tối của app như các bảng khác (cả 2 scheme).

## C2 Builder
Các ô màu, mỗi ô là 1 swatch bấm mở picker:
| Ô | Mặc định khi chưa đặt | Dùng cho |
|---|---|---|
| Accent (bắt buộc) | accent bảng màu gốc | nút chính, mục chọn, focus, chữ link, accent-soft |
| Vệt sáng 1 | hue accent - 40° | `--ccm-bg-a` |
| Vệt sáng 2 | hue accent + 50° | `--ccm-bg-b` (và `--ccm-bg-c` sinh giữa 2 vệt) |
| Sắc nền | hue accent, chroma rất thấp | `--ccm-canvas`, `--ccm-material`, `--ccm-material-pop`, `--ccm-well`, ink ngả nhẹ theo hue |
Thanh trượt "Độ trong kính" (mặc định như bảng gốc). Nút "Bắt đầu từ bảng màu…" (chọn 1 trong 9 bảng làm hạt giống), "Đặt lại", "Tự sinh vệt sáng từ accent" (bật mặc định, tắt thì 2 vệt chọn tay).

## C3 Picker kính
- Ô vuông saturation × brightness, thanh hue, ô nhập hex (#rgb / #rrggbb, sai thì viền lỗi + không áp), hàng mẫu = accent của 9 bảng màu, hàng "dùng gần đây" (tối đa 8).
- Popover kính (`GlassDialog`/lớp nổi hiện có hoặc popover Teleport body + fixed), Esc đóng, phím mũi tên chỉnh ô vuông / thanh (Shift = bước lớn), aria đủ (`role=slider` có `aria-valuetext`).
- Đổi là áp ngay (live), gộp ghi localStorage bằng debounce ~200ms.

## C4 Sinh màu (hàm thuần `apps/web/src/lib/next/customTheme.ts`)
- Làm việc trong OKLCH. Từ mỗi màu chọn sinh bản light + dark (giữ hue, chỉnh L / C theo scheme).
- Chặn tương phản: `--ccm-accent-ink` ≥ 4.5:1 trên canvas và material; `--ccm-on-accent` chọn đen / trắng theo tương phản lớn hơn (≥ 4.5:1); ink / ink-muted / ink-faint ≥ 4.5:1 trên material. Màu chọn quá nhạt / quá đậm thì tự nhích L và báo chip "Đã chỉnh độ sáng để chữ đủ rõ".
- Ra đúng bộ biến như một khối trong `next/palettes.css` (canvas, bg-a/b/c, material, material-pop, ink, ink-muted, ink-faint, accent, accent-ink, on-accent, accent-hi, well) + độ trong kính (`--ccm-glass`).
- Áp: khi `data-next-theme="custom"` set biến inline trên `documentElement.style` (xoá khi đổi sang bảng khác / về classic). Lưu `ccm.next.custom` (JSON có version), parse lỗi thì về mặc định.

## C5 Kiểm
- Vitest cho customTheme (sinh 2 scheme, tương phản ≥ 4.5 với màu cực: #ffffff, #000000, #ffff00, #0000ff, xám), parse / lỗi storage, picker hex parse.
- Classic không đổi; 9 bảng cũ không đổi.
