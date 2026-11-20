<script setup lang="ts">
import { computed, h, ref, type VNode } from 'vue'
import { useI18n } from 'vue-i18n'
import { parseMarkdown, type Block, type Inline } from '../lib/markdown'

const props = defineProps<{ text: string }>()
const { t } = useI18n()
const blocks = computed(() => parseMarkdown(props.text))
const copied = ref<number | null>(null)

function inline(nodes: Inline[]): Array<VNode | string> {
  return nodes.map((n) => {
    switch (n.k) {
      case 'text':
        return n.v
      case 'code':
        return h('code', n.v)
      case 'strong':
        return h('strong', inline(n.c))
      case 'em':
        return h('em', inline(n.c))
      case 'link':
        return h('a', { href: n.href, target: '_blank', rel: 'noopener noreferrer' }, inline(n.c))
    }
  })
}

async function copy(index: number, text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = index
    setTimeout(() => {
      if (copied.value === index) copied.value = null
    }, 1500)
  } catch {
    copied.value = null
  }
}

function block(b: Block, index: number): VNode {
  switch (b.k) {
    case 'p':
      return h('p', inline(b.c))
    case 'h':
      return h(`h${Math.min(6, b.level + 2)}`, { class: 'mh' }, inline(b.c))
    case 'code':
      return h('div', { class: 'code', 'data-testid': 'chat-code' }, [
        h('div', { class: 'code-head' }, [
          h('span', b.lang || t('chat.code')),
          h('button', { type: 'button', onClick: () => void copy(index, b.text) }, copied.value === index ? t('chat.copied') : t('chat.copy')),
        ]),
        h('pre', [h('code', b.text)]),
      ])
    case 'list':
      return h(b.ordered ? 'ol' : 'ul', b.ordered && b.start !== 1 ? { start: b.start } : {}, b.items.map((item) => h('li', inline(item))))
    case 'quote':
      return h('blockquote', inline(b.c))
    case 'hr':
      return h('hr')
    case 'table':
      return h('div', { class: 'table' }, [
        h('table', [h('thead', [h('tr', b.head.map((c) => h('th', inline(c))))]), h('tbody', b.rows.map((r) => h('tr', r.map((c) => h('td', inline(c))))))]),
      ])
  }
}

const Render = () => h('div', { class: 'md' }, blocks.value.map((b, i) => block(b, i)))
</script>

<template>
  <Render />
</template>

<style scoped>
.md :deep(p) {
  margin: 0;
  white-space: pre-line;
}

.md > :deep(* + *) {
  margin-top: 14px;
}

.md :deep(.mh) {
  margin: 0;
  font-size: 1.02em;
  font-weight: 700;
  line-height: 1.4;
}

.md :deep(ul),
.md :deep(ol) {
  margin: 0;
  padding-left: 22px;
}

.md :deep(li + li) {
  margin-top: 6px;
}

.md :deep(li::marker) {
  color: var(--ccm-ink-muted);
}

.md :deep(code) {
  font-family: var(--font-mono);
  font-size: 0.82em;
  padding: 2px 6px;
  border-radius: 6px;
  background: var(--ccm-fill);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.md :deep(a) {
  color: var(--ccm-accent-ink);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.md :deep(blockquote) {
  margin: 0;
  padding-left: 14px;
  border-left: 2px solid var(--ccm-line-strong);
  color: var(--ccm-ink-muted);
}

.md :deep(hr) {
  border: 0;
  border-top: 1px solid var(--ccm-line);
}

.md :deep(.code) {
  border-radius: 20px;
  overflow: hidden;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.md :deep(.code-head) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 6px 0 16px;
  font-size: 12.5px;
  color: var(--ccm-ink-muted);
}

.md :deep(.code-head button) {
  min-height: 44px;
  padding: 0 12px;
  border: 0;
  border-radius: 22px;
  background: transparent;
  color: var(--ccm-ink-muted);
  font-size: 12.5px;
  font-weight: 600;
}

.md :deep(.code pre) {
  margin: 0;
  padding: 0 16px 16px;
  overflow-x: auto;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.6;
}

.md :deep(.code pre code) {
  padding: 0;
  background: none;
  box-shadow: none;
  font-size: inherit;
}

.md :deep(.table) {
  max-width: 100%;
  overflow-x: auto;
  border-radius: 14px;
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.md :deep(table) {
  border-collapse: collapse;
  font-size: 14px;
  line-height: 1.45;
}

.md :deep(th),
.md :deep(td) {
  padding: 8px 12px;
  border-bottom: 1px solid var(--ccm-line);
  text-align: left;
  vertical-align: top;
}

.md :deep(th) {
  font-weight: 600;
}
</style>
