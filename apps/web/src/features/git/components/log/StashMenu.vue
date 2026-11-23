<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Archive, ArchiveRestore, CircleAlert, LoaderCircle, Trash2 } from 'lucide-vue-next'
import { hasStash, stashList, stashRun, type StashEntry } from '../../../../core/lib/preview'
import { stashErrorKey, stashTitle } from '../../lib/stash'

const props = defineProps<{ root: string | null; active: boolean }>()
const emit = defineEmits<{ changed: [] }>()
const { t } = useI18n()
const open = ref(false)
const items = ref<StashEntry[]>([])
const loading = ref(false)
const busy = ref<string | null>(null)
const note = ref<{ text: string; failed: boolean } | null>(null)
const rootEl = ref<HTMLElement | null>(null)
const panel = ref<HTMLElement | null>(null)
let token = 0

async function load(): Promise<void> {
  const root = props.root
  if (!hasStash || !root) {
    items.value = []
    return
  }
  const mine = ++token
  loading.value = true
  const res = await stashList(root)
  if (mine !== token) return
  loading.value = false
  if (res.ok) items.value = res.stashes
  else note.value = { text: t(stashErrorKey(res.error), { detail: res.detail ?? '' }), failed: true }
}

watch(
  () => [props.root, props.active] as const,
  ([root, active], prev) => {
    if (root !== prev?.[0]) {
      items.value = []
      note.value = null
      open.value = false
    }
    if (root && active) void load()
  },
  { immediate: true },
)

const count = computed(() => items.value.length)

function toggle(): void {
  open.value = !open.value
  if (!open.value) return
  note.value = null
  void load()
  void nextTick(() => panel.value?.querySelector<HTMLElement>('button')?.focus())
}

function away(e: PointerEvent): void {
  if (open.value && !rootEl.value?.contains(e.target as Node)) open.value = false
}

function key(e: KeyboardEvent): void {
  if (e.key === 'Escape' && open.value) {
    e.stopPropagation()
    open.value = false
    rootEl.value?.querySelector<HTMLElement>('[data-stash-menu-button]')?.focus()
  }
}

window.addEventListener('pointerdown', away, true)
onBeforeUnmount(() => window.removeEventListener('pointerdown', away, true))

async function run(op: 'apply' | 'pop' | 'drop', s: StashEntry): Promise<void> {
  const root = props.root
  if (!root || busy.value) return
  busy.value = s.hash
  note.value = null
  const res = await stashRun(root, { op, hash: s.hash })
  busy.value = null
  if (res.ok) {
    note.value = { text: t(`v112stash.done.${op}`, { name: stashTitle(s) }), failed: false }
    emit('changed')
  } else if (res.error !== 'cancelled') {
    note.value = { text: t(stashErrorKey(res.error), { detail: res.detail ?? '' }), failed: true }
  }
  await load()
}

const when = (s: StashEntry): string => new Date(s.time * 1000).toLocaleString()
</script>

<template>
  <div v-if="hasStash && root" ref="rootEl" class="relative shrink-0" data-stash-menu @keydown="key">
    <button
      type="button"
      class="ccm-btn-plain h-6 shrink-0 gap-1 px-1.5"
      aria-haspopup="menu"
      :aria-expanded="open"
      :title="t('v112stash.listTitle')"
      :aria-label="t('v112stash.listTitle')"
      data-stash-menu-button
      @click="toggle"
    >
      <Archive :size="14" aria-hidden="true" />
      <span v-if="count" class="rounded-full bg-fill px-1.5 text-2xs tabular text-ink-muted" data-stash-count>{{ count }}</span>
    </button>
    <Transition name="ccm-menu">
      <div v-if="open" ref="panel" class="ccm-menu ccm-scroll absolute top-full right-0 z-30 mt-1 flex max-h-[60vh] w-80 max-w-[calc(100vw-24px)] flex-col overflow-y-auto p-1" style="--ccm-origin: top right" role="menu" :aria-label="t('v112stash.listTitle')" data-stash-panel>
        <p v-if="loading && !items.length" class="flex items-center gap-1.5 px-2 py-2 text-xs text-ink-muted"><LoaderCircle :size="12" class="ccm-spin" aria-hidden="true" />{{ t('v112stash.loading') }}</p>
        <p v-else-if="!items.length" class="px-2 py-2 text-xs text-ink-muted" data-stash-empty>{{ t('v112stash.empty') }}</p>
        <ul v-else class="flex flex-col">
          <li v-for="s in items" :key="s.hash" class="flex flex-col gap-1 rounded-inner px-2 py-1.5 hover:bg-fill" data-stash-row>
            <div class="flex min-w-0 items-center gap-1.5">
              <ArchiveRestore :size="13" class="shrink-0 text-ink-faint" aria-hidden="true" />
              <span class="min-w-0 flex-1 truncate text-xs font-medium text-ink" :title="stashTitle(s)">{{ stashTitle(s) }}</span>
              <span v-if="s.untracked" class="shrink-0 rounded-full bg-fill px-1.5 text-2xs text-ink-muted">{{ t('v112stash.untrackedTag') }}</span>
            </div>
            <p class="truncate text-2xs text-ink-faint" :title="when(s)">{{ s.branch ? `${s.branch}, ` : '' }}{{ when(s) }}</p>
            <div class="flex items-center gap-1">
              <button type="button" role="menuitem" class="ccm-btn h-6 px-2 text-2xs" :disabled="!!busy" data-stash-apply @click="run('apply', s)">{{ t('v112stash.apply') }}</button>
              <button type="button" role="menuitem" class="ccm-btn h-6 px-2 text-2xs" :disabled="!!busy" data-stash-pop @click="run('pop', s)">{{ t('v112stash.pop') }}</button>
              <span class="flex-1" />
              <LoaderCircle v-if="busy === s.hash" :size="12" class="ccm-spin text-ink-faint" aria-hidden="true" />
              <button type="button" role="menuitem" class="ccm-btn-plain h-6 gap-1 px-2 text-2xs text-st-error" :disabled="!!busy" data-stash-drop @click="run('drop', s)">
                <Trash2 :size="12" aria-hidden="true" />{{ t('v112stash.drop') }}
              </button>
            </div>
          </li>
        </ul>
        <p v-if="note" class="mt-1 flex items-start gap-1.5 border-t border-line px-2 pt-1.5 pb-1 text-2xs" :class="note.failed ? 'text-st-error' : 'text-ink-muted'" role="status" data-stash-note>
          <CircleAlert v-if="note.failed" :size="12" class="mt-px shrink-0" aria-hidden="true" />
          <span class="min-w-0 flex-1 break-words">{{ note.text }}</span>
        </p>
      </div>
    </Transition>
  </div>
</template>
