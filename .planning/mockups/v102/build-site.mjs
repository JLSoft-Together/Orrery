import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const root = process.argv[2];
const src = join(root, 'src');
const out = join(root, 'site', 'public');
mkdirSync(out, { recursive: true });

const boards = [
  { file: 'Main.dc.html', out: 'workspace.html', w: 1440, h: 900, name: 'Workspace 1440', desc: '8 session, 2 cần bạn, 2 pane, click menu, toast' },
  { file: 'Compact.dc.html', out: 'compact.html', w: 1280, h: 800, name: 'Rail thu gọn + Explorer', desc: '1280px, flyout project, Explorer drawer' },
  { file: 'CtrlK.dc.html', out: 'ctrlk.html', w: 1440, h: 900, name: 'Ctrl+K', desc: 'Nhóm Session, Cần bạn, Đang chạy, Gần đây' },
  { file: 'Settings.dc.html', out: 'settings.html', w: 1440, h: 900, name: 'Settings, Giao diện', desc: 'Dialog kính, chuyển giao diện, palette, motion' },
  { file: 'Drag.dc.html', out: 'drag.html', w: 1440, h: 900, name: 'Kéo thả pane', desc: 'Góp ý 1, 3, 4, 5, 6: la bàn, đổi chỗ, quá hẹp, sang tab, từ Explorer (đổi ở ô scenario)' },
  { file: 'Split.dc.html', out: 'split.html', w: 1440, h: 900, name: 'Chia pane và bố cục', desc: 'Góp ý 2, 7, 8, 9, 10, 11: bố cục dựng sẵn, thanh chia, tỉ lệ, hoàn tác, gợi ý tách tab, phím' },
];

function toPlain(html, b) {
  const helmet = (html.match(/<helmet>([\s\S]*?)<\/helmet>/) || [, ''])[1];
  let body = (html.match(/<x-dc>([\s\S]*?)<\/x-dc>/) || [, ''])[1].replace(/<helmet>[\s\S]*?<\/helmet>/, '');
  const props = JSON.parse(((html.match(/data-props='([^']*)'/) || [, '{}'])[1]).replace(/&amp;/g, '&').replace(/&#39;/g, "'"));
  const logic = (html.match(/<script type="text\/x-dc"[^>]*>([\s\S]*?)<\/script>/) || [, ''])[1];
  const enums = Object.entries(props).filter(([k, v]) => k[0] !== '$' && v.editor === 'enum');
  const controls = enums.map(([k, v]) => `<label>${k}<select data-k="${k}">${v.options.map(o => `<option${o === v.default ? ' selected' : ''}>${o}</option>`).join('')}</select></label>`).join('');
  return `<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=${b.w + 40}">
<title>${b.name}, Orrery Sương kính</title>
${helmet}
<style>
.mk-bar{position:sticky;top:0;z-index:50;display:flex;flex-wrap:wrap;gap:14px;align-items:center;padding:10px 20px;background:#0F171D;color:#E4EEF2;font:600 14px Manrope,system-ui,sans-serif}
.mk-bar a{color:#8FD3E5;text-decoration:none}
.mk-bar select{margin-left:6px;font:inherit;padding:4px 6px;border-radius:6px}
.mk-wrap{padding:20px;background:#0b1015}
</style>
</head>
<body style="margin:0;background:#0b1015">
<div class="mk-bar"><a href="./">← Tất cả màn</a><span>${b.name}</span>${controls}</div>
<div class="mk-wrap"><div id="mk-root"></div></div>
<template id="mk-tpl">${body}</template>
<script>
class DCLogic { constructor(){ this.props = {}; this.state = {}; } setState(s){ Object.assign(this.state, s); render(); } }
${logic}
const comp = new Component();
const defaults = ${JSON.stringify(Object.fromEntries(Object.entries(props).filter(([k]) => k[0] !== '$').map(([k, v]) => [k, v.default])))};
comp.props = Object.assign({}, defaults);
new URLSearchParams(location.search).forEach((v, k) => { if (k in defaults) comp.props[k] = v; });
document.querySelectorAll('.mk-bar select').forEach(s => { if (comp.props[s.dataset.k] != null) s.value = comp.props[s.dataset.k]; });
document.querySelectorAll('.mk-bar select').forEach(s => s.addEventListener('change', () => { comp.props[s.dataset.k] = s.value; render(); }));
function get(scope, path) { return path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), scope); }
function fill(str, scope) { return str.replace(/\\{\\{\\s*([^}]+?)\\s*\\}\\}/g, (_, p) => { if (p === 'true') return 'true'; if (p === 'false') return 'false'; const v = get(scope, p); return typeof v === 'function' ? '' : (v ?? ''); }); }
function walk(node, scope) {
  const frag = document.createDocumentFragment();
  for (const ch of Array.from(node.childNodes)) {
    if (ch.nodeType === 3) { frag.appendChild(document.createTextNode(fill(ch.textContent, scope))); continue; }
    if (ch.nodeType !== 1) continue;
    const tag = ch.tagName.toLowerCase();
    if (tag === 'sc-if') { const m = ch.getAttribute('value').match(/\\{\\{\\s*([^}]+?)\\s*\\}\\}/); const v = m ? (m[1] === 'true' ? true : m[1] === 'false' ? false : get(scope, m[1])) : false; if (v) frag.appendChild(walk(ch, scope)); continue; }
    if (tag === 'sc-for') { const lp = ch.getAttribute('list').match(/\\{\\{\\s*([^}]+?)\\s*\\}\\}/)[1]; const as = ch.getAttribute('as'); (get(scope, lp) || []).forEach((it, i) => frag.appendChild(walk(ch, Object.assign({}, scope, { [as]: it, $index: i })))); continue; }
    const el = ch.namespaceURI === 'http://www.w3.org/1999/xhtml' ? document.createElement(ch.localName) : document.createElementNS(ch.namespaceURI, ch.localName);
    for (const a of Array.from(ch.attributes)) {
      if (/^on[A-Z]/.test(a.name) || /^on[a-z]+$/.test(a.name) && a.value.includes('{{')) { const m = a.value.match(/\\{\\{\\s*([^}]+?)\\s*\\}\\}/); const fn = m && get(scope, m[1]); if (typeof fn === 'function') el.addEventListener(a.name.slice(2).toLowerCase(), fn); continue; }
      if (a.name.startsWith('hint-')) continue;
      el.setAttribute(a.name, fill(a.value, scope));
    }
    el.appendChild(walk(ch.content || ch, scope));
    frag.appendChild(el);
  }
  return frag;
}
function render() {
  const vals = comp.renderVals();
  const root = document.getElementById('mk-root');
  root.replaceChildren(walk(document.getElementById('mk-tpl').content, vals));
}
render();
</script>
</body>
</html>
`;
}

const made = [];
for (const b of boards) {
  let html;
  try { html = readFileSync(join(src, b.file), 'utf8'); } catch { continue; }
  writeFileSync(join(out, b.out), toPlain(html, b));
  made.push(b);
}

const flatOf = made.filter(b => ['workspace.html', 'settings.html', 'split.html'].includes(b.out)).map(b => Object.assign({}, b, { q: '?style=flat', name: b.name + ' (flat)', desc: 'Bản flat để so: nền phẳng, không blur, bo góc nhỏ, tab gạch chân' }));
const cards = made.map(b => Object.assign({ q: '' }, b)).concat(flatOf).map(b => `<a class="card" href="./${b.out.replace(".html","")}${b.q}"><div class="frame" style="aspect-ratio:${b.w}/${b.h}"><iframe src="./${b.out.replace(".html","")}${b.q}" tabindex="-1" title="${b.name}" style="width:${b.w + 40}px;height:${b.h + 100}px"></iframe></div><div class="cap"><b>${b.name}</b><span>${b.desc}</span></div></a>`).join('\n');
writeFileSync(join(out, 'index.html'), `<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Orrery Sương kính</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Manrope:wght@500;700&display=swap">
<style>
:root{--bg:#EEF3F6;--ink:#1B2A33;--muted:#53656F;--card:rgb(255 255 255/.7);--line:rgb(28 60 80/.12)}
@media (prefers-color-scheme:dark){:root{--bg:#0F171D;--ink:#E4EEF2;--muted:#9FB2BB;--card:rgb(255 255 255/.06);--line:rgb(255 255 255/.1)}}
body{margin:0;background:radial-gradient(55% 45% at 6% 0%,rgb(42 160 140/.25),transparent 72%),radial-gradient(50% 55% at 100% 6%,rgb(90 110 220/.22),transparent 70%),var(--bg);color:var(--ink);font:15px/1.5 Manrope,system-ui,sans-serif;min-height:100vh}
main{max-width:1200px;margin:0 auto;padding:28px 16px 48px}
h1{margin:0 0 4px;font-size:26px}
p{margin:0 0 24px;color:var(--muted)}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(min(100%,520px),1fr));gap:18px}
.card{display:flex;flex-direction:column;border-radius:18px;overflow:hidden;background:var(--card);border:1px solid var(--line);color:inherit;text-decoration:none}
.frame{position:relative;overflow:hidden;container-type:inline-size}
.frame iframe{position:absolute;left:0;top:0;border:0;transform-origin:0 0;pointer-events:none}
.cap{padding:12px 14px;display:flex;flex-direction:column;gap:2px}
.cap span{color:var(--muted);font-size:13px}
</style>
</head>
<body>
<main>
<h1>Orrery Sương kính, UI next gen</h1>
<p>Mockup v102 từ phân tích UI 15/11/2026. Bấm vào màn để xem cỡ thật, có công tắc sáng / tối và các tuỳ chọn ở thanh trên. Trên điện thoại kéo hai ngón để phóng to.</p>
<div class="grid">
${cards}
</div>
</main>
<script>
function fit(){document.querySelectorAll('.frame').forEach(f=>{const i=f.querySelector('iframe');const w=parseFloat(i.style.width);const s=f.clientWidth/w;i.style.transform='translate(0,'+(-60*s)+'px) scale('+s+')';});}
addEventListener('resize',fit);fit();
</script>
</body>
</html>
`);
console.log('ok ' + made.map(b => b.out).join(' '));
