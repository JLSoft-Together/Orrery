<script setup lang="ts">
import { computed, h, onBeforeUnmount, ref, type VNode } from 'vue'
import { useI18n } from 'vue-i18n'
import { parseMarkdown, type Block, type Inline } from './markdown'

const props = defineProps<{ text: string }>()
const { t } = useI18n()
const blocks = computed(() => parseMarkdown(props.text))
const copied = ref<number | null>(null)
let timer: ReturnType<typeof setTimeout> | null = null

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})

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
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
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
      return h('div', { class: 'code', 'data-chat-code': '' }, [
        h('div', { class: 'code-head' }, [
          h('span', b.lang || t('chatView.code')),
          h('button', { type: 'button', class: 'copy', 'data-chat-copy': '', onClick: () => void copy(index, b.text) }, copied.value === index ? t('chatView.copied') : t('chatView.copy')),
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
.md {
  min-width: 0;
  overflow-wrap: anywhere;
}

.md :deep(p) {
  margin: 0;
  white-space: pre-line;
}

.md > :deep(* + *) {
  margin-top: 8px;
}

.md :deep(.mh) {
  margin: 0;
  font-size: 1em;
  font-weight: 650;
}

.md :deep(ul),
.md :deep(ol) {
  margin: 0;
  padding-left: 20px;
}

.md :deep(ul) {
  list-style: disc;
}

.md :deep(ol) {
  list-style: decimal;
}

.md :deep(li + li) {
  margin-top: 2px;
}

.md :deep(li::marker) {
  color: var(--ccm-ink-muted);
}

.md :deep(code) {
  font-family: var(--font-mono);
  font-size: 0.9em;
  padding: 1px 4px;
  border-radius: var(--radius-inner);
  background: var(--ccm-fill);
}

.md :deep(a) {
  color: var(--ccm-accent-ink);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.md :deep(blockquote) {
  margin: 0;
  padding-left: 10px;
  border-left: 2px solid var(--ccm-line-strong);
  color: var(--ccm-ink-muted);
}

.md :deep(hr) {
  border: 0;
  border-top: 1px solid var(--ccm-line);
}

.md :deep(.code) {
  overflow: hidden;
  border-radius: var(--radius-control);
  background: var(--ccm-control);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.md :deep(.code-head) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 24px;
  padding: 0 4px 0 10px;
  font-size: 11px;
  color: var(--ccm-ink-muted);
}

.md :deep(.copy) {
  height: 20px;
  padding: 0 8px;
  border-radius: var(--radius-inner);
  color: var(--ccm-ink-muted);
  font-weight: 600;
  cursor: pointer;
  transition: background-color var(--ccm-dur-fast) ease-out;
}

.md :deep(.copy:hover) {
  background: var(--ccm-fill);
  color: var(--ccm-ink);
}

.md :deep(.code pre) {
  margin: 0;
  padding: 0 10px 8px;
  overflow-x: auto;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
}

.md :deep(.code pre code) {
  padding: 0;
  background: none;
  font-size: inherit;
}

.md :deep(.table) {
  max-width: 100%;
  overflow-x: auto;
  border-radius: var(--radius-inner);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.md :deep(table) {
  border-collapse: collapse;
}

.md :deep(th),
.md :deep(td) {
  padding: 4px 8px;
  border-bottom: 1px solid var(--ccm-line);
  text-align: left;
  vertical-align: top;
}

.md :deep(th) {
  font-weight: 600;
}
</style>
