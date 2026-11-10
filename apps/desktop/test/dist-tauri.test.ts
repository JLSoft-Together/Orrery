import { describe, expect, it } from 'vitest'
import { artifactName, jobs, pickOs, sidecarName, TARGETS } from '../../../scripts/dist-tauri.mjs'
import { pickAsset } from '../src/update/pick'

describe('dist-tauri', () => {
  it('names the bundles the way Electron 3.4 updaters look for them', () => {
    const version = '3.5.0'
    const names = [...TARGETS.win, ...TARGETS.mac].map((s) => artifactName(version, s))
    expect(names).toEqual(['Orrery-Setup-3.5.0.exe', 'Orrery-3.5.0-arm64.dmg', 'Orrery-3.5.0-x64.dmg'])
    const assets = names.map((name) => ({ name, size: 1, url: `https://github.com/x/${name}`, sha256: null }))
    expect(pickAsset(assets, version, 'nsis', 'x64')?.name).toBe('Orrery-Setup-3.5.0.exe')
    expect(pickAsset(assets, version, 'mac', 'arm64')?.name).toBe('Orrery-3.5.0-arm64.dmg')
    expect(pickAsset(assets, version, 'mac', 'x64')?.name).toBe('Orrery-3.5.0-x64.dmg')
    expect(pickAsset(assets, version, 'portable', 'x64')).toBeNull()
  })

  it('stages sidecars with the target triple Tauri expects', () => {
    expect(sidecarName('orrery-core', 'x86_64-pc-windows-msvc')).toBe('orrery-core-x86_64-pc-windows-msvc.exe')
    expect(sidecarName('orrery-bridge', 'aarch64-apple-darwin')).toBe('orrery-bridge-aarch64-apple-darwin')
  })

  it('picks the OS from flags first, then the host', () => {
    expect(pickOs(['--win'], 'darwin')).toBe('win')
    expect(pickOs([], 'darwin')).toBe('mac')
    expect(pickOs([], 'win32')).toBe('win')
    expect(pickOs([], 'linux')).toBeNull()
  })

  it('keeps cargo jobs within the machine budget', () => {
    expect(jobs({ JIN_MAX_THREADS: '4' }, 16)).toBe('4')
    expect(jobs({}, 2)).toBe('2')
    expect(jobs({ JIN_MAX_THREADS: 'x' }, 32)).toBe('6')
  })
})
