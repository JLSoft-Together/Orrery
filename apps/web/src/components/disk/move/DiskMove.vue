<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowRight, CircleAlert, CircleCheck, FolderInput, FolderOpen, HardDrive, Lock, RotateCcw, Undo2, X } from 'lucide-vue-next'
import { diskMapReveal, formatSize } from '../../../lib/diskMap'
import { baseName } from '../../../lib/diskClean'
import { errorKey, percent, type MoveCandidate } from '../../../lib/diskMove'
import { useDiskMoveStore } from '../../../stores/diskMove'

const { t, locale } = useI18n()
const store = useDiskMoveStore()

const size = (n: number | undefined): string => (typeof n === 'number' ? formatSize(n, locale.value) : t('diskMove.sizeLater'))
const when = (at: number): string => new Date(at).toLocaleDateString(locale.value, { dateStyle: 'medium' })
const kindLabel = (c: MoveCandidate): string => t(`diskMove.kind.${c.kind}`)
const errorText = (e: string | undefined, holder?: string): string => t(`diskMove.error.${errorKey(e)}`, { holder: holder ?? '' })
const progressText = computed(() => {
  const p = store.progress
  if (!p) return store.planning ? t('diskMove.phase.check') : ''
  const base = t(`diskMove.phase.${p.phase}`)
  return p.phase === 'copy' ? `${base} ${formatSize(p.bytes, locale.value)} / ${formatSize(p.total, locale.value)}` : base
})
const noticeText = computed(() => {
  const n = store.notice
  if (!n) return ''
  if (n.ok) return n.leftover ? t('diskMove.done.leftover', { path: n.leftover }) : t(`diskMove.done.${n.kind}`)
  const head = errorText(n.error, n.holder)
  return n.rolledBack ? `${head} ${t('diskMove.rolledBack')}` : head
})
const shown = computed(() => store.candidates.filter((c) => c.block !== 'missing' || c.kind === 'picked' || c.kind === 'clean'))

onMounted(() => void store.init())
onBeforeUnmount(() => store.dispose())
</script>

<template>
  <section v-if="store.supported" class="rounded-card border border-line" :aria-label="t('diskMove.title')" data-move>
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line px-3 py-2">
      <HardDrive :size="15" class="shrink-0 text-ink-muted" aria-hidden="true" />
      <div class="min-w-0 flex-1 basis-56">
        <h3 class="text-sm font-semibold">{{ t('diskMove.title') }}</h3>
        <p class="mt-0.5 text-2xs leading-relaxed text-ink-muted">{{ t('diskMove.hint') }}</p>
      </div>
      <button type="button" class="ccm-btn shrink-0" :disabled="store.busy" data-move-pick @click="store.pick()">
        <FolderInput :size="14" aria-hidden="true" />{{ t('diskMove.pick') }}
      </button>
    </header>

    <div v-for="p in store.pending" :key="p.id" class="flex flex-wrap items-center gap-2 border-b border-line bg-warning-soft px-3 py-2 text-xs" role="alert" :data-move-pending="p.id">
      <CircleAlert :size="14" class="shrink-0 text-warning" aria-hidden="true" />
      <p class="min-w-0 flex-1 basis-60">
        {{ t(`diskMove.pending.${p.kind}.${p.stage}`) }}
        <span class="block truncate font-mono text-2xs text-ink-muted" :title="p.source">{{ p.source }}</span>
      </p>
      <button v-if="p.stage === 'linked' || (p.kind === 'back' && p.stage === 'renamed')" type="button" class="ccm-btn shrink-0" :disabled="store.busy" data-move-finish @click="store.recover(p.id, 'finish')">
        {{ t('diskMove.finish') }}
      </button>
      <button v-if="!(p.kind === 'back' && p.stage === 'linked')" type="button" class="ccm-btn ccm-btn-primary shrink-0" :disabled="store.busy" data-move-restore @click="store.recover(p.id, 'restore')">
        <RotateCcw :size="13" aria-hidden="true" />{{ t('diskMove.restore') }}
      </button>
    </div>

    <ul class="divide-y divide-line" data-move-list>
      <li v-for="c in shown" :key="c.id" class="px-3 py-2 text-xs" :data-move-candidate="c.path" :data-move-block="c.block || undefined">
        <div class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 sm:flex-nowrap">
          <div class="min-w-0 flex-1 basis-48">
            <p class="truncate font-medium" :title="c.path">{{ kindLabel(c) }}<template v-if="c.kind === 'picked' || c.kind === 'clean'">: {{ baseName(c.path) }}</template></p>
            <p class="truncate text-2xs text-ink-muted" :title="c.path">{{ c.guide ? t('diskMove.dockerGuide') : c.path }}</p>
          </div>
          <span v-if="c.block" class="ccm-chip ccm-chip-warning inline-flex shrink-0 items-center gap-1" :title="t(`diskMove.block.${c.block}`)" data-move-blocked>
            <Lock :size="11" aria-hidden="true" />{{ t(`diskMove.blockShort.${c.block}`) }}
          </span>
          <span v-if="!c.guide" class="w-20 shrink-0 text-right tabular-nums text-ink-muted">{{ c.block ? '' : size(c.size) }}</span>
          <button v-if="!c.guide && !c.block" type="button" class="ccm-btn shrink-0" :disabled="store.busy" :aria-expanded="store.chosen === c.id" data-move-start @click="store.choose(store.chosen === c.id ? null : c.id)">
            <ArrowRight :size="13" aria-hidden="true" />{{ t('diskMove.move') }}
          </button>
          <button v-if="!c.guide && c.block !== 'missing'" type="button" class="ccm-btn-plain shrink-0" :title="t('diskClean.reveal')" :aria-label="t('diskClean.reveal')" @click="diskMapReveal(c.path)">
            <FolderOpen :size="13" aria-hidden="true" />
          </button>
        </div>

        <div v-if="store.chosen === c.id" class="mt-2 flex flex-col gap-2 rounded-control bg-fill px-3 py-2" data-move-panel>
          <p class="ccm-label">{{ t('diskMove.to') }}</p>
          <p v-if="!store.volumes.length" class="text-2xs text-ink-muted">{{ t('diskMove.noVolumes') }}</p>
          <div role="radiogroup" :aria-label="t('diskMove.to')" class="flex flex-col gap-1">
            <label v-for="v in store.volumes" :key="v.mount" class="flex min-w-0 items-center gap-2" :class="v.ok ? '' : 'text-ink-muted'" :data-move-volume="v.mount" :data-move-volume-reason="v.reason || undefined">
              <input type="radio" name="move-volume" class="accent-[var(--ccm-accent)]" :value="v.mount" :checked="store.volume === v.mount" :disabled="!v.ok && v.reason !== 'space' || store.busy" @change="store.volume = v.mount; store.plan = null" />
              <span class="min-w-0 flex-1 truncate" :title="v.mount">{{ v.label ? `${v.label} (${v.mount})` : v.mount }}</span>
              <span class="shrink-0 tabular-nums text-2xs">{{ t('diskMove.free', { size: formatSize(v.free, locale) }) }}</span>
              <span v-if="v.reason" class="shrink-0 text-2xs text-warning">{{ t(`diskMove.volumeReason.${v.reason}`) }}</span>
            </label>
          </div>
          <div v-if="store.plan" class="text-2xs leading-relaxed" data-move-plan>
            <p class="truncate font-mono" :title="store.plan.target">{{ store.plan.source }} <span aria-hidden="true">&gt;</span> {{ store.plan.target }}</p>
            <p class="text-ink-muted">{{ t('diskMove.planInfo', { size: formatSize(store.plan.bytes, locale), n: store.plan.files, need: formatSize(store.plan.need, locale), free: formatSize(store.plan.free, locale) }) }}</p>
          </div>
          <div v-if="store.busy" class="flex flex-col gap-1" role="status" data-move-progress>
            <p class="text-2xs text-ink-muted">{{ progressText }}</p>
            <div class="h-1.5 overflow-hidden rounded-full bg-fill-strong">
              <div class="h-full bg-accent" :style="{ width: `${store.progress?.phase === 'copy' ? percent(store.progress) : store.progress ? 100 : 5}%` }" />
            </div>
          </div>
          <div class="flex flex-wrap items-center justify-end gap-2">
            <button v-if="store.busy" type="button" class="ccm-btn" data-move-cancel @click="store.cancel()">{{ t('diskMove.cancel') }}</button>
            <button v-else type="button" class="ccm-btn-plain text-xs" @click="store.choose(null)">{{ t('diskMove.close') }}</button>
            <button v-if="!store.plan" type="button" class="ccm-btn ccm-btn-primary" :disabled="!store.volume || store.busy" data-move-check @click="store.makePlan()">{{ t('diskMove.check') }}</button>
            <button v-else type="button" class="ccm-btn ccm-btn-primary" :disabled="store.busy" data-move-run @click="store.run()">
              <ArrowRight :size="13" aria-hidden="true" />{{ t('diskMove.run', { size: formatSize(store.plan.bytes, locale) }) }}
            </button>
          </div>
        </div>
      </li>
    </ul>

    <template v-if="store.moves.length">
      <h4 class="ccm-label border-t border-line px-3 pb-1 pt-2">{{ t('diskMove.moved') }}</h4>
      <ul class="divide-y divide-line" data-move-done>
        <li v-for="m in store.moves" :key="m.id" class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2 text-xs sm:flex-nowrap" :data-move-record="m.source">
          <div class="min-w-0 flex-1 basis-48">
            <p class="truncate font-medium" :title="m.source">{{ baseName(m.source) }} <span class="text-ink-muted" aria-hidden="true">&gt;</span> {{ m.target }}</p>
            <p class="truncate text-2xs text-ink-muted" :title="m.source">{{ t('diskMove.linkAt', { path: m.source, date: when(m.at) }) }}</p>
            <p v-if="m.leftover" class="truncate text-2xs text-warning" :title="m.leftover">{{ t('diskMove.leftover') }}</p>
          </div>
          <span class="w-20 shrink-0 text-right tabular-nums text-ink-muted">{{ formatSize(m.bytes, locale) }}</span>
          <button type="button" class="ccm-btn shrink-0" :disabled="store.busy || !!m.leftover" data-move-back @click="store.back(m.id)">
            <Undo2 :size="13" aria-hidden="true" />{{ t('diskMove.back') }}
          </button>
        </li>
      </ul>
    </template>

    <div v-if="store.running && !store.chosen" class="border-t border-line px-3 py-2" role="status" data-move-progress>
      <p class="text-2xs text-ink-muted">{{ progressText }}</p>
      <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-fill-strong">
        <div class="h-full bg-accent" :style="{ width: `${store.progress?.phase === 'copy' ? percent(store.progress) : store.progress ? 100 : 5}%` }" />
      </div>
    </div>

    <div v-if="store.notice" class="flex items-start gap-2 border-t border-line px-3 py-2 text-xs" :class="store.notice.ok ? 'bg-st-done-soft' : 'bg-warning-soft'" role="status" data-move-notice>
      <component :is="store.notice.ok ? CircleCheck : CircleAlert" :size="14" class="mt-px shrink-0" :class="store.notice.ok ? 'text-success' : 'text-warning'" aria-hidden="true" />
      <p class="min-w-0 flex-1 break-words">{{ noticeText }}</p>
      <button type="button" class="ccm-btn-plain -my-1.5 shrink-0" :aria-label="t('diskClean.report.dismiss')" @click="store.dismiss()">
        <X :size="13" aria-hidden="true" />
      </button>
    </div>
  </section>
</template>
