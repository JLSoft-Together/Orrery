# relay: apps/relay, apps/layout-daemon, packages/mist-tokens, apps/mobile

## apps/relay (Cloudflare Worker, Durable Object)

Đường vào: `apps/relay/src/index.ts`, `wrangler.jsonc`. Workspace npm riêng (`!apps/relay` trong root `package.json`).

| File | Vai trò |
|---|---|
| `apps/relay/src/room.ts` | Durable Object phòng: ghép máy desktop với điện thoại, chuyển byte |
| `apps/relay/src/app.ts` | route HTTP / WS |
| `apps/relay/src/guard.ts`, `cidr.ts`, `bounce.ts` | chặn IP, giới hạn, trả lỗi |
| `apps/relay/src/pair.ts`, `token.ts`, `sign.ts` | pair, token, chữ ký |
| `apps/relay/src/web-bundle.ts` | phục vụ PWA (`scripts/relay-web.mjs`, `relay-bundle.mjs`) |

Bất biến: relay không giải mã nội dung (Noise end-to-end); test `apps/relay/test/*.test.ts`. Docs `docs/remote-integration.md`.

## apps/layout-daemon (Kotlin + Gradle)

Daemon layoutlib (Paparazzi) cho XML Layout Preview, JSON-RPC 2.0 qua stdio. `apps/layout-daemon/README.md`, `kit.json` (kit pin: sha + size), `apps/layout-daemon/build.gradle.kts`. Phát hành kit: `node scripts/kit-release.mjs` (JDK 17-24). Phía Rust: `core-desk-run.md`.

## packages/mist-tokens

Token màu / palette dùng chung cho web desktop và PWA (`@ccm/mist-tokens/tokens.css`, `palettes.css`). Dùng ở `web-next.md`.

## apps/mobile (Flutter + Rust)

Index riêng: `apps/mobile/.code_index/mobile.md` (đọc khi làm app điện thoại). Entry `apps/mobile/lib/main.dart`; Rust `apps/mobile/rust`; QA `node apps/mobile/scripts/check.mjs`. Docs `docs/orrery-remote.md`.
