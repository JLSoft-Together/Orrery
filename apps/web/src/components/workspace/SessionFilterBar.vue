<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleHelp, Layers } from 'lucide-vue-next'
import AppSelect, { type SelectOption } from '../AppSelect.vue'
import StatusIcon from '../StatusIcon.vue'
import { UNKNOWN_FILTER } from '../../lib/turnEdits'

export interface SessionChoice {
  id: string
  name: string
  status: string
  count: number
}

const props = defineProps<{ sessions: SessionChoice[]; unknown: number; total: number }>()
const model = defineModel<string | null>({ required: true })
const { t } = useI18n()

const ALL = '~all'
const value = computed({
  get: () => model.value ?? ALL,
  set: (v: string) => (model.value = v === ALL ? null : v),
})

interface Seg {
  value: string
  label: string
  count: number
  status?: string
  title: string
}

const segs = computed<Seg[]>(() => [
  { value: ALL, label: t('v104.turnChanges.all'), count: props.total, title: t('v104.turnChanges.all') },
  ...props.sessions.map((s) => ({ value: s.id, label: s.name, count: s.count, status: s.status, title: t('v104.turnChanges.sessionTip', { name: s.name, n: s.count }) })),
  ...(props.unknown ? [{ value: UNKNOWN_FILTER, label: t('v104.turnChanges.unknown'), count: props.unknown, title: t('v104.turnChanges.unknownTip') }] : []),
])

const options = computed<SelectOption<string>[]>(() => segs.value.map((s) => ({ value: s.value, label: `${s.label} (${s.count})`, hint: s.status ? t(`status.${s.status}`) : undefined })))

function onKey(e: KeyboardEvent): void {
  const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0
  if (!step) return
  e.preventDefault()
  const list = segs.value
  const at = list.findIndex((s) => s.value === value.value)
  const next = (at + step + list.length) % list.length
  value.value = list[next]!.value
  ;(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus()
}
</script>

<template>
  <div class="flex min-w-0 shrink-0 items-center gap-2 border-b border-line px-2 py-1" data-session-filter :data-filter="value">
    <Layers :size="13" class="shrink-0 text-ink-faint" aria-hidden="true" />
    <div class="ccm-seg ccm-scroll flex min-w-0 gap-0.5 overflow-x-auto rounded-control p-0.5 @max-[440px]:hidden" role="radiogroup" :aria-label="t('v104.turnChanges.filter')" @keydown="onKey">
      <button
        v-for="s in segs"
        :key="s.value"
        type="button"
        role="radio"
        :aria-checked="value === s.value"
        :tabindex="value === s.value ? 0 : -1"
        :title="s.title"
        :data-session-seg="s.value"
        class="inline-flex h-6 max-w-44 min-w-0 shrink-0 cursor-pointer items-center gap-1.5 rounded-inner px-2 text-2xs font-medium whitespace-nowrap transition-colors duration-(--ccm-dur-fast) ease-out-quint"
        :class="value === s.value ? 'ccm-seg-thumb ccm-seg-on' : 'text-ink-muted hover:text-ink'"
        @click="value = s.value"
      >
        <StatusIcon v-if="s.status" :status="s.status" :size="12" />
        <CircleHelp v-else-if="s.value === UNKNOWN_FILTER" :size="12" class="shrink-0 text-ink-faint" aria-hidden="true" />
        <span class="min-w-0 truncate">{{ s.label }}</span>
        <span v-if="s.status" class="sr-only">{{ t(`status.${s.status}`) }}</span>
        <span class="shrink-0 tabular text-ink-faint">{{ s.count }}</span>
      </button>
    </div>
    <AppSelect v-model="value" class="hidden min-w-0 flex-1 @max-[440px]:flex" :options="options" :label="t('v104.turnChanges.filter')" size="sm" data-session-select />
    <span class="min-w-0 flex-1 @max-[440px]:hidden" />
    <slot />
  </div>
</template>
