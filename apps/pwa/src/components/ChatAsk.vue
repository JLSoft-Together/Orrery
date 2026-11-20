<script setup lang="ts">
import { Check, List, Lock, SquareTerminal, Undo2 } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RemoteMenuItem } from '../../../../packages/shared/src/remote'
import { ANSWER_UNDO_MS, answerChoices, askKind, askText } from '../lib/paneAnswer'
import type { WsPane } from '../lib/workspaceModel'

export interface AskPick {
  keys: string
  label: string
}

const props = defineProps<{ pane: WsPane; cmd: string | null; enabled: boolean; pending: string | null }>()
const emit = defineEmits<{ pick: [pick: AskPick]; undo: []; terminal: [] }>()
const { t } = useI18n()

const menu = computed(() => props.pane.menu ?? null)
const kind = computed(() => (menu.value ? (menu.value.kind === 'permission' || menu.value.kind === 'codex-approval' ? 'permission' : 'menu') : askKind(props.pane)))
const question = computed(() => {
  const q = menu.value?.question?.trim()
  if (q) return q
  const text = askText(props.pane)
  if (text) return text
  return t(kind.value === 'permission' ? 'chat.ask.permission' : 'chat.ask.question')
})
const grid = computed(() => kind.value === 'permission' && (menu.value?.items.length ?? 0) <= 3)
const fallback = computed(() => (menu.value ? [] : answerChoices(props.pane)))

function pickItem(item: RemoteMenuItem): void {
  if (item.other) {
    emit('terminal')
    return
  }
  emit('pick', { keys: `menu:${item.index}`, label: item.label })
}
</script>

<template>
  <div v-if="pending !== null" class="sent" role="status" data-testid="chat-ask-sent">
    <div class="sent-row">
      <Check :size="18" :stroke-width="2.4" aria-hidden="true" />
      <span class="sent-text">{{ t('chat.sent') }} <b>{{ pending }}</b></span>
      <button type="button" class="undo" data-testid="chat-ask-undo" @click="emit('undo')"><Undo2 :size="16" :stroke-width="2.2" aria-hidden="true" />{{ t('chat.undo') }}</button>
    </div>
    <div class="undo-bar" aria-hidden="true"><i :style="{ animationDuration: `${ANSWER_UNDO_MS}ms` }" /></div>
  </div>
  <div v-else class="ask" data-testid="chat-ask" :data-kind="kind ?? 'question'">
    <div class="ask-head">
      <Lock v-if="kind === 'permission'" :size="16" :stroke-width="2.2" aria-hidden="true" />
      <List v-else :size="16" :stroke-width="2.2" aria-hidden="true" />
      <span>{{ question }}</span>
    </div>
    <code v-if="kind === 'permission' && cmd" class="cmd">{{ cmd }}</code>
    <div v-if="menu && grid" class="ask-btns">
      <button
        v-for="(item, i) in menu.items"
        :key="item.index"
        type="button"
        class="btn-a"
        :class="{ primary: i === 0, span2: i === 2 }"
        :disabled="!enabled"
        :data-testid="`chat-ask-item-${item.index}`"
        @click="pickItem(item)"
      >
        {{ item.label }}
      </button>
    </div>
    <div v-else-if="menu" class="ask-opts">
      <button v-for="item in menu.items" :key="item.index" type="button" class="btn-a opt" :disabled="!enabled" :aria-pressed="item.checked === undefined ? undefined : item.checked" :data-testid="`chat-ask-item-${item.index}`" @click="pickItem(item)">
        <span class="num" aria-hidden="true">{{ item.other ? '…' : item.index }}</span>
        <span class="opt-text">{{ item.label }}</span>
        <SquareTerminal v-if="item.other" :size="16" :stroke-width="2" aria-hidden="true" />
      </button>
    </div>
    <div v-else-if="fallback.length" class="ask-btns">
      <button v-for="(c, i) in fallback" :key="c.keys" type="button" class="btn-a" :class="{ primary: i === 0 }" :disabled="!enabled" :data-testid="`chat-ask-${c.label}`" @click="emit('pick', { keys: c.keys, label: t(`chat.ask.${c.label}`) })">
        {{ t(`chat.ask.${c.label}`) }}
      </button>
    </div>
    <button v-else type="button" class="btn-a full" data-testid="chat-ask-terminal" @click="emit('terminal')">
      <SquareTerminal :size="18" :stroke-width="2" aria-hidden="true" />{{ t('chat.ask.openTerm') }}
    </button>
  </div>
</template>

<style scoped>
.ask {
  padding: 16px;
  border-radius: 26px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.ask-head {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 0.875rem;
  font-weight: 700;
  color: var(--ccm-waiting);
  overflow-wrap: anywhere;
}

.ask-head svg {
  flex: none;
  margin-top: 2px;
}

.cmd {
  display: block;
  margin-top: 12px;
  padding: 10px 14px;
  border-radius: 14px;
  background: var(--ccm-canvas);
  font-family: var(--font-mono);
  font-size: 0.875rem;
  color: var(--ccm-ink);
  overflow-wrap: anywhere;
}

.ask-btns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-top: 14px;
}

.ask-opts {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 12px;
}

.btn-a {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 46px;
  padding: 0 18px;
  border: 0;
  border-radius: 23px;
  background: var(--ccm-canvas);
  box-shadow: inset 0 0 0 1px var(--ccm-line-strong);
  color: var(--ccm-ink);
  font-size: 0.94rem;
  font-weight: 600;
  overflow-wrap: anywhere;
}

.btn-a.primary {
  background: var(--ccm-ink);
  color: var(--ccm-canvas);
  box-shadow: none;
}

.btn-a.span2,
.btn-a.full {
  grid-column: span 2;
}

.btn-a.full {
  width: 100%;
  margin-top: 14px;
}

.btn-a.opt {
  justify-content: flex-start;
  padding: 8px 14px;
  border-radius: 18px;
  text-align: left;
}

.btn-a[aria-pressed='true'] {
  box-shadow: inset 0 0 0 2px var(--ccm-ink);
}

.btn-a:disabled {
  opacity: 0.45;
}

.opt-text {
  flex: 1;
  min-width: 0;
}

.num {
  display: inline-grid;
  place-items: center;
  flex: none;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--ccm-fill);
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--ccm-ink-muted);
}

.sent {
  overflow: hidden;
  border-radius: 24px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.sent-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 54px;
  padding: 0 6px 0 18px;
}

.sent-row > svg {
  flex: none;
  color: var(--ccm-done);
}

.sent-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.undo {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: none;
  min-height: 44px;
  margin-left: auto;
  padding: 0 14px;
  border: 0;
  border-radius: 22px;
  background: transparent;
  color: var(--ccm-ink);
  font-weight: 700;
}

.undo-bar {
  height: 3px;
  background: var(--ccm-line);
}

.undo-bar i {
  display: block;
  height: 100%;
  background: var(--ccm-accent);
  transform-origin: left;
  animation: shrink linear forwards;
}

@keyframes shrink {
  from {
    transform: scaleX(1);
  }
  to {
    transform: scaleX(0);
  }
}
</style>
