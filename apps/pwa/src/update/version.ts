export interface VersionInfo {
  web: string
  proto: number
  minProto: number
}

export type UpdateState = 'current' | 'newer' | 'protoOutdated'

export function parseVersion(raw: unknown): VersionInfo | null {
  if (!raw || typeof raw !== 'object') return null
  const { web, proto, minProto } = raw as Record<string, unknown>
  if (typeof web !== 'string' || !web) return null
  if (typeof proto !== 'number' || typeof minProto !== 'number') return null
  return { web, proto, minProto }
}

export async function fetchVersion(url: string, fetcher: typeof fetch = fetch): Promise<VersionInfo | null> {
  try {
    const response = await fetcher(url, { cache: 'no-store' })
    if (!response.ok) return null
    return parseVersion(await response.json())
  } catch {
    return null
  }
}

export function judgeVersion(baseline: string | null, remote: VersionInfo, ownProto: number): UpdateState {
  if (ownProto < remote.minProto || ownProto > remote.proto) return 'protoOutdated'
  if (baseline !== null && baseline !== remote.web) return 'newer'
  return 'current'
}

export class UpdateChecker {
  baseline: string | null = null
  private readonly url: string
  private readonly ownProto: number
  private readonly fetcher: typeof fetch

  constructor(url: string, ownProto: number, fetcher: typeof fetch = fetch) {
    this.url = url
    this.ownProto = ownProto
    this.fetcher = fetcher
  }

  async check(): Promise<{ state: UpdateState; info: VersionInfo } | null> {
    const info = await fetchVersion(this.url, this.fetcher)
    if (!info) return null
    this.baseline ??= info.web
    return { state: judgeVersion(this.baseline, info, this.ownProto), info }
  }
}
