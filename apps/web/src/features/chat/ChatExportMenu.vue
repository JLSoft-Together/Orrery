<script setup lang="ts">
import { Check, ClipboardCopy, FileDown, LoaderCircle, Send, Share } from 'lucide-vue-next'
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTelegramStore } from '@features/telegram'
import type { ChatCall, ChatTarget } from './feed'
import { runExport, type ExportHow, type ExportOutcome } from './export'

const props = withDefaults(defineProps<{ target: ChatTarget; call?: ChatCall; telegram?: boolean }>(), { call: undefined, telegram: undefined })
const { t } = useI18n()
const tg = useTelegramStore()
tg.start()

const open = ref(false)
const busy = ref(false)
const outcome = ref<ExportOutcome | null>(null)
const trigger = ref<HTMLButtonElement | null>(null)
const menu = ref<HTMLElement | null>(null)
let timer: ReturnType<typeof setTimeout> | null = null

const canTelegram = computed(() => props.telegram ?? (tg.ready && tg.enabled))
const items = (): HTMLElement[] => [...(menu.value?.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)') ?? [])]

function close(refocus = true): void {
  open.value = false
  document.removeEventListener('pointerdown', onOutside, true)
  if (refocus) trigger.value?.focus()
}

function onOutside(e: PointerEvent): void {
  const node = e.target as Node
  if (!menu.value?.contains(node) && !trigger.value?.contains(node)) close(false)
}

async function toggle(): Promise<void> {
  if (open.value) return close()
  open.value = true
  document.addEventListener('pointerdown', onOutside, true)
  await nextTick()
  items()[0]?.focus()
}

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    return close()
  }
  if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') return
  e.preventDefault()
  const list = items()
  const at = list.indexOf(document.activeElement as HTMLElement)
  const next = e.key === 'Home' ? 0 : e.key === 'End' ? list.length - 1 : (at + (e.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length
  list[next]?.focus()
}

async function pick(how: ExportHow): Promise<void> {
  close()
  busy.value = true
  outcome.value = null
  const result = await runExport(props.target, how, { call: props.call })
  busy.value = false
  outcome.value = result
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => (outcome.value = null), 3000)
}

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onOutside, true)
  if (timer) clearTimeout(timer)
})

const rowClass =
  'flex w-full cursor-pointer items-center gap-2 rounded-inner px-2 py-1.5 text-left text-xs text-ink hover:bg-fill focus-visible:bg-accent focus-visible:text-on-accent focus-visible:outline-none'
</script>

<template>
  <div class="relative flex shrink-0 items-center gap-2" data-chat-export>
    <span v-if="busy" class="flex items-center gap-1 text-2xs text-ink-muted" role="status" data-chat-export-busy>
      <LoaderCircle :size="12" class="ccm-spin" aria-hidden="true" />{{ t('chatExport.working') }}
    </span>
    <span v-else-if="outcome" class="flex items-center gap-1 text-2xs" :class="outcome === 'failed' ? 'text-st-error' : 'text-ink-muted'" role="status" data-chat-export-result :data-outcome="outcome">
      <Check v-if="outcome !== 'failed' && outcome !== 'cancelled'" :size="12" aria-hidden="true" />{{ t(`chatExport.result.${outcome}`) }}
    </span>
    <button
      ref="trigger"
      type="button"
      class="ccm-btn h-6 gap-1 px-2 text-2xs"
      :disabled="busy"
      aria-haspopup="menu"
      :aria-expanded="open"
      data-chat-export-button
      @click="toggle"
    >
      <Share :size="12" aria-hidden="true" />{{ t('chatExport.button') }}
    </button>
    <Transition name="ccm-fade">
      <div v-if="open" ref="menu" class="ccm-menu ccm-pop absolute top-full right-0 z-30 mt-1 flex w-52 flex-col p-1" role="menu" :aria-label="t('chatExport.menu')" data-chat-export-menu @keydown="onKey">
        <button type="button" role="menuitem" :class="rowClass" data-chat-export-copy @click="pick('copy')">
          <ClipboardCopy :size="14" class="shrink-0" aria-hidden="true" />{{ t('chatExport.copy') }}
        </button>
        <button type="button" role="menuitem" :class="rowClass" data-chat-export-save @click="pick('save')">
          <FileDown :size="14" class="shrink-0" aria-hidden="true" />{{ t('chatExport.save') }}
        </button>
        <button v-if="canTelegram" type="button" role="menuitem" :class="rowClass" data-chat-export-telegram @click="pick('telegram')">
          <Send :size="14" class="shrink-0" aria-hidden="true" />{{ t('chatExport.telegram') }}
        </button>
      </div>
    </Transition>
  </div>
</template>
