<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAsync } from '../lib/useAsync'
import { viewerData } from '../lib/viewerApi'
import { parseDiff } from '../lib/viewerModel'
import { useHubStore } from '../stores/hub'
import BottomSheet from './BottomSheet.vue'
import DiffLines from './DiffLines.vue'

const props = defineProps<{ machine: string; root: string; path: string }>()
const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()
const hub = useHubStore()

const diff = useAsync(async () => {
  const data = await viewerData((type, fields, ms) => hub.command(props.machine, type, fields, ms), 'git.diff', { root: props.root, path: props.path })
  const m = (data && typeof data === 'object' ? data : {}) as Record<string, unknown>
  if (m.binary === true) return { note: 'diffBinary' as string | null, lines: [] as ReturnType<typeof parseDiff> }
  if (m.tooLarge === true) return { note: 'diffLarge', lines: [] }
  const text = typeof m.text === 'string' ? m.text : ''
  return { note: text.trim() === '' ? 'diffEmpty' : null, lines: parseDiff(text) }
})

function errorText(code: string): string {
  return t(`err.${code}`)
}
</script>

<template>
  <BottomSheet :title="path" testid="chat-diff" @close="emit('close')">
    <p v-if="diff.loading.value" class="note">{{ t('common.loading') }}</p>
    <p v-else-if="diff.error.value" class="note bad" role="alert">{{ errorText(diff.error.value) }}</p>
    <p v-else-if="diff.data.value?.note" class="note" data-testid="chat-diff-note">{{ t(`chat.${diff.data.value.note}`) }}</p>
    <div v-else class="scroll"><DiffLines :lines="diff.data.value?.lines ?? []" /></div>
  </BottomSheet>
</template>

<style scoped>
.scroll {
  max-height: 60vh;
  overflow: auto;
  border-radius: 16px;
}

.note {
  margin: 0;
  padding: 8px 0 12px;
  color: var(--ccm-ink-muted);
}

.bad {
  color: var(--ccm-error);
}
</style>
