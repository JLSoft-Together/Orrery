# WP-v88-kit report: XML Preview opt-in, kit version riêng

## Kết quả

- Build app (Electron `apps/desktop/build.mjs`, Tauri `scripts/dist-tauri.mjs`) không còn Gradle / Java. Gradle chỉ còn trong `scripts/kit-release.mjs` (JDK 17-24) và CI job `layout-daemon.yml`.
- Pin kit: `apps/layout-daemon/kit.json` (version `1`, tag `layout-daemon-v1`, asset `layout-daemon-1.zip`, url, size, sha256). Hiện là PLACEHOLDER (size 0, sha chữ): `apps/layout-daemon/build/dist/` chỉ có `layout-daemon-3.5.0.zip`, không có zip version 1.
- Electron: `build.mjs` đọc kit.json (`daemon-kit.mjs` `readKitPin` / `kitEmbed`) và nhúng `__DAEMON_KIT__`. Placeholder thì không nhúng; `--dist` dừng với thông báo.
- Tauri: `apps/shell/src/resources.rs` `include_str!(kit.json)` + `embedded_kit` (chỉ nhận sha 64 hex, size > 0). Bỏ env `ORRERY_DAEMON_KIT`. `dist-tauri.mjs` `daemonKit()` dừng nếu placeholder.
- Dev: dùng `apps/layout-daemon/build/libs/layout-daemon.jar` nếu có, không thì dùng pin (TS `main.ts` `layoutKitSpec`, Rust `apply_layout_env`).
- Kit cũ cài theo app version (`daemon/3.5.0`): coi là chưa cài (cách an toàn, không chạy jar cũ). `kitsToPrune` giờ xoá mọi kit khác kit hiện tại khi cài; Settings hiện "Đang có kit 3.5.0, build này cần kit 1", nút Cập nhật, nút Gỡ xoá sạch.
- Cài / gỡ: kênh mới `ccm:layout-install-status`, `ccm:layout-uninstall` ở cả TS (`layout/install.ts`, `layout/ipc.ts`) và Rust (`core-desk/src/layout/install.rs`, `ipc_render.rs`). Cài dùng kênh có sẵn `ccm:layout-download` (vẫn có native confirm kèm dung lượng). Gỡ: dừng daemon, xoá cả `<dataDir>/layoutlib`, từ chối khi đang tải (`busy`), phát event bundle `missing`.
- UI: Settings, Integrations, "XML Preview" (`components/XmlPreviewSettings.vue`: trạng thái, version kit, dung lượng đĩa, dung lượng tải, Cài / Cập nhật / Thử lại / Gỡ, progress từ `ccm:layout-event`), có trong search settings (`xmlPreview` env). Pane XML (`LayoutPreview.vue`) và drawable (`DrawablePreview.vue`) hiện nút "Cài XML Preview…" khi chưa cài, đứng trước bước kiểm JDK; không tải ngầm. i18n en + vi (`i18n/xml-preview-v88.ts`, `layout-v32.ts`).
- CI: `desktop.yml` bỏ job `daemon`, bỏ setup-java, thêm bước tag = version; `release.yml` bỏ check / upload zip; mới `layout-daemon.yml` (dispatch với `version` + `publish`; push `apps/layout-daemon/**` chỉ build artifact). `test.yml` (test daemon) giữ nguyên.

## Lệnh chủ nhân cần chạy một lần

```
export JAVA_HOME=~/.gradle/jdks/jdk-21.0.13+9/Contents/Home
node scripts/kit-release.mjs            # build, pack build/dist/layout-daemon-1.zip, ghi sha thật vào apps/layout-daemon/kit.json
gh release create layout-daemon-v1 apps/layout-daemon/build/dist/layout-daemon-1.zip --repo JLSoft-Together/Orrery --title "Layout daemon kit 1" --notes "..."   # script in sẵn lệnh đúng
```

Rồi commit `kit.json`. Hoặc chạy workflow `layout-daemon` với `publish` = true (tự release và commit kit.json). Đổi kit sau này: `--version 2` (version bất biến, release đã có thì CI từ chối).

## Cổng

- core: `cargo test -j 4` 879 passed, 0 failed; clippy sạch. shell: 102 passed; clippy sạch.
- desktop: 1715 passed, 29 skipped (twin `desk-layout.twin.test.ts` 12 passed sau khi `cargo build -p core-bin` debug).
- web: 1908 + 12 mới passed; `DevicesView.test.ts` "qr popover" flake 1 lần trong suite đầy, chạy riêng 25/25 pass, cả trên bản stash.
- `npm run typecheck` sạch.
- Không dist, không release build, không E2E, không commit, không upload.

## Lưu ý

- `ccm:layout-render` / MCP `preview_layout` khi chưa cài vẫn trả `bundle-missing` (câu "open a layout once and download it" vẫn đúng, nút nằm trong pane).
- `ccm:layout-uninstall` xoá cả `models/` và `aars/` (cache dẫn xuất từ project), render cache tự tạo lại.
- Pane XML: thứ tự kiểm đổi thành jar → cài → JDK (trước là jar → JDK → cài).
- Lỡ chạy `node scripts/dist-tauri.mjs --bogus` khi kiểm thử: nó chạy `npm run build -w @ccm/web` rồi bị cắt; không để lại thay đổi nào ngoài `apps/web/dist` (gitignore).
