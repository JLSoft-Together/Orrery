<script setup lang="ts">
import { Clock } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

defineProps<{ text: string | null; enabled: boolean }>()
const emit = defineEmits<{ keys: [keys: string] }>()
const { t } = useI18n()
</script>

<template>
  <div class="ask" role="group" data-testid="permission-bar">
    <p class="text"><Clock :size="18" :stroke-width="2.2" aria-hidden="true" /><span>{{ text ?? t('status.asked') }}</span></p>
    <div class="buttons">
      <button type="button" class="btn btn-primary" :disabled="!enabled" data-testid="answer-approve" @click="emit('keys', 'approve')">{{ t('term.answerYes') }}</button>
      <button type="button" class="btn" :disabled="!enabled" data-testid="answer-2" @click="emit('keys', '2')">{{ t('term.answerYesAlways') }}</button>
      <button type="button" class="btn" :disabled="!enabled" data-testid="answer-deny" @click="emit('keys', 'deny')">{{ t('term.answerNo') }}</button>
      <button type="button" class="btn" :disabled="!enabled" data-testid="answer-stop" @click="emit('keys', 'stop')">{{ t('term.answerStop') }}</button>
      <button type="button" class="btn" :disabled="!enabled" data-testid="answer-esc" @click="emit('keys', 'esc')">Esc</button>
    </div>
  </div>
</template>

<style scoped>
.ask {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 8px 12px 4px;
  padding: 12px;
  border-radius: var(--radius-card);
  border: 1px solid color-mix(in srgb, var(--ccm-waiting) 40%, transparent);
  background: color-mix(in srgb, var(--ccm-waiting) 10%, var(--ccm-surface));
}

.text {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-weight: 600;
  color: var(--ccm-ink);
}

.text svg {
  flex: none;
  color: var(--ccm-waiting);
}

.text span {
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
