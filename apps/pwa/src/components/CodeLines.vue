<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ text: string }>()
const { t } = useI18n()
const LIMIT = 4000
const all = computed(() => props.text.replace(/\r\n/g, '\n').split('\n'))
const lines = computed(() => all.value.slice(0, LIMIT))
</script>

<template>
  <div class="code" data-testid="code-view" :data-lines="all.length">
    <div v-for="(line, i) in lines" :key="i" class="row">
      <span class="no">{{ i + 1 }}</span>
      <span class="txt">{{ line }}</span>
    </div>
    <p v-if="all.length > LIMIT" class="cut">{{ t('viewer.cut', { n: LIMIT }) }}</p>
  </div>
</template>

<style scoped>
.code {
  min-width: max-content;
  padding: 10px 0;
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-family: var(--font-mono);
  font-size: 0.8125rem;
  line-height: 1.55;
}

.row {
  display: flex;
  gap: 12px;
  padding: 0 14px 0 8px;
}

.no {
  flex: none;
  min-width: 32px;
  text-align: right;
  color: var(--ccm-term-faint);
  user-select: none;
}

.txt {
  white-space: pre;
}

.cut {
  margin: 8px 14px;
  color: var(--ccm-term-faint);
}
</style>
