<script setup lang="ts">
import { ChevronRight, Pencil } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RemoteChatFile, RemoteChatFiles } from '../../../../packages/shared/src/remote'
import { fileTotals, splitPath } from '../lib/chatModel'

const props = defineProps<{ item: RemoteChatFiles }>()
const emit = defineEmits<{ open: [file: RemoteChatFile, root: string] }>()
const { t } = useI18n()
const totals = computed(() => fileTotals(props.item.files))
const count = computed(() => props.item.files.length + (props.item.more ?? 0))
</script>

<template>
  <div class="files" data-testid="chat-files" :data-id="item.id">
    <div class="files-head">
      <Pencil :size="17" :stroke-width="2" aria-hidden="true" />
      <span>{{ t('chat.files', { n: count }, count) }}</span>
      <span class="sum"><span class="add">+{{ totals.add }}</span><span v-if="totals.del" class="del"> -{{ totals.del }}</span></span>
    </div>
    <button v-for="f in item.files" :key="f.path" type="button" class="f-row" :disabled="f.out" :data-testid="`chat-file-${f.path}`" @click="emit('open', f, item.root)">
      <span class="f-name"><span class="dir">{{ splitPath(f.path).dir }}</span>{{ splitPath(f.path).name }}</span>
      <span v-if="f.out" class="out">{{ t('chat.outside') }}</span>
      <template v-else>
        <span v-if="f.add" class="add">+{{ f.add }}</span>
        <span v-if="f.del" class="del">-{{ f.del }}</span>
        <ChevronRight :size="16" :stroke-width="2" aria-hidden="true" />
      </template>
    </button>
    <p v-if="item.more" class="more">{{ t('chat.filesMore', { n: item.more }) }}</p>
  </div>
</template>

<style scoped>
.files {
  overflow: hidden;
  border-radius: 24px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
}

.files-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 50px;
  padding: 0 18px;
  font-size: 0.9rem;
  font-weight: 600;
}

.files-head svg {
  color: var(--ccm-ink-muted);
}

.sum {
  margin-left: auto;
}

.add,
.del {
  font-family: var(--font-mono);
  font-size: 0.78rem;
  font-variant-numeric: tabular-nums;
}

.add {
  color: var(--ccm-diff-add);
}

.del {
  color: var(--ccm-diff-del);
}

.f-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 46px;
  padding: 0 14px 0 18px;
  border: 0;
  border-top: 1px solid var(--ccm-line);
  background: transparent;
  color: var(--ccm-ink);
  text-align: left;
}

.f-row:disabled {
  opacity: 1;
  color: var(--ccm-ink-muted);
}

.f-row svg {
  flex: none;
  color: var(--ccm-ink-muted);
}

.f-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 0.8125rem;
}

.dir,
.out {
  color: var(--ccm-ink-muted);
}

.out {
  font-size: 0.78rem;
}

.more {
  margin: 0;
  padding: 10px 18px 12px;
  border-top: 1px solid var(--ccm-line);
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}
</style>
