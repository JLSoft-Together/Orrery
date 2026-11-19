<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { editDirty, leaveOk } from '../lib/editGuard'
import { useAction } from '../lib/useAction'
import { isDirty, saveFile } from '../lib/viewerActions'
import { ViewerError, type Commander } from '../lib/viewerApi'
import NoticeLine from './NoticeLine.vue'

const props = defineProps<{ command: Commander; path: string; text: string; base: number }>()
const emit = defineEmits<{ saved: [text: string, mtimeMs: number]; cancel: []; reload: [] }>()
const { t } = useI18n()
const draft = ref(props.text)
const conflict = ref(false)
const { busy, notice, perform } = useAction()

watch(draft, (value) => (editDirty.value = isDirty(props.text, value)), { immediate: true })
onBeforeUnmount(() => (editDirty.value = false))

async function save(): Promise<void> {
  if (busy.value || !isDirty(props.text, draft.value)) return
  const text = draft.value
  conflict.value = false
  const result = await perform(async () => {
    try {
      const saved = await saveFile(props.command, props.path, text, props.base)
      return saved.kind === 'saved' ? { kind: 'done' as const, data: { mtimeMs: saved.mtimeMs } } : saved
    } catch (error) {
      if (error instanceof ViewerError && error.code === 'conflict') conflict.value = true
      throw error
    }
  })
  if (result?.kind === 'done') emit('saved', text, Number(result.data.mtimeMs))
}

async function cancel(): Promise<void> {
  if (await leaveOk(t)) emit('cancel')
}
</script>

<template>
  <div class="editor" data-testid="editor-view" :data-dirty="isDirty(text, draft) ? '1' : '0'">
    <div v-if="conflict" class="conflict" role="alert" data-testid="edit-conflict">
      <p>{{ t('edit.conflict') }}</p>
      <button type="button" class="btn" data-testid="edit-reload" @click="emit('reload')">{{ t('edit.reload') }}</button>
    </div>
    <textarea v-model="draft" class="area mono" data-testid="editor" :aria-label="t('edit.label')" spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" wrap="off" />
    <NoticeLine :notice="conflict ? null : notice" />
    <div class="bar">
      <button type="button" class="btn" data-testid="edit-cancel" @click="cancel">{{ t('edit.cancel') }}</button>
      <button type="button" class="btn btn-primary" :disabled="busy || !isDirty(text, draft)" data-testid="edit-save" @click="save">{{ t('edit.save') }}</button>
    </div>
  </div>
</template>

<style scoped>
.editor {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
}

.area {
  flex: 1;
  min-height: 0;
  width: 100%;
  padding: 10px 16px;
  border: 0;
  border-radius: 0;
  resize: none;
  overflow: auto;
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-size: 16px;
  line-height: 1.5;
  tab-size: 2;
  outline: none;
}

.conflict {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 16px 8px;
  padding: 10px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-error);
  color: var(--ccm-error);
  font-weight: 600;
}

.conflict p {
  flex: 1;
  margin: 0;
}

.bar {
  display: flex;
  gap: 8px;
  padding: 10px 16px calc(10px + env(safe-area-inset-bottom));
  border-top: 1px solid var(--ccm-line);
  flex: none;
}

.bar .btn {
  flex: 1;
}
</style>
