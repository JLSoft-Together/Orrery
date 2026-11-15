<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, ExternalLink, RefreshCw, Search } from 'lucide-vue-next'
import { formatSize } from '../../../lib/diskMap'
import { APP_SORTS, type AppSort } from '../../../lib/diskApps'
import { isMac } from '../../../lib/platform'
import { useDiskAppsStore } from '../../../stores/diskApps'
import AppLeftovers from './AppLeftovers.vue'
import AppRow from './AppRow.vue'

const { t, locale } = useI18n()
const store = useDiskAppsStore()
const mac = isMac

const notice = computed(() => {
  const n = store.notice
  if (!n) return null
  const key = n.res.ok ? (n.res.mode ?? 'started') : (n.res.error ?? 'failed')
  return { ok: n.res.ok, text: t(`diskApps.result.${key}`, { name: n.name }) }
})
const showSettings = computed(() => store.result?.storeApps === 'settings' || (!!store.notice && ['unparsable', 'missing'].includes(store.notice.res.error ?? '')))

onMounted(() => store.open())
</script>

<template>
  <section class="mt-4" data-disk-apps>
    <p class="max-w-[80ch] text-xs text-ink-muted">{{ mac ? t('diskApps.hintMac') : t('diskApps.hint') }}</p>

    <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
      <div class="relative min-w-0 flex-1 basis-56">
        <Search :size="13" class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-ink-faint" aria-hidden="true" />
        <input v-model="store.query" type="search" spellcheck="false" autocomplete="off" class="ccm-field w-full pl-7" :aria-label="t('diskApps.search')" :placeholder="t('diskApps.search')" data-app-query />
      </div>
      <label class="flex items-center gap-1.5 text-ink-muted">
        {{ t('diskApps.sort.label') }}
        <select v-model="store.sort" class="ccm-field" data-app-sort>
          <option v-for="s in APP_SORTS" :key="s" :value="s as AppSort">{{ t(`diskApps.sort.${s}`) }}</option>
        </select>
      </label>
      <button type="button" class="ccm-btn" :disabled="store.loading" data-app-refresh @click="store.load()">
        <RefreshCw :size="12" :class="store.loading ? 'animate-spin' : ''" aria-hidden="true" />{{ t('diskApps.refresh') }}
      </button>
      <button v-if="showSettings" type="button" class="ccm-btn" :title="t('diskApps.storeHint')" data-app-settings @click="store.openSettings()">
        <ExternalLink :size="12" aria-hidden="true" />{{ t('diskApps.storeApps') }}
      </button>
    </div>

    <p v-if="notice" class="mt-3 flex items-start gap-2 rounded-card px-3 py-2 text-xs" :class="notice.ok ? 'bg-accent-soft' : 'bg-warning-soft'" role="status" data-app-notice>
      <component :is="notice.ok ? CircleCheck : CircleAlert" :size="13" class="mt-0.5 shrink-0" :class="notice.ok ? 'text-accent-ink' : 'text-warning'" aria-hidden="true" />{{ notice.text }}
    </p>

    <AppLeftovers v-if="store.leftovers || store.scanning" :mac="mac" />

    <section v-if="store.removed.length" class="mt-3 rounded-card border border-line" data-app-removed>
      <header class="border-b border-line px-3 py-2">
        <h3 class="text-sm font-semibold">{{ t('diskApps.removed') }}</h3>
        <p class="text-2xs text-ink-muted">{{ t('diskApps.removedHint') }}</p>
      </header>
      <ul class="divide-y divide-line">
        <AppRow v-for="a in store.removed" :key="a.id" :app="a" :mac="mac" removed :busy="store.busy === a.id" :scanning="store.scanning === a.id" @leftovers="store.scanLeftovers" />
      </ul>
    </section>

    <div class="mt-3 flex items-center justify-between gap-2 text-xs text-ink-muted">
      <span class="tabular-nums" data-app-count>{{ store.result ? t('diskApps.count', { n: store.apps.length, size: formatSize(store.totalBytes, locale) }) : '' }}</span>
    </div>

    <div class="ccm-panel mt-1 min-w-0 overflow-hidden rounded-card border border-line">
      <p v-if="store.loading && !store.result" class="px-3 py-6 text-center text-xs text-ink-muted" data-app-loading>{{ t('diskApps.loading') }}</p>
      <p v-else-if="store.result && !store.result.ok && !store.apps.length" class="px-3 py-6 text-center text-xs text-ink-muted" role="alert" data-app-failed>{{ t('diskApps.failed') }}</p>
      <p v-else-if="store.result && !store.shown.length" class="px-3 py-6 text-center text-xs text-ink-muted">{{ t('diskApps.empty') }}</p>
      <ul v-else class="max-h-[36rem] divide-y divide-line overflow-y-auto" data-app-list>
        <AppRow
          v-for="a in store.shown"
          :key="a.id"
          :app="a"
          :mac="mac"
          :busy="store.busy === a.id"
          :scanning="store.scanning === a.id"
          @uninstall="store.uninstall"
          @leftovers="store.scanLeftovers"
        />
      </ul>
    </div>
  </section>
</template>
