import { computed } from 'vue'
import { hasAgentData } from '../lib/agentData'
import { hasBackup } from '../lib/backup'
import { hasDisk } from '@features/disk'
import { hasAwake, hasCapture } from '../lib/capture'
import { desktop, hasDeepSeek, hasRemote, hasRun, hasTerminals } from '../../../core/lib/desktop'
import { hasGitHub } from '../lib/github'
import { hasXmlInstall } from '@features/run'
import { hasPhone } from '@features/remote'
import { hasTelegram } from '@features/telegram'
import { isMac } from '../../../core/lib/platform'
import { hasGrants } from '../../../core/lib/preview'
import { hasUpdater } from '@features/update'
import { useExtrasStore } from '../../../core/stores/extras'
import type { SettingsEnv } from '../components'

/** Which settings rows this build shows; the same flags SettingsMenu uses, so a palette hit never points at a hidden row. */
export function useSettingsEnv() {
  const extras = useExtrasStore()
  return computed<SettingsEnv>(() => ({
    terminals: hasTerminals,
    mac: isMac,
    loginItem: Boolean(desktop?.loginItem),
    zoom: Boolean(desktop?.zoomApp),
    awake: hasAwake,
    github: hasGitHub,
    run: hasRun,
    capture: hasCapture,
    agentData: hasAgentData,
    phone: hasPhone,
    telegram: hasTelegram,
    remote: hasRemote,
    disk: hasDisk,
    updater: hasUpdater,
    backup: hasBackup,
    grants: hasGrants,
    statusLine: Boolean(extras.statusLineCommand),
    toast: extras.toast === 'on' || extras.toast === 'off',
    notifyData: Boolean(extras.notify),
    deepseek: hasDeepSeek,
    xmlPreview: hasXmlInstall,
    bugReport: Boolean(desktop?.bugReportSave),
    desktop: Boolean(desktop),
  }))
}
