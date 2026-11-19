import { writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { sprite } from './icons.mjs'
import { SCREENS } from './screens.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const out = join(here, '..', 'mockup', 'index.html')

const radio = (name, id, label, checked) => `<input type="radio" name="${name}" id="${id}"${checked ? ' checked' : ''}><label for="${id}">${label}</label>`

const controls = `<form class="ctl" aria-label="Chọn bản xem">
<fieldset><legend>Bảng màu</legend>${radio('pal', 'pal-mist', 'Sương kính', true)}${radio('pal', 'pal-nebula', 'Tinh vân')}${radio('pal', 'pal-galaxy', 'Ngân hà (tối)')}</fieldset>
<fieldset><legend>Sáng / tối</legend>${radio('sc', 'sc-light', 'Sáng', true)}${radio('sc', 'sc-dark', 'Tối')}</fieldset>
<fieldset><legend>Kính</legend>${radio('gl', 'gl-on', 'Bình thường', true)}${radio('gl', 'gl-reduced', 'Giảm trong suốt')}</fieldset>
<fieldset><legend>Thiết bị</legend>${radio('dv', 'dv-both', 'Cả hai', true)}${radio('dv', 'dv-phone', 'iPhone 390×844')}${radio('dv', 'dv-ipad', 'iPad 820×1180')}</fieldset>
<span class="note">WP-68a · mockup tĩnh, không JS, không font mạng</span>
</form>`

const sections = SCREENS.map((s) => `<section class="shot" id="s-${s.id}">
<div class="shot-h"><h2>${s.title}</h2><p>${s.note}</p><span class="gpu">Blur: ${s.blur}</span></div>
<div class="shot-row">
${s.phone()}
${s.ipad()}
</div>
</section>`).join('\n')

const html = `<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Orrery web app · Sương kính · mockup WP-68a</title>
<link rel="stylesheet" href="tokens.css">
<link rel="stylesheet" href="mist.css">
</head>
<body>
${sprite()}
${controls}
<main class="gallery">
${sections}
</main>
</body>
</html>
`

writeFileSync(out, html)
console.log(`ok ${SCREENS.length} screens -> ${out}`)
