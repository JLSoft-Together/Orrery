---
date: 2026-11-13
project: Orrery
category: config
tags: [tauri, acl, capability, notification, remote-url, webview]
status: fixed
files: [apps/shell/capabilities/default.json::local]
---

# Web Notification bị Tauri ACL chặn (trang app là remote URL)

## Triệu chứng
`[unhandledrejection] Command plugin:notification|notify not allowed by ACL`
Gọi thử: `notification.is_permission_granted not allowed on window "main", webview "main", URL: http://127.0.0.1:4479/ ... allowed on: [windows: "main", URL: local]`

## Nguyên nhân
Web gọi `new Notification(...)`; plugin notification của Tauri chuyển thành `plugin:notification|notify`. Capability `local` chỉ có `core:default` và chỉ áp cho URL local, còn trang app tải từ collector `http://127.0.0.1:<port>` nên Tauri coi là remote.

## Solution
`capabilities/default.json`: thêm `"notification:default"` và `"remote": { "urls": ["http://127.0.0.1:*"] }`. Cửa sổ main chỉ điều hướng tới app URL (`on_navigation`) nên không mở cho trang lạ. Kiểm bằng `__TAURI_INTERNALS__.invoke('plugin:notification|is_permission_granted')` → `true`.

## Context
apps/shell (Tauri 2.12.5), Orrery 4.0.2–4.0.4 Windows.
