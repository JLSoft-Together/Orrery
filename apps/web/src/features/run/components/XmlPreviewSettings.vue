<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck } from 'lucide-vue-next'
import { layoutDownload, layoutInstallStatus, layoutUninstall, onLayoutEvent, type LayoutInstallStatus } from '../lib/layoutPreview'
import { installKind, megabytes } from '../lib/xmlPreviewInstall'

const props = defineProps<{ open: boolean }>()

const { t } = useI18n()
const status = ref<LayoutInstallStatus | null>(null)
const pct = ref<number | null>(null)
const working = ref<'install' | 'remove' | null>(null)
const note = ref<'' | 'cancelled' | 'removeFailed' | 'busy'>('')
const removeCode = ref('')

const kind = computed(() => (status.value ? installKind(status.value) : null))
const downloading = computed(() => kind.value === 'downloading' || working.value === 'install')
const errorCode = computed(() => (status.value?.bundle.state === 'error' ? status.value.bundle.code : ''))
const canInstall = computed(() => !!status.value?.available && kind.value !== 'installed' && !downloading.value && working.value === null)
const canRemove = computed(() => !!status.value && status.value.sizeBytes > 0 && !downloading.value && working.value === null)

async function refresh(): Promise<void> {
  status.value = await layoutInstallStatus()
}

watch(
  () => props.open,
  (open) => {
    if (open) void refresh()
  },
  { immediate: true },
)

const off = onLayoutEvent((e) => {
  if (e.type !== 'bundle') return
  pct.value = e.status.state === 'downloading' ? e.status.pct : null
  if (e.status.state !== 'downloading') void refresh()
})
onBeforeUnmount(off)

async function install(): Promise<void> {
  note.value = ''
  pct.value = 0
  working.value = 'install'
  const res = await layoutDownload()
  working.value = null
  pct.value = null
  if (res?.state === 'cancelled') note.value = 'cancelled'
  await refresh()
}

async function remove(): Promise<void> {
  note.value = ''
  working.value = 'remove'
  const res = await layoutUninstall()
  working.value = null
  if (res && !res.ok) {
    note.value = res.code === 'busy' ? 'busy' : 'removeFailed'
    removeCode.value = res.code ?? ''
  }
  if (res) status.value = res.status
  else await refresh()
}
</script>

<template>
  <section v-if="status" aria-labelledby="set-xmlpreview" class="mt-6" data-xml-preview-settings>
    <h3 id="set-xmlpreview" class="mb-1.5 px-3 text-2xs font-semibold text-ink-muted">{{ t('xmlPreview.section') }}</h3>
    <div class="ccm-group divide-y divide-line overflow-hidden">
      <div class="flex min-w-0 items-center gap-2 px-3 py-2">
        <div class="min-w-0 flex-1">
          <p class="text-sm text-ink">{{ t('xmlPreview.status') }}</p>
          <p class="mt-0.5 flex items-center gap-1.5 text-2xs" :class="kind === 'installed' ? 'text-success' : 'text-ink-muted'" data-xml-state>
            <CircleCheck v-if="kind === 'installed'" :size="13" aria-hidden="true" />
            <CircleAlert v-else-if="kind === 'unavailable' || kind === 'error'" :size="13" class="text-warning" aria-hidden="true" />
            <template v-if="kind === 'installed'">{{ t('xmlPreview.installedVersion', { version: status.version ?? status.pinnedVersion ?? '' }) }}</template>
            <template v-else-if="kind === 'update'">{{ t('xmlPreview.updateAvailable', { version: status.version ?? '', pinned: status.pinnedVersion ?? '' }) }}</template>
            <template v-else-if="kind === 'unavailable'">{{ t('xmlPreview.unavailable') }}</template>
            <template v-else-if="kind === 'downloading'">{{ t('xmlPreview.downloading', { pct: pct ?? 0 }) }}</template>
            <template v-else>{{ t('xmlPreview.notInstalled') }}</template>
          </p>
        </div>
        <button v-if="canInstall || downloading" type="button" class="ccm-btn ccm-btn-primary shrink-0" :disabled="downloading" data-xml-install @click="install">
          {{ kind === 'error' ? t('xmlPreview.retry') : kind === 'update' ? t('xmlPreview.update') : t('xmlPreview.install') }}
        </button>
        <button type="button" class="ccm-btn shrink-0" :disabled="!canRemove" data-xml-remove @click="remove">
          {{ working === 'remove' ? t('xmlPreview.removing') : t('xmlPreview.remove') }}
        </button>
      </div>
      <div v-if="downloading" class="px-3 py-2">
        <div class="h-1.5 w-full overflow-hidden rounded-full bg-fill" role="progressbar" aria-valuemin="0" aria-valuemax="100" :aria-valuenow="pct ?? 0" :aria-label="t('xmlPreview.downloading', { pct: pct ?? 0 })" data-xml-progress>
          <div class="h-full origin-left rounded-full bg-accent transition-transform duration-(--ccm-dur-base)" :style="{ transform: `scaleX(${(pct ?? 0) / 100})` }" />
        </div>
      </div>
      <div v-if="status.available" class="flex items-center justify-between gap-3 px-3 py-2 text-sm">
        <span>{{ t('xmlPreview.version') }}</span>
        <span class="tabular text-ink-muted" data-xml-version>{{ status.version ?? '-' }}</span>
      </div>
      <div class="flex items-center justify-between gap-3 px-3 py-2 text-sm">
        <span>{{ t('xmlPreview.size') }}</span>
        <span class="tabular text-ink-muted" data-xml-size>{{ t('xmlPreview.sizeValue', { mb: megabytes(status.sizeBytes) }) }}</span>
      </div>
      <div v-if="status.available && status.downloadBytes > 0" class="flex items-center justify-between gap-3 px-3 py-2 text-sm">
        <span>{{ t('xmlPreview.downloadSize') }}</span>
        <span class="tabular text-ink-muted">{{ t('xmlPreview.sizeValue', { mb: megabytes(status.downloadBytes) }) }}</span>
      </div>
      <p v-if="kind === 'error'" class="flex items-start gap-1.5 px-3 py-2 text-2xs text-st-error" role="alert">
        <CircleAlert :size="13" class="mt-px shrink-0" aria-hidden="true" />{{ t('xmlPreview.error', { code: errorCode }) }}
      </p>
      <p v-if="note === 'removeFailed'" class="px-3 py-2 text-2xs text-st-error" role="alert">{{ t('xmlPreview.removeFailed', { code: removeCode }) }}</p>
      <p v-else-if="note === 'busy'" class="px-3 py-2 text-2xs text-warning" role="alert">{{ t('xmlPreview.busy') }}</p>
      <p v-else-if="note === 'cancelled'" class="px-3 py-2 text-2xs text-ink-muted" role="status">{{ t('xmlPreview.cancelled') }}</p>
    </div>
    <p class="mt-1.5 px-3 text-2xs leading-relaxed text-ink-faint">{{ kind === 'unavailable' ? t('xmlPreview.unavailableHint') : t('xmlPreview.hint') }} {{ t('xmlPreview.javaHint') }}</p>
  </section>
</template>
