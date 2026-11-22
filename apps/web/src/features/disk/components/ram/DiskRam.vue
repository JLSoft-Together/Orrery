<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, Layers, List, LoaderCircle, Lock, Search, Square } from 'lucide-vue-next'
import { formatSize } from '../../lib/diskMap'
import { filterRam, formatPct, procBase, ramShare, type RamKillTarget, type RamLock, type RamTag } from '../../lib/diskApps'
import { useDiskRamStore, type RamView } from '../../stores/diskRam'
import SegmentedControl from '../../../../core/components/ui/SegmentedControl.vue'

const { t, locale } = useI18n()
const store = useDiskRamStore()

interface Row {
  key: string
  name: string
  sub: string
  ramBytes: number | null
  cpu: number | null
  tag: RamTag | null
  lock: RamLock | null
  targets: RamKillTarget[]
}

const views = computed(() => [
  { value: 'apps' as RamView, label: t('diskRam.view.apps'), icon: Layers },
  { value: 'procs' as RamView, label: t('diskRam.view.procs'), icon: List },
])
const total = computed(() => store.snap?.totalMem ?? 0)
const usedPct = computed(() => (total.value ? Math.round((store.usedMem / total.value) * 100) : 0))
const rows = computed<Row[]>(() => {
  const procs = store.snap?.procs ?? []
  const list: Row[] =
    store.view === 'apps'
      ? store.groups.map((g) => ({ key: `g:${g.key}`, name: g.name, sub: t('diskRam.count', { n: g.count }), ramBytes: g.ramBytes, cpu: g.cpu, tag: g.tag, lock: g.targets.length ? null : g.lock, targets: g.targets }))
      : procs.map((p) => ({ key: `p:${p.pid}`, name: procBase(p.name) || String(p.pid), sub: `pid ${p.pid}`, ramBytes: p.ramBytes, cpu: p.cpu, tag: p.tag, lock: p.lock, targets: p.lock || !p.start ? [] : [{ pid: p.pid, start: p.start }] }))
  return filterRam(list, store.query).slice(0, 300)
})
const notice = computed(() => {
  const n = store.notice
  if (!n) return null
  return { ok: n.res.ok, text: n.res.ok ? t('diskRam.result.ok', { n: n.res.killed, name: n.name }) : t(`diskRam.result.${n.res.error ?? 'failed'}`, { name: n.name }) }
})
const tagClass = (tag: RamTag): string => (tag === 'agent' ? 'ccm-chip-working' : tag === 'dev' ? 'ccm-chip-waiting' : 'ccm-chip-quiet')

onMounted(() => {
  store.active = true
  store.start()
})
onBeforeUnmount(() => store.stop())
</script>

<template>
  <section class="mt-4" data-disk-ram>
    <p class="max-w-[80ch] text-xs text-ink-muted">{{ t('diskRam.hint') }}</p>

    <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2">
      <div class="min-w-0 flex-1 basis-64" data-ram-used>
        <div class="flex items-baseline justify-between gap-2 text-xs">
          <span class="font-medium">{{ t('diskRam.used') }}</span>
          <span class="tabular-nums text-ink-muted">{{ store.snap?.ok ? t('diskRam.usedOf', { used: formatSize(store.usedMem, locale), total: formatSize(total, locale) }) : '' }} <template v-if="store.snap?.ok">({{ usedPct }}%)</template></span>
        </div>
        <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-fill" role="progressbar" :aria-label="t('diskRam.used')" :aria-valuenow="usedPct" aria-valuemin="0" aria-valuemax="100">
          <div class="h-full rounded-full transition-[width] duration-300" :class="usedPct >= 90 ? 'bg-warning' : 'bg-accent'" :style="{ width: `${usedPct}%` }" />
        </div>
      </div>
      <SegmentedControl v-model="store.view" :options="views" :label="t('diskRam.view.label')" data-ram-view />
    </div>

    <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
      <div class="relative min-w-0 flex-1 basis-56">
        <Search :size="13" class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-ink-faint" aria-hidden="true" />
        <input v-model="store.query" type="search" spellcheck="false" autocomplete="off" class="ccm-field w-full pl-7" :aria-label="t('diskRam.search')" :placeholder="t('diskRam.search')" data-ram-query />
      </div>
    </div>

    <p v-if="notice" class="mt-3 flex items-start gap-2 rounded-card px-3 py-2 text-xs" :class="notice.ok ? 'bg-accent-soft' : 'bg-warning-soft'" role="status" data-ram-notice>
      <component :is="notice.ok ? CircleCheck : CircleAlert" :size="13" class="mt-0.5 shrink-0" :class="notice.ok ? 'text-accent-ink' : 'text-warning'" aria-hidden="true" />{{ notice.text }}
    </p>

    <div class="ccm-panel mt-3 min-w-0 overflow-hidden rounded-card border border-line">
      <div class="flex items-center gap-3 border-b border-line px-3 py-1.5 text-2xs text-ink-muted" aria-hidden="true">
        <span class="min-w-0 flex-1">{{ t('diskRam.name') }}</span>
        <span class="w-20 shrink-0 text-right">{{ t('diskRam.ram') }}</span>
        <span class="hidden w-14 shrink-0 text-right sm:block">{{ t('diskRam.share') }}</span>
        <span class="w-14 shrink-0 text-right">{{ t('diskRam.cpu') }}</span>
        <span class="w-24 shrink-0" />
      </div>
      <p v-if="!store.snap" class="px-3 py-6 text-center text-xs text-ink-muted" data-ram-loading>{{ t('diskRam.loading') }}</p>
      <p v-else-if="!store.snap.ok" class="px-3 py-6 text-center text-xs text-ink-muted" role="alert">{{ t('diskRam.failed') }}</p>
      <p v-else-if="!rows.length" class="px-3 py-6 text-center text-xs text-ink-muted">{{ t('diskRam.empty') }}</p>
      <ul v-else class="max-h-[36rem] divide-y divide-line overflow-y-auto" data-ram-list>
        <li v-for="r in rows" :key="r.key" class="flex min-w-0 items-center gap-3 px-3 py-1.5 text-xs" :data-ram-row="r.name" :data-ram-key="r.key">
          <div class="min-w-0 flex-1">
            <p class="flex min-w-0 items-center gap-1.5">
              <span class="truncate font-medium" :title="r.name">{{ r.name }}</span>
              <span v-if="r.tag" class="ccm-chip shrink-0" :class="tagClass(r.tag)" data-ram-tag>{{ t(`diskRam.tag.${r.tag}`) }}</span>
            </p>
            <p class="truncate text-2xs text-ink-muted">{{ r.sub }}</p>
          </div>
          <span class="w-20 shrink-0 text-right tabular-nums">{{ formatSize(r.ramBytes, locale) }}</span>
          <span class="hidden w-14 shrink-0 text-right tabular-nums text-ink-muted sm:block">{{ formatPct(ramShare(r.ramBytes, total)) }}</span>
          <span class="w-14 shrink-0 text-right tabular-nums text-ink-muted">{{ formatPct(r.cpu) }}</span>
          <span class="flex w-24 shrink-0 justify-end">
            <span v-if="r.lock" class="inline-flex items-center gap-1 text-2xs text-ink-muted" :title="t(`diskRam.lock.${r.lock}`)" data-ram-lock>
              <Lock :size="11" aria-hidden="true" /><span class="sr-only">{{ t(`diskRam.lock.${r.lock}`) }}</span>
            </span>
            <button
              v-else
              type="button"
              class="ccm-btn ccm-btn-danger"
              :disabled="!!store.killing || !r.targets.length"
              :aria-label="`${t('diskRam.kill')} ${r.name}`"
              data-ram-kill
              @click="store.kill(r.key, r.name, r.targets)"
            >
              <LoaderCircle v-if="store.killing === r.key" :size="12" class="animate-spin" aria-hidden="true" />
              <Square v-else :size="11" aria-hidden="true" />{{ t('diskRam.kill') }}
            </button>
          </span>
        </li>
      </ul>
    </div>
  </section>
</template>
