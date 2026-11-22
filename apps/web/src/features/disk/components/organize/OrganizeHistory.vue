<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { CircleAlert, FolderInput, Undo2, X } from 'lucide-vue-next'
import { formatSize } from '../../lib/diskMap'
import { baseName } from '../../lib/organize'
import { useOrganizeStore } from '../../stores/organize'

const { t, te, locale } = useI18n()
const store = useOrganizeStore()

const when = (at: number): string => new Date(at).toLocaleString(locale.value, { dateStyle: 'short', timeStyle: 'short' })
const iso = (at: number): string => (Number.isFinite(at) ? new Date(at).toISOString() : '')
const skipText = (r: string): string => (te(`diskOrganize.skip.${r}`) ? t(`diskOrganize.skip.${r}`, { min: Math.max(1, Math.round((store.state?.stableMs ?? 120_000) / 60_000)) }) : r)
const errText = (r: string): string => (te(`diskOrganize.error.${r}`) ? t(`diskOrganize.error.${r}`) : r)
</script>

<template>
  <section class="rounded-card border border-line" :aria-label="t('diskOrganize.history.title')" data-organize-history>
    <header class="flex items-center justify-between gap-2 border-b border-line px-3 py-2">
      <h3 class="text-xs font-semibold">{{ t('diskOrganize.history.title') }}</h3>
      <p v-if="store.state && store.state.watching.length" class="text-2xs text-ink-muted" data-organize-watching>{{ t('diskOrganize.auto.watching', { n: store.state.watching.length }) }}</p>
    </header>
    <div v-if="store.undoResult" class="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2 text-xs" role="status" data-organize-undo-result>
      <template v-if="store.undoResult.error">
        <CircleAlert :size="13" class="text-warning" aria-hidden="true" /><span class="flex-1">{{ errText(store.undoResult.error) }}</span>
      </template>
      <span v-else class="min-w-0 flex-1">
        {{ t('diskOrganize.undoResult', { n: store.undoResult.restored }) }}
        <span v-if="store.undoResult.conflicts.length" class="text-warning">, {{ t('diskOrganize.undoConflicts', { n: store.undoResult.conflicts.length }) }}</span>
      </span>
      <button type="button" class="ccm-btn-plain" :aria-label="t('diskOrganize.dismiss')" @click="store.undoResult = null"><X :size="13" aria-hidden="true" /></button>
      <ul v-if="store.undoResult.conflicts.length" class="w-full space-y-0.5 text-ink-muted">
        <li v-for="c in store.undoResult.conflicts" :key="c.name" class="truncate">{{ c.name }}: {{ skipText(c.reason) }}</li>
      </ul>
    </div>
    <p v-if="!store.runs.length" class="px-3 py-4 text-center text-xs text-ink-muted">{{ t('diskOrganize.history.empty') }}</p>
    <ul v-else class="max-h-56 divide-y divide-line overflow-y-auto overflow-x-hidden">
      <li v-for="r in store.runs" :key="r.id" class="flex items-center gap-2 px-3 py-1.5 text-xs" :data-organize-run-row="r.id">
        <FolderInput :size="13" class="shrink-0 text-ink-muted" aria-hidden="true" />
        <span class="min-w-0 flex-1 truncate" :title="r.source">
          {{ t('diskOrganize.history.run', { n: r.moved, name: baseName(r.source) }) }}
          <span v-if="r.auto" class="text-ink-muted">({{ t('diskOrganize.history.auto') }})</span>
        </span>
        <span class="shrink-0 tabular-nums">{{ formatSize(r.bytes, locale) }}</span>
        <time class="w-28 shrink-0 text-right text-2xs text-ink-muted tabular-nums" :datetime="iso(r.at)">{{ when(r.at) }}</time>
        <span v-if="r.undone" class="w-24 shrink-0 text-right text-2xs text-ink-muted">{{ t('diskOrganize.history.undone') }}</span>
        <button v-else type="button" class="ccm-btn w-24 shrink-0 justify-center" :disabled="!!store.undoing" data-organize-undo @click="store.undo(r.id)">
          <Undo2 :size="13" aria-hidden="true" />{{ store.undoing === r.id ? t('diskOrganize.history.undoing') : t('diskOrganize.history.undo') }}
        </button>
      </li>
    </ul>
  </section>
</template>
