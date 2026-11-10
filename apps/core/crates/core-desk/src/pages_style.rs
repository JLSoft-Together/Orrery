pub const STILL: [&str; 6] = [r##"
  "##, r##" :is(.stage, .float, .mark, .ring, .part, .sun, .orbit, .spin, .word span, .bar, .status, .hint, .ver, body) { animation: none; }
  "##, r##" .orbit { opacity: 1; stroke-dashoffset: 0; }
  "##, r##" :is(.part, .sun) { opacity: 1; transform: none; }
  "##, r##" .caret { animation: none; opacity: 0; }
  "##, r##" .bar::after { animation: none; width: 100%; }"##];

pub const STYLE_A: &str = r##"
* { box-sizing: border-box; }
html, body { margin: 0; overflow-x: hidden; }
html:not(.static) body { overflow: hidden; }
body::before { content: ""; position: fixed; inset: 0 0 auto; height: 40px; -webkit-app-region: drag; app-region: drag; }
body { min-height: 100vh; display: grid; place-items: center; padding: 24px 16px; background: var(--canvas); color: var(--ink);
  font: 14px/1.5 -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  -webkit-font-smoothing: antialiased; }
main { width: 100%; max-width: 760px; display: grid; gap: 16px; justify-items: center; text-align: center; }
:root { --settle: 1500ms; --text: 1680ms; --step: 40ms; --after: calc(var(--text) + var(--n) * var(--step));
  --mark: clamp(96px, 17vmin, 148px); --radius: calc(var(--mark) * .236); --word: clamp(34px, 6.4vmin, 56px); }
.stage { position: relative; width: var(--mark); height: var(--mark); animation: settle .7s cubic-bezier(.65,0,.35,1) var(--settle) both; }
.float { position: absolute; inset: 0; animation: float 4.6s ease-in-out calc(var(--settle) + 700ms) infinite; }
.mark { width: var(--mark); height: var(--mark); border-radius: var(--radius); display: block; overflow: hidden;
  box-shadow: 0 2px 4px rgb(0 0 0 / .18), 0 18px 48px rgb(0 0 0 / .18); animation: tile .5s cubic-bezier(.34,1.4,.64,1) both; }
.ring { position: absolute; inset: 0; border-radius: var(--radius); border: 2px solid #E8915A; opacity: 0; pointer-events: none;
  animation: ring .8s cubic-bezier(.2,.8,.2,1) 1400ms forwards; }
.sun { transform-box: fill-box; transform-origin: center; transform: scale(0);
  animation: light .5s cubic-bezier(.34,1.45,.64,1) 250ms forwards, glow 3.6s ease-in-out 2600ms infinite; }
.orbit { opacity: 0; stroke-dasharray: 1; stroke-dashoffset: 1; animation: draw .55s cubic-bezier(.65,0,.35,1) var(--at) forwards; }
.part { transform-box: fill-box; transform-origin: center; opacity: 0; animation: place .42s cubic-bezier(.34,1.45,.64,1) var(--at) forwards; }
.spin { transform-box: view-box; transform-origin: 512px 512px; animation: spin var(--turn) linear 1600ms infinite; }
.word { margin: 14px 0 0; font-size: var(--word); line-height: 1.15; font-weight: 600; letter-spacing: -0.015em; white-space: nowrap; display: flex; align-items: baseline; }
.word span { display: inline-block; white-space: pre; animation: rise .5s cubic-bezier(.2,.8,.2,1) both; animation-delay: calc(var(--text) + var(--i) * var(--step)); }
.caret { width: 3px; height: .9em; margin-left: 4px; align-self: center; border-radius: 1.5px; background: var(--accent);
  animation: blink 1s steps(1) calc(var(--after) + 200ms) infinite both; }
.bar { position: relative; width: clamp(200px, 30vmin, 280px); height: 3px; margin-top: 18px; border-radius: 2px; overflow: hidden; background: var(--accent-soft);
  animation: fade .35s ease-out calc(var(--after) + 300ms) both; }
.bar::after { content: ""; position: absolute; inset: 0; width: 40%; border-radius: 2px; background: var(--accent); animation: slide 1.2s cubic-bezier(.45,0,.55,1) infinite; }
.status { margin: 0; color: var(--muted); font-size: 15px; animation: fade .35s ease-out calc(var(--after) + 300ms) both; }
.hint { margin: -10px 0 0; color: var(--faint); font-size: 13px; animation: fade .35s ease-out calc(var(--after) + 400ms) both; }
.ver { position: fixed; bottom: 18px; left: 0; right: 0; margin: 0; color: var(--faint); font-size: 12px; font-variant-numeric: tabular-nums; text-align: center;
  animation: fade .4s ease-out calc(var(--after) + 400ms) both; }
.out body { animation: leave .28s cubic-bezier(.4,0,1,1) forwards; }
@keyframes tile { from { opacity: 0; transform: scale(.55); } to { opacity: 1; transform: none; } }
@keyframes settle { from { transform: translateY(calc(var(--mark) * .8)) scale(1.55); } to { transform: none; } }
@keyframes float { 0%, 100% { transform: none; } 50% { transform: translateY(calc(var(--mark) * -.045)); } }
@keyframes place { from { opacity: 0; transform: var(--from); } 40% { opacity: 1; } to { opacity: 1; transform: none; } }
@keyframes light { to { transform: none; } }
@keyframes glow { 0%, 100% { transform: none; } 50% { transform: scale(1.08); } }
@keyframes draw { from { opacity: 1; } to { opacity: 1; stroke-dashoffset: 0; } }
@keyframes spin { to { transform: rotate(360deg); } }
@keyframes ring { 0% { opacity: .9; transform: scale(1); } 100% { opacity: 0; transform: scale(1.6); } }
@keyframes rise { from { opacity: 0; transform: translateY(.3em); filter: blur(6px); } to { opacity: 1; transform: none; filter: none; } }
@keyframes fade { from { opacity: 0; } to { opacity: 1; } }
@keyframes leave { to { opacity: 0; transform: scale(.97); filter: blur(4px); } }
@keyframes blink { 0%, 49% { opacity: 0; } 50%, 100% { opacity: 1; } }
@keyframes slide { from { transform: translateX(-100%); } to { transform: translateX(250%); } }
@media (prefers-reduced-motion: reduce) {
"##;
pub const STILL_A: &str = r##":root"##;
pub const STYLE_B: &str = r##"
}
"##;
pub const STILL_B: &str = r##".static"##;
pub const STYLE_C: &str = r##"
h1.title { margin: 0; font-size: 18px; font-weight: 600; }
p { margin: 0; color: var(--muted); font-size: 14px; overflow-wrap: anywhere; }
.err { color: var(--error); font-weight: 600; }
a.btn { display: inline-flex; align-items: center; height: 40px; padding: 0 16px; border-radius: 8px; background: var(--accent); color: var(--on-accent);
  font-weight: 600; text-decoration: none; }
a.btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
pre { width: 100%; max-height: 260px; overflow: auto; margin: 0; padding: 12px; text-align: left; border-radius: 10px; border: 1px solid var(--line);
  background: var(--surface); color: var(--muted); font: 12px/1.5 ui-monospace, "Cascadia Mono", Consolas, monospace; white-space: pre-wrap; overflow-wrap: anywhere; }
"##;
