# WP-50a — báo cáo: Noise wasm trong WebKit, bắt tay với desktop native qua relay local

**Kết luận: ĐẠT.** `core-remote-proto` build sang wasm32 (wasm-bindgen 0.2.130) cho ra đúng từng byte như native ở cả ghép `Noise_XXpsk3` và phiên `Noise_IK` (28/28 vector, chạy trong Node, WebKit và Chromium). Playwright WebKit 27.2 (Windows) ghép + nối IK + gửi / nhận frame với peer desktop Rust native qua `wrangler dev` local: 11/11 kiểm tra, Chromium cũng 11/11. wasm prod 223 KiB thô / 84 KiB gzip. Rủi ro còn lại nằm ở Safari iOS thật (CSP `wasm-unsafe-eval`, Lockdown Mode, nền / tiếp tục), không nằm ở mã hoá.

## Đã làm

| File | Nội dung |
|---|---|
| `apps/core/crates/core-remote-proto/Cargo.toml` | feature `test-vectors = []` (mặc định tắt) |
| `core-remote-proto/src/pair/mod.rs` | `PairInitiator::start_fixed` (chỉ khi `test-vectors`), gọi `start_with(.., Some(e))` |
| `core-remote-proto/src/session.rs` | `SessionInitiator::start_fixed` (chỉ khi `test-vectors`) |
| `apps/core/Cargo.toml`, `Cargo.lock` | thêm member `crates/core-remote-wasm`; lock thêm `wasm-bindgen 0.2.130`, `js-sys 0.3.107`, `bumpalo`, `rustversion`, macro crates |
| `apps/core/crates/core-remote-wasm/` (mới) | cdylib + rlib. `lib.rs`: `Identity` (`generate`, `fromPrivate`, getter `publicKey`, `exportPrivate`), `decodeQr` (JSON không có `code`), `protoVersion`, `error_code` / `fail` / `secs`. `handshake.rs`: `Pairing` (`fromQr`, `fromParts`, getter `msg1`, `finish(msg2) → msg3`, getter `desktopStatic` / `machineId`, `intoTransport`), `Session` (`start`, getter `msg1`, `finish(msg2) → Transport`); `fromQrFixed` / `startFixed` chỉ có khi feature `test-vectors`. `channel.rs`: `Transport` (`seal`, `open`, `sealMessage(kind, stream, payload, claim) → Uint8Array[]` đã chia khúc 60 KiB, `openMessage(ct, reassembler) → Message?`, `isClosed`, `remoteStatic`), `Reassembler` (`push`, `pendingStreams`, `clear`), `Message` (`kind`, `stream`, `claim`, `payload`, `text`), `encodeFrames` |
| `scripts/spike-v98/wasm/build.mjs` | wasm release → `pkg/`, wasm debug + `test-vectors` → `pkg-vectors/`, peer; kích thước thô / gzip -9 / brotli 11 → `docs/qa/v98/WP-50a/sizes.json` |
| `scripts/spike-v98/wasm/web/vectors.mjs` | `checkVectors(wasm, vectors)`: 28 kiểm tra dùng chung Node + trình duyệt |
| `scripts/spike-v98/wasm/vectors-node.mjs` | lệnh vector chéo lặp được |
| `scripts/spike-v98/wasm/peer/` (crate riêng, `[workspace]` rỗng) | `main.rs`: `POST /register` (Ed25519, HTTP thô qua `TcpStream`), WS `/m/<id>?ts&sig` (tungstenite sync), in `{machine, qr, bootToken}`, vòng đọc envelope §1. `conn.rs`: `Conn::{New, Pairing, Live, Done}` — msg1 50 B → `PairResponder`, `pair.ok {machine, name, relay, relayToken}` (token 30 ngày, `issue_token`); khác → `session::accept` (chỉ thiết bị đã ghép, đúng key), `hello` → `{re, ok, machine, name, version, minProto, locale}`, `echo`, `pty_in` → `pty_out` cùng stream, ping → ping; lỗi → control close `4400` / `4401` / `4426`. Thoát khi stdin đóng |
| `scripts/spike-v98/wasm/web/{index.html,spike.mjs}` | trang tĩnh: nạp wasm, đọc relay từ QR, ghép qua relay bằng token mồi, nối lại bằng `relayToken`, hello / echo / 20 × pty 64 KiB / ping, thử key lạ, đo |
| `scripts/spike-v98/wasm/run.mjs` | một lệnh: build nếu thiếu → vector Node → `wrangler dev` (state ở thư mục tạm, xoá khi xong) → mỗi trình duyệt một peer mới → Playwright → `results.json` + tóm tắt; `taskkill /T` mọi tiến trình con, kể cả khi lỗi / Ctrl+C |
| `scripts/spike-v98/wasm/{README.md,package.json,.gitignore}` | `playwright 1.65.0` (exact); ignore `node_modules`, `.bin`, `pkg*`, `peer/Cargo.lock` |
| `docs/qa/v98/WP-50a/{results,sizes}.json` | số đo lần chạy cuối |

## Test chạy thật

| Lệnh | Kết quả |
|---|---|
| `cargo test -p core-remote-proto` (trước + sau) | 40 pass / 0 fail (1 + 9 + 2 + 8 + 8 + 7 + 5), không đổi |
| `cargo test -p core-remote-proto --features test-vectors --test pair --test session` | 15 pass / 0 fail |
| `cargo build -p core-remote-wasm --target wasm32-unknown-unknown` (có / không `test-vectors`) | sạch, không warning |
| `cargo clippy -p core-remote-wasm` native `--all-targets --all-features` + wasm32 mặc định + wasm32 `--all-features`, `-D warnings` | sạch |
| `cargo clippy -p core-remote-proto --all-targets --all-features -D warnings`; clippy peer | sạch |
| `node vectors-node.mjs` | 28 pass / 0 fail |
| `node run.mjs` (chạy 4 lần, exit 0) | WebKit 27.2: 11/11 + vector trong trang 28/28; Chromium (Chrome 157.0.8103.39): 11/11 + 28/28 |

Vector phủ phía điện thoại: key tĩnh public từ private, export private, QR decode, XXpsk3 msg1 + msg3 + `desktopStatic` + transport "hello", IK msg1 + `remoteStatic` + transport "ping" — so hex với `tests/vectors.json`; msg2 native của cả 2 bắt tay được wasm giải đúng. Âm: `mkey` sai → `keyMismatch` trước msg3, QR hết hạn → `expired`, msg2 IK bị sửa 1 bit → `noise`, kind lạ → `unknownKind`, `claim` trên json → `badFrame`; frame 64 KiB = 2 khúc (header `0203deadbeef` / `0202deadbeef`), ghép lại đúng byte, `claim` giữ. Kiểm tra trong trang: QR có relay + mkey, msg1 ghép 50 B, `pair.ok` có `relayToken`, key desktop = `mkey`, IK welcome, json echo, 20 × pty 64 KiB nguyên vẹn, ping, transport còn mở, key lạ cùng `deviceId` bị đóng `4401` qua relay.

## Số đo (lần chạy cuối, relay + peer + trình duyệt cùng máy, headless)

| Chỉ số | WebKit 27.2 (Win) | Chrome 157 |
|---|---|---|
| Nạp wasm (`init`: fetch + compile + instantiate, prod) | 13 ms | 3.6 ms |
| Ghép: mở WS / bắt tay (msg1 → `pair.ok`) / riêng crypto | 37 / 56 / 2 ms | 8.3 / 9.5 / 3.4 ms |
| IK: mở WS / bắt tay (msg1 → transport) / riêng crypto | 64 / 28 / 2 ms | 7.9 / 4.5 / 1.3 ms |
| `hello` → welcome · json echo · ping | 31 · 32 · 32 ms | 2.2 · 2.3 · 3.2 ms |
| Round-trip 64 KiB pty (n = 20) min / median / p95 / max | 31 / 47 / 62 / 62 ms | 17.9 / 18.3 / 20.7 / 20.7 ms |
| `seal` 64 × 60 KiB | 234 MiB/s | 225 MiB/s |

Kích thước (`sizes.json`, release `opt-level = "s"`, LTO, chưa chạy `wasm-opt`): `.wasm` sau wasm-bindgen **222.9 KiB thô / 83.9 KiB gzip / 69.4 KiB brotli**; glue `core_remote_wasm.js` 36.5 KiB / 4.6 KiB gzip. Bản `test-vectors` debug 1.8 MiB (chỉ để test, không ship).

Đọc số: crypto wasm trên WebKit ngang Chromium (seal 234 vs 225 MiB/s, bắt tay ≤ 2 ms). Độ trễ ~30 ms mỗi message WebKit có cả ở ping rỗng, nên đến từ tầng WebSocket / vòng sự kiện của bản WebKit Windows (WinCairo), không phải wasm. WebKit làm tròn `performance.now()` về 1 ms.

## Lý do / invariant (không ghi trong code)

- getrandom: cây có 2 bản — 0.4.3 (`core-remote-proto`) và 0.3.4 (snow `use-getrandom`). Trên `wasm32-unknown-unknown` cả hai cần feature `wasm_js`; với 0.3.4 cờ `RUSTFLAGS --cfg getrandom_backend="wasm_js"` không còn cần (README 0.3.4: "allowed but no longer required"). Feature bật ở `core-remote-wasm`, trong `[target.'cfg(all(target_family = "wasm", target_os = "unknown"))'.dependencies]` (alias `getrandom03` cho 0.3), đúng khuyến nghị getrandom: chỉ crate đã phụ thuộc wasm-bindgen mới bật. Native không đổi gì. Hệ quả: `cargo build -p core-remote-proto --target wasm32-unknown-unknown` một mình vẫn lỗi `compile_error!` của getrandom — có chủ ý; build wasm luôn qua `core-remote-wasm`.
- `start_fixed` chỉ là vỏ gọi `start_with(.., Some(e))` dưới feature `test-vectors`; bản prod (`pkg/`) không có `fromQrFixed` / `startFixed`. Không có bản responder trong wasm vì điện thoại luôn là initiator.
- Khoá riêng chỉ ra JS qua `Identity.exportPrivate()`; `Pairing` / `Session` / `Transport` không lộ key. `decodeQr` bỏ `code` khỏi JSON.
- Thời gian (`now`, `exp`) qua JS là `number` (f64), kiểm số nguyên ≥ 0 ≤ 2^53 − 1 rồi đổi u64 — tránh BigInt.
- Lỗi ra JS = `Error` với `message` là mã ngắn ổn định (`noise`, `keyMismatch`, `expired`, `badFrame`, …, `badTime`).
- Build release chỉ cho riêng crate wasm (vài giây) để có kích thước và thời gian đại diện; kiểm tra đúng / sai (vector) chạy trên bản debug. Release profile workspace có `strip = true` làm mất section `target_features` → wasm-bindgen cảnh báo không dò được feature wasm; vô hại ở đây, WP-54 nên chạy `wasm-opt -Os` trên output wasm-bindgen (thường bớt thêm 10–20 %).
- Peer là crate riêng (`[workspace]` rỗng, lock khởi tạo bằng bản copy `apps/core/Cargo.lock` để giữ đúng phiên bản, `CARGO_TARGET_DIR=apps/core/target`) để `apps/core/Cargo.lock` không nhận dep spike; dep mới duy nhất `ed25519-dalek 2.2` (+ `ed25519`, `signature`, `der`, `const-oid`); tungstenite 0.30 đã có trong lock.
- Relay hiện bắt token cho mọi `/p/` nên chưa có cách ghép qua relay khi điện thoại chưa có token: spike dùng token mồi `pair-bootstrap` do peer in ra (thay cho luồng của WP-52).
- Sau `pair.ok`, gateway LAN đóng `1000`; qua relay control close chỉ nhận `4400/4401/4426/4503` nên spike để điện thoại tự đóng. WP-52 quyết định: thêm `1000` vào `DESKTOP_CLOSE` của relay, hoặc điện thoại luôn tự đóng sau `pair.ok`.
- serde_json không giữ thứ tự key (không bật `preserve_order`): test JS không được so chuỗi JSON thô.

## Việc còn dở

- Chưa chạy trên Safari iOS / iPadOS thật (ngoài phạm vi máy này) — xem danh sách dưới.
- Chưa chạy `wasm-opt`, chưa thử `simd128` (không cần: crypto đã > 200 MiB/s).
- Peer không gửi ping keepalive 15 s (WP-30) và không giới hạn inbox / 5 s như gateway thật — đủ cho spike, không dùng làm gateway.

## Rủi ro cho iPhone thật (WebKit Windows của Playwright ≠ Safari iOS)

1. **CSP**: Safari cần `script-src 'wasm-unsafe-eval'` (Safari ≥ 16) để compile WebAssembly; CSP chặt của #25 / WP-53 phải có nó, nếu không wasm không nạp. Kiểm ở WP-50bc / WP-53 trên máy thật.
2. **Lockdown Mode** tắt WebAssembly trong Safari → web app không chạy được; cần màn báo lỗi rõ ("tắt Lockdown Mode cho trang này") — không có đường JS thuần nếu giữ quyết định #2.
3. **JIT / tầng compile**: Windows WebKit có JIT; iOS dùng JavaScriptCore cùng họ nhưng thời gian compile lần đầu, bộ nhớ của PWA và chính sách JIT khác. Đo lại `init` lần đầu / có cache, bắt tay, 64 KiB trên iPhone (mục tiêu: `init` < 100 ms, bắt tay crypto < 20 ms).
4. **Wasm features**: Rust 1.99 mặc định bật `sign-ext`, `mutable-globals`, `reference-types`, `multivalue`, `bulk-memory`, `nontrapping-fptoint` (không SIMD). Safari iOS 16.4+ (mốc tối thiểu cho Web Push PWA) có đủ — xác nhận bằng chạy thật trên bản iOS thấp nhất cần hỗ trợ.
5. **WebCrypto / RNG**: getrandom `wasm_js` gọi `crypto.getRandomValues` — có trên Safari, kể cả trang http; trang thật là https nên không vướng secure context.
6. **Mạng thật**: `wss://` qua TLS Cloudflare + 4G thêm độ trễ; ~30 ms / message ở WebKit Windows không đại diện — đo lại RTT 64 KiB trên iPhone.
7. **Nền / tiếp tục**: Safari treo WebSocket khi PWA xuống nền hoặc khoá màn; cần nối lại IK nhanh (crypto 2 ms, phần lớn là mở WS) và giữ `Identity` trong IndexedDB.
8. **MIME**: `instantiateStreaming` cần `application/wasm` từ `/app/*` của relay (WP-53).

## Gợi ý API cho WP-54

- Dùng `pkg/` (`--target web`) nguyên trạng; bọc 1 lớp TS mỏng `NoiseLink { pair(qr, socket), connect(desktopStatic, socket), send(kind, stream, bytes, claim), onMessage }`, dùng `sealMessage` / `openMessage` để không phải tự chia / ghép khúc trong JS.
- Gọi `.free()` (hoặc `using` khi có) cho `Pairing` / `Session` sau khi lấy transport; glue có `FinalizationRegistry` nhưng không nên trông vào GC cho object giữ key.
- Lưu khoá: `exportPrivate()` → bọc AES-GCM bằng `CryptoKey` không trích xuất được (WebCrypto) trước khi cất IndexedDB, `Identity.fromPrivate` khi mở app; xoá bản rõ ngay.
- Bản `test-vectors` chỉ dùng cho Vitest / CI (`vectors.mjs` dùng lại được); không đưa vào bundle.
- QR v2 (WP-52) đổi `QrPayload` ở crate gốc là đủ — `Pairing.fromQr` dùng thẳng `QrPayload::decode`; `fromParts` giữ cho "dán mã" khi không quét được QR.
