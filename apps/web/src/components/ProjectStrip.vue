<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { FolderTree } from 'lucide-vue-next'
import type { ProjectGroup } from '../lib/projectGroups'
import { formatSize } from '../lib/diskMap'
import { junkOf, useDiskSizes } from '../composables/useDiskSizes'
import { useDiskCleanStore } from '../stores/diskClean'
import AppSelect, { type SelectOption } from './AppSelect.vue'

const props = defineProps<{ groups: readonly ProjectGroup[]; modelValue: string | null }>()
const emit = defineEmits<{ 'update:modelValue': [key: string | null] }>()

const { t, locale } = useI18n()
const clean = useDiskCleanStore()
const sizes = useDiskSizes(() => props.groups.map((g) => g.dir).filter(Boolean))
function hintOf(g: ProjectGroup): string {
  const base = g.waiting ? `${g.sessions} · ${t('sessions.projects.waiting', { n: g.waiting })}` : String(g.sessions)
  if (!g.dir) return base
  const parts = [base]
  const size = sizes.sizeOf(g.dir)
  if (size !== null) parts.push(formatSize(size, locale.value))
  const junk = junkOf(g.dir, clean)
  if (junk !== null) parts.push(t('diskSizes.tipJunk', { size: formatSize(junk, locale.value) }))
  return parts.join(', ')
}

const nameOf = (g: ProjectGroup) => g.name || t('sessions.projects.noProject')
const options = computed<SelectOption<string | null>[]>(() => [
  { value: null, label: t('monitorV79.projectAll'), hint: String(props.groups.reduce((n, g) => n + g.sessions, 0)) },
  ...props.groups.map((g) => ({
    value: g.key,
    label: nameOf(g),
    hint: hintOf(g),
  })),
])
const model = computed({
  get: () => props.modelValue,
  set: (key: string | null) => emit('update:modelValue', key),
})
</script>

<template>
  <AppSelect v-if="groups.length > 1" v-model="model" :options="options" :label="t('sessions.projects.label')" :icon="FolderTree" size="sm" data-project-filter />
</template>
