<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { EyeOff, FileDiff, FileWarning, GitBranch } from 'lucide-vue-next'
import { useClashActions } from '../../composables/useClashActions'
import { othersOf, summarize } from '../../lib/clash'
import { useAgentTouchesStore } from '../../stores/agentTouches'

const WIDTH = 340

const props = defineProps<{ terminalId: string; canWorktree?: boolean }>()
const emit = defineEmits<{ worktree: [] }>()

const { t } = useI18n()
const touches = useAgentTouchesStore()
const actions = useClashActions()

const list = computed(() => touches.clashesOf.get(props.terminalId) ?? [])
const rows = computed(() =>
  list.value.map((c) => ({ clash: c, file: actions.fileName(c, props.terminalId), peers: othersOf(c, props.terminalId).map((p) => p.name).join(', ') })),
)
const label = computed(() => (list.value.length > 1 ? t('v104.clash.chipN', { n: list.value.length }) : t('v104.clash.chip')))
const tip = computed(() => {
  const { shown, more } = summarize(rows.value)
  return [...shown.map((r) => t('v104.clash.tip', { peer: r.peers, file: r.file })), ...(more ? [t('v104.clash.more', { n: more })] : [])].join('\n')
})

const button = ref<HTMLButtonElement | null>(null)
const pop = ref<HTMLElement | null>(null)
const anchor = ref<DOMRect | null>(null)

const style = computed(() => {
  const a = anchor.value
  if (!a) return {}
  const left = Math.max(12, Math.min(a.right - WIDTH, window.innerWidth - WIDTH - 12))
  return { left: `${left}px`, top: `${a.bottom + 6}px`, width: `min(${WIDTH}px, calc(100vw - 24px))` }
})

function toggle(): void {
  anchor.value = anchor.value ? null : (button.value?.getBoundingClientRect() ?? null)
}

function close(refocus = false): void {
  anchor.value = null
  if (refocus) button.value?.focus()
}

function onOutside(e: PointerEvent): void {
  const target = e.target as Node
  if (!pop.value?.contains(target) && !button.value?.contains(target)) close()
}

watch(anchor, async (a, before) => {
  if (a && !before) {
    document.addEventListener('pointerdown', onOutside, true)
    await nextTick()
    pop.value?.focus()
  } else if (!a) document.removeEventListener('pointerdown', onOutside, true)
})
watch(
  () => list.value.length,
  (n) => {
    if (!n) close()
  },
)
onBeforeUnmount(() => document.removeEventListener('pointerdown', onOutside, true))

function worktree(): void {
  close()
  emit('worktree')
}
</script>

<template>
  <span v-if="list.length" class="inline-flex min-w-0 shrink" data-pane-chip>
    <button
      ref="button"
      type="button"
      class="inline-flex h-5 max-w-full min-w-6 cursor-pointer items-center gap-1 rounded-inner bg-warning-soft px-1.5 text-2xs font-medium whitespace-nowrap text-warning transition-colors duration-(--ccm-dur-fast) ease-out-quint hover:text-ink"
      :title="tip"
      :aria-label="`${label}. ${tip}`"
      aria-haspopup="dialog"
      :aria-expanded="!!anchor"
      :data-clash-chip="list.length"
      data-no-drag
      @click="toggle"
    >
      <FileWarning :size="12" class="shrink-0" aria-hidden="true" />
      <span class="min-w-0 truncate @max-[520px]:hidden">{{ label }}</span>
      <span v-if="list.length > 1" class="hidden tabular-nums @max-[520px]:inline">{{ list.length }}</span>
    </button>
    <Teleport to="body">
      <Transition name="ccm-menu">
        <div
          v-if="anchor"
          ref="pop"
          class="ccm-menu glass-pop fixed z-50 flex flex-col gap-1 p-2 outline-none"
          :style="style"
          role="dialog"
          tabindex="-1"
          :aria-label="t('v104.clash.listTitle')"
          data-clash-pop
          @keydown.esc.stop="close(true)"
        >
          <p class="flex items-center gap-1.5 px-1.5 pt-0.5 pb-1 text-xs font-semibold text-ink">
            <FileWarning :size="13" class="shrink-0 text-warning" aria-hidden="true" />
            <span class="min-w-0 truncate">{{ t('v104.clash.listTitle') }}</span>
          </p>
          <ul class="flex max-h-72 flex-col gap-0.5 overflow-y-auto">
            <li v-for="r in rows" :key="r.clash.fileKey" class="flex min-w-0 items-center gap-2 rounded-inner px-1.5 py-1 hover:bg-fill" :data-clash-row="r.clash.fileKey">
              <span class="min-w-0 flex-1">
                <span class="block truncate font-mono text-2xs text-ink" :title="r.clash.path">{{ r.file }}</span>
                <span class="block truncate text-2xs text-ink-muted" :title="r.peers">{{ r.peers }}</span>
              </span>
              <button
                type="button"
                class="inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-inner text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill-strong hover:text-ink"
                :title="t('v104.clash.viewDiff')"
                :aria-label="`${t('v104.clash.viewDiff')}: ${r.file}`"
                data-clash-diff
                @click="actions.viewDiff(r.clash, terminalId); close()"
              >
                <FileDiff :size="13" aria-hidden="true" />
              </button>
              <button
                type="button"
                class="inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-inner text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill-strong hover:text-ink"
                :title="t('v104.clash.ignore')"
                :aria-label="`${t('v104.clash.ignore')}: ${r.file}`"
                data-clash-ignore
                @click="actions.ignore(r.clash, terminalId)"
              >
                <EyeOff :size="13" aria-hidden="true" />
              </button>
            </li>
          </ul>
          <button
            v-if="canWorktree"
            type="button"
            class="mt-0.5 inline-flex min-w-0 cursor-pointer items-center gap-1.5 rounded-inner border-t-[0.5px] border-line px-1.5 pt-1.5 pb-1 text-left text-2xs text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:text-ink"
            :title="t('v104.clash.worktreeHint')"
            data-clash-worktree
            @click="worktree"
          >
            <GitBranch :size="12" class="shrink-0" aria-hidden="true" />
            <span class="min-w-0 truncate">{{ t('v104.clash.worktree') }}</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </span>
</template>
