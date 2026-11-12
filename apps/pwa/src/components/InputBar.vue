<script setup lang="ts">
import { Keyboard, Send } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ enabled: boolean; direct: boolean }>()
const emit = defineEmits<{ send: [text: string]; direct: []; focus: [focused: boolean] }>()
const { t } = useI18n()
const text = ref('')
const field = ref<HTMLTextAreaElement | null>(null)
const ready = computed(() => props.enabled && text.value.trim() !== '')

function grow(): void {
  const el = field.value
  if (!el) return
  el.rows = Math.min(4, Math.max(1, text.value.split('\n').length))
}

function submit(): void {
  if (!ready.value) return
  emit('send', text.value)
  text.value = ''
  grow()
}
</script>

<template>
  <div class="bar">
    <button type="button" class="direct" :class="{ on: direct }" :aria-pressed="direct" :aria-label="t('term.direct')" :disabled="!enabled" data-testid="direct-toggle" @click="emit('direct')">
      <Keyboard :size="22" :stroke-width="2" aria-hidden="true" />
    </button>
    <textarea
      ref="field"
      v-model="text"
      class="field"
      rows="1"
      enterkeyhint="enter"
      autocapitalize="off"
      autocomplete="off"
      autocorrect="off"
      spellcheck="false"
      :disabled="!enabled"
      :placeholder="enabled ? t('term.inputHint') : t('term.inputOffline')"
      data-testid="line-input"
      @input="grow"
      @focus="emit('focus', true)"
      @blur="emit('focus', false)"
    />
    <button type="button" class="send" :disabled="!ready" :aria-label="t('term.send')" data-testid="line-send" @click="submit">
      <Send :size="20" :stroke-width="2" aria-hidden="true" />
    </button>
  </div>
</template>

<style scoped>
.bar {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 0 12px calc(10px + env(safe-area-inset-bottom));
  flex: none;
}

.direct,
.send {
  display: grid;
  place-items: center;
  flex: none;
  width: 48px;
  height: 48px;
  border-radius: 24px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.direct.on {
  color: var(--ccm-accent-ink);
  background: color-mix(in srgb, var(--ccm-accent) 14%, transparent);
}

.send {
  background: var(--ccm-accent);
  color: var(--ccm-on-accent);
  transition: opacity 150ms var(--ease-out);
}

.send:disabled {
  opacity: 0.4;
}

.field {
  flex: 1;
  min-width: 0;
  min-height: 48px;
  max-height: 120px;
  padding: 13px 18px;
  border-radius: 24px;
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-size: 1rem;
  line-height: 1.25;
  resize: none;
}
</style>
