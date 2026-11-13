---
date: 2026-11-13
project: Orrery
category: ui
tags: [attachment, paste, clipboard, image, alt+v, preview, terminal]
status: fixed
files: [apps/web/src/stores/attachments.ts::reconcile, apps/web/src/composables/usePaneAttachments.ts::fillPixels, apps/web/src/components/workspace/TermPane.vue::paste]
---

# Ảnh paste vào session hiện trong Attachments nhưng không preview

## Triệu chứng
Paste ảnh từ clipboard vào pane Claude (Alt+V hoặc Ctrl+V ảnh lớn): dòng `[Image #N]` có trong tray Attachments, icon ảnh nhưng không bấm xem / không có thumbnail.

## Nguyên nhân
1. Alt+V (phím paste ảnh của Claude Code trên Windows) đi thẳng PTY, không qua `TermPane.paste()`; `reconcile` tạo row `{ kind:'image', source:'terminal' }` không có `thumb/view/data`.
2. Ctrl+V: `paste()` gửi phím trước rồi mới `await imageData(blob)` → scan thấy `[Image #N]` trước khi có pixel.

## Solution
`reconcile` trả `blind` (row ảnh mới không pixel) → `usePaneAttachments.fillPixels` đọc ảnh clipboard điền `thumb/view` (bỏ qua lần đọc prompt đầu, không ghi đè row đã có pixel). `paste()` gọi `imageData` + `notePaste` trước `conn.write(key)`.

## Context
apps/web, Orrery 4.0.4, Claude Code 2.1.318, Windows.
