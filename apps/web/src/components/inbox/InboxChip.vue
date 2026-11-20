<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Inbox } from 'lucide-vue-next'
import { isMac } from '../../lib/platform'
import { askedShortcutText, shortcutText } from '../../lib/shortcuts'
import { useInboxStore } from '../../stores/inbox'
import { useUiGenStore } from '../../stores/uiGen'

const { t } = useI18n()
const inbox = useInboxStore()
const uiGen = useUiGenStore()
const chip = ref<HTMLButtonElement | null>(null)

const TONE = { waiting: 'text-st-waiting!', error: 'text-st-error!', warning: 'text-warning!', muted: 'text-ink-muted!' } as const
const tip = computed(() => t('v104.inbox.chipTip', { keys: shortcutText('wsAttention', isMac), next: askedShortcutText(isMac) }))

function onClick(): void {
  inbox.toggle(chip.value)
}
</script>

<template>
  <button
    v-if="inbox.count"
    ref="chip"
    type="button"
    class="ccm-btn shrink-0"
    :class="TONE[inbox.tone]"
    :title="tip"
    aria-haspopup="dialog"
    :aria-expanded="inbox.open && inbox.trigger === chip"
    :data-tone="inbox.tone"
    data-inbox-chip
    @click="onClick"
  >
    <Inbox :size="14" aria-hidden="true" />
    <span class="tabular">{{ t('v104.inbox.chip', { n: inbox.count }) }}</span>
    <kbd v-if="uiGen.isNext" class="ccm-next-kbd" aria-hidden="true">{{ shortcutText('wsAsked', isMac) }}</kbd>
  </button>
</template>
