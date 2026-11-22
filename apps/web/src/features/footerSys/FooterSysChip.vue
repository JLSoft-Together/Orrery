<script setup lang="ts">
import { computed, onBeforeUnmount, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { TriangleAlert } from 'lucide-vue-next'
import { useResourcesStore } from '../../stores/resources'
import { useSettingsStore } from '../../stores/settings'
import { sysLevel, sysPct, sysTips } from './format'
import { useSysStatsStore } from './store'

const { t, locale } = useI18n()
const settings = useSettingsStore()
const stats = useSysStatsStore()
const resources = useResourcesStore()

let running = false
function sync(on: boolean): void {
  if (on === running) return
  running = on
  if (on) stats.start()
  else stats.stop()
}
watch(() => settings.footerSys, sync, { immediate: true })
onBeforeUnmount(() => sync(false))

const cpu = computed(() => sysPct(stats.sample?.cpu ?? null))
const ram = computed(() => sysPct(stats.sample?.ram ?? null))
const show = computed(() => settings.footerSys && (cpu.value !== null || ram.value !== null))
const tip = computed(() => {
  if (!stats.sample) return ''
  const tips = sysTips(stats.sample, locale.value, t)
  return [tips.cpu, tips.ram].filter(Boolean).join('\n')
})
const aria = computed(() => `${t('footerSys.aria')}: ${tip.value.replace('\n', ', ')}`)
</script>

<template>
  <button
    v-if="show"
    type="button"
    class="ccm-press inline-flex h-[18px] shrink-0 cursor-pointer items-center gap-[5px] self-center whitespace-nowrap rounded-[6px] border border-line px-1.5 font-mono text-2xs tabular-nums transition-colors duration-(--ccm-dur-fast) hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent"
    :title="tip"
    :aria-label="aria"
    data-footer-sys
    @click="resources.show()"
  >
    <template v-if="cpu !== null">
      <span class="text-[9px] font-semibold uppercase tracking-wide text-ink-muted">{{ t('footerSys.cpu') }}</span>
      <span class="inline-flex items-center gap-0.5" :class="sysLevel(cpu) === 'warn' ? 'font-semibold text-warning' : 'text-ink'" :data-sys-level="sysLevel(cpu)" data-sys-cpu>
        <TriangleAlert v-if="sysLevel(cpu) === 'warn'" :size="10" aria-hidden="true" />{{ cpu }}%
      </span>
    </template>
    <span v-if="cpu !== null && ram !== null" class="h-2.5 w-px bg-line-strong" aria-hidden="true" />
    <template v-if="ram !== null">
      <span class="text-[9px] font-semibold uppercase tracking-wide text-ink-muted">{{ t('footerSys.ram') }}</span>
      <span class="inline-flex items-center gap-0.5" :class="sysLevel(ram) === 'warn' ? 'font-semibold text-warning' : 'text-ink'" :data-sys-level="sysLevel(ram)" data-sys-ram>
        <TriangleAlert v-if="sysLevel(ram) === 'warn'" :size="10" aria-hidden="true" />{{ ram }}%
      </span>
    </template>
  </button>
</template>
