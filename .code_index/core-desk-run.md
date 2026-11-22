# core-desk-run: Run pane, Layout / Drawable / Flutter preview

Đường vào: `apps/core/crates/core-desk/src/run/ipc.rs` (`register`, kênh `ccm:run-*`), `layout/ipc.rs` (`ccm:layout-*`), `flutter_preview/ipc.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-desk/src/run/manager.rs` | `new_manager`, state các lần chạy (`Live`, `State`) |
| `apps/core/crates/core-desk/src/run/android/mod.rs` | build / install / launch Android (Gradle, adb) |
| `apps/core/crates/core-desk/src/run/flutter/mod.rs` | `flutter run`, hot reload |
| `apps/core/crates/core-desk/src/run/configs.rs` | run config lưu theo project, import từ IDE |
| `apps/core/crates/core-desk/src/run/settings.rs` | SDK path, toolchain |
| `apps/core/crates/core-desk/src/run/parse.rs` | phân tích log: crash, ANR, lỗi build |
| `apps/core/crates/core-desk/src/run/sessions.rs` | session chạy, phát `ccm:run-event` |
| `apps/core/crates/core-desk/src/layout/daemon.rs` | daemon layoutlib (JSON-RPC stdio) |
| `apps/core/crates/core-desk/src/layout/install.rs` | cài kit pin ở `apps/layout-daemon/kit.json` |
| `apps/core/crates/core-desk/src/layout/model.rs` | model dự án Gradle cho preview |
| `apps/core/crates/core-desk/src/layout/ipc_render.rs` | render XML / drawable |
| `apps/core/crates/core-desk/src/flutter_preview/service.rs` | widget preview live |
| `apps/core/crates/core-desk/src/flutter_preview/host_template.dart` | host Dart chạy widget |

Phía vỏ: `apps/shell/src/host_run.rs` (owner, confirm, `run.adbUse`, `layout.urlFor`), `resources.rs` (`apply_layout_env`).

## Bất biến

- Daemon layout không bật sẵn từ lúc mở app; chỉ warmup khi có tín hiệu XML Android (xem memory "No always-on JVM").
- Config chạy lạ (`trust-config`) và cài lại (`reinstall`) phải qua confirm native / confirm trên điện thoại.
- Kit layout tải về chỉ khi sha 64 hex + size khớp; JDK 17-24 khi build kit (`scripts/kit-release.mjs`).

## Liên kết

- Web: `web-run.md`. Plan: `.planning/PLAN-v31-*.md`, `PLAN-v32-*.md`. E2E: `run-pane`, `layout-speed`, `flutter-preview`, `drawable-preview`.
