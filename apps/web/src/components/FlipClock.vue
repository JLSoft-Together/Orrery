<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { usePageActive } from '../composables/usePageActive'
import FlipChar from './FlipChar.vue'

const { t } = useI18n()
const nowMs = ref(Date.now())
let timer: number | undefined

// Ticks on the second boundary so the seconds flip in step with the system clock.
const active = usePageActive()
function tick(): void {
  nowMs.value = Date.now()
  timer = window.setTimeout(tick, 1000 - (nowMs.value % 1000) + 5)
}
function halt(): void {
  window.clearTimeout(timer)
  timer = undefined
}
watch(active, (on) => {
  halt()
  if (on) tick()
})
onMounted(() => {
  if (active.value) tick()
})
onBeforeUnmount(halt)

const pad = (n: number) => String(n).padStart(2, '0')
const parts = computed(() => {
  const d = new Date(nowMs.value)
  return {
    time: [pad(d.getHours()), pad(d.getMinutes()), pad(d.getSeconds())],
    day: t(`clock.days.${d.getDay()}`),
    date: `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`,
    iso: d.toISOString(),
  }
})
const label = computed(() => `${parts.value.time.join(':')} - ${parts.value.day} - ${parts.value.date}`)
</script>

<template>
  <time :datetime="parts.iso" :aria-label="label" class="items-center gap-1.5 text-xs leading-none tabular select-none">
    <span class="inline-flex items-center font-semibold text-ink" aria-hidden="true">
      <template v-for="(pair, i) in parts.time" :key="i">
        <span v-if="i" class="px-px text-ink-faint">:</span>
        <FlipChar :value="pair[0]!" /><FlipChar :value="pair[1]!" />
      </template>
    </span>
    <span class="hidden items-center gap-1.5 text-ink-muted 3xl:inline-flex" aria-hidden="true">
      <span class="text-ink-faint">-</span>
      <FlipChar :value="parts.day" />
      <span class="text-ink-faint">-</span>
      <span class="inline-flex"><FlipChar v-for="(c, i) in parts.date" :key="i" :value="c" /></span>
    </span>
  </time>
</template>
