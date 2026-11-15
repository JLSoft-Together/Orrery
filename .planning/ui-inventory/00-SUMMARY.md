# 00 - Tổng hợp phân tích UI cho v102 Sương kính

Nguồn: 01-workspace, 02-shell-overlays-viewers, 03-monitor-usage-history, 04-settings-devices-ext-disk (subagent đọc code),
05-codex-ux-review, 06-codex-design-proposals (Codex), 07-motion. Ngày 15/11/2026, đọc trên working tree `feat_clean_memory`
(có v100 đã commit, v101 còn WIP).

## 1. Sự thật cần sửa trong plan

| Mục | Plan ghi | Code thật | Đề xuất mới |
|---|---|---|---|
| Rail | 264px | 240 mặc định, 200-560 (`lib/sideWidth.ts`) | 264 mặc định, 224-360 |
| Cột avatar | 56px, vòng trạng thái | 52px, chưa có vòng | 56px, nhóm theo project + flyout 280px |
| Explorer | 270px | 280px cột cố định, 5 view | 288px drawer, pin khi viewport >= 1440 và stage còn >= 720 |
| Tab | pill | 28px | strip 36, pill 28 |
| Pane head | 34 | 28 (term), 36 (viewer toolbar) | 34 thống nhất, 1 `PaneHeader` |
| Row rail | 2 dòng | 24px, dòng 2 chỉ khi working / cần bạn | 48-56px, dòng 2 chọn 1 detail ưu tiên |
| Header app | bỏ | 40px + footer 26px | bỏ, nav vào rail head / foot |
| Settings | page | dialog 1040x760, 12 trang | giữ dialog kính (đề xuất), nav trái 220 |
| Model trên rail | hiện | có dữ liệu (`agents.mainOf(id).model`, `lib/model.ts::modelLabel`), chưa hiện ở Workspace | thêm `usePaneInfo.modelOf` |
| Blur | chrome + overlay | chưa có `backdrop-filter` nào | như plan, tối đa 4 lớp blur cùng lúc |
| Browser pane | chưa nói | `<webview>` trong DOM, overlay HTML đè lên được | PoC blur trên webview ở WP11 |

## 2. Nợ thiết kế phải giải trong WP1 (design system)

- Dialog lặp scaffold 55 lần, focus trap tay 25 file, 9 chiều rộng → 1 `GlassDialog` (480 / 640 / 880).
- Menu tự viết 41 file, 15 hằng WIDTH → 1 `GlassMenu` (220-280, row 32).
- Segmented tự viết 10 file + 1 kiểu tab gạch chân → `SegmentedControl` duy nhất.
- 3 kiểu chip khác chiều cao → 1 chip h20.
- 69 `rounded-[Npx]` lẻ → thang 24 / 18 / 12 / 8 / pill.
- `text-2xs` = `text-xs` = 12px → 2xs = 11px.
- Không có Tooltip, 1002 `title=` → `GlassTooltip` (450ms, focus hiện ngay).
- 7 viewer tự dựng header → `PaneHeader`.
- Context 3 kiểu (meter, ContextRing, ContextGauge) → 1 gauge.
- "Cần bạn" ở 5 bề mặt, màu chờ lệch `warning` / `st-waiting` → 1 taxonomy + 1 token.
- Code thừa không mount: SessionList, WaitingQueue, ActivityFeed, ToolTimeline, MetricsStrip, SessionSummary, OrbitRings, SessionTabs, DevServerChip. Không vẽ, xem xét xoá ở bản sau.
- UI còn dùng " · " làm ngăn cách, trái GLOSSARY.

## 3. Đề xuất chính (đã gộp Codex + subagent)

- **Taxonomy "Cần bạn"**: cần trả lời, cần duyệt quyền, lỗi (riêng), xong chưa xem (mục riêng, không phải Cần bạn). Mỗi loại glyph + chữ, màu là kênh phụ. Hàng có Mở, hoãn, undo 5s.
- **Không đảo thứ tự dưới con trỏ**: status đổi không làm row nhảy khi đang hover; pin giữ chỗ.
- **Mở session cần bạn** giữ split layout, focus đúng pane, không tự nhảy khi có việc mới (toast "2 phiên cần bạn" không lấy focus).
- **Responsive**: >= 1440 rail 264; 1200-1439 rail 248; 1024-1199 rail 56; < 1024 rail overlay.
- **Ctrl+K** thêm nhóm Sessions (Cần bạn, Đang chạy, Xong chưa xem, Gần đây), khớp model / branch / repo.
- **Settings → Giao diện**: preview palette bằng mini Workspace thật; đổi tên tránh lặp chữ "Giao diện" 3 lần.
- **Motion**: setting Đầy đủ / Giảm / Tắt (`data-motion`); dừng mọi loop khi cửa sổ ẩn; chỉ transform + opacity; bỏ `ccm-meteor-pane` (animate background-position mọi panel); tab pill / segmented thumb chuyển sang transform; dust rút 700ms; signal 1 lần 450ms khi waiting / error, không pulse mãi.
- **Màu** Sương kính sáng / tối, type scale, kích thước, component spec: bảng đầy đủ ở 06.

## 4. Chủ nhân đã chốt (15/11/2026)

1. **Nền terminal**: cho chọn trong Settings. Mặc định đặc theo palette; công tắc "Terminal trong suốt" (92% / 55%, sao Ngân hà xuyên vào).
2. **Rail**: cả hai cách nhóm, segmented "Project | Trạng thái" ở đầu rail, nhớ lựa chọn (`ccm.next.railGroup`). Khối "Cần bạn" sticky ở cả hai chế độ.
3. **Settings**: giữ dialog, đổi sang kính (1040x760, nav trái).
4. **Motion**: setting 3 mức Đầy đủ / Giảm / Tắt; bỏ sao băng trong panel (giữ sao băng nền); tín hiệu waiting / lỗi 1 lần 450ms, không nhấp nháy mãi; tan bụi 700ms.

## 4b. Đề xuất kéo thả + split (15/11, chờ chủ nhân chọn)

Hiện có: drop preview viền 2px accent + nền accent-soft, ghost chữ h-7, divider kéo + phím, ratio 15-85%. Chưa có snap, preset bố cục, vùng thả hiện trước.

1. La bàn 5 vùng trên pane đang hover (trái, phải, trên, dưới, giữa = đổi chỗ), có icon + nhãn kết quả ("Chia phải, 50%").
2. Màu: chia = accent 12% + viền accent 2px; đổi chỗ = viền nét đứt quanh cả 2 pane; không thả được (pane < 220px) = viền đỏ + chữ "Quá hẹp".
3. Ghost = pill kính 220x40 có glyph agent + tên + chip trạng thái; pane nguồn phủ lớp mờ, không dùng opacity.
4. Divider: hit 12px, hover hiện vạch accent 2px + chấm grip; khi kéo hiện "62 / 38"; snap 1/3, 1/2, 2/3 (giữ Alt bỏ snap); double click = 50/50.
5. Preset bố cục ở tab strip: 1, 2 cột, 3 cột, 2x2, 1 lớn + 2 nhỏ, "Chia đều".
6. Kéo vào tab pill giữ 600ms thì nhảy sang tab đó (thả pane sang tab khác).
7. Thả file từ Explorer / row từ rail lên canvas dùng cùng la bàn.
8. Toast "Đã chia pane, Hoàn tác Ctrl+Z" sau mỗi lần thả.
9. Từ 5 pane trong 1 tab ở 1440px: gợi ý tách sang tab mới.
10. Phím thay kéo: Ctrl+\ chia phải, Ctrl+Alt+mũi tên dời pane, hiện trong menu pane.

## 5. Mockup cần vẽ (trước WP8-WP10)

1. Workspace 1600x900 sáng + tối, 2 pane, rail đủ 4 trạng thái.
2. Workspace 8 session, 2 cần bạn, 1 lỗi, rail cuộn.
3. Rail thu gọn + flyout project.
4. Explorer drawer overlay + ghim.
5. Ctrl+K (rỗng, có kết quả, không có kết quả).
6. Mission Control 8 session.
7. Settings → Giao diện (chuyển cũ / mới, palette, giảm trong suốt, motion).
8. 6 pane + zoom; cửa sổ 1024.
9. Monitor (Bảng, Quỹ đạo), Usage, History, Inbox trong shell mới.
10. Devices, Extensions, Ổ đĩa (sau khi v101 xong), click menu dưới pane.
