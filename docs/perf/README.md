# Perf baseline (RAM / CPU / GPU)

`scripts/perf-bench.mjs` starts an isolated instance of the app, drives it over CDP, samples its whole process tree and writes one JSON. It never touches a running Orrery: own `--user-data-dir`, own `CCM_PORT`, `CCM_DATA_DIR`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `CODEWHALE_HOME` and remote-debugging port (all temp, deleted at the end). No flags that turn off renderer backgrounding or occlusion, so the numbers include real throttling behaviour. The instance, the sampler and the demo fixture run at BelowNormal priority.

## Run

```
node scripts/perf-bench.mjs --scenario all --seconds 120 --trace --label 3.4.0 --out docs/perf/3.4.0-win32.json
```

| Option | Meaning |
|---|---|
| `--exe <path>` | Packaged app. Default: `C:\Program Files\Orrery\Orrery.exe` (win32), `/Applications/Orrery.app/...` (darwin), else `node_modules/electron` + `apps/desktop/stage` |
| `--stage` | Run the dev build: `node_modules/electron` + `apps/desktop/stage` (build web + desktop first) |
| `--scenario` | `all` or a comma list of `idle-monitor`, `idle-workspace`, `demo-6x3`, `demo-30x6` |
| `--seconds N` | Sampling window per scenario (default 120), taken after `--settle` seconds (default 20) of doing nothing |
| `--trace` | After sampling, record a Chromium trace for `--trace-seconds` (default 25) and keep only a summary in the JSON; the raw trace stays in a temp folder that is deleted |
| `--profile` | Demo scenarios only: after sampling, CDP `Profiler` on the main renderer for `--profile-seconds` (default 25, 0.5 ms interval); keeps top 30 functions by self and total time, time per source file, DOM node count per region (header, rail, map, activity) and then store patches and actions per second over `--patch-seconds` (default 10, taken after the profile so the probe does not distort it) under `scenarios.<name>.profile`. `--profile-dir <dir>` also keeps the raw `.cpuprofile` (open in DevTools) |
| `--label`, `--out` | Name part of the default output `docs/perf/<label>-<platform>.json`, or an explicit path |

Profile function names come from the minified bundle (`file:line:col`, 1-based; no source maps in the build), so map them by reading the chunk at that position. Vue component counts are empty in a production build.

Scenarios run one after another, never at the same time.

| Scenario | State |
|---|---|
| `idle-monitor` | Monitor view, no sessions |
| `idle-workspace` | Workspace view, 4 shell panes running |
| `demo-6x3` / `demo-30x6` | Monitor view, `scripts/demo-fixture.mjs` with 6 sessions x 3 subagents / 30 x 6 |

## JSON

`scenarios.<name>.roles` has one row per process role (`TOTAL`, `main`, `renderer`, `gpu`, `collector`, `pty-host`, `network`, `audio`, `crashpad`, `pty-child` = shells under the PTY host):

| Column | Meaning |
|---|---|
| `procs` | Average process count of the role |
| `privateMB` / `privateMBMax` | Private bytes summed over the role, average / max over the window (macOS: RSS, no private figure) |
| `rssMB` | Working set summed over the role, average |
| `cpuAvgPct` / `cpuP95Pct` | CPU time per wall time, percent of one core, summed over the role; average / 95th percentile of the 2 s ticks |
| `cpuSec` | CPU seconds used by the role during the window |

`pids` lists every process with the same columns plus its command line. `pageState` (trace runs) holds running CSS animations, elements with `backdrop-filter`, infinite CSS animations and canvases. `trace` holds the per-thread busy time (`ThreadControllerImpl::RunTask`), events per second (frames, timers, GC), top events per thread, top `FunctionCall` sites (minified bundle, `file:line:col`), timer and requestAnimationFrame install sites, and the page-side count of `requestAnimationFrame` / `setTimeout` / `setInterval` callers.

CPU numbers are one-core percentages: 100 means one full core, the machine total is `cores x 100`. Sampling: Windows uses `Get-Process` every 2 s in one long-lived PowerShell; macOS uses `ps` (cputime has 10 ms resolution).
