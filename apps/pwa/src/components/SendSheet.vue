<script setup lang="ts">
import { Send } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { REMOTE_TEXT_MAX } from '../lib/limits'
import BottomSheet from './BottomSheet.vue'

defineProps<{ title: string; busy: boolean; error: string | null }>()
const emit = defineEmits<{ send: [text: string]; close: [] }>()
const { t } = useI18n()
const text = ref('')
const ready = computed(() => text.value.trim() !== '' && text.value.length <= REMOTE_TEXT_MAX)
</script>

<template>
  <BottomSheet :title="title" testid="send-sheet" @close="emit('close')">
    <textarea v-model="text" class="field" rows="4" :maxlength="REMOTE_TEXT_MAX" :placeholder="t('send.hint')" data-testid="send-text" />
    <p class="count mono">{{ text.length }} / {{ REMOTE_TEXT_MAX }}</p>
    <p v-if="error" class="error" role="alert" data-testid="send-error">{{ error }}</p>
    <button type="button" class="btn btn-primary go" :disabled="!ready || busy" data-testid="send-go" @click="emit('send', text)">
      <Send :size="18" :stroke-width="2" aria-hidden="true" />{{ t('send.go') }}
    </button>
  </BottomSheet>
</template>

<style scoped>
.field {
  width: 100%;
  padding: 12px 14px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-size: 1rem;
  resize: none;
}

.count {
  margin: 4px 0 8px;
  text-align: right;
  font-size: 0.75rem;
  color: var(--ccm-ink-faint);
}

.error {
  margin: 0 0 8px;
  color: var(--ccm-error);
  font-weight: 600;
}

.go {
  width: 100%;
}
</style>
