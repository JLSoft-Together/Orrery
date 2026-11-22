import type { DrawableKind } from '@ccm/shared'

export type BundleStatus = { state: 'missing' } | { state: 'downloading'; pct: number } | { state: 'ready'; dir: string } | { state: 'error'; code: string }
export type JdkStatus = { ok: true; home: string; version: number } | { ok: false; code: 'jdk-missing' | 'jdk-old'; detail?: string }
export type DaemonState = 'stopped' | 'starting' | 'ready' | 'failed'

export interface LayoutInstallStatus {
  available: boolean
  installed: boolean
  version: string | null
  pinnedVersion: string | null
  sizeBytes: number
  downloadBytes: number
  bundle: BundleStatus
}

export interface LayoutUninstallResult {
  ok: boolean
  code?: string
  status: LayoutInstallStatus
}

export interface LayoutStatus {
  bundle: BundleStatus
  jdk: JdkStatus
  daemon: DaemonState
  jar: boolean
  /** API level layoutlib renders with; the bundled version supports exactly one. */
  apiLevel?: number
  /** Cold-start stage while the daemon is starting (v46). */
  stage?: InitStage | null
}

export interface LayoutDevice {
  width: number
  height: number
  density: number
}

export interface LayoutViewNode {
  class: string
  xmlId: string | null
  line: number
  column: number
  bounds: [number, number, number, number]
  children: LayoutViewNode[]
}

export interface LayoutDiagnostic {
  severity: 'error' | 'warning' | 'info'
  message: string
  line?: number
  column?: number
}

export type LayoutRenderResult =
  | {
      ok: true
      pngUrl: string | null
      width: number
      height: number
      viewTree: LayoutViewNode | null
      diagnostics: LayoutDiagnostic[]
      renderMs: number
      frameworkOnly?: boolean
      /** No project model, the bundled ConstraintLayout / AppCompat / Material kit rendered it. */
      builtinLibs?: boolean
      /** No Gradle model: project res read from res/ folders and cached library AARs, no project classes. */
      sourceOnly?: boolean
      /** No Gradle model: classes from the project's last build (mtime of its R.jar). */
      prebuiltAt?: number
      cached?: boolean
    }
  | { ok: false; code: string; detail?: string }

export type ModelState =
  | { state: 'none' }
  | { state: 'running'; since: number; compile?: boolean }
  | { state: 'ready'; at: number; resCount: number; classpathCount: number; warnings: number }
  | { state: 'error'; code: string; detail?: string; at: number }

export interface LayoutModelInfo {
  target: { root: string; module: string; name: string } | null
  /** Layout of a library module: `target.module` is the app whose model is borrowed; `linked` false = no app depends on it. */
  library?: { module: string; candidates: string[]; linked: boolean }
  reason?: 'no-gradle' | 'no-app'
  state: ModelState
  hasModel: boolean
  themes: string[]
  /** .kt / .java changed after the model was built. */
  codeChanged?: boolean
}

export interface LayoutRenderRequest {
  file: string
  device: LayoutDevice
  theme: string | null
  night: boolean
  locale: string | null
  /** 0 = main view, 1..3 = compare cells; renders of different slots do not cancel each other. */
  slot?: number
  /** v42: unsaved editor text rendered instead of the file (resources still resolve from `file`). */
  text?: string
  /** v46: image px per device px (`renderScale`); geometry in the result stays in device px. */
  scale?: number
}

export type LayoutEvent =
  | { type: 'bundle'; status: BundleStatus }
  | { type: 'daemon'; state: DaemonState; detail?: string }
  | { type: 'model'; root: string; module: string; state: ModelState }
  | { type: 'project-changed'; root: string; module: string }
  | { type: 'code-changed'; root: string; module: string }
  | { type: 'daemon-stage'; stage: InitStage }
  | { type: 'loading'; kind: 'kit' | 'project'; active: boolean }

export type InitStage = 'jvm' | 'setup' | 'prepare' | 'warmup'

export interface DrawableSwatch {
  state: string
  /** `#AARRGGBB` */
  color: string
}

export type DrawableRenderResult =
  | {
      ok: true
      kind: DrawableKind
      pngUrl: string | null
      width: number
      height: number
      swatches: DrawableSwatch[] | null
      diagnostics: LayoutDiagnostic[]
      renderMs: number
      frameworkOnly: boolean
      sourceOnly?: boolean
    }
  | { ok: false; code: string; detail?: string }

export interface DrawableRenderRequest {
  file: string
  /** dp */
  width: number
  height: number
  density: number
  night: boolean
  states: string[]
  /** v42: unsaved editor text rendered instead of the file. */
  text?: string
}

export interface DirEntry {
  /** Leaf name as the filesystem reports it. */
  name: string
  /** Absolute path, already canonicalised by main. */
  path: string
  directory: boolean
  /** Symlink resolving outside the project: a leaf that never opens. */
  outOfRoot: boolean
}
