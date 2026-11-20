<script setup lang="ts">
import { ChevronRight, TriangleAlert, Wrench } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RemoteChatTools } from '../../../../packages/shared/src/remote'
import { capitalize, toolParts } from '../lib/chatModel'

const props = defineProps<{ item: RemoteChatTools }>()
const { t } = useI18n()
const open = ref(false)

const sentence = computed(() => {
  const parts = toolParts(props.item.sum).map((p) => t(`chat.tool.${p.cat}`, { n: p.n }, p.n))
  if (parts.length === 0) {
    const n = props.item.items.length + (props.item.more ?? 0)
    parts.push(t('chat.tool.other', { n }, n))
  }
  return capitalize(t('chat.tools', { list: parts.join(', ') }))
})
const failed = computed(() => props.item.items.some((x) => x.err))
</script>

<template>
  <div class="note" data-testid="chat-tools" :data-id="item.id">
    <button type="button" class="tool-line" :aria-expanded="open" data-testid="chat-tools-toggle" @click="open = !open">
      <Wrench :size="16" :stroke-width="2" aria-hidden="true" />
      <span>{{ sentence }}</span>
      <TriangleAlert v-if="failed" class="err" :size="15" :stroke-width="2.2" :aria-label="t('chat.toolErr')" />
      <ChevronRight class="chev" :size="14" :stroke-width="2.2" aria-hidden="true" />
    </button>
    <ul v-if="open" class="tool-list" data-testid="chat-tools-list">
      <li v-for="(x, i) in item.items" :key="i" :class="{ bad: x.err }">
        <span>{{ x.tool }}</span><template v-if="x.target"> <code>{{ x.target }}</code></template><span v-if="x.err" class="err-text"> {{ t('chat.toolErr') }}</span>
      </li>
      <li v-if="item.more" class="more">{{ t('chat.toolMore', { n: item.more }) }}</li>
    </ul>
  </div>
</template>

<style scoped>
.note {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  margin: -8px 0;
}

.tool-line {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 44px;
  max-width: 100%;
  padding: 0 12px 0 0;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
  font-size: 0.875rem;
  text-align: left;
}

.chev {
  flex: none;
  transition: transform var(--ccm-dur-fast) var(--ease-out);
}

.tool-line[aria-expanded='true'] .chev {
  transform: rotate(90deg);
}

.err,
.err-text {
  color: var(--ccm-error);
}

.tool-list {
  margin: 0 0 4px 7px;
  padding: 2px 0 2px 16px;
  list-style: none;
  border-left: 1.5px solid var(--ccm-line-strong);
  font-size: 0.84rem;
  color: var(--ccm-ink-muted);
  overflow-wrap: anywhere;
}

.tool-list li {
  padding: 3px 0;
}

.tool-list code {
  font-family: var(--font-mono);
  font-size: 0.78rem;
  color: var(--ccm-ink);
}
</style>
