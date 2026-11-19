<script setup lang="ts">
import { GitCommitHorizontal } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

defineProps<{ can: boolean; busy: boolean }>()
const message = defineModel<string>({ required: true })
const emit = defineEmits<{ commit: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="box" data-testid="commit-box">
    <input v-model="message" class="input" type="text" enterkeyhint="done" autocomplete="off" autocapitalize="sentences" :placeholder="t('git.message')" :aria-label="t('git.message')" data-testid="commit-message" @keydown.enter.prevent="can && !busy && emit('commit')" />
    <button type="button" class="btn btn-primary" :disabled="!can || busy" data-testid="commit-go" @click="emit('commit')"><GitCommitHorizontal :size="20" :stroke-width="2" aria-hidden="true" />{{ t('git.commit') }}</button>
  </div>
</template>

<style scoped>
.box {
  display: flex;
  gap: 8px;
  padding: 10px 16px calc(10px + env(safe-area-inset-bottom));
  border-top: 1px solid var(--ccm-line);
  flex: none;
}

.input {
  flex: 1;
  min-width: 0;
  min-height: 44px;
  padding: 0 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-size: 16px;
}
</style>
