const MAU_BASE = {
  light: {
    canvas: '#f7f7f7', side: '#f2f2f2', surface: '#ffffff', raised: '#fafafa', control: '#ffffff',
    line: '#e5e5e5', 'line-strong': '#d4d4d4',
    ink: '#1f1f1f', 'ink-muted': '#525252', 'ink-faint': '#636363',
    working: '#1f5fbf', waiting: '#7a5a00', idle: '#636363', done: '#1b6e37', error: '#c02a2a',
    'agent-claude': '#b85a35', 'agent-codex': '#6750b8', 'agent-shell': '#17786f',
    'diff-add': '#1b6e37', 'diff-del': '#c02a2a',
    well: '#ffffff', 'well-ink': '#1f1f1f', 'well-dim': '#636363',
    'well-green': '#1b6e37', 'well-red': '#c02a2a', 'well-blue': '#1f5fbf', 'well-yellow': '#7a5a00',
  },
  dark: {
    canvas: '#1e1e1e', side: '#181818', surface: '#252525', raised: '#2b2b2b', control: '#333333',
    line: '#333333', 'line-strong': '#444444',
    ink: '#e6e6e6', 'ink-muted': '#b0b0b0', 'ink-faint': '#9a9a9a',
    working: '#7fb0ff', waiting: '#e5c07b', idle: '#9a9a9a', done: '#6fcf8f', error: '#f28b82',
    'agent-claude': '#d9825f', 'agent-codex': '#a89cf0', 'agent-shell': '#5cc0b4',
    'diff-add': '#6fcf8f', 'diff-del': '#f28b82',
    well: '#121212', 'well-ink': '#e6e6e6', 'well-dim': '#8c8c8c',
    'well-green': '#6fcf8f', 'well-red': '#f28b82', 'well-blue': '#8ab4f8', 'well-yellow': '#e5c07b',
  },
}

function mauPreset(id, name, tag, la, li, da, dOn, di) {
  return {
    id, name, tag,
    light: { ...MAU_BASE.light, accent: la, 'accent-ink': li, 'on-accent': '#ffffff', 'well-accent': li },
    dark: { ...MAU_BASE.dark, accent: da, 'accent-ink': di, 'on-accent': dOn, 'well-accent': di },
  }
}

const MAU_PALETTES = [
  mauPreset('blue', 'Xanh', 'Xanh dương chuẩn', '#0a66d6', '#0758bd', '#4c9aff', '#0a1a33', '#7fb3ff'),
  mauPreset('mono', 'Đen trắng', 'Không màu nhấn', '#262626', '#262626', '#e6e6e6', '#1e1e1e', '#f0f0f0'),
  mauPreset('green', 'Lục', 'Xanh lá', '#1f7a44', '#1a6a3b', '#5cc185', '#0c2416', '#7dd3a0'),
  mauPreset('violet', 'Tím', 'Tím xanh', '#5b4fcf', '#4f43bd', '#9d94f5', '#191536', '#b3abff'),
  mauPreset('teal', 'Xanh ngọc', 'Xanh ngọc', '#0f7575', '#0c6767', '#4cc3c3', '#052222', '#72d6d6'),
]

const MAU_AVOID = [
  { name: 'error', h: 25 }, { name: 'agent-claude', h: 45 }, { name: 'waiting', h: 85 }, { name: 'done', h: 145 },
  { name: 'agent-shell', h: 185 }, { name: 'working', h: 255 }, { name: 'agent-codex', h: 290 },
]
const MAU_HUE_GAP = 18

function mauOklchToHex(L, C, h) {
  let c = C
  for (let i = 0; i < 40; i++) {
    const a = c * Math.cos((h * Math.PI) / 180)
    const b = c * Math.sin((h * Math.PI) / 180)
    const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3
    const lin = [
      4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
      -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
      -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
    ]
    if (lin.every((v) => v >= -0.0005 && v <= 1.0005) || c <= 0) {
      return '#' + lin.map((v) => {
        const x = Math.min(1, Math.max(0, v))
        const g = x <= 0.0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - 0.055
        return Math.round(g * 255).toString(16).padStart(2, '0')
      }).join('')
    }
    c = Math.max(0, c - 0.005)
  }
  return '#808080'
}

function mauRng(seed) {
  let t = seed >>> 0
  return () => {
    t = (t + 0x6d2b79f5) >>> 0
    let r = Math.imul(t ^ (t >>> 15), 1 | t)
    r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296
  }
}

function mauHueDist(a, b) {
  const d = Math.abs(a - b) % 360
  return Math.min(d, 360 - d)
}

function mauAllowedHues() {
  const out = []
  for (let h = 0; h < 360; h++) if (MAU_AVOID.every((x) => mauHueDist(h, x.h) >= MAU_HUE_GAP)) out.push(h)
  return out
}

const MAU_GEN_LADDER = {
  light: { canvas: 0.976, side: 0.962, surface: 1, raised: 0.986, control: 1, line: 0.918, 'line-strong': 0.86, ink: 0.235, 'ink-muted': 0.44, 'ink-faint': 0.5, well: 0.995, 'well-ink': 0.235, 'well-dim': 0.5 },
  dark: { canvas: 0.235, side: 0.205, surface: 0.255, raised: 0.278, control: 0.31, line: 0.31, 'line-strong': 0.37, ink: 0.925, 'ink-muted': 0.77, 'ink-faint': 0.7, well: 0.175, 'well-ink': 0.925, 'well-dim': 0.66 },
}

function mauGenerate(scheme, seed) {
  const rnd = mauRng(seed)
  const hues = mauAllowedHues()
  for (let attempt = 0; attempt < 20; attempt++) {
    const h = hues[Math.floor(rnd() * hues.length)]
    const tint = rnd() * 0.006
    const ac = 0.08 + rnd() * 0.05
    const t = { ...MAU_BASE[scheme] }
    for (const [k, L] of Object.entries(MAU_GEN_LADDER[scheme])) {
      const isInk = k.includes('ink') || k === 'well-dim'
      t[k] = mauOklchToHex(L, isInk ? tint * 0.6 : tint, h)
    }
    let aL = scheme === 'light' ? 0.5 + rnd() * 0.06 : 0.72 + rnd() * 0.08
    for (let i = 0; i < 30; i++) {
      t.accent = mauOklchToHex(aL, ac, h)
      t['accent-ink'] = mauOklchToHex(scheme === 'light' ? aL - 0.05 : aL + 0.06, ac * 0.9, h)
      t['on-accent'] = scheme === 'light' ? '#ffffff' : mauOklchToHex(0.2, 0.03, h)
      t['well-accent'] = scheme === 'light' ? t['accent-ink'] : t['accent-ink']
      if (mauChecks(t).every((r) => r.ratio >= r.min)) return { id: 'gen-' + seed, name: 'Trộn', tag: 'hue ' + h, seed, hue: h, chroma: +ac.toFixed(3), tint: +tint.toFixed(4), [scheme]: t, tokens: t }
      aL += scheme === 'light' ? -0.015 : 0.015
    }
  }
  return { id: 'gen-fallback', name: 'Trộn', tag: 'Xanh', seed, hue: 255, tokens: MAU_PALETTES[0][scheme], [scheme]: MAU_PALETTES[0][scheme] }
}

const MAU_CURRENT = {
  next: [
    { id: 'mist', name: 'Sương kính', modes: ['#eef3f6', '#0f171d'], accent: '#2a7f9e', well: '#111a20', keep: true },
    { id: 'nebula', name: 'Tinh vân', modes: ['#f2effa', '#0d0a18'], accent: '#6a4fc9', well: '#15111f', keep: false },
    { id: 'aurora', name: 'Cực quang', modes: ['#edf6f2', '#0c1718'], accent: '#287e68', well: '#0e1a18', keep: false },
    { id: 'graphite', name: 'Than chì', modes: ['#e8ebef', '#111113'], accent: '#52677a', well: '#111113', keep: true },
    { id: 'dawn', name: 'Bình minh sương', modes: ['#f7eee9', '#211512'], accent: '#b4532f', well: '#1d1412', keep: false },
    { id: 'galaxy', name: 'Ngân hà', modes: ['#000000'], accent: '#e8835d', well: '#030304', keep: true, stars: true },
    { id: 'orrery', name: 'Orrery', modes: ['#f3f3f6', '#131315'], accent: '#bd5128', well: '#1a1a1c', keep: true },
    { id: 'console', name: 'Console', modes: ['#0c0d10'], accent: '#3dd6f5', well: '#06070a', keep: false },
    { id: 'custom', name: 'Tự chọn', modes: ['#f4f4f4', '#1c1c1c'], accent: '#888888', well: '#222222', keep: true, custom: true },
  ],
  classic: [
    { id: 'light/dark', name: 'Apple', modes: ['#ececee', '#1e1e1e'], accent: '#0071e3', well: '#111113', keep: false },
    { id: 'xcode', name: 'Xcode', modes: ['#ececec', '#18181c'], accent: '#0a63d8', well: '#ffffff', keep: false },
    { id: 'github', name: 'GitHub', modes: ['#f6f8fa', '#010409'], accent: '#0969da', well: '#0d1117', keep: false },
    { id: 'solarized', name: 'Solarized', modes: ['#eee8d5', '#00212b'], accent: '#1f6fa8', well: '#002b36', keep: false },
    { id: 'islands', name: 'Islands', modes: ['#e9eaee', '#26282c'], accent: '#3871e1', well: '#191a1c', keep: false },
    { id: 'monokai', name: 'Monokai', modes: ['#1e1f1c'], accent: '#66d9ef', well: '#272822', keep: false },
    { id: 'dracula', name: 'Dracula', modes: ['#1e1f29'], accent: '#bd93f9', well: '#21222c', keep: false },
    { id: 'material', name: 'Material', modes: ['#1e272c'], accent: '#80cbc4', well: '#263238', keep: false },
    { id: 'one-dark', name: 'One Dark', modes: ['#21252b'], accent: '#61afef', well: '#282c34', keep: false },
  ],
}

function mauHex(h) {
  const n = parseInt(h.slice(1), 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => v / 255)
}
function mauMix(a, b, p) {
  const x = mauHex(a)
  const y = mauHex(b)
  return '#' + x.map((v, i) => Math.round((v * p + y[i] * (1 - p)) * 255).toString(16).padStart(2, '0')).join('')
}
function mauLum(h) {
  const [r, g, b] = mauHex(h).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}
function mauRatio(a, b) {
  const [x, y] = [mauLum(a), mauLum(b)].sort((p, q) => q - p)
  return (x + 0.05) / (y + 0.05)
}

const MAU_TEXT = ['ink', 'ink-muted', 'ink-faint', 'accent-ink', 'working', 'waiting', 'idle', 'done', 'error', 'diff-add', 'diff-del']
const MAU_GLYPH = ['accent', 'agent-claude', 'agent-codex', 'agent-shell']
const MAU_WELL = ['well-ink', 'well-dim', 'well-accent', 'well-green', 'well-red', 'well-blue', 'well-yellow']
const MAU_SOFT = { working: 0.12, waiting: 0.14, error: 0.12, done: 0.14, 'diff-add': 0.14, 'diff-del': 0.12, 'accent-ink': 0.14 }

function mauChecks(t) {
  const rows = []
  for (const k of MAU_TEXT) for (const bg of ['canvas', 'side', 'surface', 'raised']) rows.push({ fg: k, bg, ratio: mauRatio(t[k], t[bg]), min: 4.5 })
  for (const [k, p] of Object.entries(MAU_SOFT)) {
    const src = k === 'accent-ink' ? t.accent : t[k]
    rows.push({ fg: k, bg: k + '-soft', ratio: mauRatio(t[k], mauMix(src, t.surface, p)), min: 4.5 })
  }
  rows.push({ fg: 'on-accent', bg: 'accent', ratio: mauRatio(t['on-accent'], t.accent), min: 4.5 })
  rows.push({ fg: 'ink', bg: 'control', ratio: mauRatio(t.ink, t.control), min: 4.5 })
  for (const k of MAU_GLYPH) rows.push({ fg: k, bg: 'surface', ratio: mauRatio(t[k], t.surface), min: 3 })
  for (const k of MAU_WELL) rows.push({ fg: k, bg: 'well', ratio: mauRatio(t[k], t.well), min: 4.5 })
  return rows
}

if (typeof module !== 'undefined') module.exports = { MAU_PALETTES, MAU_BASE, MAU_CURRENT, MAU_AVOID, MAU_HUE_GAP, mauAllowedHues, mauGenerate, mauChecks, mauRatio, mauMix }
