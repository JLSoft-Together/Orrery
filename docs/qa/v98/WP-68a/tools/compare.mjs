import { writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { SCREENS } from './screens.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const out = join(here, '..', 'compare.html')

const OLD = {
  home: ['../WP-55a/home-phone-light-webkit.png', '../WP-55a/home-ipad-light-webkit.png'],
  pair: ['../WP-55a/pair-scan-phone-light-webkit.png', '../WP-55a/pair-code-ipad-light-webkit.png'],
  lock: ['../WP-65/lock-offer-phone-light-chromium.png', ''],
  monitor: ['../WP-55a/monitor-a-phone-light-webkit.png', '../WP-55a/monitor-a-ipad-light-webkit.png'],
  limit: ['../WP-55a/monitor-offline-phone-light-webkit.png', ''],
  detail: ['../WP-55a/detail-phone-light-webkit.png', '../WP-55a/detail-ipad-light-webkit.png'],
  'ws-a': ['../WP-55b/ws-a-phone-light-webkit.png', '../WP-55b/ws-a-ipad-light-webkit.png'],
  term: ['../WP-55b/term-phone-light-webkit.png', '../WP-55b/term-ipad-light-webkit.png'],
  'ws-b': ['../WP-55b/ws-b-phone-light-webkit.png', '../WP-55b/ws-b-ipad-light-webkit.png'],
  files: ['../WP-55b/file-phone-light-webkit.png', '../WP-55b/file-ipad-light-webkit.png'],
  diff: ['../WP-55b/diff-phone-light-webkit.png', '../WP-55b/diff-ipad-light-webkit.png'],
  git: ['../WP-61b/changes-phone-light-webkit.png', '../WP-61b/changes-ipad-light-webkit.png'],
  run: ['../WP-61b/run-phone-light-webkit.png', '../WP-61b/run-ipad-light-webkit.png'],
  settings: ['../WP-55a/settings-phone-light-webkit.png', '../WP-55a/settings-ipad-light-webkit.png'],
}

const NOTE_OLD = { limit: 'Ảnh cũ gần nhất: banner mất kết nối (banner limit chưa có ảnh riêng)' }

const VARIANTS = [
  ['mist-light', 'Sương kính sáng'],
  ['mist-dark', 'Sương kính tối'],
  ['nebula-light', 'Tinh vân sáng'],
  ['nebula-dark', 'Tinh vân tối'],
  ['galaxy-dark', 'Ngân hà'],
  ['mist-light-reduced', 'Giảm trong suốt'],
]

const fig = (src, cap, cls = '') => src
  ? `<figure class="${cls}"><a href="${src}"><img src="${src}" alt="${cap}" loading="lazy"></a><figcaption>${cap}</figcaption></figure>`
  : `<figure class="${cls} none"><div class="empty">Không có ảnh cũ</div><figcaption>${cap}</figcaption></figure>`

const rows = SCREENS.map((s) => {
  const [oldPhone, oldPad] = OLD[s.id]
  const phoneRow = `<div class="pair phone">${fig(oldPhone, 'Cũ')}${fig(`${s.id}-phone-mist-light-chromium.jpg`, 'Mới · sáng')}${fig(`${s.id}-phone-mist-dark-chromium.jpg`, 'Mới · tối')}</div>`
  const padRow = `<div class="pair pad">${fig(oldPad, 'Cũ · iPad')}${fig(`${s.id}-ipad-mist-light-chromium.jpg`, 'Mới · iPad sáng')}</div>`
  const variants = `<details><summary>6 biến thể WebKit (iPhone + iPad)</summary><div class="vars">${VARIANTS.map(([v, n]) => fig(`${s.id}-phone-${v}-webkit.jpg`, n, 'v')).join('')}</div><div class="vars pad">${VARIANTS.map(([v, n]) => fig(`${s.id}-ipad-${v}-webkit.jpg`, `iPad · ${n}`, 'v')).join('')}</div></details>`
  return `<section id="${s.id}"><header><h2>${s.title}</h2><p>${s.note}</p><p class="blur">Lớp blur: ${s.blur}</p>${NOTE_OLD[s.id] ? `<p class="warn">${NOTE_OLD[s.id]}</p>` : ''}</header>${phoneRow}${padRow}${variants}</section>`
}).join('\n')

const nav = SCREENS.map((s) => `<a href="#${s.id}">${s.title}</a>`).join('')

const html = `<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>WP-68a · web app cũ | Sương kính</title>
<link rel="stylesheet" href="compare.css">
</head>
<body>
<header class="top">
<h1>Web app iPhone / iPad: cũ | Sương kính</h1>
<p>Ảnh "Mới" chụp bằng Chromium (Edge) vì Playwright WebKit trên Windows không vẽ backdrop-filter. Biến thể đủ bảng màu chụp bằng WebKit nằm trong mục mở rộng của từng màn. Mockup sống: <a href="mockup/index.html">mockup/index.html</a> (chọn bảng màu, sáng / tối, Giảm trong suốt, thiết bị).</p>
<nav>${nav}</nav>
</header>
<main>
${rows}
</main>
</body>
</html>
`

writeFileSync(out, html)
console.log(`ok compare -> ${out}`)
