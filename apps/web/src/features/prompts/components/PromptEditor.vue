<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ShieldAlert } from 'lucide-vue-next'
import AppSelect from '../../../core/components/AppSelect.vue'
import { baseName, looksSecret, MAX_BODY, MAX_TITLE, SCOPE_ALL, type PromptDraft } from '../lib/promptModel'
import { PROMPT_VARS } from '../lib/promptVars'

const props = defineProps<{ draft: PromptDraft; dirs: readonly string[]; busy: boolean; error: string | null }>()
const emit = defineEmits<{ save: [draft: PromptDraft]; cancel: [] }>()

const { t } = useI18n()
const title = ref(props.draft.title)
const body = ref(props.draft.body)
const scope = ref(props.draft.scope)
const nameInput = ref<HTMLInputElement | null>(null)

const scopeOptions = computed(() => {
  const dirs = props.dirs.includes(scope.value) || scope.value === SCOPE_ALL ? props.dirs : [scope.value, ...props.dirs]
  return [{ value: SCOPE_ALL, label: t('savedPrompts.scopeAll') }, ...dirs.map((d) => ({ value: d, label: baseName(d), hint: d }))]
})
const canSave = computed(() => !!title.value.trim() && !!body.value.trim() && !props.busy)
const secret = computed(() => looksSecret(body.value))
const vars = PROMPT_VARS.map((v) => `{${v}}`).join(', ')

function submit(): void {
  if (!canSave.value) return
  emit('save', { id: props.draft.id, title: title.value.trim(), body: body.value, scope: scope.value })
}

onMounted(() => void nextTick(() => nameInput.value?.focus()))
</script>

<template>
  <form class="flex flex-col gap-2.5 border-t border-line bg-fill px-3 py-3" data-saved-prompt-editor @submit.prevent="submit" @keydown.esc.stop.prevent="emit('cancel')">
    <label class="flex flex-col gap-1 text-xs text-ink-muted">
      {{ t('savedPrompts.fieldTitle') }}
      <input ref="nameInput" v-model="title" class="ccm-field h-8 text-sm text-ink" :maxlength="MAX_TITLE" autocomplete="off" data-saved-prompt-title />
    </label>
    <label class="flex flex-col gap-1 text-xs text-ink-muted">
      {{ t('savedPrompts.fieldBody') }}
      <textarea
        v-model="body"
        class="ccm-field min-h-28 resize-y py-1.5 font-mono text-xs leading-relaxed text-ink"
        :maxlength="MAX_BODY"
        spellcheck="false"
        data-saved-prompt-body
        @keydown.ctrl.enter.prevent="submit"
        @keydown.meta.enter.prevent="submit"
      />
    </label>
    <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 text-2xs text-ink-faint">
      <span>{{ t('savedPrompts.bodyHint', { vars }) }}</span>
      <span class="tabular" data-saved-prompt-count>{{ t('savedPrompts.chars', { n: body.length, max: MAX_BODY }) }}</span>
    </div>
    <p v-if="secret" class="flex items-center gap-1.5 text-xs text-warning" role="status" data-saved-prompt-secret>
      <ShieldAlert :size="13" class="shrink-0" aria-hidden="true" />{{ t('savedPrompts.secretWarn') }}
    </p>
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-xs text-ink-muted">{{ t('savedPrompts.scopeLabel') }}</span>
      <AppSelect v-model="scope" :options="scopeOptions" :label="t('savedPrompts.scopeLabel')" size="sm" data-saved-prompt-scope />
    </div>
    <p v-if="error" class="text-xs text-warning" role="alert" data-saved-prompt-error>{{ error }}</p>
    <div class="flex items-center justify-end gap-2">
      <button type="button" class="ccm-btn h-7 text-xs" data-saved-prompt-cancel @click="emit('cancel')">{{ t('savedPrompts.cancel') }}</button>
      <button type="submit" class="ccm-btn ccm-btn-primary h-7 text-xs" :disabled="!canSave" data-saved-prompt-save>{{ busy ? t('savedPrompts.saving') : t('savedPrompts.save') }}</button>
    </div>
  </form>
</template>
