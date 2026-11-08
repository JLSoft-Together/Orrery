# PLAN 2026-11-08 — generic agent-workflow API (owner approved all, 2026-11-08)

Orrery adds generic extension points any agent workflow can use. Never mention a specific workflow in code / docs.
Caller = MCP `orrery` (from the pane's agent or from hooks using `ORRERY_API` + `ORRERY_TOKEN`).

| WP | Item | Shape |
|---|---|---|
| 1a | `open_session` returns the pane id | `structuredContent.paneId` = id usable with `send_to_session` / `close_session` / `wait_session`; `name` allowed for codex too |
| 1b | `list_sessions` per pane `limit?: {resetsAt, window}` (Claude at usage limit, from collector `limitHit`) and `idleSince?` (ISO) | |
| 1c | `wait_session {session, until: 'idle'\|'exited', timeoutMs ≤ 600000, default 120000}` | returns `{status, reached: bool, waitedMs}`; no turn summary (hard rule) |
| 2 | `<cwd>/.orrery/status.json` `{phase ≤ 40, progress 0–1, note ≤ 80}` shown on the pane (header chip, Mission Control card) | sanitized, fs watch not polling, size cap 4 KB |
| 3 | `run_log {match?: regex ≤ 200, since?: cursor}` → result adds `cursor` | |
| 4 | `telegram_send {buttons?: [{label ≤ 32, value ≤ 200}] ≤ 6}`; owner tap → `value` typed into the calling pane, submitted only if idle | **5th core-principle exception**, owner 2026-11-08; CLAUDE.md + docs |

Not doing: last-turn summary; submit for shell panes.

Order: WP1 ∥ WP2 → WP3 → WP4. Each WP: code + tests + docs (§11w / §11an) + `.code_index/`. Typecheck + Vitest gate; no build.
