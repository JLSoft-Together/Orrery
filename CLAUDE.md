# CLAUDE.md

Guidance for Claude Code in this repo. Scope source of truth: `PLAN.md`. Verified data sources: `docs/claude-code-integration.md` (Claude Code 2.1.289), `docs/codex-integration.md` (Codex CLI 0.162.0). History (v1 to v108): `docs/features-history.md`.

## What this is

**Orrery**: a local-only desktop app (Windows exe, macOS dmg; Tauri 2 shell + Rust core + Vue web UI) that observes running Claude Code and Codex CLI sessions and shows sessions, terminals and the main-agent to sub-agent hierarchy in realtime, plus a Workspace (terminal / browser / file / git panes), Disk manager, Devices, Run pane, Telegram bot and a phone app (Orrery Remote). Old names (Claude Code Monitor, Agent Monitor, GolAIm) left a deliberate prefix: npm `@ccm/*`, `CCM_*` env, `ccm.*` storage keys, appId `dev.ccm.monitor`, data dir `ccm`, userData folder `Agent Monitor`. Electron and the Node collector were removed in v110.

## Commands

npm workspaces (no pnpm). From repo root:

- `npm install`; `npm test` (Vitest; one file: `npm test -w @ccm/web -- <path>`); `npm run typecheck` (`tsc` / `vue-tsc`; no linter, typecheck + tests are the check).
- `npm run qa`: fast check from `git diff` (`scripts/qa-fast.mjs`): typecheck changed workspaces, `vitest related`, `cargo test` of touched crates, mapped E2E. Flags `--dry`, `--no-e2e`, `--no-cargo`, `--e2e=a,b`, `--base=<ref>`, `--jobs=<n>`, `--all-e2e-map`.
- `npm run dev`: `orrery-core serve --dev` (`127.0.0.1:4317`, no app key, no terminals) + Vite (`127.0.0.1:5173`). Give it its own `CCM_DATA_DIR`.
- `npm run shell:dev`: build web, generate the shim (`scripts/gen-desk-shim.mjs`), run the Tauri shell (debug: port 4419, temp data dir).
- `npm run dist` (host OS only: `dist:win` NSIS, `dist:mac` dmg, via `scripts/dist-tauri.mjs`).
- E2E (CDP, Tauri shell): `npm run e2e -w @ccm/shell` runs all, or `node apps/shell/e2e/<name>.cjs`. Build first: `npm run build -w @ccm/web`, `node scripts/gen-desk-shim.mjs`, `cd apps/shell && cargo build -j 4`.
- Rust: `cargo test --locked -j 4` in `apps/core` and in `apps/shell` (PowerShell on Windows; Git Bash `link` shadows `link.exe`).
- Phone check: `node scripts/qa-remote.mjs`. Fake sessions: `node scripts/demo-fixture.mjs --sessions=5 --subagents=3`, run with `CLAUDE_CONFIG_DIR=<printed root> CCM_VERIFY_PROCESSES=0`. Index check: `node scripts/index-check.mjs`.
- Env vars: `docs/env.md`. The app only serves when started by the shell (`CCM_APP_KEY`) or with `--dev`.

## Architecture

```
apps/shell          Tauri 2 shell (Rust): window, tray, shim `window.ccmDesktop`, hosts, MCP `orrery`, phone remote host, disk
apps/core/crates    Rust: core-serve (orrery-core serve), core-http, core-store, core-monitor, core-transcript,
                    core-sources, core-procs, core-pty, core-desk (all `ccm:*` channels), core-disk, core-toast,
                    core-bridge, core-remote-proto / -wasm
apps/web            Vue 3 + TS + Vite + Tailwind + Pinia + vue-i18n: core (shared), app, features/<f> under src/
packages/shared     event protocol + shared types; packages/mist-tokens, packages/habblaud (Office, lazy)
apps/pwa, apps/mobile, apps/relay   phone clients and Cloudflare relay; apps/layout-daemon: Kotlin layoutlib daemon
```

Contracts: on connect one `snapshot`, then incremental events (`packages/shared/src/types.ts`); unknown events / fields ignored, unknown statuses map to `unknown`. Identity comes from process / session, never the title. Prefer official hooks and session files over scraping ANSI; do not guess undocumented formats. On disconnect keep the last state, marked stale. No high-frequency polling, no database.

## Core principle and owner-approved exceptions

**Observe, Normalize, Broadcast, Render.** Monitoring never sends prompts to Claude Code, never calls the Claude API, never spawns another LLM agent (zero extra tokens). Exceptions, all owner-approved, full text in `docs/core-principle.md` (do not add one without recording it there first):

1. v51 Compact: submits one handoff prompt into an idle pane the owner picked, then `/compact` and the resume line (switch on by default) (§11x).
2. v81 `hand_back` from a DeepSeek pane submits one fixed prompt into the Claude / Codex pane that opened it (§11am).
3. v82 `/bot-tele-receive`: a message from the paired Telegram account mentioning the bot is submitted into that project's pane, even while the agent works (§11an).
4. Tap on a `telegram_send` button an agent attached types its value into that pane (§11an).
5. MCP `open_session` with `submit: true` submits its prompt in the new pane; links never submit (§11w).
6. v100 click menu: click, button bar, queue chip or toast button on a Claude / Codex choice menu types the chosen key; the app never answers a menu by itself (§11as); since v113 a tap on a push choice button opens the app first, then sends the same key.
7. v97 Orrery Remote: paired, end-to-end encrypted phone may open / close panes, type, edit files, run git / commands; prompts submit immediately; remote control off by default; secrets never leave the machine (§11au).
8. v98 push: phone notifications carry agent, session, project and the pending question, secret-masked, cut to 120 characters; per-phone "Include content" switch; since v113 also up to 3 masked choice labels (24 characters), waiting count and 2 pane labels for widget / tile (`docs/remote-integration.md` §7.6).

## Hard rules

- Never show token counts without reliable data; never invent values.
- Never expose session contents, env, secrets or raw tool arguments to the frontend or logs without sanitizing. Exception: `tool.span` carries absolute `path` + `access` of Edit / Write / MultiEdit / NotebookEdit / Read only (§11ar).
- No horizontal page scroll (`overflow-x-hidden`).
- All UI strings via vue-i18n (`en`, `vi`); all colors via theme tokens (dark / light); status = icon + text + color, never color alone. Rules: `.planning/COLOR-SYSTEM.md`.
- Shape tokens (`--radius-*`, `--ccm-*` in `style.css` / `themes.css`) and utilities (`ccm-panel`, `ccm-chip`, `ccm-seg`...): never branch on the theme id in a component.
- Design system Apple (macOS HIG), spec `.planning/AUDIT-v18-deck-reference.md` §4. Themes: `orrery-light` / `orrery-dark`, `galaxy` (dark default); older themes behind "View all". Details: `docs/visual-style.md`.
- Fonts: system stack, mono stack; user-picked fonts are bundled (`fonts.css`), never load fonts from the network.
- Visual style: dense but calm developer tool, no gradients, no `backdrop-filter`. Documented exceptions: faint static stars of Galaxy (`ccm.stars`, `ccm.starsFx`, `ccm.starsPanels`), and the mist background under `:root[data-ui='next']`. No blur anywhere: since 23/11 Mist always renders solid panels (`data-glass='reduced'`), the Reduce transparency switch is gone (blur layers made the window freeze while resizing).
- Before frontend work load skills `frontend-design:frontend-design` and `ui-ux-pro-max:ui-ux-pro-max`; for new UI or features make an HTML wireframe first.
- No comments in code (owner rule). "Why" / invariant notes go to `.code_index/`, `.planning/` or `bug_pool/`.

## Code index

Read `.code_index/lookup.md` first (task to `path::Symbol`), then 1-2 module files (`.code_index/README.md` lists them; each is at most 8 KB). After a task update the module file and one line in `.code_index/_meta.md`; run `node scripts/index-check.mjs`. Web layers (core shared, app, features with `index.ts` + `README.md`) and import rules: `apps/web/src/importRules.test.ts`; move files with `scripts/web-move.mjs`.

Deviations from `PLAN.md` are allowed only when a discovered Claude Code limitation forces them, and must be documented in `docs/claude-code-integration.md`.
