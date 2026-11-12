const sheets = new Map<HTMLElement, CSSStyleSheet>()
let installed = false

function supported(): boolean {
  return typeof CSSStyleSheet !== 'undefined' && typeof CSSStyleSheet.prototype.replaceSync === 'function' && 'adoptedStyleSheets' in document
}

function sheetHost(make: (tag: string) => HTMLElement): HTMLElement {
  const host = make('div')
  host.hidden = true
  const sheet = new CSSStyleSheet()
  let text = ''
  Object.defineProperty(host, 'textContent', {
    get: () => text,
    set: (value: string | null) => {
      text = value ?? ''
      try {
        sheet.replaceSync(text)
      } catch {
        return
      }
    },
  })
  sheets.set(host, sheet)
  document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet]
  return host
}

export function pruneXtermStyles(): void {
  const gone: CSSStyleSheet[] = []
  for (const [host, sheet] of sheets) {
    if (host.isConnected) continue
    sheets.delete(host)
    gone.push(sheet)
  }
  if (gone.length) document.adoptedStyleSheets = document.adoptedStyleSheets.filter((s) => !gone.includes(s))
}

export function allowXtermStyles(): void {
  if (installed || !supported()) return
  installed = true
  const original = document.createElement.bind(document) as (tag: string, options?: ElementCreationOptions) => HTMLElement
  const patched = (tag: string, options?: ElementCreationOptions): HTMLElement => {
    if (tag.toLowerCase() !== 'style') return original(tag, options)
    pruneXtermStyles()
    return sheetHost(original)
  }
  document.createElement = patched as typeof document.createElement
}
