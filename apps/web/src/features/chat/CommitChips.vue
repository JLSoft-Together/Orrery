<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Bot } from 'lucide-vue-next'
import type { CommitSessionChip, CommitSessions } from '@ccm/shared'
import { chipPercent } from './commitSessions'

defineProps<{ sessions: CommitSessions }>()
const emit = defineEmits<{ open: [chip: CommitSessionChip] }>()
const { t } = useI18n()

function name(c: CommitSessionChip): string {
  const agent = t(`chatCommit.agent.${c.agent}`)
  return c.label ? `${agent} ${c.label}` : agent
}

function tip(c: CommitSessionChip): string {
  const head = t(c.strong ? 'chatCommit.strong' : 'chatCommit.weak', { agent: t(`chatCommit.agent.${c.agent}`), label: c.label, pct: chipPercent(c) })
  const lines = [head]
  if (c.prompt) lines.push(t('chatCommit.prompt', { text: c.prompt }))
  lines.push(t('chatCommit.open'))
  return lines.join('\n')
}
</script>

<template>
  <span class="inline-flex shrink-0 items-center gap-1" data-commit-chips>
    <button
      v-for="c in sessions.chips"
      :key="`${c.agent}:${c.session}`"
      type="button"
      class="inline-flex max-w-32 min-w-0 cursor-pointer items-center gap-0.5 rounded-inner border px-1 text-2xs leading-4 transition-colors duration-(--ccm-dur-fast) focus-visible:ring-2 focus-visible:ring-focus focus-visible:outline-none"
      :class="c.strong ? 'border-transparent bg-accent-soft text-accent-ink hover:bg-accent hover:text-on-accent' : 'border-dashed border-line bg-transparent text-ink-muted opacity-80 hover:bg-fill hover:text-ink hover:opacity-100'"
      :title="tip(c)"
      :aria-label="tip(c)"
      :data-commit-chip="c.strong ? 'strong' : 'weak'"
      :data-session="c.session"
      :data-turn="c.turn"
      @click.stop="emit('open', c)"
      @dblclick.stop
    >
      <Bot :size="10" class="shrink-0" aria-hidden="true" />
      <span class="truncate">{{ name(c) }}</span>
      <span v-if="!c.strong" class="shrink-0 font-semibold" aria-hidden="true">?</span>
    </button>
    <span v-if="sessions.more > 0" class="shrink-0 text-2xs text-ink-faint" :title="t('chatCommit.more', { n: sessions.more })" data-commit-chips-more>+{{ sessions.more }}</span>
  </span>
</template>
