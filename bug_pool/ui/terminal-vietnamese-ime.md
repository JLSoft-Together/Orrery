---
date: 2026-11-11
project: Orrery
category: ui
tags: [ime, telex, xterm, wkwebview, webview2, terminal, vietnamese]
status: fixed
files: [apps/web/src/lib/termIme.ts::installReplacementIme, apps/web/src/components/workspace/TermPane.vue::installReplacementIme, apps/web/src/components/workspace/DropTerminal.vue::installReplacementIme]
---

# Gõ tiếng Việt (Telex) trong terminal mất / trùng chữ

## Triệu chứng
macOS (Tauri WKWebView), bộ gõ Apple Vietnamese Telex: gõ trong Claude pane rồi bấm Delete ra `/uuuu`, chữ có dấu không thay chữ cũ, chữ đầu sau dấu cách bị mất.

## Nguyên nhân
- WebKit gửi `beforeinput` / `input` với `inputType = insertReplacementText` (thay cả từ trong textarea ẩn, không có composition). xterm 6 bỏ qua loại này → PTY không nhận được phần thay.
- `insertText` đến trước keydown → `_keyPressHandled` của xterm bỏ chữ đầu sau dấu cách.
- Textarea ẩn của xterm không khớp dòng lệnh sau Backspace / Enter nên vùng bị thay tính sai.

## Solution
`installReplacementIme(term)`:
- Với `insertReplacementText` thì gửi `\x7f` × phần khác nhau + phần mới (`replacementBytes`).
- Giữ textarea khớp dòng lệnh: Backspace bỏ 1 code point; Enter / mũi tên / ctrl / meta thì xoá textarea (`textareaSyncFor`).
- Gửi lại `insertText` mà xterm đã bỏ (`droppedInsert`), trừ khi đang trong composition.

Kiểm tra:
- Unit test `termIme.test.ts`.
- E2E `apps/desktop/e2e/term-ime.cjs` trên WebView2: composition kiểu Microsoft Telex, Backspace + ký tự kiểu Unikey, xoá giữa chừng → byte vào PTY đúng từng phím.
- Chủ nhân đã gõ tay: macOS Telex và Windows Unikey / EVKey đều OK.

## Context
Orrery 4.0.2 (chưa release), xterm 6. Tauri shell: WKWebView trên macOS, WebView2 trên Windows. Chromium dùng composition events nên đi nhánh khác, fix không ảnh hưởng.
