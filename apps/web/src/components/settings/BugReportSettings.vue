<script setup lang="ts">
import { computed, onMounted, ref, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bug, CircleAlert, FileText, HardDrive, LoaderCircle, Save } from 'lucide-vue-next'
import { useDiagnosticsReport } from '../../composables/useDiagnosticsReport'
import { formatBytes, groupItems, overLimit, totalBytes } from '../../lib/bugReport'
import { desktop, type BugReportItem, type BugReportPreview } from '../../lib/desktop'
import { errorLogCount, errorLogText } from '../../lib/errorLog'
import { useConnectionStore } from '../../stores/connection'
import { useSettingsStore } from '../../stores/settings'
import { useToastsStore } from '../../stores/toasts'
import ToggleSwitch from '../ToggleSwitch.vue'
import SettingSection from './SettingSection.vue'

const { t, locale } = useI18n()
const settings = useSettingsStore()
const toasts = useToastsStore()
const connection = useConnectionStore()
const { report } = useDiagnosticsReport()

const KIND_ICON: Record<BugReportItem['kind'], Component> = { report: FileText, log: FileText, crash: Bug, dump: HardDrive }
const KIND_TONE: Record<BugReportItem['kind'], string> = { report: 'text-ink-muted', log: 'text-ink-muted', crash: 'text-warning', dump: 'text-warning' }

const preview = ref<BugReportPreview | null>(null)
const loaded = ref(false)
const dumps = ref(false)
const saving = ref(false)
const uiErrors = ref(0)

const groups = computed(() => groupItems(preview.value?.items ?? []))
const total = computed(() => totalBytes(preview.value?.items ?? [], dumps.value))
const limit = computed(() => preview.value?.limit ?? 0)
const over = computed(() => overLimit(total.value, limit.value))
const size = (n: number): string => formatBytes(n, locale.value)

onMounted(async () => {
  uiErrors.value = errorLogCount()
  connection.loadDiagnostics()
  const res = await desktop?.bugReportPreview?.().catch(() => null)
  preview.value = res?.ok ? res : null
  loaded.value = true
})

function failed(): void {
  toasts.push({ id: 'bug-report-result', kind: 'warning', title: t('bugReport.section'), text: t('bugReport.error'), timeoutMs: 8000, attrs: { 'data-bug-report-toast-result': 'error' } })
}

async function save(): Promise<void> {
  const bridge = desktop?.bugReportSave
  if (!bridge || saving.value) return
  saving.value = true
  try {
    const res = await bridge({ page: report(), errors: errorLogText(), dumps: dumps.value }).catch(() => null)
    if (!res || (!res.ok && res.code === 'io')) return failed()
    if (!res.ok) return
    const path = res.path
    const notes = [res.cut.length ? t('bugReport.cut', { names: res.cut.join(', ') }) : '', res.skipped.length ? t('bugReport.skipped', { names: res.skipped.join(', ') }) : ''].filter(Boolean)
    const reveal = desktop?.reveal
    toasts.push({
      id: 'bug-report-result',
      kind: 'info',
      title: t('bugReport.saved'),
      text: path,
      detail: t('bugReport.savedDetail', { size: size(res.bytes), files: res.files }),
      meta: notes.length ? notes.join(' ') : undefined,
      timeoutMs: 12_000,
      attrs: { 'data-bug-report-toast-result': 'ok' },
      actions: reveal ? [{ label: t('bugReport.reveal'), primary: true, run: () => void reveal(path).catch(() => false) }] : undefined,
    })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <SettingSection id="bugReport" :title="t('bugReport.section')" data-bug-report data-setting-row="bugReport">
    <p class="px-3 py-2 text-2xs text-ink-muted">{{ t('bugReport.hint') }}</p>

    <div class="border-t border-line" data-bug-report-list>
      <p v-if="!loaded" class="flex items-center gap-2 px-3 py-2 text-2xs text-ink-muted" role="status">
        <LoaderCircle :size="13" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />{{ t('bugReport.loading') }}
      </p>
      <p v-else-if="!preview" class="px-3 py-2 text-2xs text-ink-faint">{{ t('bugReport.unavailable') }}</p>
      <p v-else-if="!groups.length" class="px-3 py-2 text-2xs text-ink-faint">{{ t('bugReport.empty') }}</p>
      <template v-else>
        <div v-for="g in groups" :key="g.kind" class="min-w-0" :data-bug-report-group="g.kind">
          <div class="flex items-baseline gap-2 bg-fill px-3 py-1 text-2xs font-semibold text-ink-muted">
            <span class="min-w-0 flex-1 truncate">{{ t(`bugReport.kind.${g.kind}`) }}</span>
            <span class="shrink-0 tabular font-normal">{{ size(g.bytes) }}</span>
          </div>
          <ul>
            <li
              v-for="i in g.items"
              :key="i.name"
              class="flex min-w-0 items-center gap-2 px-3 py-1 text-xs"
              :class="i.kind === 'dump' && !dumps ? 'opacity-50' : ''"
              :data-bug-report-item="i.name"
            >
              <component :is="KIND_ICON[i.kind]" :size="13" class="shrink-0" :class="KIND_TONE[i.kind]" aria-hidden="true" />
              <span class="min-w-0 flex-1 truncate text-ink" :title="i.name">{{ i.name }}</span>
              <span class="shrink-0 tabular text-ink-muted">{{ size(i.bytes) }}</span>
            </li>
          </ul>
        </div>
        <div class="flex items-baseline gap-2 border-t border-line px-3 py-2 text-xs" data-bug-report-total>
          <span class="min-w-0 flex-1 font-medium text-ink">{{ t('bugReport.totalLabel') }}</span>
          <span class="shrink-0 tabular" :class="over ? 'text-warning' : 'text-ink-muted'">{{ t('bugReport.total', { size: size(total), limit: size(limit) }) }}</span>
        </div>
        <p v-if="over" class="flex items-start gap-1.5 px-3 pb-2 text-2xs text-warning" data-bug-report-over>
          <CircleAlert :size="13" class="mt-px shrink-0" aria-hidden="true" />{{ t('bugReport.over') }}
        </p>
      </template>
      <p class="px-3 pb-2 text-2xs text-ink-muted tabular" :class="loaded && preview && groups.length ? '' : 'pt-1'" data-bug-report-errors>{{ t('bugReport.uiErrors', { n: uiErrors }) }}</p>
    </div>

    <div class="border-t border-line px-3">
      <ToggleSwitch v-model="dumps" :label="t('bugReport.dumps')" :hint="t('bugReport.dumpsHint')" data-bug-report-dumps />
    </div>

    <div class="border-t border-line px-3">
      <ToggleSwitch v-model="settings.bugReportNudge" :label="t('bugReport.nudge')" :hint="t('bugReport.nudgeHint')" data-setting-row="bugReportNudge" data-bug-report-nudge />
    </div>

    <div class="flex justify-end border-t border-line px-3 py-2">
      <button type="button" class="ccm-btn ccm-btn-primary" :disabled="saving" :aria-busy="saving" data-bug-report-save @click="save">
        <LoaderCircle v-if="saving" :size="14" class="animate-spin motion-reduce:animate-none" aria-hidden="true" /><Save v-else :size="14" aria-hidden="true" />
        {{ saving ? t('bugReport.saving') : t('bugReport.save') }}
      </button>
    </div>
  </SettingSection>
</template>
