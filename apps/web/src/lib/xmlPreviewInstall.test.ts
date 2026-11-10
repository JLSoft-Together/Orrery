import { describe, expect, it } from 'vitest'
import type { LayoutInstallStatus } from './layoutPreview'
import { installKind, megabytes } from './xmlPreviewInstall'

const base: LayoutInstallStatus = { available: true, installed: false, version: null, pinnedVersion: '1', sizeBytes: 0, downloadBytes: 1, bundle: { state: 'missing' } }

describe('xml preview install', () => {
  it('derives one state from the status', () => {
    expect(installKind({ ...base, available: false })).toBe('unavailable')
    expect(installKind(base)).toBe('not-installed')
    expect(installKind({ ...base, installed: true, version: '1', bundle: { state: 'ready', dir: '/x' } })).toBe('installed')
    expect(installKind({ ...base, version: '3.5.0' })).toBe('update')
    expect(installKind({ ...base, bundle: { state: 'error', code: 'daemon-network' } })).toBe('error')
    expect(installKind({ ...base, bundle: { state: 'downloading', pct: 4 } })).toBe('downloading')
  })

  it('shows sizes in whole megabytes', () => {
    expect(megabytes(0)).toBe('<1')
    expect(megabytes(190 * 1024 * 1024 + 5)).toBe('190')
  })
})
