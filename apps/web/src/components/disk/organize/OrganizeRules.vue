<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Plus, X } from 'lucide-vue-next'
import { emptyRule, ORGANIZE_MAX_RULES, ORGANIZE_RULE_KINDS, ruleValid, type OrganizeRule } from '../../../lib/organize'

const props = defineProps<{ rules: OrganizeRule[] }>()
const emit = defineEmits<{ save: [rules: OrganizeRule[]] }>()
const { t } = useI18n()

const draft = ref<OrganizeRule[]>([])
const saved = ref(false)
watch(
  () => props.rules,
  (r) => (draft.value = r.map((x) => ({ ...x }))),
  { immediate: true },
)

const dirty = computed(() => JSON.stringify(draft.value) !== JSON.stringify(props.rules))
const allValid = computed(() => draft.value.every(ruleValid))

function add(): void {
  if (draft.value.length < ORGANIZE_MAX_RULES) draft.value.push(emptyRule())
  saved.value = false
}

function remove(i: number): void {
  draft.value.splice(i, 1)
  saved.value = false
}

function save(): void {
  if (!allValid.value) return
  emit('save', draft.value.map((r) => ({ ...r })))
  saved.value = true
}
</script>

<template>
  <details class="rounded-control border border-line px-3 py-2" :open="rules.length > 0" data-organize-rules>
    <summary class="cursor-pointer select-none text-xs font-semibold">{{ t('diskOrganize.rules.title') }} ({{ rules.length }})</summary>
    <p class="mt-1 text-2xs text-ink-muted">{{ t('diskOrganize.rules.hint') }}</p>
    <ul class="mt-2 flex flex-col gap-1.5">
      <li v-for="(r, i) in draft" :key="i" class="flex flex-wrap items-center gap-1.5 text-xs" data-organize-rule>
        <select v-model="r.kind" class="ccm-field w-40" :aria-label="t('diskOrganize.rules.kind.ext')" @change="saved = false">
          <option v-for="k in ORGANIZE_RULE_KINDS" :key="k" :value="k">{{ t(`diskOrganize.rules.kind.${k}`) }}</option>
        </select>
        <input
          v-model="r.value"
          :type="r.kind === 'size' ? 'number' : 'text'"
          min="0"
          step="any"
          class="ccm-field w-32 min-w-0"
          :placeholder="r.kind === 'ext' ? 'pdf' : r.kind === 'size' ? '500' : ''"
          :aria-label="t('diskOrganize.rules.value')"
          data-organize-rule-value
          @input="saved = false"
        />
        <span class="text-ink-muted" aria-hidden="true">&gt;</span>
        <input v-model="r.folder" type="text" maxlength="64" class="ccm-field w-40 min-w-0" :placeholder="t('diskOrganize.rules.folder')" :aria-label="t('diskOrganize.rules.folder')" data-organize-rule-folder @input="saved = false" />
        <button type="button" class="ccm-btn-plain px-1" :aria-label="t('diskOrganize.rules.remove')" :title="t('diskOrganize.rules.remove')" @click="remove(i)"><X :size="13" aria-hidden="true" /></button>
      </li>
    </ul>
    <div class="mt-2 flex flex-wrap items-center gap-2">
      <button type="button" class="ccm-btn" :disabled="draft.length >= ORGANIZE_MAX_RULES" data-organize-rule-add @click="add"><Plus :size="13" aria-hidden="true" />{{ t('diskOrganize.rules.add') }}</button>
      <button type="button" class="ccm-btn ccm-btn-primary" :disabled="!dirty || !allValid" data-organize-rule-save @click="save">{{ t('diskOrganize.rules.save') }}</button>
      <span v-if="!allValid" class="text-2xs text-warning">{{ t('diskOrganize.rules.invalid') }}</span>
      <span v-else-if="saved && !dirty" class="text-2xs text-ink-muted">{{ t('diskOrganize.rules.saved') }}</span>
    </div>
  </details>
</template>
