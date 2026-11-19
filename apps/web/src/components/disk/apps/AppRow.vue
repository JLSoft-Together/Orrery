<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Brush, LoaderCircle, Lock, Trash2 } from 'lucide-vue-next'
import { formatSize } from '../../../lib/diskMap'
import type { InstalledApp } from '../../../lib/diskApps'

const props = defineProps<{ app: InstalledApp; mac: boolean; busy: boolean; scanning: boolean; removed?: boolean }>()
const emit = defineEmits<{ uninstall: [app: InstalledApp]; leftovers: [app: InstalledApp] }>()
const { t, locale } = useI18n()

const date = (ms: number | null): string => (ms ? new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }).format(ms) : '')
const meta = computed(() => {
  const a = props.app
  const parts = [a.publisher ?? (a.source === 'registry' ? t('diskApps.publisherUnknown') : a.bundleId ?? ''), a.version ?? '']
  if (a.installedAt) parts.push(t('diskApps.installed', { date: date(a.installedAt) }))
  if (a.lastUsed) parts.push(t('diskApps.lastUsed', { date: date(a.lastUsed) }))
  return parts.filter(Boolean).join(', ')
})
const sizeTitle = computed(() => (props.app.sizeFrom === 'estimate' ? t('diskApps.estimate') : props.app.sizeFrom === 'index' ? t('diskApps.fromIndex') : undefined))
const canUninstall = computed(() => !props.removed && !props.app.blocked && (props.mac || !!props.app.uninstall))
</script>

<template>
  <li class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2 text-xs sm:flex-nowrap" :data-app-row="app.name">
    <div class="min-w-0 flex-1 basis-56">
      <p class="truncate font-medium" :title="app.location ?? app.name">{{ app.name }}</p>
      <p class="truncate text-2xs text-ink-muted" :title="meta">{{ meta }}</p>
    </div>
    <span v-if="app.arch === '32'" class="ccm-chip ccm-chip-quiet shrink-0">{{ t('diskApps.arch32') }}</span>
    <span v-if="app.scope === 'user' && app.source === 'registry'" class="ccm-chip ccm-chip-quiet hidden shrink-0 md:inline-flex">{{ t('diskApps.scopeUser') }}</span>
    <span v-if="app.blocked" class="ccm-chip ccm-chip-warning inline-flex shrink-0 items-center gap-1" data-app-blocked>
      <Lock :size="11" aria-hidden="true" />{{ t(`diskApps.blocked.${app.blocked}`) }}
    </span>
    <span class="w-20 shrink-0 text-right tabular-nums" :class="app.sizeFrom === 'estimate' ? 'text-ink-muted' : ''" :title="sizeTitle" data-app-size>{{ formatSize(app.size, locale) }}</span>
    <button v-if="!app.blocked" type="button" class="ccm-btn shrink-0" :disabled="busy || scanning" data-app-leftovers @click="emit('leftovers', app)">
      <LoaderCircle v-if="scanning" :size="12" class="animate-spin" aria-hidden="true" />
      <Brush v-else :size="12" aria-hidden="true" />{{ t('diskApps.leftovers') }}
    </button>
    <button
      v-if="canUninstall"
      type="button"
      class="ccm-btn ccm-btn-danger shrink-0"
      :disabled="busy"
      :aria-label="`${mac ? t('diskApps.uninstallMac') : t('diskApps.uninstall')} ${app.name}`"
      data-app-uninstall
      @click="emit('uninstall', app)"
    >
      <LoaderCircle v-if="busy" :size="12" class="animate-spin" aria-hidden="true" />
      <Trash2 v-else :size="12" aria-hidden="true" />{{ mac ? t('diskApps.uninstallMac') : t('diskApps.uninstall') }}
    </button>
    <span v-else-if="!removed && !app.blocked" class="max-w-[12rem] shrink-0 truncate text-2xs text-ink-muted" :title="t('diskApps.noUninstaller')">{{ t('diskApps.noUninstaller') }}</span>
  </li>
</template>
