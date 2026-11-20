const MAU_PALETTES = [
  {
    id: 'fog',
    name: 'Sương xám',
    tag: 'Lạnh, trung tính',
    light: {
      canvas: '#eef1f4', side: '#e7ebef', surface: '#f9fafb', raised: '#f2f4f6', control: '#ffffff',
      line: '#dde2e7', 'line-strong': '#c9d0d7',
      ink: '#1f2933', 'ink-muted': '#4b5763', 'ink-faint': '#5c6874',
      accent: '#3d6f8e', 'accent-ink': '#335f7b', 'on-accent': '#ffffff',
      working: '#2f5f9e', waiting: '#7a5a10', idle: '#5c6874', done: '#2f6b4a', error: '#a63d40',
      'agent-claude': '#b8603f', 'agent-codex': '#6656b0', 'agent-shell': '#2f7a72',
      'diff-add': '#2f6b4a', 'diff-del': '#a63d40',
      well: '#1e252c', 'well-ink': '#dfe5ea', 'well-dim': '#8f9ca8', 'well-accent': '#9cc3d8',
      'well-green': '#8fcfa6', 'well-red': '#eba0a2', 'well-blue': '#9dbde3', 'well-yellow': '#e2c98a',
    },
    dark: {
      canvas: '#14181c', side: '#171c21', surface: '#1c2228', raised: '#222930', control: '#2a323a',
      line: '#2b333b', 'line-strong': '#3a444e',
      ink: '#e3e8ec', 'ink-muted': '#a9b4be', 'ink-faint': '#95a1ac',
      accent: '#7fa9c4', 'accent-ink': '#9cc0d6', 'on-accent': '#0f1c25',
      working: '#8fb2e6', waiting: '#e0c27a', idle: '#9ca8b3', done: '#8cc9a5', error: '#eb9fa1',
      'agent-claude': '#d4876a', 'agent-codex': '#a89ce6', 'agent-shell': '#6cc0b5',
      'diff-add': '#8cc9a5', 'diff-del': '#eb9fa1',
      well: '#101418', 'well-ink': '#dfe5ea', 'well-dim': '#8693a0', 'well-accent': '#9cc3d8',
      'well-green': '#8fcfa6', 'well-red': '#eba0a2', 'well-blue': '#9dbde3', 'well-yellow': '#e2c98a',
    },
  },
  {
    id: 'moss',
    name: 'Rêu',
    tag: 'Xanh lá dịu',
    light: {
      canvas: '#eff1ec', side: '#e8ebe4', surface: '#fafbf8', raised: '#f3f5f0', control: '#ffffff',
      line: '#dde1d7', 'line-strong': '#c8cec0',
      ink: '#20271f', 'ink-muted': '#4d574b', 'ink-faint': '#5c6759',
      accent: '#4b7556', 'accent-ink': '#3f6649', 'on-accent': '#ffffff',
      working: '#3d6491', waiting: '#7a5c12', idle: '#5c6759', done: '#2e6b3c', error: '#a8433c',
      'agent-claude': '#b5603e', 'agent-codex': '#6a5aa8', 'agent-shell': '#2f766c',
      'diff-add': '#2e6b3c', 'diff-del': '#a8433c',
      well: '#222821', 'well-ink': '#e1e6dc', 'well-dim': '#929d8f', 'well-accent': '#a8c8ae',
      'well-green': '#9fd0a6', 'well-red': '#eaa199', 'well-blue': '#a2bfe0', 'well-yellow': '#e0c98a',
    },
    dark: {
      canvas: '#151915', side: '#181c17', surface: '#1d221c', raised: '#232922', control: '#2b322a',
      line: '#2c332b', 'line-strong': '#3b4439',
      ink: '#e4e8df', 'ink-muted': '#aab3a5', 'ink-faint': '#97a191',
      accent: '#94b89c', 'accent-ink': '#a8c8ae', 'on-accent': '#13211a',
      working: '#93b3dc', waiting: '#dcc27f', idle: '#9ea799', done: '#9fd0a6', error: '#e9a098',
      'agent-claude': '#d38a6c', 'agent-codex': '#ab9fe0', 'agent-shell': '#79c2b4',
      'diff-add': '#9fd0a6', 'diff-del': '#e9a098',
      well: '#101310', 'well-ink': '#e1e6dc', 'well-dim': '#87927f', 'well-accent': '#a8c8ae',
      'well-green': '#9fd0a6', 'well-red': '#eaa199', 'well-blue': '#a2bfe0', 'well-yellow': '#e0c98a',
    },
  },
  {
    id: 'paper',
    name: 'Giấy',
    tag: 'Ấm, terminal sáng',
    light: {
      canvas: '#f3efe8', side: '#ede8e0', surface: '#fbf9f5', raised: '#f5f1eb', control: '#fffdfa',
      line: '#e2dcd2', 'line-strong': '#cfc7ba',
      ink: '#2a2420', 'ink-muted': '#5a5048', 'ink-faint': '#685e55',
      accent: '#4a6585', 'accent-ink': '#405872', 'on-accent': '#ffffff',
      working: '#3b5f8f', waiting: '#7a580c', idle: '#685e55', done: '#3d6a3a', error: '#a53b35',
      'agent-claude': '#b0573a', 'agent-codex': '#6a58a8', 'agent-shell': '#2e7268',
      'diff-add': '#3d6a3a', 'diff-del': '#a53b35',
      well: '#f0ebe3', 'well-ink': '#2a2420', 'well-dim': '#665c53', 'well-accent': '#405872',
      'well-green': '#38623a', 'well-red': '#9c3832', 'well-blue': '#3b5b88', 'well-yellow': '#6f520c',
    },
    dark: {
      canvas: '#1a1715', side: '#1d1a17', surface: '#221f1c', raised: '#292522', control: '#312c28',
      line: '#332e2a', 'line-strong': '#443d37',
      ink: '#ece6df', 'ink-muted': '#b5aca2', 'ink-faint': '#a0978d',
      accent: '#9db4cf', 'accent-ink': '#b0c4db', 'on-accent': '#16202c',
      working: '#9db4dc', waiting: '#dfc282', idle: '#a39a91', done: '#a3c99a', error: '#e89d95',
      'agent-claude': '#d98c6c', 'agent-codex': '#b1a3e0', 'agent-shell': '#7fc0b0',
      'diff-add': '#a3c99a', 'diff-del': '#e89d95',
      well: '#141210', 'well-ink': '#e6e0d8', 'well-dim': '#8d8379', 'well-accent': '#b0c4db',
      'well-green': '#a3c99a', 'well-red': '#e89d95', 'well-blue': '#a5bde0', 'well-yellow': '#dfc282',
    },
  },
  {
    id: 'indigo',
    name: 'Chàm',
    tag: 'Tím than nhạt',
    light: {
      canvas: '#f0f0f5', side: '#e9e9f0', surface: '#fafafd', raised: '#f3f3f8', control: '#ffffff',
      line: '#dfdfe8', 'line-strong': '#cacad7',
      ink: '#232336', 'ink-muted': '#4f4f66', 'ink-faint': '#5f5f77',
      accent: '#585ca4', 'accent-ink': '#4c5096', 'on-accent': '#ffffff',
      working: '#2a6585', waiting: '#7a5a14', idle: '#5f5f77', done: '#2f6b51', error: '#a5405a',
      'agent-claude': '#b65f42', 'agent-codex': '#7656b0', 'agent-shell': '#2b7774',
      'diff-add': '#2f6b51', 'diff-del': '#a5405a',
      well: '#1d1d2b', 'well-ink': '#e2e2ee', 'well-dim': '#9292ab', 'well-accent': '#b6b9ee',
      'well-green': '#92ceb0', 'well-red': '#eba3b4', 'well-blue': '#9ccbe0', 'well-yellow': '#e2c784',
    },
    dark: {
      canvas: '#15151d', side: '#181821', surface: '#1d1d27', raised: '#23232f', control: '#2b2b38',
      line: '#2c2c39', 'line-strong': '#3b3b4b',
      ink: '#e6e6f0', 'ink-muted': '#ababc2', 'ink-faint': '#9898b0',
      accent: '#a3a7e6', 'accent-ink': '#b6b9ee', 'on-accent': '#171a3a',
      working: '#8cc4dc', waiting: '#e2c784', idle: '#a0a0b6', done: '#92ceb0', error: '#eba0b2',
      'agent-claude': '#d98b6e', 'agent-codex': '#bea3ec', 'agent-shell': '#78c4bd',
      'diff-add': '#92ceb0', 'diff-del': '#eba0b2',
      well: '#101017', 'well-ink': '#e2e2ee', 'well-dim': '#8a8aa3', 'well-accent': '#b6b9ee',
      'well-green': '#92ceb0', 'well-red': '#eba3b4', 'well-blue': '#9ccbe0', 'well-yellow': '#e2c784',
    },
  },
  {
    id: 'lotus',
    name: 'Sen',
    tag: 'Hồng tro',
    light: {
      canvas: '#f2f0f1', side: '#ebe8ea', surface: '#fbfafa', raised: '#f4f2f3', control: '#ffffff',
      line: '#e1dcdf', 'line-strong': '#cdc6ca',
      ink: '#2a2328', 'ink-muted': '#574d54', 'ink-faint': '#665c63',
      accent: '#864d69', 'accent-ink': '#78435d', 'on-accent': '#ffffff',
      working: '#3b6090', waiting: '#7a5a12', idle: '#665c63', done: '#356a46', error: '#a53e3e',
      'agent-claude': '#b35e3f', 'agent-codex': '#6457ad', 'agent-shell': '#2e7670',
      'diff-add': '#356a46', 'diff-del': '#a53e3e',
      well: '#221d21', 'well-ink': '#ebe4e8', 'well-dim': '#9d9198', 'well-accent': '#e2b6cc',
      'well-green': '#98cca8', 'well-red': '#ec9f9b', 'well-blue': '#a0bce2', 'well-yellow': '#e0c482',
    },
    dark: {
      canvas: '#181517', side: '#1b181a', surface: '#211d20', raised: '#282326', control: '#302a2e',
      line: '#322c30', 'line-strong': '#433b40',
      ink: '#ede6ea', 'ink-muted': '#b6abb2', 'ink-faint': '#a1969d',
      accent: '#d6a3bd', 'accent-ink': '#e2b6cc', 'on-accent': '#2a1420',
      working: '#98b6e0', waiting: '#e0c482', idle: '#a59aa1', done: '#98cca8', error: '#ec9f9b',
      'agent-claude': '#d98b6d', 'agent-codex': '#ada2e6', 'agent-shell': '#7bc1b6',
      'diff-add': '#98cca8', 'diff-del': '#ec9f9b',
      well: '#121012', 'well-ink': '#ebe4e8', 'well-dim': '#928790', 'well-accent': '#e2b6cc',
      'well-green': '#98cca8', 'well-red': '#ec9f9b', 'well-blue': '#a0bce2', 'well-yellow': '#e0c482',
    },
  },
]

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

if (typeof module !== 'undefined') module.exports = { MAU_PALETTES, MAU_CURRENT, mauChecks, mauRatio, mauMix }
