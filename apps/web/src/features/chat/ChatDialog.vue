<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { MessageSquareText, X } from 'lucide-vue-next'
import { retireOverlay } from '../../core/lib/motion'
import ChatView from './ChatView.vue'
import { chipTarget, type ChatDialogRequest } from './commitSessions'
import type { ChatCall } from './feed'

const props = withDefaults(defineProps<{ request: ChatDialogRequest | null; call?: ChatCall }>(), { call: undefined })
const emit = defineEmits<{ close: []; openFile: [path: string] }>()
const { t } = useI18n()
const sheet = ref<HTMLElement | null>(null)
let restoreFocus: HTMLElement | null = null

const target = computed(() => (props.request ? chipTarget(props.request.chip) : null))
const agentName = computed(() => (props.request ? t(`chatCommit.agent.${props.request.chip.agent}`) : ''))

watch(
  () => props.request,
  async (req, old) => {
    if (!req) {
      if (restoreFocus?.isConnected) restoreFocus.focus()
      restoreFocus = null
      return
    }
    if (!old) restoreFocus = document.activeElement as HTMLElement | null
    await nextTick()
    sheet.value?.querySelector<HTMLElement>('[data-chat-dialog-close]')?.focus()
  },
)
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" @before-leave="(el) => retireOverlay(el.firstElementChild!)">
      <div v-if="request && target" class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[6vh] sm:px-4" @mousedown.self="emit('close')">
        <div
          ref="sheet"
          role="dialog"
          aria-modal="true"
          aria-labelledby="chat-dialog-title"
          class="ccm-sheet flex h-[82vh] w-full max-w-[760px] flex-col overflow-hidden"
          data-chat-dialog
          @keydown.esc.stop="emit('close')"
        >
          <header class="flex shrink-0 items-center gap-2 border-b border-line px-4 py-2.5">
            <MessageSquareText :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
            <div class="min-w-0 flex-1">
              <h2 id="chat-dialog-title" class="truncate text-sm font-semibold text-ink">{{ t('chatCommit.title', { agent: agentName, label: request.chip.label }) }}</h2>
              <p class="truncate text-xs text-ink-faint">{{ t('chatCommit.sub', { hash: request.hash.slice(0, 8) }) }}</p>
            </div>
            <button type="button" class="ccm-btn-plain shrink-0" :title="t('chatCommit.close')" :aria-label="t('chatCommit.close')" data-chat-dialog-close @click="emit('close')">
              <X :size="14" aria-hidden="true" />
            </button>
          </header>
          <div class="min-h-0 flex-1">
            <ChatView :target="target" :active="true" :agent-label="agentName" :call="call" :focus-turn="request.chip.turn" @open-file="(p) => emit('openFile', p)" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
