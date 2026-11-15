<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowRightLeft, Copy, FolderOpen, Lock, Play, ShieldAlert, Trash2, TriangleAlert } from 'lucide-vue-next'
import { formatSize } from '../../../lib/diskMap'
import { ageDays, baseName, isStale, lockReason, pickable, relTo, SDK_IDS, type CleanItem } from '../../../lib/diskClean'

const props = defineProps<{ item: CleanItem; checked: boolean; open: string[]; days: number; now: number; busy?: boolean; canMove?: boolean }>()
const emit = defineEmits<{ toggle: [path: string]; reveal: [path: string]; emptyBin: []; command: [path: string]; move: [path: string] }>()

const { t, locale } = useI18n()

const lock = computed(() => lockReason(props.item, props.open))
const can = computed(() => pickable(props.item, props.open))
const system = computed(() => props.item.group === 'system')
const age = computed(() => ageDays(props.item.lastTouched, props.now))
const stale = computed(() => isStale(props.item, props.days, props.now))
const label = computed(() => {
  const it = props.item
  if (it.category === 'cache' && it.kind) return t(`diskClean.cacheKind.${it.kind}`)
  if (it.category === 'docker') return t(`diskClean.dockerKind.${it.kind}`)
  if (SDK_IDS.includes(it.category) && it.kind) return it.detail ? `${t(`diskClean.sdkKind.${it.kind}`)}: ${it.detail}` : t(`diskClean.sdkKind.${it.kind}`)
  if (system.value) {
    if (it.file) return baseName(it.path)
    if (it.detail) return it.kind === 'app' ? it.detail : `${t(`diskClean.sysKind.${it.kind}`)}: ${it.detail.split(', ').slice(1).join(', ')}`
    return it.kind ? t(`diskClean.sysKind.${it.kind}`) : baseName(it.path)
  }
  return relTo(it.path, it.project)
})
const command = computed(() => props.item.action === 'command')
const movable = computed(() => !!props.canMove && (props.item.category === 'cache' || props.item.kind === 'avd') && !props.item.action && !props.item.file && !lock.value)
const showAge = computed(() => props.item.category !== 'cache' && !command.value && (!system.value || typeof props.item.lastTouched === 'number'))
const ageText = computed(() => (age.value === null ? '' : age.value === 0 ? t('diskClean.ageToday') : t('diskClean.age', { n: age.value })))
const sizeText = computed(() => (props.item.unknown ? t('diskClean.sizeUnknown') : formatSize(props.item.size, locale.value)))
const lockText = computed(() => (lock.value === 'admin' ? t('diskClean.adminShort') : t('diskClean.lockedShort')))

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
    :class="checked && can ? 'bg-accent-soft' : ''"
    :data-clean-row="item.path"
    :data-clean-locked="lock || undefined"
  >
    <input
      type="checkbox"
      class="size-3.5 shrink-0 accent-[var(--ccm-accent)] disabled:opacity-40"
      :checked="checked && can"
      :disabled="!can"
      :aria-label="item.path"
      data-clean-check
      @change="emit('toggle', item.path)"
    />
    <div class="min-w-0 flex-1 basis-48">
      <p class="truncate font-medium" :title="item.path">{{ label }}</p>
      <p class="truncate text-2xs text-ink-muted" :title="item.path">
        <template v-if="item.project">{{ t('diskClean.project', { name: baseName(item.project) }) }}</template>
        <template v-else-if="item.category === 'docker'">{{ t('diskClean.dockerCount', { n: item.files ?? 0 }) }}</template>
        <template v-else-if="item.kind === 'simUnavailable'">{{ t('diskClean.simCount', { n: item.files ?? 0 }) }}</template>
        <template v-else>{{ item.path }}</template>
        <template v-if="item.category === 'cache'">, {{ t('diskClean.cacheNote') }}</template>
        <template v-if="item.action === 'files' && item.files">, {{ t('diskClean.files', { n: item.files }) }}</template>
        <template v-if="item.action === 'bin' && item.files">, {{ t('diskClean.binItems', { n: item.files }) }}</template>
      </p>
    </div>
    <span v-if="lock" class="ccm-chip ccm-chip-warning inline-flex shrink-0 items-center gap-1" :title="t(`diskClean.locked.${lock}`)" data-clean-lock>
      <component :is="lock === 'admin' ? ShieldAlert : Lock" :size="11" aria-hidden="true" />{{ lockText }}
      <span class="sr-only">{{ t(`diskClean.locked.${lock}`) }}</span>
    </span>
    <span v-if="item.warn" class="ccm-chip ccm-chip-warning inline-flex shrink-0 items-center gap-1" :title="t('diskClean.warnLoss')" data-clean-warn>
      <TriangleAlert :size="11" aria-hidden="true" />{{ t('diskClean.warnShort') }}
    </span>
    <span v-if="showAge" class="w-24 shrink-0 text-2xs tabular-nums" :class="stale ? 'text-ink' : 'text-ink-muted'" data-clean-age>{{ ageText || t('diskClean.ageUnknown') }}</span>
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
    <button
      v-if="item.action === 'bin' && !lock"
      type="button"
      class="ccm-btn ccm-btn-danger shrink-0"
      :disabled="busy || !item.size"
      data-clean-empty-bin
      @click="emit('emptyBin')"
    >
      <Trash2 :size="12" aria-hidden="true" />{{ t('diskClean.emptyBin') }}
    </button>
    <button v-if="command && !lock" type="button" class="ccm-btn shrink-0" :disabled="busy" data-clean-command @click="emit('command', item.path)">
      <Play :size="12" aria-hidden="true" />{{ t('diskClean.runCommand') }}
    </button>
    <button v-if="movable" type="button" class="ccm-btn-plain shrink-0" :disabled="busy" :title="t('diskClean.moveDrive')" :aria-label="t('diskClean.moveDrive')" data-clean-move @click="emit('move', item.path)">
      <ArrowRightLeft :size="13" aria-hidden="true" />
    </button>
    <span class="w-16 shrink-0 text-right tabular-nums" data-clean-size>{{ sizeText }}</span>
    <button v-if="!command" type="button" class="ccm-btn-plain shrink-0" :title="t('diskClean.reveal')" :aria-label="t('diskClean.reveal')" @click="emit('reveal', item.path)">
      <FolderOpen :size="13" aria-hidden="true" />
    </button>
  </li>
</template>
