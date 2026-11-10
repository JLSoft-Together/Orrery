export type El = { name: string; attrs: [string, { str: string } | { int: number } | { bool: boolean }][]; children?: El[] }
export function axml(root: El, ids: number[], strings: string[]): Buffer {
  const idx = (s: string): number => {
    let i = strings.indexOf(s)
    if (i < 0) i = strings.push(s) - 1
    return i
  }
  const body: Buffer[] = []
  const walk = (el: El): void => {
    const attrs = el.attrs.map(([name, v]) => {
      const a = Buffer.alloc(20)
      a.writeUInt32LE(0xffffffff, 0)
      a.writeUInt32LE(idx(name), 4)
      a.writeUInt16LE(8, 12)
      if ('str' in v) {
        a.writeUInt32LE(idx(v.str), 8)
        a[15] = 0x03
        a.writeUInt32LE(idx(v.str), 16)
      } else {
        a.writeUInt32LE(0xffffffff, 8)
        a[15] = 'int' in v ? 0x10 : 0x12
        a.writeUInt32LE('int' in v ? v.int : v.bool ? 0xffffffff : 0, 16)
      }
      return a
    })
    const start = Buffer.alloc(36)
    start.writeUInt16LE(0x0102, 0)
    start.writeUInt16LE(16, 2)
    start.writeUInt32LE(36 + 20 * attrs.length, 4)
    start.writeUInt32LE(0xffffffff, 12)
    start.writeUInt32LE(0xffffffff, 16)
    start.writeUInt32LE(idx(el.name), 20)
    start.writeUInt16LE(20, 24)
    start.writeUInt16LE(20, 26)
    start.writeUInt16LE(attrs.length, 28)
    body.push(start, ...attrs)
    for (const c of el.children ?? []) walk(c)
    const end = Buffer.alloc(24)
    end.writeUInt16LE(0x0103, 0)
    end.writeUInt16LE(16, 2)
    end.writeUInt32LE(24, 4)
    end.writeUInt32LE(0xffffffff, 16)
    end.writeUInt32LE(idx(el.name), 20)
    body.push(end)
  }
  walk(root)
  const strData = Buffer.concat(strings.map((s) => {
    const b = Buffer.alloc(2 + s.length * 2 + 2)
    b.writeUInt16LE(s.length, 0)
    b.write(s, 2, 'utf16le')
    return b
  }))
  const padded = Buffer.concat([strData, Buffer.alloc((4 - (strData.length % 4)) % 4)])
  const offsets = Buffer.alloc(strings.length * 4)
  let off = 0
  strings.forEach((s, i) => {
    offsets.writeUInt32LE(off, i * 4)
    off += 4 + s.length * 2
  })
  const poolHead = Buffer.alloc(28)
  poolHead.writeUInt16LE(0x0001, 0)
  poolHead.writeUInt16LE(28, 2)
  poolHead.writeUInt32LE(28 + offsets.length + padded.length, 4)
  poolHead.writeUInt32LE(strings.length, 8)
  poolHead.writeUInt32LE(28 + offsets.length, 20)
  const map = Buffer.alloc(8 + ids.length * 4)
  map.writeUInt16LE(0x0180, 0)
  map.writeUInt16LE(8, 2)
  map.writeUInt32LE(map.length, 4)
  ids.forEach((id, i) => map.writeUInt32LE(id, 8 + i * 4))
  const rest = Buffer.concat([poolHead, offsets, padded, map, ...body])
  const head = Buffer.alloc(8)
  head.writeUInt16LE(0x0003, 0)
  head.writeUInt16LE(8, 2)
  head.writeUInt32LE(8 + rest.length, 4)
  return Buffer.concat([head, rest])
}

export const MANIFEST: El = {
  name: 'manifest',
  attrs: [
    ['', { int: 42 }],
    ['versionName', { str: '1.2.3' }],
    ['package', { str: 'com.example.app' }],
  ],
  children: [
    { name: 'uses-sdk', attrs: [['minSdkVersion', { int: 24 }], ['targetSdkVersion', { int: 35 }]] },
    { name: 'uses-permission', attrs: [['name', { str: 'android.permission.INTERNET' }]] },
    { name: 'uses-permission', attrs: [['name', { str: 'android.permission.CAMERA' }]] },
    { name: 'application', attrs: [['debuggable', { bool: true }]] },
  ],
}
export const apkManifest = () => axml(MANIFEST, [0x0101021b], [''])

export function pb(no: number, payload: Buffer | string): Buffer {
  const data = typeof payload === 'string' ? Buffer.from(payload) : payload
  const varint = (n: number): Buffer => {
    const out: number[] = []
    while (n > 0x7f) {
      out.push((n & 0x7f) | 0x80)
      n >>>= 7
    }
    out.push(n)
    return Buffer.from(out)
  }
  return Buffer.concat([varint((no << 3) | 2), varint(data.length), data])
}
export const pbAttr = (name: string, value: string) => pb(4, Buffer.concat([pb(2, name), pb(3, value)]))
export const pbElement = (name: string, attrs: Buffer[], children: Buffer[] = []) => Buffer.concat([pb(3, name), ...attrs, ...children.map((c) => pb(5, pb(1, c)))])
export const aabManifest = () =>
  pb(1, pbElement('manifest', [pbAttr('versionCode', '7'), pbAttr('versionName', '2.0'), pbAttr('package', 'com.x')], [pbElement('uses-sdk', [pbAttr('minSdkVersion', '26'), pbAttr('targetSdkVersion', '35')])]))

export function dex(methods: number): Buffer {
  const b = Buffer.alloc(0x200)
  b.write('dex\n035\0', 0, 'latin1')
  b.writeUInt32LE(methods, 0x58)
  return b
}

