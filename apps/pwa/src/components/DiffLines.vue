<script setup lang="ts">
import type { DiffLine } from '../lib/viewerModel'

defineProps<{ lines: DiffLine[] }>()
</script>

<template>
  <div class="diff" data-testid="diff-view">
    <div v-for="(line, i) in lines" :key="i" class="row" :class="line.kind" :data-kind="line.kind">
      <template v-if="line.kind === 'hunk'">
        <span class="txt">{{ line.text }}</span>
      </template>
      <template v-else>
        <span class="no">{{ line.kind === 'add' ? line.newNo : (line.oldNo ?? line.newNo) }}</span>
        <span class="sign" aria-hidden="true">{{ line.kind === 'add' ? '+' : line.kind === 'del' ? '−' : '' }}</span>
        <span class="txt">{{ line.text }}</span>
      </template>
    </div>
  </div>
</template>

<style scoped>
.diff {
  min-width: max-content;
  padding: 4px 0 10px;
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  line-height: 1.55;
}

.row {
  display: flex;
  gap: 8px;
  padding: 0 14px 0 8px;
}

.no {
  flex: none;
  min-width: 32px;
  text-align: right;
  color: var(--ccm-term-faint);
  user-select: none;
}

.sign {
  flex: none;
  width: 12px;
}

.txt {
  white-space: pre;
}

.add {
  background: var(--ccm-diff-add-fill);
}

.add .sign {
  color: var(--ccm-diff-add);
}

.del {
  background: var(--ccm-diff-del-fill);
}

.del .sign,
.del .no {
  color: var(--ccm-diff-del);
}

.hunk {
  margin: 4px 0;
  padding-top: 2px;
  padding-bottom: 2px;
  background: var(--ccm-diff-hunk-fill);
  color: var(--ccm-diff-hunk);
}
</style>
