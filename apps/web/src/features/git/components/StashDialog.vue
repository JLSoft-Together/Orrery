<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Archive, X } from 'lucide-vue-next'
import { retireOverlay } from '../../../core/lib/motion'

const props = defineProps<{ request: { name: string; branch: string | null; hasUntracked: boolean; busy: boolean } | null }>()
const emit = defineEmits<{ submit: [name: string, untracked: boolean]; close: [] }>()
const { t } = useI18n()
const name = ref('')
const untracked = ref(false)
const sheet = ref<HTMLElement | null>(null)
const input = ref<HTMLInputElement | null>(null)
let restoreFocus: HTMLElement | null = null

watch(
  () => props.request,
  async (req, prev) => {
    if (!req) {
      if (restoreFocus?.isConnected) restoreFocus.focus()
      restoreFocus = null
      return
    }
    if (prev) return
    restoreFocus = document.activeElement as HTMLElement | null
    name.value = req.name
    untracked.value = false
    await nextTick()
    input.value?.focus()
    input.value?.select()
  },
)

function submit(): void {
  if (props.request && !props.request.busy) emit('submit', name.value, untracked.value && props.request.hasUntracked)
}

function trap(e: KeyboardEvent): void {
  if (e.key !== 'Tab' || !sheet.value) return
  const items = [...sheet.value.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled])')].filter((n) => n.offsetParent)
  const at = items.indexOf(document.activeElement as HTMLElement)
  e.preventDefault()
  items[(at + (e.shiftKey ? -1 : 1) + items.length) % items.length]?.focus()
}
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" @before-leave="(el) => retireOverlay(el.firstElementChild!)">
      <div v-if="request" class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[8vh] sm:px-4 sm:pt-[12vh]" @mousedown.self="emit('close')">
        <div ref="sheet" role="dialog" aria-modal="true" aria-labelledby="v112-stash-title" class="ccm-sheet flex w-full max-w-[420px] flex-col overflow-hidden" data-stash-dialog @keydown.esc.stop="emit('close')" @keydown="trap">
          <header class="flex items-center gap-2 border-b border-line px-4 py-3">
            <Archive :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
            <div class="min-w-0 flex-1">
              <h2 id="v112-stash-title" class="truncate text-sm font-semibold text-ink">{{ t('v112stash.dialogTitle') }}</h2>
              <p v-if="request.branch" class="truncate text-xs text-ink-faint" :title="request.branch">{{ t('v112stash.dialogBranch', { branch: request.branch }) }}</p>
            </div>
            <button type="button" class="ccm-btn-plain shrink-0" :title="t('v112stash.cancel')" :aria-label="t('v112stash.cancel')" @click="emit('close')">
              <X :size="14" aria-hidden="true" />
            </button>
          </header>
          <form class="flex flex-col gap-2 px-4 py-3" novalidate @submit.prevent="submit">
            <label class="flex flex-col gap-1 text-xs font-medium text-ink-muted">
              {{ t('v112stash.nameLabel') }}
              <input ref="input" v-model="name" type="text" maxlength="200" spellcheck="false" autocomplete="off" class="ccm-field h-8 text-sm" data-stash-name />
            </label>
            <label v-if="request.hasUntracked" class="flex cursor-pointer items-center gap-2 text-xs text-ink">
              <input v-model="untracked" type="checkbox" class="accent-[var(--ccm-accent)]" data-stash-untracked />{{ t('v112stash.untracked') }}
            </label>
            <p class="text-2xs text-ink-faint">{{ t('v112stash.dialogHint') }}</p>
            <div class="flex justify-end gap-2">
              <button type="button" class="ccm-btn" @click="emit('close')">{{ t('v112stash.cancel') }}</button>
              <button type="submit" class="ccm-btn ccm-btn-primary" :disabled="request.busy" data-stash-submit>{{ t('v112stash.stash') }}</button>
            </div>
          </form>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
