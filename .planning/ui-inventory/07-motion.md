# 07 - Motion: hiện trạng + đề xuất cho Sương kính (v102)

Phạm vi quét: `apps/web/src/**` (không có `apps/web/index.html` trong thư mục này, `rg` báo không tìm thấy). Đường dẫn tính từ `apps/web/src/`. Số dòng đo ngày 15/11/2026 trên `feat_clean_memory`. Mục "chưa rõ" = chưa đọc hết code, cần xác minh trước khi sửa.

Kết luận ngắn: hệ motion hiện tại **đã có token và kỷ luật khá tốt** (transform + opacity, exit nhanh hơn enter, có `prefers-reduced-motion`). Việc của v102 là **giữ bộ token, thêm 3 thứ**: (1) setting Reduce Motion của app (hiện chưa có), (2) quy tắc "không animate dưới backdrop-filter", (3) dọn các chỗ animate layout / background-position.

---

## Part A - Inventory hiện trạng

### A0. Token và cơ chế dùng chung

| Thứ | Vị trí | Giá trị |
|---|---|---|
| Easing | `style.css:102-106` | `--ease-out-quint` cubic-bezier(0.22,1,0.36,1); `--ease-spring` cubic-bezier(0.34,1.32,0.64,1) (overshoot 3.4%); `--ease-exit` cubic-bezier(0.4,0,1,1) |
| Duration | `style.css:148-154` | `--ccm-dur-press` 100ms, `fast` 150ms, `base` 220ms, `slow` 300ms, `emph` 520ms, `signal` 900ms, `--ccm-stagger` 28ms |
| JS helper | `lib/motion.ts` | `reducedMotion()`, `motionMs()`, `pinLeaving`, `retireOverlay`, `withViewTransition`, `bootNodeDelay`; hằng JS riêng `EASE_ENTER/SPRING/EXIT` (`motion.ts:~33-35`) trùng giá trị CSS nhưng khai báo lại |
| Class Vue Transition | `style.css` | `ccm-list` (1406-1432), `ccm-view` (1435-1439), `ccm-fade` (1448-1452), `ccm-panel-left/right` (1460-1480), `ccm-sheet-up` (1484), `ccm-kbd` (1502), `ccm-menu` (1564-1575), `ccm-dialog` (1584-1596), `ccm-scrim` (1609), `ccm-sheet` (1619), `ccm-tab` (1777), `ccm-flip` (1835), `ccm-swap` (1054), `ccm-dot` (1105), `ccm-rail-word` (1970) |
| Tailwind | nhiều file | `duration-(--ccm-dur-*)` + `ease-out-quint` / `ease-spring` / `ease-exit` (token hóa đúng) |

Số đếm: 39 `@keyframes` trong `style.css` (447-2048) + 5 trong component (`PeerEdge.vue:187`, `OrbitStarMap.vue:498`, `FilePane.vue:1788`, `LayoutPreview.vue:953/958/964`, `SubagentOrbit.vue:116`); khoảng 150 chỗ `<Transition>` / `<TransitionGroup>` (rg ở trên); khoảng 30 file dùng `requestAnimationFrame`.

### A1. Reduced motion - cách áp dụng

- **Chỉ có media query hệ điều hành** `prefers-reduced-motion: reduce`. **Không có setting Reduce Motion trong app, không có attribute `data-motion` trên root** (rg `data-motion|reduceMotion|reduce-motion` = 0 kết quả).
- CSS: 15 khối `@media` trong `style.css` (506, 1084, 1099, 1124, 1217, 1359, 1395, 1522, 1533, 1673, 1746, 1878, 1898, 1983, 2054) + `MainNode.vue:337`, `SubagentOrbit.vue:121`, `OrbitStarMap.vue:503`, `FilePane.vue:1793`, `LayoutPreview.vue:972`, `LayoutBoxes.vue:60`.
- Khối chung `style.css:~1548-1559`: ép `transition-property` chỉ còn opacity/color/background-color/border-color/outline-color/box-shadow/fill/stroke/visibility, và `view-transition` về 0ms. Nghĩa là reduced = bỏ transform, còn fade màu. Hợp lý.
- JS: `reducedMotion()` (`lib/motion.ts:3-9`; lỗi `matchMedia` thì trả `true`) được gọi trong `dust.ts:103,391`, `useCountUp.ts:12`, `KeyHint.vue`, `LottieArt.vue:58`, `LayoutPreview.vue:148`, `withViewTransition` (`motion.ts:71`). `play()` (`motion.ts:37`) còn thoát khi `document.hidden`.
- Lệch: `reducedMotion()` đọc matchMedia **mỗi lần gọi**, không reactive; đổi setting OS giữa chừng không cập nhật animation đang chạy (chấp nhận được).
- Lỗ hổng: các spinner dùng `animate-spin` (Tailwind) ở `CompactView.vue:183,209`, `DeviceConnectBar.vue:161-183`, `DeviceEmulators.vue:61,96` **không có `motion-reduce:animate-none`** (chỉ `BackupSettings.vue:104-171` có). `.ccm-spin` (`style.css:951`) có tắt ở khối reduced (xác nhận qua khối 1083-1090, chưa rõ phủ 100%).
- Setting liên quan: `ccm.dust` (`stores/settings.ts:108,149`, UI `SettingsAppearance.vue:130`, mặc định **tắt**), `ccm.starsFx` off / twinkle (mặc định) / meteors (`settings.ts:112,157`, root `data-stars-fx`), `ccm.stars`, `ccm.starsPanels` (root `data-stars`, `data-stars-panels`).

### A2. Bảng motion theo nhóm

Cột "Perf": OK = chỉ transform/opacity; RỦI RO = animate layout / paint / filter / nhiều element.

**Nền Ngân hà (sao, sao băng)**

| Motion | file:line | Trigger | Property | Duration / easing | Reduced | Perf |
|---|---|---|---|---|---|---|
| `ccm-twinkle` lớp html::after | `style.css:429`, keyframes 447 | luôn chạy khi `data-stars-fx != off` | opacity 1 -> 0.04 | 3.4s ease-in-out infinite, `will-change: opacity` | có (506-515) | OK, 1 layer |
| twinkle trong panel `::before` | `style.css:799, 804` | `data-stars-panels` on | opacity | 4.1s / 4.7s, delay âm | có | RỦI RO nhẹ: nhân theo số `.ccm-panel` + `.ccm-sky-well` (mỗi pane 1 layer) |
| `ccm-meteor-a/b` body::before/::after | `style.css:440,444`, 470, 488 | `data-stars-fx=meteors` | translate + opacity | 7s / 11s linear infinite | có, `display:none` | OK, 2 element |
| `ccm-meteor-pane` | `style.css:773`, 457 | meteors + panels | **background-position** của mọi `.ccm-panel` / `.ccm-sky-well`, `background-attachment: fixed` | 9s linear, bay ~1.2s mỗi 9s | có | **RỦI RO cao**: repaint toàn bộ panel + terminal well mỗi frame trong lúc bay; dưới `backdrop-filter` sẽ buộc re-blur. CLAUDE.md đã ghi nhận |

**Keyframe tiện ích (`style.css`)**

| Keyframe | Dòng | Dùng cho | Property | Duration / easing |
|---|---|---|---|---|
| `ccm-spin` | 944 / 951 | `.ccm-spin` loader | transform rotate | 2.4s linear infinite; `.ccm-spin-once` 0.9s quint |
| `ccm-enter`, `ccm-pop` | 960, 971 | mount chung | opacity, transform | `base` quint |
| `ccm-flash` | 982 / 998 | báo thay đổi | chưa rõ (bg/opacity) | `signal` 900ms quint |
| `ccm-breathe` | 1001 / 1018, 1967 | `.ccm-attention::before` (box-shadow ring) và `.ccm-rail-bar` (thanh working ở rail) | opacity (ring dùng box-shadow tĩnh, chỉ opacity chạy) | 2.2s ease-in-out, **3 vòng rồi dừng** |
| `ccm-node-in`, `ccm-pill-in`, `ccm-glyph-pop` | 1021, 1032, 1044 | node / pill Agent Map | transform, opacity | `slow`/`base` spring |
| `ccm-swap-in/fade` | 1070, 1079 | đổi nội dung nhỏ | opacity, translate | `fast` quint in, 100ms exit |
| `ccm-office-fullscreen-in` | 1093 | Office full screen | chưa rõ | `slow` quint |
| `ccm-peer-ring/sweep/hold` | 1136-1223 | cạnh peer giữa 2 session | transform/opacity, sweep chưa rõ | 600ms quint; sweep 1.4s linear; hold 1.2s / 4s |
| `ccm-grow`, `ccm-grow-x` | 1227, 1239 | cột/thanh biểu đồ | transform scale (chưa rõ có `width`) | `emph` 520ms quint |
| `ccm-highlight` | 1250 | dòng mới ở ActivityFeed (`ActivityFeed.vue:240`) | bg (chưa rõ) | 1.6s quint |
| `ccm-shimmer` | 1260 / 1273 | skeleton | opacity 0.45 <-> 0.9 | 1.4s ease-in-out infinite |
| `ccm-tick` | 1276 / 1285 | số đổi | opacity + translateY(-3px) | `slow` quint |
| `ccm-pulse` | 1288 / 1299 | chấm chú ý | transform scale 1 -> 1.18 | 1.6s ease-in-out 3 vòng |
| `ccm-quota-flash/delta/fade/shine`, `ccm-token-glow` | 1303-1400 | quota Plan limits | chưa rõ (glow/shadow) | 1.2s, 2.2s, 1.6-2.2s, 900ms |
| `ccm-kbd-in` | 1496 / 1503 | badge phím tắt | transform | `base` spring, stagger theo toạ độ x tối đa 220ms (`KeyHint.vue:~46`) |
| `ccm-edge-draw` | 1664 | cạnh Agent Map | stroke-dashoffset | `emph` quint (paint SVG) |
| `ccm-to-tray` / `ccm-from-tray` | 1710, 1728 | node về/ra khay | **clip-path** + transform | 560ms / 460ms quint (clip-path = paint) |
| `ccm-title-in`, `ccm-caret`, `ccm-title-wave` | 1866-1893 | tiêu đề terminal gõ chữ | opacity/transform | `emph`; caret 1.1s step-end 4 vòng |
| `ccm-boot-spin/pulse` | 2043, 2048 | màn boot | rotate / opacity | 1.1s linear; 1.4s ease-in-out infinite |

**Trong component**

| Motion | file:line | Ghi chú |
|---|---|---|
| `star-orbit` | `monitor/OrbitStarMap.vue:498` | rotate 360 infinite, reduced có tắt (503). Quỹ đạo bản đồ sao |
| `ccm-orbit-turn` | `workspace/SubagentOrbit.vue:116` | rotate 14s linear infinite, reduced tắt |
| `file-flash` | `workspace/FilePane.vue:1788` | flash dòng file; reduced có |
| `lp-fade/first/slide` | `workspace/LayoutPreview.vue:953-964` | + JS `zoomFlip` 2 rAF (210); reduced có (148, 972) |
| `ccm-peer-edge-in` | `PeerEdge.vue:187`; rAF 95-108 | cạnh peer vẽ bằng rAF; `TreeEdge.vue:72-78` rAF; chỉ hai chỗ này có `usePageActive` (tạm dừng khi ẩn) trong nhóm rAF |
| `AgentMap` rAF | `AgentMap.vue:511,867,939,996` | đo cạnh, bật `nodeMotion` sau frame đầu (tránh animate lúc boot) |

**Vue Transition / Group chính (đã có token)**

| Thành phần | file:line | Property | Duration / easing |
|---|---|---|---|
| Đổi view (Monitor/Workspace...) `ccm-view` | `App.vue:189,199`; css 1435 | opacity | enter `base` quint, leave `fast` exit |
| Menu / popover `ccm-menu` | nhiều (vd `TermPane.vue:1280`, `WorkspaceView.vue:1916`) | opacity + `scale` 0.96 từ `--ccm-origin` | opacity `fast`, scale `base`; leave opacity `fast` |
| Dialog `ccm-dialog` + scrim | ~40 file (vd `CommandPalette.vue:744`) | opacity (backdrop) + `scale` của `.ccm-sheet` | enter `base` quint; leave `fast` exit |
| Panel trượt `ccm-panel-left/right` | css 1460 | opacity + translate +-24px | `base` / `fast` |
| Sheet `ccm-sheet-up` | css 1484 | translate | `base` / `fast` |
| Toast | `ToastHost.vue:38-43` | Tailwind: opacity + translate-y-1; move transform | enter `base`, leave `fast`, move `base` (chỉ `motion-safe`) |
| ZoomHud | `ZoomHud.vue:23-27` | opacity + translate-y | giống toast |
| List `ccm-list` (rail rows `RailClusterView.vue:282`, tab, queue, jobs...) | css 1406-1432 | opacity + translateY(-6px) in; scale 0.98 out; `-move` transform | `base` / `fast`; leaving item `position:absolute` (`pinLeaving`) |
| Tab `ccm-tab` | `WorkspaceView.vue:1773`; css 1777 | transform/opacity (chưa rõ chi tiết) | `base` |
| Tab pill trượt | `WorkspaceView.vue:1763,1769`; `useTabStripVisuals.ts:19` | transform + **width** | `slow` quint; `pillReady` sau 1 rAF để không animate lần đầu |
| Segmented thumb | `ui/SegmentedControl.vue:29,65` | transform + **width** | `slow` **ease-spring**; có `thumbReady` |
| Rail fold project | `style.css:1902-1966` (`ccm-rail-fold`, `-chevron`, `-stem`) | **grid-template-rows** + transform/opacity; stagger `--fold-i * 28ms` | in `base` quint, out `fast` exit + delay 40ms |
| Rail row text swap `ccm-rail-word` | `RailRow.vue:179`, `RailGroupHead.vue:128,145` | opacity | `fast` |
| Rail row chip `ccm-fade` | `RailRow.vue:160,186-197` | opacity | `base` in / `fast` out |
| Glyph pane `ccm-glyph-pane` | css 1923 | SVG `x,y,width,height` | `base` quint (layout SVG, nhỏ) |
| ContextRing / AgentStatusChip gauge | `ContextRing.vue:52,75`; `AgentStatusChip.vue:41` | `stroke-dashoffset,stroke` `slow`; thanh `width` `base` | **width animate** ở AgentStatusChip |
| Pane ring / dim | `WorkspaceView.vue:2038,2161`; `TermPane.vue:1227,1477,1482,1511` | outline-color, opacity (dim overlay `bg-canvas/45`) | `base` / `fast` / `slow` lẫn lộn |
| Divider splitter hover | `WorkspaceView.vue:2187` | background-color | `fast` |
| Drop-zone preview | `WorkspaceView.vue:2195` | **left, top, width, height** (`motion-safe`) | `fast` (layout prop; 1 element) |
| Pane morph khi split/close/toggle sidebar | `TermPane.vue:1230` (`view-transition-name: ccm-pane-<key>`); css 1755-1766; `withViewTransition` (`motion.ts:68`) | View Transitions API (snapshot, clip) | `base` quint |
| View transition root | `style.css:939` | crossfade root | `base` |
| Mission Control | `MissionControl.vue:278-451`; `mission/missionMotion.ts:77` | WAAPI FLIP transform + opacity (scale 0.96 -> 1, translateY 8px); tile <-> body | `dur('base'|'fast')` + `ease('out-quint'|'exit')` (helper trong `lib/motion.ts`) |
| AttachmentLook | `AttachmentLook.vue:56,63` | WAAPI transform + opacity | `base` quint / `fast` exit |
| KeyHint WAAPI | `KeyHint.vue:27-36` | transform, `borderBottomWidth`, `backgroundColor` | **220ms / 300ms hằng số cứng**, không qua token |
| FlipChar / FlipClock | `FlipChar.vue:7`; css 1835 | rotateX(-90deg) + translateY + opacity | `slow` quint, `backface-visibility: hidden` |
| CountNum | `composables/useCountUp.ts:5-23` | rAF cập nhật text, ease out quart | **450ms mặc định, hằng số cứng**; tắt khi reduced |
| LottieArt | `LottieArt.vue:49-60` | SVG renderer lottie-web light | reduced: dừng ở frame cuối (hoặc 0 nếu loop); chưa rõ có pause khi off-screen |
| Dust "disintegrate" (delete) | `lib/dust.ts:4-15,281,357,363,394`; `useCloseFlow.ts:207,231,254` | canvas WAAPI scatter + view-transition `ccm-dust` | scatter **1000ms** cubic-bezier(0.4,0,0.7,1), stagger 24ms/flake, `MAX_PIXELS 160000`; gather 540ms/layer quint, fade `fadeMs()`; bắt đầu từ `desktop.captureRect`, **mặc định tắt** (`ccm.dust`), tắt khi reduced |
| Undo close (reverse) | `useCloseFlow.ts:207-254` | scale 0.98 -> 1 + opacity, giữ pane ẩn `holds` 4000ms | `ms` từ token, ease `cs` |
| Disk canvas | `disk/useCanvas.ts:53` | rAF paint canvas | chưa rõ có dừng khi ẩn |
| Office timelapse | `office/OfficeTimelapse.vue:183-213` | rAF loop canvas | Office đang paused (memory), chưa đề xuất |
| Onboarding tour | `OnboardingTour.vue:58,199` | spotlight, scrim | chưa rõ |
| Spinner chung | A1 | rotate | `animate-spin` 1s linear (Tailwind) vs `.ccm-spin` 2.4s: **hai tốc độ khác nhau** |

### A3. Giá trị duration / easing đang dùng - độ lệch

1. **Token hóa tốt** cho phần lớn UI (`fast` 150 / `base` 220 / `slow` 300). Easing: quint cho enter, `ease-exit` cho leave, spring cho thumb / node / kbd.
2. **Hằng số cứng ngoài token**: `KeyHint.vue` 220ms + 300ms; `useCountUp.ts` 450ms; `dust.ts` 1000 / 540 / 24ms; `useCloseFlow.ts` 4000ms; `.ccm-swap-leave-active` 100ms (`style.css:1060`, trùng `press` nhưng ghi số); `ccm-to-tray` 560ms / `from-tray` 460ms / `peer-ring` 600ms; `ccm-spin` 2.4s vs 0.9s vs Tailwind 1s; breathe 2.2s, pulse 1.6s, shimmer 1.4s, boot 1.1s / 1.4s.
3. **Lệch easing**: `ease-in-out` (CSS mặc định) ở twinkle / breathe / pulse / shimmer / boot; `linear` ở spin / meteor / sweep; `step-end` ở caret. Chấp nhận cho loop, nhưng chưa có token riêng.
4. **Spring chỉ ở vài chỗ** (thumb 300ms, node, kbd, pill). Segmented thumb dùng spring nhưng tab pill (cùng bản chất) dùng quint: **không nhất quán**.
5. **Pane dim 220ms ngay cả khi leave** (`WorkspaceView.vue:2163`, `leave` = `base` thay vì `fast`): lệch quy tắc "exit nhanh hơn enter".
6. **Hai cơ chế đóng/mở overlay** cùng tồn tại: `<Transition>` CSS (dialog, menu) và WAAPI (`motion.ts play`, Mission Control). Cùng token nhưng khác chỗ khai báo.

### A4. Thứ đang chuyển động mà không cần

- `ccm-twinkle` ở `html::after` chạy vô hạn khi cửa sổ mất focus / ẩn: `usePageActive` không thấy gắn cho CSS animation (chưa rõ có `animation-play-state` theo focus).
- `ccm-meteor-pane`: sao băng vẽ lại **mọi panel + terminal well** (background-position), chỉ để trang trí. Rủi ro lớn nhất khi sang blur.
- Twinkle `::before` trong từng panel (nhân theo số pane).
- `ccm-peer-sweep` 1.4s, `ccm-quota-delta` 2.2s, `ccm-highlight` 1.6s: dài hơn khoảng đủ để mắt nhận ra (đề xuất <= 900ms).
- `ccm-kbd` stagger theo vị trí x khi bật hint (trang trí).
- Spinner xoay 2.4s (`ccm-spin`) cho việc hiếm, vẫn chạy khi ẩn.
- Dust 1s trên delete (đã tắt mặc định, giữ nhưng xem B).
- FlipChar rotateX 3D cho số đồng hồ (`backface-visibility`, tạo layer riêng mỗi char).

### A5. Thay đổi trạng thái chưa có feedback (nên có)

| Sự kiện | Hiện tại | Nên |
|---|---|---|
| Session đổi status working <-> asked <-> idle ở rail row | `ccm-fade` chip + `ccm-rail-word`, thanh `ccm-rail-bar` breathe 3 vòng; **dot trạng thái** đổi màu (chưa rõ có `transition: background-color`) | cross-fade dot 150ms + 1 lần pulse khi vào `asked` |
| Pane vừa chuyển sang "Cần bạn" khi pane đó không focus | `ccm-attention` ring 3 vòng (`style.css:1010-1019`) | giữ, thêm nảy nhẹ cho rail row tương ứng (hiện chỉ chip) |
| Session mới xuất hiện trong rail (từ `open_session` / MCP / link) | `ccm-list` enter translateY(-6px) | ok; thêm highlight 1 lần để biết "vừa mở" (hiện `ccm-highlight` chỉ ở ActivityFeed) |
| Lỗi session (`failed`) | đổi chip | rung ngang 1 lần nhỏ hoặc flash viền danh sách (reduced: chỉ đổi màu) |
| Copy / lưu thành công (button) | chưa rõ chung | chuyển icon sang check 150ms |
| Kéo splitter | màu divider đổi | OK; chưa có snap feedback khi chạm min width (chưa rõ) |
| Context gauge vượt ngưỡng (warn / critical) | stroke màu đổi `slow` | thêm 1 lần pulse khi qua ngưỡng |
| Đổi theme / palette | root view-transition crossfade có (`style.css:939`) khi gọi qua `withViewTransition` (chưa rõ ThemePicker gọi) | xác nhận, xem B |
| Chuyển Monitor <-> Workspace | `ccm-view` opacity | giữ, thêm translate nhẹ (xem B) |
| Pane reconnect / stale (mất collector) | chưa rõ | mờ nhẹ + chấm vàng, không loop |

---

## Part B - Đề xuất hệ motion Sương kính

Nguyên tắc: giữ nguyên bộ token đã có (đã đo ổn), đổi tên nhẹ cho gọn và thêm token còn thiếu. **Tên cũ giữ làm alias** để giao diện classic không đổi (bất biến plan: classic render như 4.0.5).

### B1. Token (`data-ui="next"`, thêm vào `tokens` của mist)

| Token | Giá trị | Dùng cho |
|---|---|---|
| `--ccm-dur-instant` | 80ms | press, toggle, hover màu (alias cũ `press` 100ms) |
| `--ccm-dur-fast` | 150ms | leave, tooltip, chip, fade nhỏ |
| `--ccm-dur-base` | 220ms | enter menu/dialog/row, pane ring |
| `--ccm-dur-slow` | 320ms | drawer, tab pill, thumb, view switch |
| `--ccm-dur-loop-slow` | 2400ms | breathe / shimmer (loop) |
| `--ccm-dur-loop-fast` | 900ms | spinner xoay |
| `--ccm-stagger` | 24ms, cap 6 phần tử (144ms) | rail fold, kbd |
| `--ease-standard` | cubic-bezier(0.2, 0, 0, 1) | chuyển trạng thái màu / vị trí cố định |
| `--ease-enter` (emphasized) | cubic-bezier(0.22, 1, 0.36, 1) (= quint, giữ) | enter, trượt vào |
| `--ease-exit` | cubic-bezier(0.4, 0, 1, 1) (giữ) | leave |
| `--ease-spring` | cubic-bezier(0.34, 1.32, 0.64, 1) (giữ) | thumb, pill, node: **chỉ** phần tử có thể "nảy" 1 lần |
| `--ease-loop` | ease-in-out (breathe / shimmer), linear (spin) | loop |

Bỏ `--ccm-dur-emph` 520ms và `--ccm-dur-signal` 900ms khỏi UI next (gộp: biểu đồ dùng `slow` 320 + stagger; signal một lần 600ms ghi riêng ở từng spec). JS: đọc token 1 lần qua `getComputedStyle` vào `lib/motion.ts` (thay hằng cứng `EASE_*`, `KeyHint` 220/300, `useCountUp` 450).

### B2. Quy tắc

1. **Chỉ animate `transform` + `opacity`** (kể cả `translate`, `scale`, `rotate` độc lập). Màu (`background-color`, `border-color`, `color`) cho hover / state chỉ ở element **không nằm trên layer blur lớn** và không quá 150ms. Cấm animate: `width/height/top/left`, `grid-template-rows`, `clip-path`, `box-shadow`, `filter`, `backdrop-filter`, `background-position`.
2. **Không bao giờ animate blur.** Panel glass **bật blur ngay, tĩnh**; fade-in một panel kính bằng opacity trên **wrapper** chứa `backdrop-filter`: Chromium sẽ tính lại blur mỗi frame khi opacity đổi, nên với panel lớn (rail, Explorer) dùng **translate/scale trên wrapper không có blur** hoặc để panel hiện tức thì + nội dung fade. Menu / dialog nhỏ (<= 360x480px) fade opacity chấp nhận được.
3. **Overlay đè xuống pane terminal**: không fade lớp có blur phủ xterm WebGL (Explorer drawer, Ctrl+K scrim). Dùng scrim opacity phẳng (không blur) animate, panel blur chỉ translate. Hoặc scrim không blur + panel blur tĩnh trượt bằng `translate`.
4. **Tối đa đồng thời**: 3 animation vào/ra overlay, 8 row rail (stagger cap 6), 1 loop "attention" mỗi pane (3 vòng rồi dừng), 1 spinner xoay mỗi vùng. Loop vô hạn toàn app <= 3 (twinkle, 1 working indicator tổng hợp, boot).
5. **Tạm dừng khi ẩn**: `visibilitychange` hidden hoặc mất focus > 5s đặt root `data-active="off"` -> CSS `animation-play-state: paused` cho mọi loop; rAF loop (PeerEdge, TreeEdge, canvas) đã dùng `usePageActive` thì gom lại một chỗ. Off-screen: `IntersectionObserver` cho LottieArt và canvas (chưa rõ hiện có).
6. **Layer**: `will-change` chỉ cho twinkle (đã có); không `will-change` cho mọi row. Layer blur không nhân đôi: tối đa 4 phần tử `backdrop-filter` cùng lúc (rail, tab bar, drawer, dialog).
7. **Terminal không scale**: không animate scale / blur lên xterm; pane morph vẫn bằng View Transition clip (giữ `style.css:1755-1766`).
8. **Một nguồn setting**: thêm **Settings -> Appearance -> Motion** (Full / Reduced / Off), lưu `ccm.motion`, root `data-motion="full|reduced|off"`; mặc định theo `prefers-reduced-motion` của OS (OS reduce -> Reduced). `reducedMotion()` trong `lib/motion.ts` đọc `data-motion` rồi mới tới matchMedia.

### B3. Ánh xạ reduced motion

| Mức | Chuyển động | Hành vi |
|---|---|---|
| Full | tất cả | như bảng B4 |
| Reduced (mặc định khi OS bật) | transform, scale, translate, loop trang trí, sao băng, dust, FLIP, view transition | bỏ hết transform; giữ **fade opacity <= 120ms** và đổi màu; loop trạng thái (working) thành chấm tĩnh + văn bản; spinner thay bằng chấm nhấp nháy 1 lần hoặc text "..." |
| Off | mọi thứ | tức thì, 0ms, kể cả fade; chỉ giữ spinner xoay tối thiểu **hoặc** icon tĩnh (tuỳ chủ nhân) |

Bắt buộc luôn: trạng thái không chỉ bằng màu hoặc chuyển động (đã đúng luật dự án: icon + text + color).

### B4. Spec theo component (next UI)

| Thành phần | Enter / thay đổi | Leave | Property | Duration + easing | Reduced |
|---|---|---|---|---|---|
| Rail row đổi status (working / asked / idle / failed) | dot cross-fade màu; chip text đổi `ccm-rail-word`; vào `asked`: 1 pulse scale 1 -> 1.25 -> 1 trên dot | - | opacity, color dot, transform dot | 150ms standard; pulse 480ms spring, 1 lần | chỉ đổi màu 120ms |
| Thẻ "Cần bạn" (hàng đợi) | vào: translateY(-6px) + opacity, stagger 24ms | ra: opacity + scale 0.98 (không trượt, thẻ đã được xử lý) | transform, opacity | in 220 ease-enter; out 150 ease-exit | opacity 100ms |
| Attention pulse | viền ring cạnh row/pane: opacity 0.35 <-> 0.9 (ring tĩnh, chỉ opacity chạy) | tắt khi pane được focus hoặc trả lời | opacity | 1.6s ease-in-out, **3 vòng rồi dừng**; lặp lại mỗi 60s nếu chưa xử lý (mới) | ring tĩnh không chạy |
| Rail thu gọn -> cột avatar | rail đổi chiều rộng **tức thì** (layout), nội dung text fade out 100ms, avatar fade in 150ms delay 60ms; dùng 1 View Transition (như pane morph) thay vì animate width | ngược lại | opacity + View Transition clip | 220 standard | tức thì |
| Tab thêm | tab mới: opacity + `scale 0.96 -> 1` | tab đóng: opacity + scale 0.98, `pinLeaving`, các tab còn lại `-move` transform | transform, opacity | in 220 enter; out 150 exit; move 220 enter | opacity 100ms |
| Tab pill (nền active) | **trượt bằng `translate` + `scaleX`** thay vì `width` | - | transform | 320 ease-spring (đồng bộ với segmented thumb) | tức thì |
| Tab reorder (kéo) | item theo con trỏ không animate, các tab khác `-move` transform | - | transform | 220 enter | tức thì |
| Pane mở / đóng / split | View Transition `ccm-pane` giữ nguyên (clip, không scale terminal) | đóng: dust nếu bật, không thì fade 150 | View Transition | 220 quint | 0ms (đã có) |
| Pane zoom | View Transition cùng cơ chế | - | VT clip | 220 quint | 0ms |
| Explorer drawer (đè lên pane) | panel **trượt translateX(-24px) + opacity** vào; nội dung bên trong không animate riêng; blur **tĩnh** | trượt ra 150 exit | translate, opacity | in 320 ease-enter; out 150 ease-exit | opacity 100ms |
| Explorer pin (ghim thành cột) | gọi 1 View Transition: drawer thành cột, pane co lại (clip), không animate width | ngược lại bằng VT | View Transition | 220 standard | tức thì |
| Ctrl+K palette | scrim opacity 0 -> 1 (phẳng, không blur); sheet `scale 0.97 -> 1` + opacity | opacity 100ms | opacity, scale | in 220 ease-enter; out 100 ease-exit | opacity 100ms |
| Menu / popover | opacity + `scale 0.96` từ `--ccm-origin` (giữ) | opacity 100ms | opacity, scale | in 150 / 220 (như hiện tại); out 100 | opacity |
| Dialog | scrim + sheet scale 0.96 -> 1 | scale 0.98 + opacity | opacity, scale | in 220 enter; out 150 exit | opacity |
| Toast stack | từ dưới: translateY(8px) + opacity; toast cũ đẩy lên bằng `-move` transform | opacity 150, thu nhỏ khoảng trống bằng move | transform, opacity | in 220 enter; out 150 exit; move 220 | opacity |
| Segmented thumb | trượt translate + scaleX (thay `width`) | - | transform | 320 ease-spring | tức thì |
| Chip đổi trạng thái | cross-fade (text swap `ccm-swap` 150) + đổi màu | - | opacity, color | 150 standard | 100ms |
| Context gauge | stroke-dashoffset (SVG paint, nhỏ) giữ; thanh `width` ở `AgentStatusChip` chuyển sang `scaleX` origin-left | - | transform / stroke-dashoffset | 320 standard; pulse khi qua ngưỡng warn/critical | tức thì |
| Working indicator | **chấm đập (pulsing dot) 1.6s opacity 0.5 <-> 1**, không spinner; thêm thanh `ccm-rail-bar` hiện hữu bỏ breathe, chỉ giữ tĩnh | - | opacity | loop 1.6s ease-in-out, **pause khi ẩn**, tối đa 1 loop vô hạn mỗi hàng rail (chung 1 keyframe, delay lệch) | chấm tĩnh |
| Shimmer | chỉ cho skeleton (loading list), opacity 0.45 <-> 0.9 | - | opacity | 1.4s | tĩnh 0.7 |
| Spinner (loading thao tác) | `LoaderCircle` xoay | - | rotate | 900ms linear, **một class duy nhất** `.ccm-spin` (bỏ `animate-spin`) | thay bằng icon tĩnh + text |
| Dust delete | giữ, tắt mặc định, chỉ chạy khi user bật **và** pane không blur dưới (xem rủi ro) | gather khi undo giữ 540ms | canvas WAAPI | rút scatter 1000 -> **700ms**, stagger 24ms cap tổng 400ms | tắt |
| Stars / meteors Ngân hà trong glass | twinkle: giữ 1 layer html::after (opacity); panel twinkle `::before` **bỏ** nếu panel có blur (sao nằm dưới kính, đã mờ); meteors: **bỏ `ccm-meteor-pane`** (background-position), giữ `body::before/::after` (translate + opacity), chạy dưới lớp kính nên bị blur nhẹ, hợp "sương" | - | opacity, translate | twinkle 3.4s; meteors 7s / 11s | tắt, ẩn |
| Switch view Monitor <-> Workspace | opacity + translateY(6px) rất nhẹ (view vào), view ra opacity 100ms | - | opacity, translate | in 320 enter; out 100 exit | opacity 100ms |
| Đổi palette / theme | `withViewTransition` root crossfade (giữ), không animate từng token | - | VT root | 220 | tức thì |
| Classic <-> next UI | crossfade root bằng View Transition 220ms, rồi reload layout (đổi `data-ui` trong callback) | - | VT root | 220 | tức thì |
| Mission Control | giữ FLIP WAAPI hiện tại | giữ | transform, opacity | token mới | tức thì |

### B5. Ngân sách hiệu năng

- Frame budget 16.6ms; mục tiêu: **không jank khi 8 pane terminal chạy output** và 1 overlay animate.
- Chỉ compositor: mọi animation được ghi trong B4 là transform / opacity; ngoại lệ **View Transition** (browser snapshot) và `stroke-dashoffset` SVG nhỏ.
- Số layer blur đồng thời <= 4; không chạy animation mà nhánh con của chúng có `backdrop-filter` thay đổi opacity trong lúc terminal output nhanh (đo bằng Chrome Performance, mục tiêu GPU raster < 4ms/frame).
- Loop vô hạn tổng <= 3 và đều `paused` khi `data-active="off"`.
- Test: E2E CDP đo `getAnimations().length` khi idle (mục tiêu: 0 animation không phải loop đã khai báo), kiểm `data-motion=off` -> `getAnimations()` rỗng.

### B6. Danh sách đổi Cũ -> Mới

1. Thêm setting Motion Full / Reduced / Off + `data-motion` (hiện **chưa có**), `reducedMotion()` đọc nó.
2. Token: thêm `instant` 80, `loop-slow`, `loop-fast`, `ease-standard`; `slow` 300 -> 320; bỏ `emph`/`signal` khỏi UI next; hằng số cứng (`KeyHint`, `useCountUp`, `dust`, `motion.ts EASE_*`) đọc token.
3. **Bỏ** `ccm-meteor-pane` (background-position trên mọi panel / terminal) trong UI next; bỏ twinkle `::before` của panel có blur.
4. Tab pill, segmented thumb, thanh gauge: `width` -> `transform: translate + scaleX`; đồng bộ spring cho 2 thanh trượt.
5. `ccm-rail-fold` (`grid-template-rows`) -> View Transition hoặc `height` tức thì + fade nội dung; drop-zone `left/top/width/height` -> transform nếu có thể.
6. `ccm-to-tray` / `from-tray` (`clip-path`) giữ ở Agent Map classic; next UI chưa rõ có dùng.
7. Spinner: gom `animate-spin` + `.ccm-spin` thành một class 900ms; thêm reduced cho các `animate-spin` còn thiếu (`CompactView.vue`, `DeviceConnectBar.vue`, `DeviceEmulators.vue`).
8. Pane dim leave `base` -> `fast`.
9. Working indicator: thay thanh `ccm-rail-bar` breathe bằng chấm đập (một keyframe dùng chung, pause khi ẩn).
10. Attention: ring `ccm-attention` giữ 3 vòng nhưng lặp lại sau 60s nếu chưa xử lý; thêm feedback A5 (status dot, `failed`, vượt ngưỡng context, session mới).
11. `data-active="off"` (hidden / mất focus) -> `animation-play-state: paused` toàn bộ loop; `usePageActive` gom thành 1 nguồn.
12. Dust: rút còn 700ms; không chạy khi pane có blur dưới (hoặc kiểm đo trước).
13. Chuyển UI (classic <-> next, palette) dùng root View Transition 220ms.
14. Giữ nguyên: View Transition `ccm-pane`, `pinLeaving` / `retireOverlay`, Mission Control FLIP, ZoomHud, easing quint / exit / spring.

### B7. Việc cần xác minh trước khi code (chưa rõ)

- `ccm-flash`, `ccm-highlight`, `ccm-quota-*`, `ccm-office-fullscreen-in`, `ccm-grow`: property cụ thể (box-shadow / background / width?) - đọc `style.css:982-1400`.
- ThemePicker có dùng `withViewTransition` không.
- LottieArt và `disk/useCanvas.ts` có tạm dừng khi off-screen / ẩn không.
- Animation CSS (twinkle, meteor, breathe) có dừng khi cửa sổ ẩn không (hiện chỉ rAF dùng `usePageActive`).
- Hiệu năng thực của View Transition `ccm-pane` khi pane nằm dưới `backdrop-filter` (cần đo Chromium của Electron đang dùng).
- Ảnh hưởng `backdrop-filter` + xterm WebGL: cần PoC trước khi chốt rule B2.2/B2.3.
