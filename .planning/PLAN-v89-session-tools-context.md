# PLAN v89 — Bảng MCP / plugin theo session + cảnh báo context 500k

Trạng thái: **đã code xong 2026-11-11**, chờ chủ nhân thử trên app. Typecheck sạch, Vitest pass
(trừ `desk-agent-data.twin` fail sẵn vì Windows chặn symlink), cargo test extensions pass, E2E Tauri
`v89-session-tools.cjs` 16/16. Chưa E2E nút restart `--resume` trên pane thật.

## Mục tiêu

1. Mỗi session Claude (pane Workspace, card Monitor) mở được 1 sheet liệt kê MCP server + plugin đang
   load cho **project của session đó**, bật/tắt từng cái. Ghi theo project, global không đổi.
2. Context hiện tại (token) hiện trên pane + card; vượt ngưỡng (mặc định **500k**, chỉnh ở Settings)
   thì đổi màu warning + toast 1 lần/session, có nút `Compact…`.

Lý do: context nền 84k/request (MCP `rea` ~200 tool, `figma`, `github`…) làm cache read tốn quota.

## Chủ nhân đã chốt (2026-11-11)

- Chỗ mở: **header pane Workspace + menu card Monitor**, cùng 1 sheet.
- Áp dụng: sheet có nút **Khởi động lại (`--resume`)**, chỉ bật khi agent idle; giữ hội thoại.
- Cảnh báo: **số context trên pane + card**, **toast 1 lần khi vượt**, **ngưỡng chỉnh trong Settings**.

## Thiết kế

### Dữ liệu (đọc)
- MCP: tên + scope từ `read_configured` (`core-desk/src/extensions/status.rs:18`) + `/mcp list`
  đã có (`ccm:ext-mcp`, có cả connector claude.ai / plugin MCP).
- Trạng thái tắt: `~/.claude.json` `projects[<cwd>].disabledMcpServers` (mảng tên), giống `/mcp disable`.
- Plugin: `enabledPlugins` của user `settings.json` (global) đè bởi `<cwd>/.claude/settings.local.json`.
- Chỉ trả **tên + scope + bật/tắt**. Không trả command / env / url (rule sanitize).
- Số tool / token mỗi server: **không hiện** (không có dữ liệu tin được).

### Ghi
- Channel mới `ccm:ext-project-toggle {cwd, kind: 'mcp'|'plugin', name, enabled}`, Rust `core-desk`
  `extensions/project.rs` + mirror Electron `src/extensions/project.ts`.
- `disabledMcpServers`: đọc-sửa-ghi `~/.claude.json` qua `claude::write_atomic` (backup `.ccm-backup`),
  chỉ đụng đúng key `projects[cwd].disabledMcpServers`, giữ nguyên phần còn lại.
- Plugin: `enabledPlugins[name] = false` trong `.claude/settings.local.json` của cwd; bật lại = xoá key
  (về theo global), không ghi `true`.
- Một mutation mỗi lúc (khoá chung với `mutate` của Extensions). Không native confirm (đảo lại được),
  có toast kết quả.
- `cwd` phải là thư mục project của 1 pane / session đang có (không nhận path tuỳ ý từ page).
- Giữ `orrery`: không cho tắt (toggle disabled + tooltip).

### UI
- `TermPane.vue`: entry `tools` trong overflow `more` (~:320) + chip nhỏ ở header khi rộng. Card
  Monitor: thêm entry vào `cardMenu()` (`lib/cardMenu.ts:46`).
- Component mới `SessionToolsSheet.vue`: 2 nhóm MCP / Plugin, mỗi dòng tên + scope pill + switch,
  connector claude.ai cũng tắt được theo project (tên dạng `claude.ai Google Drive`, đã thấy trong
  `~/.claude.json` máy này), plugin MCP dạng `plugin:<plugin>:<server>`. Đổi gì → banner "Áp dụng khi khởi động lại" +
  nút **Khởi động lại (`--resume`)** gọi `restart()` (TermPane.vue:848) với resume; card Monitor của
  session ngoài Workspace thì chỉ hiện dòng nhắc.
- Load skill `frontend-design` + `ui-ux-pro-max` trước khi làm; màu theo COLOR-SYSTEM, i18n en/vi.

### Cảnh báo context
- Dữ liệu có sẵn: `contextTokens` (`packages/shared/src/types.ts:113`, Rust `core-transcript/state.rs:86`).
- Setting `contextWarnTokens` (mặc định 500000, 0 = tắt) trong `stores/settings.ts`, UI ở
  `SettingsReminders.vue` cạnh `stuckMinutes`.
- `ContextGauge` (TermPane) + `SessionCard` hiện `412k`; ≥ ngưỡng → token `warning` + icon + text.
- Toast `kind: 'context'` 1 lần/session/ngưỡng (nhớ theo sessionId trong memory, reset khi compact
  làm `contextTokens` tụt dưới ngưỡng), nút `Compact…` mở flow v51 có sẵn.

## Bước

1. Rust `extensions/project.rs` (read + toggle) + test; mirror Electron + test twin.
2. Shim / preload / `lib` web API; channel vào permission `ccm.toml`.
3. `SessionToolsSheet.vue` + entry pane / card + i18n.
4. Setting ngưỡng + hiển thị + toast.
5. `.code_index/` + `docs/claude-code-integration.md` (§ mới: file + key ghi).
6. Typecheck, Vitest, cargo test, E2E Tauri mới `session-tools.cjs` (mở sheet, tắt `rea`, kiểm
   `~/.claude.json` sandbox, restart resume).

## Kiểm rủi ro (2026-11-11)
- Claude Code ghi `~/.claude.json` bằng `tmp.<pid>.<rand>` + rename, khoảng 1 lần/phút khi có session.
- Thử: ghi key `projects["F:/__ccm_probe__"].disabledMcpServers` (tmp + rename), chờ Claude ghi lại
  (06:57:52Z) → key **còn nguyên**: Claude đọc lại + merge trước khi ghi, không ghi đè từ memory. Đã xoá key thử.
- Còn khe race vài ms (Claude đọc → mình ghi → Claude ghi). Xử lý: đọc lại sau 1,5s, mất thì ghi lại 1 lần,
  vẫn mất → báo `conflict`.
- Key project dạng `F:/work/web/Orrery` (slash xuôi), file indent 2 space.

## Rủi ro
- `~/.claude.json` bị Claude Code ghi liên tục → ghi đè mất dữ liệu. Giảm: đọc ngay trước khi ghi,
  ghi atomic, chỉ sửa 1 key; test với file lớn.
- Key `disabledMcpServers` không có doc chính thức. Đã thấy thật trong `~/.claude.json` (9 project,
  gồm cả `claude.ai …` và `plugin:…:…`); vẫn diff lại sau 1 lần `/mcp disable` trên 2.1.289, ghi vào docs.
