<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Copy, FolderOpen, Lock } from 'lucide-vue-next'
import { formatSize } from '../../../lib/diskMap'
import { ageDays, baseName, isStale, lockReason, relTo, type CleanItem } from '../../../lib/diskClean'

const props = defineProps<{ item: CleanItem; checked: boolean; open: string[]; days: number; now: number }>()
const emit = defineEmits<{ toggle: [path: string]; reveal: [path: string] }>()

const { t, locale } = useI18n()

const lock = computed(() => lockReason(props.item, props.open))
const age = computed(() => ageDays(props.item.lastTouched, props.now))
const stale = computed(() => isStale(props.item, props.days, props.now))
const label = computed(() => (props.item.category === 'cache' && props.item.kind ? t(`diskClean.cacheKind.${props.item.kind}`) : relTo(props.item.path, props.item.project)))
const ageText = computed(() => (age.value === null ? '' : age.value === 0 ? t('diskClean.ageToday') : t('diskClean.age', { n: age.value })))

async function copy(): Promise<void> {
  if (!props.item.restore) return
  try {
    await navigator.clipboard.writeText(props.item.restore)
  } catch {
    return
  }
}
</script>

<template>
  <li
    class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2 text-xs sm:flex-nowrap"
    :class="checked && !lock ? 'bg-accent-soft' : ''"
    :data-clean-row="item.path"
    :data-clean-locked="lock || undefined"
  >
    <input
      type="checkbox"
      class="size-3.5 shrink-0 accent-[var(--ccm-accent)] disabled:opacity-40"
      :checked="checked && !lock"
      :disabled="!!lock"
      :aria-label="item.path"
      data-clean-check
      @change="emit('toggle', item.path)"
    />
    <div class="min-w-0 flex-1 basis-48">
      <p class="truncate font-medium" :title="item.path">{{ label }}</p>
      <p class="truncate text-2xs text-ink-muted" :title="item.path">
        <template v-if="item.project">{{ t('diskClean.project', { name: baseName(item.project) }) }}</template>
        <template v-else>{{ item.path }}</template>
        <template v-if="item.category === 'cache'">, {{ t('diskClean.cacheNote') }}</template>
      </p>
    </div>
    <span v-if="lock" class="ccm-chip ccm-chip-warning inline-flex shrink-0 items-center gap-1" :title="t(`diskClean.locked.${lock}`)" data-clean-lock>
      <Lock :size="11" aria-hidden="true" />{{ t('diskClean.lockedShort') }}
      <span class="sr-only">{{ t(`diskClean.locked.${lock}`) }}</span>
    </span>
    <span v-if="item.category !== 'cache'" class="w-24 shrink-0 text-2xs tabular-nums" :class="stale ? 'text-ink' : 'text-ink-muted'" data-clean-age>{{ ageText || t('diskClean.ageUnknown') }}</span>
    <button
      v-if="item.restore"
      type="button"
      class="hidden max-w-[12rem] shrink-0 items-center gap-1 truncate rounded-inner bg-fill px-1.5 py-0.5 font-mono text-2xs text-ink-muted hover:text-ink md:inline-flex"
      :title="`${t('diskClean.restore')} ${item.restore}. ${t('diskClean.restoreCopy')}`"
      :aria-label="`${t('diskClean.restoreCopy')}: ${item.restore}`"
      data-clean-restore
      @click="copy"
    >
      <Copy :size="11" class="shrink-0" aria-hidden="true" /><span class="truncate">{{ item.restore }}</span>
    </button>
    <span class="w-16 shrink-0 text-right tabular-nums" data-clean-size>{{ formatSize(item.size, locale) }}</span>
    <button type="button" class="ccm-btn-plain shrink-0" :title="t('diskClean.reveal')" :aria-label="t('diskClean.reveal')" @click="emit('reveal', item.path)">
      <FolderOpen :size="13" aria-hidden="true" />
    </button>
  </li>
</template>
