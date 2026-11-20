<script setup lang="ts">
import { GitBranch, SquareTerminal } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { activityKey, type ActivityItem } from '../lib/monitorModel'
import { answerChoices, askKind, askText } from '../lib/paneAnswer'
import { previewLines, type WsPane } from '../lib/workspaceModel'
import AgentTile from './AgentTile.vue'
import StatusChip from './StatusChip.vue'

const props = defineProps<{ pane: WsPane; lost?: boolean; selected?: boolean; large?: boolean; recent?: ActivityItem[]; canAnswer?: boolean; answering?: boolean }>()
const emit = defineEmits<{ open: []; answer: [keys: string] }>()
const { t } = useI18n()

function label(item: ActivityItem): string {
  const key = activityKey(item.kind)
  const head = key ? t(`act.${key}`) : item.kind
  return item.detail ? `${head} ${item.detail}` : head
}

const lines = computed(() => {
  const found = previewLines(props.pane, props.recent ?? [], label)
  return found.length ? found : [t(`status.${props.pane.rail}`)]
})
const kind = computed(() => askKind(props.pane))
const question = computed(() => askText(props.pane) ?? (kind.value ? t(`workspace.ask.${kind.value}`) : ''))
const choices = computed(() => answerChoices(props.pane))
</script>

<template>
  <div class="card pane" :class="[`p-${pane.provider}`, { selected, large, asking: kind }]" :data-testid="`pane-${pane.key}`" :data-rail="pane.rail">
    <span class="bar" aria-hidden="true" />
    <button type="button" class="hit" :data-testid="`pane-open-${pane.key}`" @click="emit('open')">
    <span class="head">
      <AgentTile v-if="large" :provider="pane.provider" :size="40" />
      <span class="who">
        <Transition name="swap" mode="out-in"><span :key="pane.title" class="title">{{ pane.title }}</span></Transition>
        <span v-if="large && pane.branch" class="branch mono"><GitBranch :size="14" :stroke-width="2" aria-hidden="true" /><span class="ellipsis">{{ pane.branch }}</span></span>
      </span>
      <StatusChip v-if="large" :rail="pane.rail" :lost="lost" />
    </span>
    <StatusChip v-if="!large" :rail="pane.rail" :lost="lost" />
    <span class="out mono" aria-hidden="true" :data-testid="`pane-preview-${pane.key}`">
      <span v-for="(line, i) in lines" :key="i" class="line">{{ line }}</span>
    </span>
    </button>
    <div v-if="kind" class="ask" role="group" :aria-label="t('workspace.ask.label', { name: pane.title })" :data-testid="`pane-ask-${pane.key}`" :data-kind="kind">
      <p class="q">{{ question }}</p>
      <div class="answers">
        <button v-for="c in choices" :key="c.keys" type="button" class="btn" :class="{ 'btn-primary': c.label === 'yes' }" :disabled="!canAnswer || answering" :data-testid="`pane-answer-${pane.key}-${c.keys}`" @click="emit('answer', c.keys)">{{ t(`workspace.ask.${c.label}`) }}</button>
        <button type="button" class="btn" :class="{ 'btn-primary': choices.length === 0 }" :disabled="!pane.canAttach" :data-testid="`pane-answer-${pane.key}-open`" @click="emit('open')"><SquareTerminal :size="18" :stroke-width="2" aria-hidden="true" />{{ t('workspace.ask.openPane') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pane {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 10px;
  width: 100%;
  min-width: 0;
  padding: 12px 12px 12px 16px;
  overflow: hidden;
  min-height: 44px;
}

.hit {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
  width: 100%;
  min-width: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
}

.ask {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}

.q {
  margin: 0;
  color: var(--ccm-ink);
  font-weight: 600;
  overflow-wrap: anywhere;
}

.answers {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.answers .btn {
  flex: 1 1 auto;
  min-height: 44px;
}

.bar {
  position: absolute;
  inset: 0 auto 0 0;
  width: 4px;
  background: var(--ccm-agent-claude);
}

.p-codex .bar {
  background: var(--ccm-agent-codex);
}

.p-shell .bar {
  background: var(--ccm-agent-shell);
}

.selected {
  border-color: var(--ccm-accent);
}

.head {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.who {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.swap-enter-active,
.swap-leave-active {
  transition: opacity 160ms var(--ease-out), transform 160ms var(--ease-out);
}

.swap-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.swap-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.large .title {
  font-size: 1.0625rem;
  font-weight: 700;
}

.branch {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.out {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  min-height: 56px;
  padding: 10px 12px;
  border-radius: var(--radius-control);
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-size: 0.75rem;
  line-height: 1.4;
}

.large .out {
  min-height: 140px;
  font-size: 0.8125rem;
}

.line {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
