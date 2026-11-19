import { I } from './icons.mjs'

export const STATUS = {
  working: { icon: 'working', text: 'Đang chạy', spin: true },
  waiting: { icon: 'waiting', text: 'Chờ bạn' },
  error: { icon: 'error', text: 'Lỗi' },
  done: { icon: 'done', text: 'Xong' },
  idle: { icon: 'idle', text: 'Rảnh' },
}

export function stIcon(st, cls = '') {
  const s = STATUS[st]
  return `<span class="st ${st}">${I(s.icon, `${cls} ${s.spin ? 'spin' : ''}`)}</span>`
}

export function chip(st, text) {
  const s = STATUS[st]
  return `<span class="chip ${st}">${I(s.icon, `s14 ${s.spin ? 'spin' : ''}`)}${text ?? s.text}</span>`
}

export const AGENT = {
  claude: { name: 'Claude', icon: 'claude' },
  codex: { name: 'Codex', icon: 'codex' },
  shell: { name: 'Terminal', icon: 'term' },
}

export const PROJECTS = [
  {
    id: 'api',
    mono: 'A',
    tone: 'mono-a',
    name: 'api',
    branch: 'feat/api',
    sessions: [
      { name: 'claude patched', agent: 'claude', model: 'Opus', st: 'waiting', time: '2p', detail: 'Chạy lệnh: npm test?', gauge: 'w72', hot: true },
      { name: 'codex review', agent: 'codex', model: 'gpt-5-codex', st: 'working', time: '1p 12s', detail: 'Edit src/app.ts', gauge: 'w35' },
    ],
  },
  {
    id: 'web',
    mono: 'W',
    tone: 'mono-b',
    name: 'web',
    branch: 'main',
    sessions: [
      { name: 'landing', agent: 'claude', model: 'Sonnet', st: 'working', time: '4p', detail: 'Bash npm run build', gauge: 'w42' },
      { name: 'npm run dev', agent: 'shell', model: '', st: 'idle', time: '18p', detail: 'vite đang chạy :5173', gauge: '' },
    ],
  },
  {
    id: 'infra',
    mono: 'I',
    tone: 'mono-c',
    name: 'infra',
    branch: 'ops/deploy',
    sessions: [
      { name: 'deploy', agent: 'claude', model: 'Opus', st: 'error', time: '6p', detail: 'Bash thoát mã 1', gauge: 'w55' },
      { name: 'docs sync', agent: 'codex', model: 'gpt-5-codex', st: 'done', time: '9p', detail: 'Xong, chưa xem', gauge: 'w20' },
    ],
  },
]

export function sessionRow(s, opts = {}) {
  const ag = AGENT[s.agent]
  const model = s.model ? `<span class="dot"></span><span>${s.model}</span>` : ''
  const gauge = s.gauge ? `<div class="gauge ${s.hot ? 'hot' : ''}"><i class="${s.gauge}"></i></div>` : ''
  return `<div class="srow ${opts.sel ? 'sel' : ''}">${stIcon(s.st)}<div class="grow"><div class="l1"><span class="t-name ell">${s.name}</span><span class="t-faint num">${s.time}</span></div><div class="l2"><b class="agent-${s.agent}">${ag.name}</b>${model}<span class="dot"></span><span class="st ${s.st}">${STATUS[s.st].text}</span><span class="ell">· ${s.detail}</span></div></div>${gauge}</div>`
}

export function projectCard(p, opts = {}) {
  const rows = p.sessions.map((s, i) => sessionRow(s, { sel: opts.sel === `${p.id}:${i}` })).join('')
  return `<section class="card proj ${opts.dim ? 'dim' : ''}"><div class="proj-h"><span class="tile sm ${p.tone}">${p.mono}</span><span class="t-name">${p.name}</span><span class="t-faint">${I('branch', 's14')}</span><span class="t-faint t-mono">${p.branch}</span><span class="t-faint num">${p.sessions.length}</span></div>${rows}</section>`
}

export function railRow(s, sel) {
  const ag = AGENT[s.agent]
  const model = s.model ? `<span class="dot"></span>${s.model}` : ''
  return `<div class="rr ${sel ? 'sel' : ''}">${stIcon(s.st, 's16')}<div class="grow"><div class="hstack"><span class="t-name ell">${s.name}</span><span class="t-faint num">${s.time}</span></div><div class="l2"><b class="agent-${s.agent}">${ag.name}</b>${model}<span class="dot"></span><span class="st ${s.st}">${STATUS[s.st].text}</span></div></div></div>`
}

export function railProjects(sel) {
  return PROJECTS.map((p) => {
    const rows = p.sessions.map((s, i) => railRow(s, sel === `${p.id}:${i}`)).join('')
    return `<div class="rp">${I('chev-d', 's14')}<span class="tile sm ${p.tone}">${p.mono}</span><span class="grow">${p.name}</span><span class="t-faint num">${p.sessions.length}</span></div>${rows}`
  }).join('')
}

export function sbar() {
  return `<div class="sbar" aria-hidden="true"><span class="num">9:41</span><span class="sbar-r">${I('signal', 's16')}${I('wifi', 's16')}<span class="sbar-batt"><i></i></span></span></div>`
}

export function tabBar(active) {
  const tabs = [
    ['monitor', 'activity', 'Giám sát', '<span class="badge">1</span>'],
    ['workspace', 'grid', 'Workspace', ''],
    ['settings', 'sliders', 'Cài đặt', ''],
  ]
  const items = tabs.map(([id, icon, label, badge]) => `<span class="tab ${id === active ? 'on' : ''}">${I(icon)}${label}${badge}</span>`).join('')
  return `<nav class="tabbar glass" aria-label="Điều hướng">${items}</nav>`
}

export function topBar({ back = false, title, sub = '', right = '', tall = '' }) {
  const b = back ? `<button class="ib back" aria-label="Quay lại">${I('chev-l', 's24')}</button>` : ''
  const h = `<h1>${title}${sub ? `<small>${sub}</small>` : ''}</h1>`
  if (!tall) return `<header class="top glass">${b}${h}${right}</header>`
  return `<header class="top tall glass"><div class="top-row">${b}${h}${right}</div><div class="top-sub">${tall}</div></header>`
}

export function quota() {
  return `<div class="quota"><b>Claude Code</b><span>5h</span><div class="qbar"><i class="w42"></i></div><span class="qpct">42%</span><span></span><span>Tuần</span><div class="qbar hot"><i class="w91"></i></div><span class="qpct hot">91%</span><b>Codex</b><span>5h</span><div class="qbar"><i class="w7"></i></div><span class="qpct">7%</span></div>`
}

export function seg(items, on, cls = '') {
  return `<div class="seg ${cls}" role="tablist">${items.map((t, i) => `<span role="tab" class="${i === on ? 'on' : ''}">${t}</span>`).join('')}</div>`
}

export function toggle(on) {
  return `<span class="toggle ${on ? 'on' : ''}" role="switch" aria-checked="${on}"><i></i></span>`
}

export function well(lines) {
  return lines.join('\n')
}

export const TERM_LINES = [
  '<span class="dim">$ claude --resume 4f2a</span>',
  '<span class="o">●</span> Đọc <span class="c">apps/api/src/routes/users.ts</span>',
  '<span class="o">●</span> Sửa <span class="c">apps/api/src/lib/util.ts</span>',
  '  <span class="g">+ 14</span> <span class="r">− 3</span>',
  '<span class="o">●</span> Bash(npm run typecheck)',
  '  <span class="dim">└ 0 lỗi, 2 cảnh báo</span>',
  '<span class="o">●</span> Cần chạy test để kiểm route mới.',
  '',
  '<span class="y">✻</span> Chạy lệnh: <span class="v">npm test</span>?',
  '  <span class="g">❯ 1. Có</span>',
  '    2. Có, luôn luôn cho lệnh này',
  '    3. Không, nói Claude làm khác',
  '',
  '<span class="dim">esc để huỷ</span>',
]

export const TERM_LONG = [
  '<span class="dim">$ claude --resume 4f2a</span>',
  '',
  '<span class="o">✻</span> Claude Code · Opus · <span class="dim">~/work/api</span>',
  '',
  '<span class="v">&gt;</span> thêm route GET /users/:id có cache',
  '',
  '<span class="o">●</span> Đọc <span class="c">apps/api/src/routes/users.ts</span>',
  '<span class="o">●</span> Đọc <span class="c">apps/api/src/lib/cache.ts</span>',
  '<span class="o">●</span> Sửa <span class="c">apps/api/src/routes/users.ts</span>',
  '  <span class="g">+ 22</span> <span class="r">− 4</span>',
  '<span class="o">●</span> Sửa <span class="c">apps/api/src/lib/util.ts</span>',
  '  <span class="g">+ 14</span> <span class="r">− 3</span>',
  '<span class="o">●</span> Bash(npm run typecheck)',
  '  <span class="dim">└ 0 lỗi, 2 cảnh báo</span>',
  '<span class="o">●</span> Route mới đọc cache trước,',
  '  hết hạn sau 60s. Cần chạy test.',
  '',
  '<span class="y">✻</span> Chạy lệnh: <span class="v">npm test</span>?',
  '  <span class="g">❯ 1. Có</span>',
  '    2. Có, luôn luôn cho lệnh này',
  '    3. Không, nói Claude làm khác',
  '',
  '<span class="dim">esc để huỷ</span>',
]

export const RUN_LOG = [
  '<span class="dim">$ npm run dev</span>',
  '',
  '<span class="g">  VITE v6.2.0</span>  sẵn sàng sau <b>412 ms</b>',
  '',
  '  <span class="g">➜</span>  Local:   <span class="c">http://127.0.0.1:5173/</span>',
  '  <span class="dim">➜  Network: dùng --host để mở</span>',
  '',
  '<span class="dim">23:48:02</span> <span class="c">[vite]</span> hmr update /src/App.vue',
  '<span class="dim">23:48:15</span> <span class="c">[vite]</span> hmr update /src/lib/util.ts',
  '<span class="dim">23:49:40</span> <span class="y">[vite]</span> page reload src/main.ts',
  '<span class="dim">23:50:01</span> <span class="c">[vite]</span> hmr update /src/App.vue',
]

export const CODE = [
  ['<span class="k1">import</span> { createHash } <span class="k1">from</span> <span class="k3">\'node:crypto\'</span>'],
  [''],
  ['<span class="k1">export function</span> <span class="k2">slugify</span>(text: <span class="k2">string</span>) {'],
  ['  <span class="k1">return</span> text'],
  ['    .normalize(<span class="k3">\'NFD\'</span>)'],
  ['    .replace(<span class="k3">/[\\u0300-\\u036f]/g</span>, <span class="k3">\'\'</span>)'],
  ['    .toLowerCase()'],
  ['    .replace(<span class="k3">/[^a-z0-9]+/g</span>, <span class="k3">\'-\'</span>)'],
  ['}'],
  [''],
  ['<span class="k1">export function</span> <span class="k2">cacheKey</span>(id: <span class="k2">string</span>) {'],
  ['  <span class="k1">const</span> h = createHash(<span class="k3">\'sha1\'</span>)'],
  ['  h.update(id)'],
  ['  <span class="k1">return</span> <span class="k3">`user:${</span>h.digest(<span class="k3">\'hex\'</span>).slice(<span class="k2">0</span>, <span class="k2">12</span>)<span class="k3">}`</span>'],
  ['}'],
  [''],
  ['<span class="k1">export const</span> TTL_MS = <span class="k2">60_000</span>'],
  [''],
  ['<span class="k1">export function</span> <span class="k2">fresh</span>(at: <span class="k2">number</span>, now = Date.now()) {'],
  ['  <span class="k1">return</span> now - at &lt; TTL_MS'],
  ['}'],
]

export function codeLines(from = 0, count = CODE.length) {
  return CODE.slice(from, from + count)
    .map(([l], i) => `<div class="cl"><span class="n">${from + i + 1}</span><span>${l || ' '}</span></div>`)
    .join('')
}

export const DIFF = [
  ['hunk', '', '@@ -12,9 +12,11 @@ router.get'],
  ['', '12', "router.get('/users/:id', async (req, res) => {"],
  ['del', '13', '  const user = await db.users.find(req.params.id)'],
  ['add', '13', '  const key = cacheKey(req.params.id)'],
  ['add', '14', '  const hit = cache.get(key)'],
  ['add', '15', '  if (hit &amp;&amp; fresh(hit.at)) return res.json(hit.user)'],
  ['', '16', '  if (!user) return res.status(404).end()'],
  ['', '17', '  res.json(user)'],
  ['', '18', '})'],
  ['hunk', '', '@@ -40,4 +42,5 @@ export default'],
  ['', '42', 'export default router'],
  ['add', '43', "export { cacheKey } from '../lib/util'"],
]

export function diffLines() {
  return DIFF.map(([k, n, t]) => {
    const s = k === 'add' ? '+' : k === 'del' ? '−' : ' '
    return `<div class="cl ${k}"><span class="n">${n}</span><span class="s">${k === 'hunk' ? '' : s}</span><span>${t}</span></div>`
  }).join('')
}
