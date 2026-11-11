import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { now, relativeTime } from '../lib/format'
import { useConnectionStore } from '../stores/connection'
import { useExtrasStore } from '../stores/extras'
import { useTerminalsStore } from '../stores/terminals'

export type DiagLevel = 'ok' | 'warn' | 'off'

export function useDiagnosticsReport() {
  const { t, locale } = useI18n()
  const extras = useExtrasStore()
  const terminals = useTerminalsStore()
  const connection = useConnectionStore()

  const ago = (iso: string | undefined) => relativeTime(iso, locale.value, now.value) ?? '—'

  const rows = computed(() => {
    const d = extras.diagnostics
    if (!d) return []
    const versions = [...new Set(terminals.live.map((x) => x.claudeVersion).filter((v): v is string => !!v))]
    const supported = connection.supportedClaudeVersion
    const mismatch = supported ? versions.filter((v) => v !== supported) : []
    const usage = d.usage
    const codex = d.codex
    return [
      { key: 'collector', level: 'ok' as DiagLevel, value: t('diag.collectorValue', { ago: ago(d.startedAt), clients: d.clients }), detail: `${d.platform}, Node ${d.node}` },
      {
        key: 'claudeRoot',
        level: (d.claudeRootFound && d.watchers.sessions && d.watchers.projects ? 'ok' : 'warn') as DiagLevel,
        value: d.claudeRootFound ? t('diag.watching', { list: [d.watchers.sessions && 'sessions', d.watchers.projects && 'projects'].filter(Boolean).join(', ') || '—' }) : t('diag.notFound'),
        detail: d.claudeRoot,
      },
      {
        key: 'sessions',
        level: 'ok' as DiagLevel,
        value: t('diag.sessionsValue', { live: d.sessions.live, ended: d.sessions.ended }),
        detail: d.verifyProcesses ? t('diag.verifyOn') : t('diag.verifyOff'),
      },
      {
        key: 'version',
        level: (mismatch.length ? 'warn' : versions.length ? 'ok' : 'off') as DiagLevel,
        value: versions.length ? versions.join(', ') : t('diag.noSession'),
        detail: supported ? t('diag.supported', { v: supported }) : '',
      },
      {
        key: 'statusLine',
        level: (d.statusLine.lastAt ? 'ok' : extras.statusLineCommand ? 'warn' : 'off') as DiagLevel,
        value: d.statusLine.lastAt ? t('diag.statusLineValue', { ago: ago(d.statusLine.lastAt), n: d.statusLine.reports }) : t('diag.statusLineNone'),
        detail: d.statusLine.lastAt ? '' : t('diag.statusLineHint'),
      },
      {
        key: 'usage',
        level: (usage?.state === 'ready' ? 'ok' : usage?.state === 'scanning' ? 'warn' : 'off') as DiagLevel,
        value: usage ? t(`diag.usage.${usage.state}`, { done: usage.filesDone, total: usage.filesTotal }) : '—',
        detail: '',
      },
      ...(codex
        ? [
            {
              key: 'codex',
              level: (codex.watching ? 'ok' : 'off') as DiagLevel,
              value: codex.watching ? t('diag.codexValue', { threads: codex.threads, processes: codex.processes }) : t('diag.notFound'),
              detail: t('diag.codexDetail', { home: codex.home }),
            },
          ]
        : []),
      { key: 'dataDir', level: 'ok' as DiagLevel, value: d.dataDir, detail: t('diag.dataDirHint') },
    ]
  })

  function report(): string {
    return rows.value.map((r) => `${t(`diag.row.${r.key}`)}: ${r.value}${r.detail ? ` (${r.detail})` : ''}`).join('\n')
  }

  return { rows, report }
}
