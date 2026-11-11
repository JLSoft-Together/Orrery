# SPEC v97 — App Orrery trên điện thoại (Android / iOS)

Trạng thái: **đã chốt** (2026-11-12). Thiết kế kỹ thuật + gói việc: `PLAN-v97-mobile.md` (PLAN thắng spec khi lệch, vd LAN không dùng TLS — PLAN §1.1 T1).

Chủ nhân chốt 2026-11-12:
- App **Orrery riêng** trên điện thoại. **Không dùng `claude remote-control`** / app Claude.
- Kết nối **cùng mạng LAN** nếu được, không thì **qua server** = **Cloudflare Workers + Durable Objects**.
- App có **quản lý Workspace** như desktop, **1 Workspace chung** desktop ↔ điện thoại (§4).
- App **Flutter** (sau spike S0).
- Cỡ terminal: **bên nào đang gõ thì cỡ bên đó**.
- Phạm vi: **làm cả** — terminal + Workspace + Explorer, file viewer / editor, Changes / diff, Git Log, Run,
  Monitor (§6.2, §6.3).
- **Cho phép** nội dung session ra điện thoại (mã hoá đầu-cuối). Session trên máy đổi thì điện thoại hiện
  theo; bước đầu chậm một chút được (mục tiêu < 1 s trên LAN, < 3 s qua relay).
- Cài đặt: **chỉ APK + TestFlight**, chưa lên store.

---

## 1. Bức tranh tổng

```
 Điện thoại (app Orrery mobile)
        │  lệnh + event (JSON) · byte terminal (nhị phân) · mã hoá đầu-cuối
        ├──────── LAN trực tiếp (cùng Wi-Fi, tìm máy bằng mDNS) ───────┐
        └──────── Relay server (khi khác mạng, chỉ chuyển byte đã mã hoá) ┤
                                                                       ▼
 Máy tính: Orrery desktop (Tauri) ── Remote gateway (mới, trong core-serve Rust)
        ├─ dashboard store (session, agent, status, tool span)  — đã có
        ├─ PTY core `orrery-core` (terminal, ring replay 256 KiB) — đã có
        └─ Workspace (tab, cây chia pane, project)  — hiện chỉ nằm trong renderer, phải mở ra (§4)
```

Điện thoại chỉ là **màn hình + bàn phím thứ 2** của Orrery trên máy: mọi pane vẫn chạy trên máy tính,
máy phải đang bật Orrery (kể cả đang ẩn ở tray).

## 2. Dùng lại được gì

| Thứ đã có | Ở đâu | Dùng cho mobile |
|---|---|---|
| WS dashboard: `snapshot` rồi batch event có `seq`, `resync` khi hụt | `core-serve`, `packages/shared/src/types.ts` | List session / agent / status realtime, giữ nguyên format |
| PTY WS `/pty`: ring replay 256 KiB, backpressure, close `4001 overflow`, max 16 pane | `apps/core/crates/core-pty` | Terminal trên điện thoại: attach song song với desktop, mở lại thì có output cũ |
| `workspace.openSession` + `OPEN_POLICY` (submit, limit 16, dock cạnh pane gọi) | `apps/web/src/stores/workspace.ts`, `packages/shared/src/orreryApi.ts` | Lệnh "mở pane" từ điện thoại đi đúng đường MCP `open_session` đang dùng |
| Trạng thái chờ / xin quyền / xong việc, `attention` | hook bridge, `stores/attention.ts` | Badge + inbox trên điện thoại |
| Telegram topic (v82), ntfy (v76) | `apps/desktop/src/telegram/`, `phone/` | Thông báo khi app mobile đóng (MVP) |
| QR + pair code + mDNS | Devices tab v61 | Mẫu UX ghép điện thoại |

## 3. Kết nối: LAN trước, server khi khác mạng

### 3.1 LAN trực tiếp
- Gateway mở port riêng `CCM_REMOTE_PORT` (mặc định **4320**) trên IP LAN, **tắt mặc định**, bật ở
  Settings → Điện thoại. Không đụng port 4317 (vẫn chỉ `127.0.0.1`).
- Quảng bá mDNS `_orrery._tcp` (tên máy, fingerprint). App mobile tự thấy máy cùng Wi-Fi.
- TLS (rustls), cert self-signed sinh lần đầu ở `<data>/remote/`; app **ghim fingerprint** lấy từ QR,
  không tin CA hệ thống.
- Firewall Windows / macOS sẽ hỏi lần đầu → Settings có hướng dẫn + nút kiểm tra.

### 3.2 Qua server (relay)
- Máy tính **gọi ra** relay (WebSocket outbound, không mở port router), điện thoại cũng gọi ra relay;
  relay ghép 2 đầu theo `machineId` và **chỉ chuyển byte đã mã hoá** — server không đọc được terminal / prompt.
- Mã hoá đầu-cuối: Noise `XX` (X25519 + ChaCha20-Poly1305), key lấy từ lúc ghép (§5). Cùng 1 lớp mã hoá
  cho cả LAN, nên app chỉ có 1 đường code, khác nhau ở chỗ mở socket.
- Chọn server:

| | Cloudflare Workers + Durable Objects | VPS tự chạy (relay Rust, cùng repo) | Fly.io |
|---|---|---|---|
| Chi phí | free / ~5 USD/tháng | 4–6 USD/tháng | ~2–5 USD/tháng |
| WebSocket giữ lâu | có, WS hibernation (không tính tiền lúc im) | có | có |
| Vận hành | không phải quản server | tự cập nhật, TLS (Caddy) | ít |
| Push APNs / FCM | gọi từ Worker được | được | được |
| Đề xuất | **chọn cái này** | nếu chủ nhân muốn giữ hết trong nhà | — |

- App tự chọn: thấy máy qua mDNS + bắt tay LAN được trong 1,5 s → LAN; không thì relay. Đổi mạng giữa
  chừng (Wi-Fi → 4G) → nối lại qua relay, attach lại pane, lấy ring replay — không mất output.

## 4. Workspace dùng chung desktop ↔ mobile (thay đổi kỹ thuật lớn nhất)

Hiện tại tab / cây chia pane / project chỉ nằm trong **renderer** (`stores/workspace.ts`, lưu
sessionStorage + `ccm.workspace.last`). Core chỉ biết PTY theo id, không biết tab nào chứa pane nào.

| Cách | Mô tả | Ưu | Nhược |
|---|---|---|---|
| **A. Renderer làm chủ** (đề xuất MVP) | Renderer gửi bản tóm tắt Workspace (tab, pane, project, tên, preset, status) cho gateway mỗi lần đổi (debounce 200 ms); lệnh từ điện thoại được gateway chuyển vào renderer, renderer chạy bằng đúng action có sẵn (`openSession`, `requestClose`, `renameTab`, `split`…) | Dùng lại 100 % logic, limit, confirm, undo; ít rủi ro | Cửa sổ desktop phải đang sống (Tauri ẩn vào tray vẫn sống → được) |
| B. Core làm chủ | Dời model Workspace xuống `core-serve`, renderer và mobile đều là client | Đúng kiến trúc lâu dài, chạy được khi renderer chết | Viết lại `stores/workspace.ts` (đang test nhiều), rủi ro lớn với IDE — ngược rule "Workspace mượt là ưu tiên 1" |

Đề xuất: **A** cho MVP, ghi B vào backlog.

Quy tắc đồng bộ:
- **Cùng 1 Workspace**: mở pane trên điện thoại thì desktop cũng có pane đó trong đúng tab project
  (và ngược lại). Đóng trên điện thoại = đóng trên desktop (vẫn có undo trên desktop).
- Pane đang bận / file chưa lưu: desktop hiện confirm thì điện thoại cũng hiện câu hỏi đó, ai trả lời trước thì xong.
- **Kích thước terminal**: 1 PTY chỉ có 1 cỡ cột / dòng. Đề xuất: bên nào **đang gõ** thì cỡ màn bên đó
  thắng; điện thoại chỉ xem thì giữ cỡ desktop và cho zoom / kéo ngang trong khung terminal (không cuộn
  ngang cả trang). Claude / Codex TUI tự vẽ lại khi đổi cỡ.

## 5. Ghép điện thoại + bảo mật

1. Desktop: Settings → Điện thoại → **Thêm điện thoại** → QR chứa `machineId`, tên máy, IP LAN + port,
   fingerprint cert, key công khai của máy, mã ghép 1 lần (**hết hạn 5 phút**).
2. Điện thoại quét QR → tạo keypair X25519 / Ed25519 trong **Keychain (iOS) / Keystore (Android)** → gửi
   yêu cầu ghép (LAN hoặc relay).
3. Desktop hiện confirm native: *"Cho phép «iPhone của …» điều khiển Orrery?"* → lưu vào
   `<data>/remote/devices.json`.

| Rủi ro | Chặn bằng |
|---|---|
| Người cùng Wi-Fi / người biết `machineId` | chưa ghép = không bắt tay Noise được; relay từ chối `machineId` không có token thiết bị |
| Mất QR (bị chụp lại) | mã 1 lần, 5 phút, bắt buộc confirm trên máy |
| Mất điện thoại | khoá app bằng Face ID / vân tay; thu hồi từng thiết bị ở desktop; thiết bị không dùng 30 ngày tự hết hạn |
| Server relay bị xâm nhập | relay chỉ thấy byte mã hoá + `machineId` |
| Bị điều khiển lúc không biết | notification desktop mỗi lần điện thoại kết nối; chip 📱 trên pane điện thoại đang xem / gõ; nút "Ngắt mọi điện thoại" ở thanh trên; activity log ghi lệnh từ điện thoại |

Rule cứng của project bị chạm (**cần chủ nhân duyệt, ghi docs §11 mới**):
- *"Không đưa nội dung session ra ngoài frontend"* → app mobile là frontend thứ 2, nhận byte terminal
  và nội dung session (đã mã hoá đầu-cuối). Env / secret / app key / token PTY **không bao giờ** ra điện thoại.
- Gõ vào terminal từ điện thoại = chủ nhân tự gõ (như gõ trên desktop), **không** phải ngoại lệ zero-token.

## 6. App mobile

### 6.1 Công nghệ

| | Flutter (đề xuất) | Capacitor bọc web Vue có sẵn |
|---|---|---|
| Dùng lại code | protocol (sinh Dart từ `types.ts`), không dùng lại UI | stores Pinia, i18n, theme, xterm.js |
| UI Workspace trên màn nhỏ | viết mới cho điện thoại (cử chỉ, bottom sheet) | UI desktop không hợp màn 6 inch, đằng nào cũng phải làm lại phần lớn |
| Terminal | `xterm` (pub.dev) — **phải đo hiệu năng** với output Claude dài trên máy thật trước khi chốt | xterm.js, đã chạy ổn |
| Key thiết bị, biometric, QR, mDNS, push | plugin chín (`flutter_secure_storage`, `local_auth`, `mobile_scanner`, `nsd`, FCM) | có plugin, kém ổn hơn |
| Stack chủ nhân | mạnh nhất, có sẵn skill build iOS / release | — |

Đề xuất **Flutter**, làm 1 spike 1 ngày đầu tiên: xterm.dart render output Claude 256 KiB + gõ tiếng Việt
(Telex) trên Android + iPhone. Spike hỏng → đổi terminal sang WebView chứa xterm.js trong app Flutter.

### 6.2 Màn hình

| Màn | Nội dung |
|---|---|
| **Máy** | list máy đã ghép (có thể nhiều PC), trạng thái LAN / server / offline, ghép máy mới |
| **Workspace — project** | list project như rail desktop: tên, số pane, pane đang chờ / xin quyền nổi lên đầu, status icon + chữ + màu |
| **Workspace — tab** | các tab của project; mở tab → các pane xếp ngang, **vuốt trái / phải** đổi pane, chấm chỉ vị trí; iPad / máy gập: chia 2 pane như desktop |
| **Pane terminal** | full màn, thanh phím phụ: Esc, Tab, Shift-Tab, Ctrl-C, ↑ ↓ ← →, `/`, Enter; khung nhập dòng (gõ tiếng Việt thoải mái rồi gửi cả dòng); chip status + tool đang chạy + thời gian |
| **Mission Control** | lưới thẻ tất cả pane (như v50 / v52), lọc theo trạng thái, chạm để mở |
| **Mở pane** | chọn project (favorites + MRU) → preset Claude / Codex / shell → (tuỳ chọn) prompt → thêm vào tab project |
| **Thao tác** | đổi tên tab, đóng pane / tab, gộp tab, resume session đã đóng, `Compact…` |
| **Inbox** | pane chờ / xin quyền / xong việc, chạm để nhảy tới pane |

### 6.3 Viewer (chủ nhân chốt làm cả)

| Viewer | Trên điện thoại | Lấy dữ liệu từ |
|---|---|---|
| **Explorer** | cây thư mục của project (lazy), git mark, tìm file nhanh | `list-dir` / `list-files` / `watch` của cổng preview (`apps/shell/src/preview_*.rs`) |
| **File** | xem có highlight, sửa nhẹ + lưu (auto save theo setting desktop), không ghi đè khi file đổi trên đĩa | `file-read` / `file-write` của core-desk |
| **Changes / diff** | list file đổi, diff dạng gộp (unified, hợp màn dọc), stage / unstage | `changes-*` |
| **Git Log** | list commit + graph rút gọn, chi tiết commit, file đổi; branch: fetch / pull / tạo / xoá (confirm) | Git Log channel hiện có |
| **Run** | chọn config, start / stop / reload, xem log, gửi lỗi cho agent | `stores/run.ts` qua renderer |
| **Monitor** | bảng session + agent tree (Quỹ đạo dạng list, không vẽ canvas); Office để sau cùng | dashboard store |
| Devices, Extensions, Settings desktop | không làm (chỉ hợp trên máy) | — |

Đường đi: lệnh viewer từ app → gateway → cùng `ccm` dispatch của vỏ Tauri với owner `remote-<deviceId>`
→ **cùng cổng preview** (chỉ path nằm trong root của pane đang sống / grant, chặn `..` / symlink ra ngoài),
cùng confirm native trên máy cho thao tác nguy hiểm (xoá branch, ghi đè…). Điện thoại không có quyền nào
desktop không có.

Việc kỹ thuật thêm: `core-serve` hiện không link `core-desk`; gateway cần 1 trait `DeskBridge` do vỏ
(`apps/shell`) cắm vào lúc boot để gọi `desk::dispatch` / `preview_ipc::dispatch`, và event `ccm:changes-updated`
/ `ccm:preview-changed` được đẩy sang app theo owner.

## 7. Lệnh và event (gateway ↔ app)

1 WebSocket `/remote/v1` (bọc trong Noise). Frame JSON cho lệnh / event, frame nhị phân cho byte terminal
(`[1 byte kênh][paneId][byte]`).

| Lệnh từ app | Làm gì | Ai thực hiện |
|---|---|---|
| `hello {deviceId, appVersion}` | bắt tay, nhận `snapshot` (session + Workspace) | gateway |
| `ws.open {project, preset, prompt?, submit?, tab?}` | mở pane | renderer `openSession` |
| `ws.close {paneKey}` / `ws.closeTab {tabKey}` | đóng (confirm nếu bận) | renderer `requestClose` |
| `ws.renameTab` · `ws.moveToTab` · `ws.reopenClosed` | như desktop | renderer |
| `pty.attach {paneId, cols?, rows?}` / `pty.detach` | xem terminal, lấy ring replay | gateway → `/pty` (token PTY giữ ở gateway) |
| `pty.input {paneId}` + frame nhị phân | gõ phím / dán | gateway → `/pty` |
| `pty.resize {paneId, cols, rows}` | khi điện thoại đang gõ (§4) | gateway |
| `confirm.answer {id, choice}` | trả lời confirm desktop đang hỏi | renderer |

| Event tới app | Nguồn |
|---|---|
| `snapshot`, `terminal.*`, `agent.*`, `tool.span`, `activity` (lọc) | dashboard store hiện có |
| `ws.snapshot` / `ws.patch` (tab, pane, tên, project, focus) | renderer → gateway |
| `confirm.ask {id, text, choices}` | renderer |
| `pty.exit`, `pty.overflow` | PTY core |

Giới hạn: lệnh `ws.*` 30 / phút / thiết bị, attach max 4 pane cùng lúc / thiết bị, frame max 1 MiB.

## 8. Thông báo khi app đóng

- iOS / Android ngắt WS khi app ở nền lâu → mở app là nối lại + `snapshot`.
- MVP: Telegram topic / ntfy đã có; tin có link `orrery://pane/<id>` mở thẳng pane trong app mobile.
- Pha sau (đã có relay): push native qua relay — desktop gửi `{machineId, paneId, loại}` **không kèm nội dung**,
  relay gọi APNs / FCM. Cần Apple Developer account + Firebase project.

## 9. Pha làm + ước lượng

| Pha | Nội dung | Ước lượng |
|---|---|---|
| S0 | Spike: xterm.dart + tiếng Việt trên 2 máy thật; Noise trong Rust (`snow`) + Dart | 1–2 ngày |
| P1 | Gateway LAN trong `core-serve`: TLS, mDNS, ghép QR, devices, Noise, snapshot / event, bridge PTY; Settings → Điện thoại; test Rust | 6–8 ngày |
| P2 | Renderer ↔ gateway: `ws.snapshot/patch`, lệnh `ws.*`, `confirm.ask`, chip 📱, kích thước terminal; test Vitest + E2E | 4–5 ngày |
| P3 | App Flutter: Máy, project, tab + vuốt pane, terminal + phím phụ, mở pane, Mission Control, inbox; Android trước, iOS TestFlight | 10–14 ngày |
| P4 | Relay (Cloudflare Workers + Durable Objects) + tự chọn LAN / relay + nối lại khi đổi mạng | 4–6 ngày |
| P5 | Viewer: Explorer + File (xem / sửa) + Changes / diff; `DeskBridge` trong vỏ | 7–9 ngày |
| P6 | Viewer: Git Log + branch, Run, Monitor dạng list | 6–8 ngày |
| P7 | Push native APNs / FCM qua relay | 3–4 ngày |

Tổng: S0–P4 (terminal + Workspace, dùng được ngoài nhà) khoảng **5–7 tuần**; thêm P5–P7 (viewer + push) khoảng **3–4 tuần** → cả bộ **8–11 tuần** làm 1 mạch. Chia gói cho subagent chạy song song (gateway Rust ‖ app Flutter ‖ relay) rút được còn khoảng 5–7 tuần.

## 10. Rủi ro

- Terminal trên màn 6 inch: Claude TUI rộng ~80+ cột; ép cỡ điện thoại (~45 cột) làm desktop vẽ lại xấu
  lúc đang xem trên máy → luật "ai gõ thì cỡ người đó" phải test kỹ.
- Gõ tiếng Việt vào terminal trên điện thoại (bàn phím iOS / Gboard gửi composition) — đã từng phải sửa riêng
  cho macOS (`lib/termIme.ts`); khung nhập dòng là đường chính để tránh.
- Workspace cách A phụ thuộc renderer: renderer reload / crash → điện thoại thấy "máy đang khởi động lại", tự nối lại.
- Apple review app điều khiển máy tính từ xa: cần tài khoản demo + máy giả cho reviewer, hoặc chỉ cài qua TestFlight.
- Relay là điểm duy nhất ngoài máy: chết thì chỉ còn LAN.

## 11. Còn treo

- **Câu 7 — thông báo khi app mobile đang đóng / ở nền** (agent xin quyền, chờ, xong việc): iOS / Android tắt kết nối
  của app ở nền nên app không tự biết. Đề xuất: P1–P6 dùng Telegram / ntfy Orrery đã có (tin có link mở thẳng pane
  trong app); P7 làm push của chính app Orrery (cần Apple Developer account — TestFlight đằng nào cũng cần — và
  1 project Firebase). Chờ chủ nhân chốt.
- Nhiều máy tính trong 1 app: spec để sẵn màn **Máy** (list nhiều PC), mặc định làm luôn vì rẻ.
