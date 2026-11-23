export type ConsoleLevel = 'warning' | 'error'

export type HandoffAction = 'screenshot' | 'console' | 'pick'

export interface ConsoleEntry {
  level: ConsoleLevel
  message: string
  source?: string
  line?: number
  at: number
}

export interface ElementPick {
  selector: string
  tag: string
  classes: string[]
  text: string
  outerHTML: string
  url: string
  rect?: PickRect
  source?: string
  css?: Record<string, string>
}

export interface PickRect {
  x: number
  y: number
  width: number
  height: number
  viewportWidth: number
  viewportHeight: number
}

export const CSS_KEYS = ['color', 'background-color', 'font-size', 'font-weight', 'line-height', 'padding', 'margin', 'border-radius', 'display', 'width', 'height'] as const
const CSS_VALUE_MAX = 80
const SOURCE_MAX = 300
const CSS_VALUE = /^[A-Za-z0-9\s.,()%#/+-]+$/
const SOURCE_PATH = /^[A-Za-z0-9_@~ .,()[\]+\-/\\:]+$/

export const CONSOLE_KEEP = 50
const MESSAGE_MAX = 300
const TEXT_MAX = 200
const HTML_MAX = 600

const SECRET = /\b(?:sk-[A-Za-z0-9_-]{8,}|gh[pousr]_[A-Za-z0-9]{16,}|github_pat_[A-Za-z0-9_]{16,}|xox[abprs]-[A-Za-z0-9-]{8,}|AKIA[0-9A-Z]{12,}|eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,})/g
const SECRET_PARAM = /([?&](?:token|access_token|id_token|key|api_key|apikey|secret|password|passwd|auth|sig|signature|code)=)[^&#\s"']+/gi
const OPAQUE = /\b[A-Za-z0-9_-]{40,}\b/g

export const truncate = (s: string, max: number): string => (s.length > max ? `${s.slice(0, max - 1)}…` : s)

/** Control chars out, whitespace collapsed, obvious secrets / tokens masked. */
export function sanitize(text: string, max: number): string {
  const clean = String(text)
    .replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '')
    .replace(/[\x00-\x08\x0b-\x1f\x7f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(SECRET, '[redacted]')
    .replace(SECRET_PARAM, '$1[redacted]')
    .replace(OPAQUE, '[redacted]')
  return truncate(clean, max)
}

/** Electron's `console-message`: numeric level (0 verbose … 3 error) in older builds, a string in newer. */
export function consoleLevel(level: unknown): ConsoleLevel | null {
  if (level === 3 || level === 'error') return 'error'
  if (level === 2 || level === 'warning' || level === 'warn') return 'warning'
  return null
}

/** Appends to a ring of the last `CONSOLE_KEEP`; repeats of the newest entry are folded. */
export function pushConsole(list: ConsoleEntry[], entry: ConsoleEntry): ConsoleEntry[] {
  const message = sanitize(entry.message, MESSAGE_MAX)
  if (!message) return list
  const last = list[list.length - 1]
  if (last && last.message === message && last.level === entry.level && last.source === entry.source && last.line === entry.line) return list
  return [...list, { ...entry, message }].slice(-CONSOLE_KEEP)
}

function shortSource(source: string | undefined, page: string): string {
  if (!source) return ''
  try {
    const u = new URL(source)
    const p = new URL(page)
    return u.origin === p.origin ? u.pathname : `${u.host}${u.pathname}`
  } catch {
    return sanitize(source, 120)
  }
}

const pageLabel = (url: string): string => {
  try {
    const u = new URL(url)
    return `${u.origin}${u.pathname}`
  } catch {
    return sanitize(url, 200)
  }
}

/** Compact block pasted into the agent's input (no trailing newline: nothing gets submitted). */
export function formatConsoleBlock(entries: readonly ConsoleEntry[], pageUrl: string): string {
  const errors = entries.filter((e) => e.level === 'error').length
  const warnings = entries.length - errors
  const counts = [errors && `${errors} error${errors === 1 ? '' : 's'}`, warnings && `${warnings} warning${warnings === 1 ? '' : 's'}`].filter(Boolean).join(', ')
  const lines = entries.map((e) => {
    const src = shortSource(e.source, pageUrl)
    return `- [${e.level}] ${e.message}${src ? ` (${src}${e.line ? `:${e.line}` : ''})` : ''}`
  })
  return [`Browser console on ${pageLabel(pageUrl)} - ${counts}:`, ...lines].join('\n')
}

export function formatElementBlock(pick: ElementPick): string {
  const tag = [pick.tag, ...pick.classes.slice(0, 6)].join('.')
  const out = [`Element on ${pageLabel(pick.url)}:`, `selector: ${sanitize(pick.selector, 300)}`, `tag: ${sanitize(tag, 200)}`]
  const text = sanitize(pick.text, TEXT_MAX)
  if (text) out.push(`text: "${text}"`)
  if (pick.source) out.push(`source: ${sanitize(pick.source, SOURCE_MAX)}`)
  const css = pick.css ? CSS_KEYS.filter((k) => pick.css![k]).map((k) => `${k}: ${pick.css![k]}`) : []
  if (css.length) out.push(`css: ${css.join('; ')}`)
  out.push(`html: ${sanitize(pick.outerHTML, HTML_MAX)}`)
  return out.join('\n')
}

function asRect(value: unknown): PickRect | undefined {
  if (!value || typeof value !== 'object') return undefined
  const v = value as Record<string, unknown>
  const num = (x: unknown): number | null => (typeof x === 'number' && Number.isFinite(x) && Math.abs(x) <= 1e6 ? x : null)
  const [x, y, width, height, viewportWidth, viewportHeight] = [v.x, v.y, v.width, v.height, v.viewportWidth, v.viewportHeight].map(num)
  if (x === null || y === null || width === null || height === null || viewportWidth === null || viewportHeight === null) return undefined
  if (width! <= 0 || height! <= 0 || viewportWidth! <= 0 || viewportHeight! <= 0) return undefined
  return { x: x!, y: y!, width: width!, height: height!, viewportWidth: viewportWidth!, viewportHeight: viewportHeight! }
}

function asSource(value: unknown): string | undefined {
  if (typeof value !== 'string' || !value || value.length > SOURCE_MAX || !SOURCE_PATH.test(value)) return undefined
  return value.trim() || undefined
}

function asCss(value: unknown): Record<string, string> | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined
  const v = value as Record<string, unknown>
  const out: Record<string, string> = {}
  for (const key of CSS_KEYS) {
    const raw = v[key]
    if (typeof raw !== 'string') continue
    const text = raw.replace(/\s+/g, ' ').trim()
    if (text && text.length <= CSS_VALUE_MAX && CSS_VALUE.test(text)) out[key] = text
  }
  return Object.keys(out).length ? out : undefined
}

export function relativeSource(source: string, root: string | undefined): string {
  const norm = (p: string): string => p.replace(/\\/g, '/')
  const file = norm(source).replace(/^\/@fs\//, '')
  if (!root) return file
  const base = norm(root).replace(/\/+$/, '')
  if (base && file.toLowerCase().startsWith(`${base.toLowerCase()}/`)) return file.slice(base.length + 1)
  return file
}

/** Validates what the guest page returned: it is untrusted input. */
export function asElementPick(value: unknown, pageUrl: string): ElementPick | null {
  if (!value || typeof value !== 'object') return null
  const v = value as Record<string, unknown>
  const str = (x: unknown, max: number): string => (typeof x === 'string' ? x.slice(0, max) : '')
  const selector = str(v.selector, 1000)
  const tag = str(v.tag, 64).toLowerCase()
  if (!selector || !/^[a-z][a-z0-9-]*$/.test(tag)) return null
  const classes = Array.isArray(v.classes) ? v.classes.filter((c): c is string => typeof c === 'string').map((c) => c.slice(0, 80)).slice(0, 12) : []
  const pick: ElementPick = { selector, tag, classes, text: str(v.text, 2000), outerHTML: str(v.outerHTML, 4000), url: pageUrl }
  const rect = asRect(v.rect)
  const source = asSource(v.source)
  const css = asCss(v.css)
  if (rect) pick.rect = rect
  if (source) pick.source = source
  if (css) pick.css = css
  return pick
}

const SAFE_COLOR = /^(?:#[0-9a-f]{3,8}|rgba?\([\d\s.,/%]+\))$/i

/**
 * Runs inside the guest page via `executeJavaScript`; resolves with the clicked element's facts or
 * null on Esc / cancel. Overlay lives in the guest only and is removed on exit.
 */
export function pickerScript(color: string): string {
  const c = JSON.stringify(SAFE_COLOR.test(color.trim()) ? color.trim() : 'Highlight')
  return `(() => new Promise((resolve) => {
  if (window.__ccmPickCancel) window.__ccmPickCancel();
  const box = document.createElement('div');
  box.style.cssText = 'position:fixed;z-index:2147483647;pointer-events:none;border:2px solid ' + ${c} + ';border-radius:3px;background:transparent;box-shadow:0 0 0 9999px rgba(0,0,0,.08);transition:all 60ms linear;display:none';
  const tip = document.createElement('div');
  tip.style.cssText = 'position:fixed;z-index:2147483647;pointer-events:none;font:12px/1.4 ui-monospace,Menlo,Consolas,monospace;padding:2px 6px;border-radius:4px;color:white;background:' + ${c} + ';display:none;max-width:60vw;overflow:hidden;white-space:nowrap;text-overflow:ellipsis';
  document.documentElement.append(box, tip);
  const prevCursor = document.documentElement.style.cursor;
  document.documentElement.style.cursor = 'crosshair';
  let current = null;
  const esc = (s) => (window.CSS && CSS.escape ? CSS.escape(s) : String(s).replace(/[^a-zA-Z0-9_-]/g, '\\\\$&'));
  const part = (el) => {
    if (el.id && document.querySelectorAll('#' + esc(el.id)).length === 1) return '#' + esc(el.id);
    let s = el.tagName.toLowerCase();
    const cls = [...el.classList].filter((k) => !/^(?:is-|has-|hover|active|focus)/.test(k)).slice(0, 2);
    if (cls.length) s += '.' + cls.map(esc).join('.');
    const parent = el.parentElement;
    if (parent) {
      const same = [...parent.children].filter((x) => x.tagName === el.tagName);
      if (same.length > 1) s += ':nth-of-type(' + (same.indexOf(el) + 1) + ')';
    }
    return s;
  };
  const selectorOf = (el) => {
    const parts = [];
    for (let n = el; n && n.nodeType === 1 && n !== document.documentElement && parts.length < 6; n = n.parentElement) {
      const p = part(n);
      parts.unshift(p);
      if (p.startsWith('#')) break;
      try { if (document.querySelectorAll(parts.join(' > ')).length === 1) break; } catch (e) { break; }
    }
    return parts.join(' > ');
  };
  const sourceOf = (el) => {
    try {
      let n = el;
      for (let i = 0; n && i < 6; i++, n = n.parentElement) {
        const vc = n.__vueParentComponent;
        const file = vc && vc.type && vc.type.__file;
        if (typeof file === 'string' && file) return file;
        const fk = Object.keys(n).find((k) => k.startsWith('__reactFiber$'));
        let fiber = fk ? n[fk] : null;
        for (let j = 0; fiber && j < 6; j++, fiber = fiber.return) {
          const d = fiber._debugSource;
          if (d && typeof d.fileName === 'string' && d.fileName) return d.fileName + (d.lineNumber ? ':' + d.lineNumber : '');
        }
      }
    } catch (e) {}
    return '';
  };
  const cssOf = (el, r) => {
    const keys = ${JSON.stringify(CSS_KEYS)};
    const out = {};
    let base = null;
    let frame = null;
    try {
      frame = document.createElement('iframe');
      frame.style.cssText = 'position:fixed;left:-9999px;width:0;height:0;border:0;visibility:hidden';
      document.documentElement.append(frame);
      const d = frame.contentDocument;
      const probe = d.createElement(el.tagName.toLowerCase());
      d.body.append(probe);
      base = frame.contentWindow.getComputedStyle(probe);
    } catch (e) { base = null; }
    const cs = getComputedStyle(el);
    for (const k of keys) {
      if (k === 'width' || k === 'height') { out[k] = Math.round(k === 'width' ? r.width : r.height) + 'px'; continue; }
      const v = cs.getPropertyValue(k).trim();
      if (!v || v === 'normal' || v === 'none' || v === '0px' || v === 'rgba(0, 0, 0, 0)') continue;
      if (base && base.getPropertyValue(k).trim() === v) continue;
      out[k] = v.slice(0, 80);
    }
    if (frame) frame.remove();
    return out;
  };
  const show = (el) => {
    const r = el.getBoundingClientRect();
    box.style.display = tip.style.display = 'block';
    box.style.left = r.left + 'px'; box.style.top = r.top + 'px';
    box.style.width = r.width + 'px'; box.style.height = r.height + 'px';
    tip.textContent = part(el) + '  ' + Math.round(r.width) + '×' + Math.round(r.height);
    tip.style.left = Math.max(0, r.left) + 'px';
    tip.style.top = (r.top > 24 ? r.top - 22 : r.bottom + 4) + 'px';
  };
  const move = (e) => {
    const el = document.elementFromPoint(e.clientX, e.clientY);
    if (el && el !== box && el !== tip && el !== current) { current = el; show(el); }
  };
  const block = (e) => { e.preventDefault(); e.stopPropagation(); e.stopImmediatePropagation(); };
  const done = (value) => {
    removeEventListener('mousemove', move, true);
    removeEventListener('click', click, true);
    removeEventListener('mousedown', block, true);
    removeEventListener('mouseup', block, true);
    removeEventListener('keydown', key, true);
    box.remove(); tip.remove();
    document.documentElement.style.cursor = prevCursor;
    window.__ccmPickCancel = null;
    resolve(value);
  };
  const click = (e) => {
    block(e);
    const el = current || document.elementFromPoint(e.clientX, e.clientY);
    if (!el) return done(null);
    const r = el.getBoundingClientRect();
    done({
      selector: selectorOf(el),
      tag: el.tagName.toLowerCase(),
      classes: [...el.classList].slice(0, 12),
      text: (el.innerText || el.textContent || '').replace(/\\s+/g, ' ').trim().slice(0, 400),
      outerHTML: el.outerHTML.slice(0, 1200),
      rect: { x: r.left, y: r.top, width: r.width, height: r.height, viewportWidth: window.innerWidth, viewportHeight: window.innerHeight },
      source: sourceOf(el),
      css: cssOf(el, r),
    });
  };
  const key = (e) => { if (e.key === 'Escape') { block(e); done(null); } };
  addEventListener('mousemove', move, true);
  addEventListener('click', click, true);
  addEventListener('mousedown', block, true);
  addEventListener('mouseup', block, true);
  addEventListener('keydown', key, true);
  window.__ccmPickCancel = () => done(null);
}))()`
}

export const PICKER_CANCEL_SCRIPT = 'window.__ccmPickCancel && window.__ccmPickCancel()'
