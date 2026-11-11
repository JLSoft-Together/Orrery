<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { GitCompareArrows, RotateCcw, X } from 'lucide-vue-next'
import { newestTurnAfter, turnSummary } from '../../lib/paneV93'
import { baseName, cpDiff, cpRestore, hasCheckpoints, type CheckpointFile } from '../../lib/preview'
import { turnPair } from '../../lib/turnReview'
import { useCheckpointsStore } from '../../stores/checkpoints'
import { dirKey } from '../../stores/favorites'

const TurnReview = defineAsyncComponent(() => import('./TurnReview.vue'))

const props = defineProps<{ cwd: string | undefined; status: string | undefined }>()
const emit = defineEmits<{ open: [path: string] }>()
const { t } = useI18n()
const store = useCheckpointsStore()

const root = ref<string | null>(null)
const endedAt = ref(0)
const files = ref<CheckpointFile[]>([])
const dismissed = ref<string | null>(null)
const review = ref(false)
const busy = ref(false)
const notice = ref<{ ok: boolean; text: string } | null>(null)
let loadSeq = 0

watch(
  () => props.status,
  (now, prev) => {
    if (now === 'working') {
      endedAt.value = 0
      files.value = []
      review.value = false
      notice.value = null
      return
    }
    if (prev === 'working' && now === 'idle' && hasCheckpoints && props.cwd) {
      endedAt.value = Date.now()
      void store.rootFor(props.cwd).then((r) => (root.value = r?.root ?? null))
    }
  },
)

const list = computed(() => (root.value ? (store.roots[dirKey(root.value)]?.list ?? []) : []))
const pair = computed(() => {
  const after = newestTurnAfter(list.value, endedAt.value)
  return after ? turnPair(list.value, after.id) : null
})

watch(
  () => pair.value?.after.id,
  async (id) => {
    const seq = ++loadSeq
    files.value = []
    const p = pair.value
    if (!id || !p || !root.value) return
    const res = await cpDiff(root.value, p.before.id, p.after.id)
    if (seq === loadSeq && res.ok) files.value = res.files
  },
  { immediate: true },
)

const summary = computed(() => turnSummary(files.value))
const shown = computed(() => !!summary.value && !!pair.value && props.status !== 'working' && dismissed.value !== pair.value.after.id)

function dismiss(): void {
  if (pair.value) dismissed.value = pair.value.after.id
  review.value = false
}

async function undo(): Promise<void> {
  const p = pair.value
  if (!p || !root.value || busy.value || !files.value.length) return
  busy.value = true
  const res = await cpRestore(root.value, p.before.id, files.value.map((f) => f.path))
  busy.value = false
  if (res.ok) {
    notice.value = { ok: true, text: t('paneV93.turn.undone') }
    files.value = []
    void store.refresh(root.value)
  } else if (res.error !== 'cancelled') notice.value = { ok: false, text: t(`workspace.checkpoints.error.${res.error}`, { detail: res.detail ?? '' }) }
}
</script>

<template>
  <Transition name="ccm-fade">
    <div
      v-if="shown && summary && pair"
      class="flex min-h-7 min-w-0 shrink-0 flex-wrap items-center gap-x-2 gap-y-1 rounded-card bg-[var(--ccm-control)] px-2 py-1 text-2xs"
      role="region"
      :aria-label="t('paneV93.turn.aria')"
      data-turn-card
    >
      <span class="font-semibold text-ink">{{ t('paneV93.turn.title') }}</span>
      <span class="text-ink-muted tabular-nums">{{ t('paneV93.turn.files', { n: summary.files }, summary.files) }}</span>
      <span class="tabular-nums"><span class="text-diff-add">+{{ summary.added }}</span> <span class="text-diff-del">−{{ summary.removed }}</span></span>
      <span class="flex min-w-0 flex-1 items-center gap-1.5 truncate font-mono text-ink-muted @max-[520px]:hidden">
        <span v-for="f in summary.top" :key="f.path" class="min-w-0 truncate" :title="f.path">{{ baseName(f.path) }}</span>
        <span v-if="summary.more" class="shrink-0 text-ink-faint">{{ t('paneV93.turn.more', { n: summary.more }) }}</span>
      </span>
      <span v-if="notice" class="text-2xs" :class="notice.ok ? 'text-ink-muted' : 'text-st-error'" role="status">{{ notice.text }}</span>
      <span class="ml-auto flex shrink-0 items-center gap-1">
        <button type="button" class="ccm-btn h-5 gap-1 px-1.5 text-2xs" data-turn-card-diff @click="review = true">
          <GitCompareArrows :size="12" aria-hidden="true" />{{ t('paneV93.turn.viewDiff') }}
        </button>
        <button v-if="hasCheckpoints" type="button" class="ccm-btn h-5 gap-1 px-1.5 text-2xs" :disabled="busy" :title="t('paneV93.turn.undoTitle')" data-turn-card-undo @click="undo">
          <RotateCcw :size="12" aria-hidden="true" />{{ t('paneV93.turn.undo') }}
        </button>
        <button type="button" class="ccm-btn-plain shrink-0" :title="t('paneV93.turn.dismiss')" :aria-label="t('paneV93.turn.dismiss')" data-turn-card-close @click="dismiss">
          <X :size="12" aria-hidden="true" />
        </button>
      </span>
      <TurnReview v-if="review && root" :root="root" :before="pair.before" :after="pair.after" :title="pair.title" @close="review = false" @open="(p: string) => emit('open', p)" />
    </div>
  </Transition>
</template>
