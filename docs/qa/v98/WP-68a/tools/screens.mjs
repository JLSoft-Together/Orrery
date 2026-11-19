import { I } from './icons.mjs'
import {
  PROJECTS, AGENT, STATUS, chip, stIcon, projectCard, railProjects, railRow, sbar, tabBar, topBar, quota, seg, toggle,
  TERM_LINES, TERM_LONG, RUN_LOG, codeLines, diffLines,
} from './parts.mjs'

const phone = (id, inner) => `<div class="dev phone" id="${id}-phone" data-shot="${id}-phone">${sbar()}${inner}<div class="home-ind"></div></div>`
const ipad = (id, inner) => `<div class="dev ipad" id="${id}-ipad" data-shot="${id}-ipad">${sbar()}${inner}<div class="home-ind"></div></div>`

const routeChip = (kind = 'relay') => `<span class="chip route">${I(kind === 'lan' ? 'wifi' : 'cloud', 's14')}${kind === 'lan' ? 'LAN' : 'Relay'}</span>`

function machineCard(m) {
  const counts = m.counts.map(([st, n]) => chip(st, `${n} ${STATUS[st].text.toLowerCase()}`)).join('')
  return `<section class="card mcard ${m.dim ? 'dim' : ''}"><div class="hstack"><span class="tile">${I('laptop')}</span><div class="grow"><div class="t-title ell">${m.name}</div><div class="t-sub">${m.sub}</div></div>${m.offline ? '<span class="chip plain">Mất kết nối</span>' : routeChip(m.route)}${I('chev-r', 's16')}</div><div class="hstack wrap">${counts}</div>${m.quota ? `<div class="sep"></div>${quota()}` : ''}<div class="sep"></div><div class="hstack"><span class="${m.offline ? 'stale' : 't-faint'} grow">${m.offline ? I('clock', 's14') : ''}${m.when}</span><span class="select t-sub">Tự động ${I('chev-d', 's14')}</span><button class="ib round acc" aria-label="Tắt thông báo">${I('bell')}</button></div></section>`
}

const MACHINES = [
  { name: 'Orrery Desk', sub: 'Orrery 4.0.7 · Windows', route: 'relay', counts: [['working', 2], ['waiting', 1], ['error', 1]], when: 'Cập nhật 23:50', quota: true },
  { name: 'MacBook Air', sub: 'Orrery 4.0.7 · macOS', route: 'lan', counts: [['working', 1], ['idle', 3]], when: 'Cập nhật 23:49' },
  { name: 'Studio iMac', sub: 'Orrery 4.0.5 · macOS', offline: true, dim: true, counts: [['idle', 2]], when: 'Dữ liệu cũ · 21:40' },
]

function railHome(sel) {
  const rows = MACHINES.map((m, i) => `<div class="rm ${sel === i ? 'sel' : ''}"><span class="tile sm">${I('laptop', 's14')}</span><div class="grow"><div class="t-name ell">${m.name}</div><div class="t-faint">${m.when}</div></div>${m.offline ? `<span class="st idle">${I('clock', 's16')}</span>` : routeChip(m.route)}</div>`).join('')
  return `<aside class="rail glass-side"><div class="rail-h"><div class="hstack"><span class="logo">O</span><span class="t-title grow">Máy của tôi</span><button class="ib" aria-label="Cài đặt">${I('sliders')}</button></div></div><div class="rail-b"><div class="sec">Đang kết nối · 2</div>${rows}</div><div class="rail-f"><span class="btn pri block">${I('plus')}Ghép máy mới</span></div></aside>`
}

function railMachine(mode, sel) {
  const body = mode === 'monitor'
    ? `${seg(['Project', 'Trạng thái'], 0, 'sm')}`
    : ''
  const list = mode === 'monitor' ? railProjects(sel) : railWorkspace(sel)
  return `<aside class="rail glass-side"><div class="rail-h"><div class="hstack"><button class="ib" aria-label="Về danh sách máy">${I('chev-l')}</button><span class="logo">O</span><div class="grow"><div class="t-name ell">Orrery Desk</div><div class="t-faint">Cập nhật 23:50</div></div>${routeChip()}</div>${seg(['Giám sát', 'Workspace'], mode === 'monitor' ? 0 : 1)}${body}</div><div class="rail-b">${list}</div><div class="rail-f">${quota()}<div class="rail-icons"><button class="ib" aria-label="Thông báo">${I('bell')}</button><button class="ib" aria-label="Ghép máy">${I('qr')}</button><button class="ib" aria-label="Đổi máy">${I('laptop')}</button><button class="ib" aria-label="Cài đặt">${I('sliders')}</button></div></div></aside>`
}

const WS_TABS = [
  { mono: 'A', tone: 'mono-a', name: 'api', panes: [PROJECTS[0].sessions[0], PROJECTS[0].sessions[1], { name: 'npm test --watch', agent: 'shell', model: '', st: 'idle', time: '31p' }] },
  { mono: 'W', tone: 'mono-b', name: 'web', panes: PROJECTS[1].sessions },
  { mono: 'I', tone: 'mono-c', name: 'infra', panes: PROJECTS[2].sessions },
]

function railWorkspace(sel) {
  return WS_TABS.map((t, ti) => {
    const rows = t.panes.map((s, i) => railRow(s, sel === `${ti}:${i}`)).join('')
    return `<div class="rp">${I('chev-d', 's14')}<span class="tile sm ${t.tone}">${t.mono}</span><span class="grow">Tab ${t.name}</span><span class="t-faint num">${t.panes.length}</span></div>${rows}`
  }).join('')
}

function railSettings(sel) {
  const items = [['lock', 'Bảo mật'], ['bell', 'Thông báo'], ['palette', 'Giao diện'], ['laptop', 'Máy đã ghép'], ['data', 'Dữ liệu'], ['info', 'Giới thiệu']]
  const rows = items.map(([ic, t], i) => `<div class="rm ${i === sel ? 'sel' : ''}"><span class="tile sm">${I(ic, 's14')}</span><span class="t-name grow">${t}</span>${I('chev-r', 's14')}</div>`).join('')
  return `<aside class="rail glass-side"><div class="rail-h"><div class="hstack"><span class="logo">O</span><span class="t-title grow">Cài đặt</span></div></div><div class="rail-b">${rows}</div><div class="rail-f"><div class="rail-icons"><button class="ib" aria-label="Giám sát">${I('activity')}</button><button class="ib" aria-label="Workspace">${I('grid')}</button><button class="ib on" aria-label="Cài đặt">${I('sliders')}</button></div></div></aside>`
}

const mtop = (title, sub, right = '', left = '') => `<header class="mtop glass">${left}<h1>${title}${sub ? `<small>${sub}</small>` : ''}</h1>${right}</header>`

function homePhone() {
  const right = `<button class="ib" aria-label="Cài đặt">${I('sliders')}</button><button class="ib add" aria-label="Ghép máy">${I('plus')}</button>`
  const body = `<div class="scroll notab"><div class="sec"><span>Đang kết nối · 2</span></div>${machineCard(MACHINES[0])}${machineCard(MACHINES[1])}<div class="sec"><span>Máy khác · 1</span></div>${machineCard(MACHINES[2])}</div>`
  const top = `<header class="top glass"><span class="logo brand">O</span><h1>Máy của tôi</h1>${right}</header>`
  return phone('home', `${top}${body}`)
}

function homeIpad() {
  const right = `${routeChip()}<button class="ib round acc" aria-label="Tắt thông báo">${I('bell')}</button><span class="btn pri sm">Mở ${I('chev-r', 's16')}</span>`
  const body = `<div class="mbody"><section class="card mcard"><div class="hstack wrap">${chip('working', '2 đang chạy')}${chip('waiting', '1 chờ bạn')}${chip('error', '1 lỗi')}${chip('done', '1 xong')}${chip('idle', '1 rảnh')}</div><div class="sep"></div>${quota()}</section><div class="sec"><span>Session theo project</span><span>6</span></div>${PROJECTS.map((p) => projectCard(p)).join('')}</div>`
  return ipad('home', `${railHome(0)}<main class="main">${mtop('Orrery Desk', 'Orrery 4.0.7 · Windows · cập nhật 23:50', right)}${body}</main>`)
}

function pairPhone() {
  const body = `<div class="scroll notab">${seg([`${I('qr', 's16')}Quét QR`, `${I('keyboard', 's16')}Nhập mã`], 0)}<div class="qr"><div class="qr-code"></div><div class="qr-frame"></div><div class="qr-cap">Đưa mã QR vào khung</div></div><p class="t-sub center">Mở Orrery trên máy tính → Cài đặt → Điện thoại.</p><section class="card pad e2e">${I('shield')}<div><div class="t-name">Kết nối mã hoá đầu cuối</div><div class="t-sub">Điện thoại và máy tính dùng chung khoá Noise. Relay chỉ chuyển byte đã mã hoá.</div></div></section><span class="btn ghost block">${I('keyboard', 's16')}Nhập mã thủ công</span></div>`
  return phone('pair', `${topBar({ back: true, title: 'Ghép máy' })}${body}`)
}

function pairIpad() {
  const left = `<section class="card pad vstack"><div class="t-name">Quét QR</div><div class="qr"><div class="qr-code"></div><div class="qr-frame"></div><div class="qr-cap">Đưa mã QR vào khung</div></div><p class="t-sub center">Mở Orrery trên máy tính → Cài đặt → Điện thoại.</p></section>`
  const right = `<section class="card pad vstack"><div class="t-name">Nhập mã</div><div class="field"><label>Mã ghép trên máy tính</label><span class="input sq filled focus t-mono">orr1-7Kq2-mX9d-Lp0a-</span><span class="t-faint">Mã chỉ dùng được vài phút. Tạo mã mới trên máy tính nếu hết hạn.</span></div><div class="field"><label>Tên thiết bị này</label><span class="input sq filled">iPad của Jin</span></div><span class="btn pri block">Ghép</span><div class="sep"></div><div class="steps"><div class="step">${stIcon('done', 's16')}Đang đọc mã</div><div class="step">${stIcon('error', 's16')}<span>Đang thử mạng LAN <span class="t-faint">· bỏ qua, chưa có TLS</span></span></div><div class="step">${stIcon('working', 's16')}<b>Đang thử qua relay</b></div></div><div class="banner warn">${I('alert')}<div><strong>Nếu máy tính hỏi, hãy bấm cho phép ở đó.</strong></div></div></section>`
  const e2e = `<section class="card pad e2e">${I('shield')}<div><div class="t-name">Kết nối mã hoá đầu cuối</div><div class="t-sub">Điện thoại và máy tính dùng chung khoá Noise. Relay chỉ chuyển byte đã mã hoá.</div></div></section>`
  return ipad('pair', `${railHome(-1)}<main class="main">${mtop('Ghép máy mới', 'Quét QR hoặc dán mã ghép')}<div class="mbody"><div class="cols c2">${left}${right}</div>${e2e}</div></main>`)
}

const FILTERS = `<div class="filters"><span class="filt on">Tất cả <b>6</b></span><span class="filt">${I('waiting', 's16')}Chờ bạn <b>1</b></span><span class="filt">${I('working', 's16 spin')}Đang chạy <b>2</b></span><span class="filt">${I('error', 's16')}Lỗi <b>1</b></span></div>`

function monitorPhone() {
  const right = `${routeChip()}<button class="ib" aria-label="Nhóm theo trạng thái">${I('rows')}</button>`
  const tall = seg(['Project', 'Trạng thái'], 0, 'sm')
  const body = `<div class="scroll tall">${FILTERS}${PROJECTS.map((p, i) => projectCard(p)).join('')}</div>`
  return phone('monitor', `${topBar({ back: true, title: 'Orrery Desk', sub: 'Cập nhật 23:50', right, tall })}${body}${tabBar('monitor')}`)
}

function sessionCard(s, p) {
  const ag = AGENT[s.agent]
  return `<section class="card pad vstack"><div class="hstack"><span class="tile sm ${s.agent}">${I(ag.icon, 's14')}</span><span class="t-name grow ell">${s.name}</span>${chip(s.st)}</div><div class="t-sub hstack"><span class="tile sm ${p.tone}">${p.mono}</span>${p.name}<span class="dot"></span><span class="t-mono">${p.branch}</span></div><div class="hstack t-sub"><b class="agent-${s.agent}">${ag.name}</b>${s.model ? `<span class="dot"></span>${s.model}` : ''}<span class="grow"></span><span class="num">${s.time}</span>${s.gauge ? `<div class="gauge ${s.hot ? 'hot' : ''}"><i class="${s.gauge}"></i></div>` : ''}</div><div class="peek ell">${s.detail}</div></section>`
}

function monitorIpadBody(limit) {
  const cards = PROJECTS.flatMap((p) => p.sessions.map((s) => sessionCard(s, p))).join('')
  const banner = limit ? limitBanner() : ''
  return `<div class="mbody">${banner}${FILTERS}<div class="grid2">${cards}</div></div>`
}

function monitorIpad() {
  return ipad('monitor', `${railMachine('monitor', '')}<main class="main">${mtop('Tổng quan', '6 session · 3 project', `<button class="ib" aria-label="Tìm">${I('search')}</button>`)}${monitorIpadBody(false)}</main>`)
}

function limitBanner() {
  return `<div class="banner" role="alert">${I('alert')}<div class="grow"><strong>Dữ liệu quá lớn để gửi</strong><p>Máy tính có 6,4 MB dữ liệu giám sát, vượt giới hạn 4 MB mỗi tin. Đóng bớt session cũ trên máy tính rồi Thử lại.</p><span class="btn sm">${I('reload', 's16')}Thử lại</span></div></div>`
}

function limitPhone() {
  const right = `<span class="chip plain">${I('clock', 's14')}23:41</span>`
  const body = `<div class="scroll">${limitBanner()}<div class="sec"><span>Dữ liệu lúc 23:41</span></div>${PROJECTS.slice(0, 2).map((p) => projectCard(p, { dim: true })).join('')}</div>`
  return phone('limit', `${topBar({ back: true, title: 'Orrery Desk', sub: 'Dữ liệu cũ', right })}${body}${tabBar('monitor')}`)
}

function limitIpad() {
  return ipad('limit', `${railMachine('monitor', '')}<main class="main">${mtop('Tổng quan', 'Dữ liệu lúc 23:41', `<span class="chip plain">${I('clock', 's14')}Dữ liệu cũ</span>`)}${monitorIpadBody(true)}</main>`)
}

function detailContent(wide) {
  const s = PROJECTS[0].sessions[0]
  const hero = `<section class="card pad vstack"><div class="hstack"><span class="tile claude">${I('claude')}</span><div class="grow"><div class="t-title">claude patched</div><div class="t-sub hstack"><span class="tile sm mono-a">A</span>api<span class="dot"></span><span class="t-mono">feat/api</span></div></div>${chip('waiting')}</div><dl class="kv"><dt>Agent</dt><dd class="agent-claude">Claude</dd><dt>Model</dt><dd>Opus</dd><dt>Bắt đầu</dt><dd class="num">21:14 · 2g 36p</dd><dt>Context</dt><dd><span class="hstack"><span class="grow"></span><span class="num">72%</span><span class="gauge hot"><i class="w72"></i></span></span></dd></dl></section>`
  const ask = `<section class="perm"><div class="perm-q">${I('waiting')}<span>Chờ bạn: Chạy lệnh <code>npm test</code>?</span></div><div class="perm-btns"><span class="btn pri">${I('term', 's16')}Trả lời trong terminal</span></div></section>`
  const subs = `<section class="card list"><div class="li flat"><span class="t-name grow">Sub-agent · 2</span></div><div class="li flat">${stIcon('working', 's16')}<div class="grow"><div class="t-name">Explore</div><div class="t-sub ell">Tìm nơi dùng cacheKey</div></div><span class="t-faint num">48s</span></div><div class="li flat">${stIcon('done', 's16')}<div class="grow"><div class="t-name">Plan</div><div class="t-sub ell">Kế hoạch cache route users</div></div><span class="t-faint num">3p</span></div></section>`
  const acts = [
    ['23:50', 'waiting', 'Chờ bạn', 'Bash · npm test'],
    ['23:49', 'done', 'Công cụ xong', 'Bash · npm run typecheck'],
    ['23:47', 'working', 'Chạy công cụ', 'Edit · src/lib/util.ts'],
    ['23:46', 'working', 'Chạy công cụ', 'Edit · src/routes/users.ts'],
    ['23:44', 'done', 'Sub-agent xong', 'Plan'],
  ]
  const tl = `<section class="card pad"><div class="t-name">Hoạt động gần đây</div>${acts.map(([w, st, t, d]) => `<div class="tl"><span class="when">${w}</span>${stIcon(st, 's16')}<div class="what"><b>${t}</b><br><code>${d}</code></div></div>`).join('')}</section>`
  if (wide) return `${hero}${ask}<div class="cols c2">${subs}${tl}</div>`
  return `${hero}${ask}${subs}${tl}`
}

function detailPhone() {
  const right = `<button class="ib round acc" aria-label="Mở terminal">${I('term')}</button>`
  return phone('detail', `${topBar({ back: true, title: 'claude patched', sub: 'api · Orrery Desk', right })}<div class="scroll">${detailContent(false)}</div>${tabBar('monitor')}`)
}

function detailIpad() {
  const right = `<span class="btn pri sm">${I('term', 's16')}Mở terminal</span>`
  return ipad('detail', `${railMachine('monitor', 'api:0')}<main class="main">${mtop('claude patched', 'api · feat/api', right)}<div class="mbody">${detailContent(true)}</div></main>`)
}

function wsTabs(on = 0) {
  return `<div class="tabs">${WS_TABS.map((t, i) => `<span class="tabp ${i === on ? 'on' : ''}"><span class="tile sm ${t.tone}">${t.mono}</span>${t.name}${i === 0 ? chip('waiting', '1') : `<span class="t-faint num">${t.panes.length}</span>`}</span>`).join('')}</div>`
}

function paneChips() {
  return `<div class="pchips"><span class="pchip on"><span class="adot"></span>claude patched</span><span class="pchip"><span class="adot codex"></span>codex review</span><span class="pchip"><span class="adot shell"></span>npm test</span></div>`
}

function paneCard(s, lines, opts = {}) {
  const ag = AGENT[s.agent]
  if (opts.grid) return `<section class="card pane ${opts.act ? 'act' : ''}"><div class="pane-h"><span class="tile sm ${s.agent}">${I(ag.icon, 's14')}</span><span class="t-name ell">${s.name}</span><button class="ib" aria-label="Thao tác pane">${I('more')}</button></div><div class="pane-b vstack">${chip(s.st)}<div class="well">${lines}</div></div></section>`
  return `<section class="card pane ${opts.act ? 'act' : ''}"><div class="pane-h"><span class="tile sm ${s.agent}">${I(ag.icon, 's14')}</span><span class="t-name ell">${s.name}</span>${chip(s.st)}<button class="ib" aria-label="Thao tác pane">${I('more')}</button></div><div class="pane-b"><div class="well">${lines}</div></div></section>`
}

function wsAPhone() {
  const right = `${routeChip()}<button class="ib" aria-label="Xem lưới pane">${I('grid')}</button>`
  const s = WS_TABS[0].panes[0]
  const body = `<div class="scroll tall">${paneChips()}${paneCard(s, TERM_LINES.join('\n'), { act: true })}<div class="hstack"><span class="btn pri grow">${I('max', 's16')}Mở toàn màn hình</span><button class="ib round" aria-label="Gửi prompt">${I('send')}</button></div><p class="hint">Vuốt ngang để đổi pane · 3 pane trong tab</p></div>`
  return phone('ws-a', `${topBar({ title: 'Workspace', sub: 'Orrery Desk', right, tall: wsTabs(0) })}${body}${tabBar('workspace')}`)
}

function wsAIpad() {
  const s = WS_TABS[0].panes[0]
  const right = `${seg([I('rows', 's16'), I('grid', 's16')], 0, 'sm')}<span class="btn sm">${I('plus', 's16')}Pane mới</span>`
  const body = `<div class="mbody">${paneChips()}${paneCard(s, TERM_LONG.join('\n'), { act: true })}<div class="hstack"><span class="btn pri grow">${I('max', 's16')}Mở toàn màn hình</span><span class="btn">${I('send', 's16')}Gửi prompt</span><span class="btn">${I('panel', 's16')}Tệp, diff, git</span></div></div>`
  return ipad('ws-a', `${railMachine('workspace', '0:0')}<main class="main">${mtop(wsTabs(0), '', right)}${body}</main>`)
}

const GRID_LINES = [
  ['Chạy lệnh: npm test?\n❯ 1. Có\n  2. Có, luôn luôn'],
  ['● Edit src/app.ts\n  + 8 − 2\n● Đọc README.md'],
  ['PASS src/util.test.ts\nPASS src/routes.test.ts\nTests: 42 passed'],
  ['vite đang chạy\n➜ Local: :5173\nhmr update App.vue'],
]

function gridPanes(n) {
  const panes = [WS_TABS[0].panes[0], WS_TABS[0].panes[1], WS_TABS[0].panes[2], PROJECTS[1].sessions[1]].slice(0, n)
  return `<div class="grid2">${panes.map((s, i) => paneCard(s, GRID_LINES[i][0], { act: i === 0, grid: true })).join('')}</div>`
}

function wsBPhone() {
  const right = `${routeChip()}<button class="ib on" aria-label="Xem từng pane">${I('grid')}</button>`
  const body = `<div class="scroll tall"><div class="filters"><span class="filt on">Tất cả <b>3</b></span><span class="filt">${I('working', 's16 spin')}Đang chạy <b>1</b></span><span class="filt">${I('waiting', 's16')}Chờ bạn <b>1</b></span></div>${gridPanes(3)}</div>`
  return phone('ws-b', `${topBar({ title: 'Workspace', sub: 'Orrery Desk', right, tall: wsTabs(0) })}${body}<span class="btn pri fab">${I('plus')}Pane mới</span>${tabBar('workspace')}`)
}

function wsBIpad() {
  const right = `${seg([I('rows', 's16'), I('grid', 's16')], 1, 'sm')}<span class="btn sm">${I('plus', 's16')}Pane mới</span>`
  const body = `<div class="mbody"><div class="filters"><span class="filt on">Tất cả <b>4</b></span><span class="filt">${I('working', 's16 spin')}Đang chạy <b>1</b></span><span class="filt">${I('waiting', 's16')}Chờ bạn <b>1</b></span></div>${gridPanes(4)}</div>`
  return ipad('ws-b', `${railMachine('workspace', '')}<main class="main">${mtop(wsTabs(0), '', right)}${body}</main>`)
}

const PERM = `<section class="perm"><div class="perm-q">${I('waiting')}<span>Chạy lệnh: <code>npm test</code>? [y/n]</span></div><div class="perm-btns"><span class="btn pri">Có</span><span class="btn">Có, luôn luôn</span><span class="btn">Không</span><span class="btn">Dừng</span><span class="btn">Esc</span></div></section>`

const DOCK_KEYS = `<div class="keys"><span class="kbd">Esc</span><span class="kbd">Tab</span><span class="kbd on">Ctrl</span><span class="kbd">Alt</span><span class="kbd">⇧Tab</span><span class="kbd">^C</span><span class="kbd">${I('up', 's16')}</span></div>`
const DOCK_INPUT = `<div class="hstack"><button class="ib" aria-label="Gõ trực tiếp vào terminal">${I('keyboard')}</button><span class="input">Gõ vào pane…</span><button class="ib round" aria-label="Gửi">${I('send')}</button></div>`

function termPhone() {
  const top = `<header class="top glass"><button class="ib back" aria-label="Quay lại">${I('chev-l', 's24')}</button><span class="picker"><span class="adot"></span><span class="grow ell">Claude · claude patched</span>${I('chev-d', 's16')}</span>${chip('waiting', 'Chờ')}<button class="ib" aria-label="Xem tệp, diff, git">${I('panel')}</button></header>`
  const body = `<div class="scroll term">${PERM}<div class="well term-full">${TERM_LONG.slice(6).join('\n')}\n<span class="v">&gt;</span> <span class="cursor"></span></div><div class="dock solid">${DOCK_KEYS}${DOCK_INPUT}</div></div>`
  return phone('term', `${top}${body}`)
}

function termIpad() {
  const left = `<span class="picker"><span class="adot"></span><span class="grow ell">Claude · claude patched</span>${I('chev-d', 's16')}</span>`
  const right = `${chip('waiting')}<span class="btn sm">${I('panel', 's16')}Tệp, diff, git</span>`
  const body = `<div class="mbody">${PERM}<div class="well term-full">${TERM_LONG.join('\n')}\n<span class="v">&gt;</span> <span class="cursor"></span></div><div class="dock solid">${DOCK_KEYS}${DOCK_INPUT}</div></div>`
  return ipad('term', `${railMachine('workspace', '0:0')}<main class="main"><header class="mtop glass">${left}${right}</header>${body}</main>`)
}

const VSEG = (on) => seg([`${I('folder', 's16')}Tệp`, 'Diff', `${I('branch', 's16')}Git`, `${I('play', 's16')}Chạy`], on)

function viewerTop(on) {
  return topBar({ back: true, title: 'claude patched', sub: 'api · feat/api', right: `<button class="ib" aria-label="Đóng">${I('x')}</button>`, tall: VSEG(on) })
}

function filesPhone() {
  const body = `<div class="scroll tall notab"><div class="crumb">${I('folder', 's14')}api<span>/</span>src<span>/</span>lib<span>/</span><b>util.ts</b></div><section class="card pad code">${codeLines(0, 21)}</section><div class="hstack"><span class="t-faint grow">21 dòng, 612 B</span><span class="btn sm">${I('pencil', 's16')}Sửa</span></div></div>`
  return phone('files', `${viewerTop(0)}${body}`)
}

const TREE = [
  ['dir', 'src', 0, true],
  ['dir', 'lib', 1, true],
  ['ts', 'cache.ts', 2],
  ['ts', 'util.ts', 2, false, true],
  ['dir', 'routes', 1],
  ['ts', 'index.ts', 1],
  ['dir', 'test', 0],
  ['json', 'package.json', 0],
  ['md', 'README.md', 0],
  ['json', 'tsconfig.json', 0],
]

function treeCard() {
  const rows = TREE.map(([k, n, d, open, sel]) => {
    const ic = k === 'dir' ? `<span class="fi dir">${I(open ? 'chev-d' : 'chev-r', 's14')}</span><span class="fi dir">${I('folder', 's16')}</span>` : `<span class="fi"></span><span class="fi ${k}">${k.toUpperCase().slice(0, 2)}</span>`
    return `<div class="li flat ${sel ? 'srow sel' : ''}">${'<span class="fi"></span>'.repeat(d)}${ic}<span class="ell ${sel ? 't-name' : ''}">${n}</span></div>`
  }).join('')
  return `<section class="card list tree">${rows}</section>`
}

function filesIpad() {
  const right = `<span class="btn sm">${I('pencil', 's16')}Sửa</span><button class="ib" aria-label="Đóng">${I('x')}</button>`
  const body = `<div class="mbody">${VSEG(0)}<div class="cols c-l">${treeCard()}<section class="card pad vstack"><div class="crumb">${I('folder', 's14')}api<span>/</span>src<span>/</span>lib<span>/</span><b>util.ts</b><span class="grow"></span><span class="t-faint">21 dòng, 612 B</span></div><div class="code">${codeLines(0, 21)}</div></section></div></div>`
  return ipad('files', `${railMachine('workspace', '0:0')}<main class="main">${mtop('claude patched', 'api · feat/api', right)}${body}</main>`)
}

function changeSummary() {
  return `<div class="hstack"><span class="chip done">${I('plus', 's14')}4 thêm</span><span class="chip error">${I('minus', 's14')}1 xoá</span><span class="grow"></span><span class="t-faint">3 tệp</span></div>`
}

function diffPhone() {
  const body = `<div class="scroll tall notab">${changeSummary()}<section class="card list"><div class="li flat"><span class="gs m">M</span><span class="grow ell t-mono">src/routes/users.ts</span><span class="plus">+3</span><span class="minus">−1</span>${I('chev-d', 's16')}</div><div class="code">${diffLines()}</div></section><section class="card list"><div class="li flat"><span class="gs m">M</span><span class="grow ell t-mono">src/lib/util.ts</span><span class="plus">+1</span><span class="minus">−0</span>${I('chev-r', 's16')}</div><div class="li flat"><span class="gs u">?</span><span class="grow ell t-mono">notes.txt</span>${I('chev-r', 's16')}</div></section></div>`
  return phone('diff', `${viewerTop(1)}${body}`)
}

function diffIpad() {
  const list = `<section class="card list"><div class="li flat srow sel"><span class="gs m">M</span><span class="grow ell t-mono">users.ts</span><span class="plus">+3</span></div><div class="li flat"><span class="gs m">M</span><span class="grow ell t-mono">util.ts</span><span class="plus">+1</span></div><div class="li flat"><span class="gs u">?</span><span class="grow ell t-mono">notes.txt</span></div></section>`
  const body = `<div class="mbody">${VSEG(1)}${changeSummary()}<div class="cols c-l">${list}<section class="card list"><div class="li flat"><span class="gs m">M</span><span class="grow ell t-mono">src/routes/users.ts</span><span class="plus">+3</span><span class="minus">−1</span></div><div class="code">${diffLines()}</div></section></div></div>`
  return ipad('diff', `${railMachine('workspace', '0:0')}<main class="main">${mtop('claude patched', 'api · feat/api', `<button class="ib" aria-label="Đóng">${I('x')}</button>`)}${body}</main>`)
}

function changesCard() {
  return `<section class="card list"><div class="li flat"><span class="t-name grow">Đã stage (1)</span><span class="btn sm">Unstage tất cả</span></div><div class="li flat"><span class="gs m">M</span><span class="grow ell t-mono">src/lib/util.ts</span><span class="plus">+1</span><span class="minus">−0</span><button class="ib" aria-label="Unstage">${I('minus')}</button></div></section><section class="card list"><div class="li flat"><span class="t-name grow">Thay đổi (2)</span><span class="btn sm">Stage tất cả</span></div><div class="li flat"><span class="gs m">M</span><span class="grow ell t-mono">src/routes/users.ts</span><span class="plus">+3</span><span class="minus">−1</span><button class="ib" aria-label="Bỏ thay đổi">${I('undo')}</button><button class="ib" aria-label="Stage">${I('plus')}</button></div><div class="li flat"><span class="gs u">?</span><span class="grow ell t-mono">notes.txt</span><button class="ib" aria-label="Bỏ thay đổi">${I('undo')}</button><button class="ib" aria-label="Stage">${I('plus')}</button></div></section>`
}

const COMMIT_BOX = `<div class="commit"><span class="input sq filled focus">Thêm cache cho route users</span><span class="btn pri">${I('commit', 's16')}Commit</span></div>`

function gitPhone() {
  const body = `<div class="scroll tall notab">${changesCard()}<div class="sec"><span>Commit gần đây</span></div><section class="card list">${[['a1f3c2e', 'Sửa thanh phím', '2g'], ['9b04d71', 'Route users trả 404', '5g'], ['77e2a90', 'Thêm test util', 'hôm qua']].map(([h, m, w]) => `<div class="li flat"><span class="t-mono t-faint">${h}</span><span class="grow ell">${m}</span><span class="t-faint">${w}</span></div>`).join('')}</section></div><div class="dock solid">${COMMIT_BOX}</div>`
  return phone('git', `${viewerTop(2)}${body}`)
}

function gitIpad() {
  const log = `<section class="card list"><div class="li flat"><span class="t-name grow">Lịch sử · main</span>${I('branch', 's16')}</div>${[['a1f3c2e', 'Sửa thanh phím', 'Jin', '2g'], ['9b04d71', 'Route users trả 404', 'Jin', '5g'], ['77e2a90', 'Thêm test util', 'codex', 'hôm qua'], ['31c8e04', 'Tách lib/cache', 'Jin', '2 ngày'], ['0de51aa', 'Khởi tạo api', 'Jin', '5 ngày']].map(([h, m, a, w]) => `<div class="li flat"><span class="t-mono t-faint">${h}</span><div class="grow"><div class="ell t-name">${m}</div><div class="t-faint">${a} · ${w}</div></div></div>`).join('')}</section>`
  const body = `<div class="mbody">${VSEG(2)}<div class="cols c2"><div class="vstack">${changesCard()}<section class="card pad vstack"><div class="field"><label>Commit message</label>${COMMIT_BOX}</div></section></div>${log}</div></div>`
  return ipad('git', `${railMachine('workspace', '0:0')}<main class="main">${mtop('claude patched', 'api · feat/api', `<button class="ib" aria-label="Đóng">${I('x')}</button>`)}${body}</main>`)
}

function runForm() {
  return `<div class="field"><label>Cấu hình run</label><span class="select"><span class="tile sm shell">${I('term', 's14')}</span><span class="grow">dev · npm run dev</span>${I('chev-d', 's16')}</span></div><div class="field"><label>Thiết bị</label><span class="select"><span class="tile sm">${I('phone', 's14')}</span><span class="grow">iPhone 15 · Simulator</span>${I('chev-d', 's16')}</span></div><div class="runbar"><span class="btn pri">${I('reload', 's16')}Restart</span><span class="btn">${I('reload', 's16')}Reload</span><span class="btn danger">${I('stop', 's16')}Dừng</span></div><div class="hstack">${chip('working', 'Đang chạy · 3p 12s')}<span class="grow"></span><span class="t-faint">pid 18244</span></div>`
}

function runPhone() {
  const body = `<div class="scroll tall notab">${runForm()}<div class="well fill">${RUN_LOG.join('\n')}</div></div>`
  return phone('run', `${viewerTop(3)}${body}`)
}

function runIpad() {
  const body = `<div class="mbody">${VSEG(3)}<div class="cols c2"><section class="card pad vstack">${runForm()}</section><div class="well">${RUN_LOG.join('\n')}</div></div></div>`
  return ipad('run', `${railMachine('workspace', '0:0')}<main class="main">${mtop('claude patched', 'api · feat/api', `<button class="ib" aria-label="Đóng">${I('x')}</button>`)}${body}</main>`)
}

function palettes() {
  const list = [['mist', 'Sương kính', true], ['nebula', 'Tinh vân'], ['aurora', 'Cực quang'], ['graphite', 'Than chì'], ['dawn', 'Bình minh'], ['galaxy', 'Ngân hà']]
  return `<div class="pals">${list.map(([id, n, on]) => `<div class="pal ${on ? 'on' : ''}"><div class="pal-sw sw-${id}"><i></i><u></u><b></b></div>${n}</div>`).join('')}</div>`
}

function appearanceCards() {
  return `<section class="card pad vstack"><div class="hstack">${I('palette', 'lead')}<span class="t-name grow">Bảng màu</span><span class="t-faint">giống máy tính</span></div>${palettes()}</section><section class="card list"><div class="li">${I('contrast', 'lead')}<span class="grow">Sáng / tối</span>${seg(['Theo máy', 'Sáng', 'Tối'], 0, 'sm')}</div><div class="li">${I('drop', 'lead')}<div class="grow"><div>Giảm trong suốt</div><div class="t-faint">Panel đặc, tắt blur. Đỡ tốn pin, cuộn mượt hơn trên máy cũ</div></div>${toggle(false)}</div><div class="li">${I('globe', 'lead')}<span class="grow">Ngôn ngữ</span><span class="val">Tiếng Việt ${I('chev-r', 's16')}</span></div></section>`
}

function settingsPhone() {
  const notif = `<section class="card list"><div class="li">${I('bell', 'lead')}<div class="grow"><div>Thông báo đẩy</div><div class="t-faint">Đang bật · 2 máy tính</div></div><span class="btn sm">Tắt</span></div><div class="li">${I('waiting', 'lead')}<span class="grow">Session chờ bạn</span>${toggle(true)}</div><div class="li">${I('done', 'lead')}<span class="grow">Session xong</span>${toggle(true)}</div></section>`
  const sec = `<section class="card list"><div class="li">${I('face', 'lead')}<div class="grow"><div>Khoá Face ID</div><div class="t-faint">Hỏi khi mở app</div></div>${toggle(true)}</div><div class="li">${I('clock', 'lead')}<span class="grow">Khoá sau khi không dùng</span><span class="val">15 phút ${I('chev-r', 's16')}</span></div></section>`
  const body = `<div class="scroll"><div class="sec"><span>Thông báo</span></div>${notif}<div class="sec"><span>Giao diện</span></div>${appearanceCards()}<div class="sec"><span>Bảo mật</span></div>${sec}</div>`
  return phone('settings', `${topBar({ title: 'Cài đặt' })}${body}${tabBar('settings')}`)
}

function settingsIpad() {
  const body = `<div class="mbody">${appearanceCards()}<section class="card pad vstack"><div class="t-name">Xem trước</div><div class="grid2">${paneCard(WS_TABS[0].panes[0], 'Chạy lệnh: npm test?\n❯ 1. Có', { act: true, grid: true })}${paneCard(WS_TABS[0].panes[1], '● Edit src/app.ts\n  + 8 − 2', { grid: true })}</div></section><p class="foot-note">Bảng màu, sáng / tối và Giảm trong suốt lưu trên điện thoại này, không đổi máy tính.</p></div>`
  return ipad('settings', `${railSettings(2)}<main class="main">${mtop('Giao diện', 'Giống UI next gen trên máy tính')}${body}</main>`)
}

function lockInner() {
  return `<div class="wiz"><i class="on"></i><i class="on"></i><i class="on"></i></div><span class="okline">${I('done', 's16')}Đã ghép Orrery Desk</span><div class="face">${I('face')}</div><h2>Bật khoá Face ID?</h2><p>Mỗi lần mở app sẽ hỏi xác thực, để người khác không đọc được session của bạn.</p><p class="t-sub hstack">${I('shield', 's16')}Có thể đổi sau trong Cài đặt.</p>`
}

function lockPhone() {
  const body = `<div class="scroll notab"><section class="card pad hero mt12">${lockInner()}</section><div class="mtauto vstack"><span class="btn pri block">${I('face', 's16')}Bật Face ID</span><span class="btn ghost block">Để sau</span></div></div>`
  return phone('lock', `${topBar({ back: true, title: 'Ghép máy' })}${body}`)
}

function lockIpad() {
  return ipad('lock', `<div class="lockpad"><section class="lockcard glass-pop hero">${lockInner()}<span class="btn pri block mt8">${I('face', 's16')}Bật Face ID</span><span class="btn ghost block">Để sau</span></section></div>`)
}

export const SCREENS = [
  { id: 'home', title: 'Home nhiều máy', note: 'Máy đang nối + máy mất kết nối, hạn mức theo máy (đề xuất)', blur: 'Phone 1 lớp (thanh trên) · iPad 2 (rail, thanh trên)', phone: homePhone, ipad: homeIpad },
  { id: 'pair', title: 'Ghép máy', note: 'Quét QR / Nhập mã; iPad hiện cả hai + tiến trình nối', blur: 'Phone 1 · iPad 2', phone: pairPhone, ipad: pairIpad },
  { id: 'lock', title: 'Bước "Bật khoá Face ID"', note: 'Ngay sau khi ghép máy đầu tiên', blur: 'Phone 1 · iPad 1 (thẻ nổi trên nền tĩnh)', phone: lockPhone, ipad: lockIpad },
  { id: 'monitor', title: 'Monitor A', note: 'Session lùi vào trong project (như rail desktop v103), lọc trạng thái', blur: 'Phone 2 (thanh trên, tab bar) · iPad 2', phone: monitorPhone, ipad: monitorIpad },
  { id: 'limit', title: 'Banner lỗi limit', note: 'Snapshot quá lớn: banner role=alert + Thử lại, dữ liệu cũ mờ đi', blur: 'Phone 2 · iPad 2', phone: limitPhone, ipad: limitIpad },
  { id: 'detail', title: 'Chi tiết session', note: 'Model, context, câu hỏi đang chờ, sub-agent, hoạt động', blur: 'Phone 2 · iPad 2', phone: detailPhone, ipad: detailIpad },
  { id: 'ws-a', title: 'Workspace A (từng pane)', note: 'Tab pill + chip pane, terminal xem trước đặc, không blur', blur: 'Phone 2 · iPad 2', phone: wsAPhone, ipad: wsAIpad },
  { id: 'term', title: 'Terminal', note: 'Thanh hỏi quyền, terminal đặc, dock phím đặc (không blur trên terminal)', blur: 'Phone 1 · iPad 2', phone: termPhone, ipad: termIpad },
  { id: 'ws-b', title: 'Workspace B (lưới)', note: 'Lưới pane, viền accent pane đang chọn', blur: 'Phone 2 · iPad 2', phone: wsBPhone, ipad: wsBIpad },
  { id: 'files', title: 'Viewer: Tệp', note: 'Nội dung tệp, nút Sửa; iPad có cây thư mục', blur: 'Phone 1 · iPad 2', phone: filesPhone, ipad: filesIpad },
  { id: 'diff', title: 'Viewer: Diff', note: 'Tổng +/−, diff unified', blur: 'Phone 1 · iPad 2', phone: diffPhone, ipad: diffIpad },
  { id: 'git', title: 'Viewer: Git commit', note: 'Stage / unstage / bỏ thay đổi + commit; iPad thêm lịch sử', blur: 'Phone 1 · iPad 2', phone: gitPhone, ipad: gitIpad },
  { id: 'run', title: 'Viewer: Chạy', note: 'Cấu hình + thiết bị, Restart / Reload / Dừng, log đặc', blur: 'Phone 1 · iPad 2', phone: runPhone, ipad: runIpad },
  { id: 'settings', title: 'Cài đặt', note: 'Mục Giao diện mới: bảng màu, sáng / tối, Giảm trong suốt', blur: 'Phone 2 · iPad 2', phone: settingsPhone, ipad: settingsIpad },
]
