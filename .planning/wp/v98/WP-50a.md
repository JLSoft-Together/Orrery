## WP-50a: Spike Noise wasm — `core-remote-proto` chạy trong WebKit, bắt tay với desktop native qua relay

- Mục tiêu: trả lời "wasm của `core-remote-proto` có bắt tay Noise (ghép XXpsk3 + phiên IK) đúng byte với desktop Rust native qua relay, chạy trong WebKit không" bằng test chạy thật + số đo.
- Spec: `.planning/PLAN-v98-ios-pwa.md` quyết định #2, WP-50 (a), rủi ro "wasm Noise lệch byte".
- Hợp đồng giao thức: `docs/remote-integration.md` §1 (relay envelope), §2 (Noise, QR), §3 (frame), §6 (mã đóng); relay: `apps/relay/README.md`, `apps/relay/src/{index,room,token}.ts`.
- Phạm vi file được đụng: tạo mới `apps/core/crates/core-remote-wasm/**` (crate binding wasm-bindgen, sẽ dùng thật cho `apps/pwa` ở WP-54), thêm crate vào `apps/core/Cargo.toml` workspace members, `apps/core/Cargo.lock`, sửa tối thiểu `apps/core/crates/core-remote-proto/**` nếu cần cho wasm (vd. feature / cfg getrandom, mở `start_with` sau feature `test-vectors`) — không đổi hành vi native; tạo mới `scripts/spike-v98/wasm/**` (harness, peer desktop giả, Playwright); `docs/qa/v98/WP-50a/**`; báo cáo. Cấm đụng: `apps/shell/**`, `apps/web/**`, `apps/mobile/**`, `apps/relay/src/**`, root `package.json`.
- Model: opus. Song song với WP-50bc (Node, không cargo).

### Việc
1. `rustup target add wasm32-unknown-unknown`; build `core-remote-proto` cho wasm32 (getrandom 0.4 cần backend wasm_js — xem cách đúng cho 0.4 và cho mọi bản getrandom khác trong cây phụ thuộc của snow). Không đổi gì phía native: `cargo test -p core-remote-proto` vẫn pass như trước.
2. Crate `core-remote-wasm` (cdylib + rlib, `wasm-bindgen`): API tối thiểu cho phía điện thoại: sinh / nạp key tĩnh, ghép initiator XXpsk3 (đọc QR v1 payload hoặc tham số tách), phiên IK initiator, `Transport` encrypt / decrypt, encode / decode frame §3 (gồm ghép khúc `more`). Kiểu dữ liệu qua `Uint8Array` / JSON; khoá bí mật không lộ ra JS ngoài khi gọi export rõ ràng để lưu. Cài `wasm-bindgen-cli` đúng bản crate (ưu tiên tải binary release thay vì `cargo install` nếu được; `cargo install` thì `-j 4`). Xuất `--target web`. Ghi kích thước `.wasm` (thô + gzip).
3. Test vector chéo: chạy wasm (Node hoặc WebKit) với ephemeral / key cố định trong `apps/core/crates/core-remote-proto/tests/vectors.json`, so từng byte với vector native. Có lệnh chạy lặp được.
4. Peer desktop giả native (Rust, đặt trong `scripts/spike-v98/wasm/` dưới dạng crate riêng hoặc example của `core-remote-wasm` — chọn cách ít phụ thuộc mới nhất, ưu tiên crate đã có trong `apps/core/Cargo.lock`): register vào relay (Ed25519), nối `/m/<machineId>?ts&sig`, đóng vai responder ghép + phiên theo §1–§3, cấp token relay cho phone (định dạng `token.ts` / `core-remote-proto/src/relay.rs`), trả `pair.ok`, rồi phiên IK: nhận `hello`, gửi `welcome`, echo frame json + pty_out 64 KiB + ping.
5. Relay local: `apps/relay` cài cục bộ theo README (`npm install --workspaces=false --include-workspace-root=false`), chạy `wrangler dev` (không deploy, không login). Không dùng relay thật của chủ nhân.
6. Playwright WebKit trên Windows (cài trong `scripts/spike-v98/wasm/`, `npx playwright install webkit`): trang tĩnh nạp wasm, nối relay `ws://` local, ghép rồi phiên IK với peer, gửi / nhận các frame ở bước 4. Đo: thời gian nạp wasm, thời gian bắt tay ghép, bắt tay IK, round-trip 64 KiB. Chạy thêm Chromium cùng test để so. Một lệnh chạy cả chuỗi (relay + peer + Playwright), in tóm tắt ngắn.
7. Ghi rõ giới hạn: WebKit Windows của Playwright ≠ Safari iOS (JIT, WebCrypto, wasm SIMD); liệt kê điều phải kiểm lại trên iPhone thật.

### Tiêu chí nghiệm thu
- [ ] `cargo build -p core-remote-wasm --target wasm32-unknown-unknown` sạch; `cargo test -p core-remote-proto` pass (số test như trước); clippy crate mới sạch.
- [ ] Vector chéo wasm = native, từng byte, cho cả XXpsk3 và IK.
- [ ] Playwright WebKit: ghép + phiên IK + frame qua relay local pass (kèm Chromium).
- [ ] Bảng số đo + kích thước wasm; kết luận "đạt / không đạt" + rủi ro còn lại cho iPhone.
- [ ] `scripts/spike-v98/wasm/README.md` ≤ 40 dòng: lệnh chạy, yêu cầu cài đặt.

### Luật (bắt buộc)
- Không viết comment trong code (kể cả `///`, `/** */`, `#`, TODO); lý do / invariant ghi vào báo cáo.
- Không `git commit` / `push`. File mới ≤ ~400 dòng. Không build release để verify.
- Tool nặng ≤ 80% CPU / RAM (`cargo -j 4`), không chạy 2 build nặng cùng lúc. Không cài gì toàn cục ngoài `rustup target` và binary `wasm-bindgen`.
- Bash: `rtk` có thể rewrite `grep` → dùng tool Grep hoặc `rtk proxy grep`.
- Tắt mọi tiến trình nền (wrangler, peer) khi xong.

### Báo cáo
`.planning/wp/v98/WP-50a-report.md`: đã làm gì (file + symbol), test chạy thật (số pass / fail, lệnh), số đo, việc còn dở, rủi ro, gợi ý API cho WP-54.

### Nhật ký
- 2026-11-12 19:55 — Bước 1–3: target wasm32 cài; `core-remote-proto` thêm feature `test-vectors` (`PairInitiator::start_fixed`, `SessionInitiator::start_fixed`); crate `core-remote-wasm` build wasm32 sạch, clippy sạch; getrandom 0.4 + 0.3 bật `wasm_js` chỉ trong crate wasm (target wasm); wasm-bindgen 0.2.130 (binary release, `scripts/spike-v98/wasm/.bin`); vector chéo Node 28/28 pass.
- 2026-11-12 20:05 — Bước 4–6: peer native `scripts/spike-v98/wasm/peer` (ed25519-dalek 2.2 + tungstenite 0.30, crate riêng), relay `wrangler dev` local, Playwright 1.65 WebKit 27.2 + Chrome 157: ghép + IK + json / pty 64 KiB / ping / key lạ 4401 qua relay, 11/11 mỗi trình duyệt, vector trong trang 28/28; `node run.mjs` một lệnh, tắt sạch tiến trình.
- 2026-11-12 20:10 — Bước 7 + báo cáo `.planning/wp/v98/WP-50a-report.md`: ĐẠT; wasm 222.9 KiB thô / 83.9 KiB gzip; rủi ro iPhone: CSP `wasm-unsafe-eval`, Lockdown Mode, đo lại trên máy thật. `cargo test -p core-remote-proto` 40/40 như trước.
