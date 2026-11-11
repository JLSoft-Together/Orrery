<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { duration, now } from '../../lib/format'
import { orbitDots, orbitEntries, orbitTone, ORBIT_MAX_DOTS } from '../../lib/paneV93'
import { statusMeta } from '../../lib/status'
import type { Agent } from '@ccm/shared'

defineOptions({ inheritAttrs: false })
const props = defineProps<{ agents: readonly Agent[] | undefined }>()
const { t } = useI18n()

const entries = computed(() => orbitEntries(props.agents))
const dots = computed(() => orbitDots(entries.value))
const tone = computed(() => orbitTone(entries.value))
const meta = computed(() => statusMeta(tone.value))
const hidden = computed(() => Math.max(0, entries.value.length - ORBIT_MAX_DOTS))
const label = computed(() => t('paneV93.orbit.label', { n: entries.value.length }, entries.value.length))

const button = ref<HTMLButtonElement | null>(null)
const pop = ref<HTMLElement | null>(null)
const at = ref<{ top: number; right: number } | null>(null)

function close(): void {
  if (!at.value) return
  at.value = null
  document.removeEventListener('pointerdown', onOutside, true)
  document.removeEventListener('keydown', onKey, true)
  window.removeEventListener('resize', close)
}

function onOutside(e: PointerEvent): void {
  const target = e.target as Node
  if (pop.value?.contains(target) || button.value?.contains(target)) return
  close()
}

function onKey(e: KeyboardEvent): void {
  if (e.key !== 'Escape') return
  e.stopPropagation()
  close()
  button.value?.focus()
}

function toggle(): void {
  if (at.value) return close()
  const r = button.value?.getBoundingClientRect()
  if (!r) return
  at.value = { top: r.bottom + 4, right: Math.max(8, window.innerWidth - r.right) }
  document.addEventListener('pointerdown', onOutside, true)
  document.addEventListener('keydown', onKey, true)
  window.addEventListener('resize', close)
}

onBeforeUnmount(close)

const since = (iso: string | undefined): string | null => duration(iso, now.value)
</script>

<template>
  <button
    v-if="entries.length"
    ref="button"
    v-bind="$attrs"
    type="button"
    class="inline-flex h-5 shrink-[8] cursor-pointer items-center gap-1.5 rounded-inner px-1.5 text-2xs font-medium whitespace-nowrap transition-colors duration-(--ccm-dur-fast) ease-out-quint hover:text-ink aria-expanded:text-ink @max-[360px]:hidden"
    :class="[meta.chip, meta.text]"
    :aria-label="t('paneV93.orbit.aria', { n: entries.length })"
    aria-haspopup="dialog"
    :aria-expanded="!!at"
    :title="t('paneV93.orbit.title')"
    :data-pane-orbit="entries.length"
    @click="toggle"
  >
    <span class="ccm-orbit relative inline-block size-4 shrink-0 rounded-full border border-current/40" aria-hidden="true">
      <span class="ccm-orbit-spin absolute inset-0">
        <span v-for="d in dots" :key="d.id" class="absolute top-1/2 left-1/2 block size-0" :style="{ transform: `rotate(${d.angle}deg) translateY(-6.5px)` }">
          <span class="absolute -top-[2px] -left-[2px] block size-1 rounded-full" :class="statusMeta(d.status).bar" />
        </span>
      </span>
    </span>
    <span class="min-w-0 truncate tabular-nums @max-[640px]:sr-only">{{ label }}</span>
  </button>
  <Teleport to="body">
    <div
      v-if="at && entries.length"
      ref="pop"
      class="ccm-sheet fixed z-50 flex max-h-72 w-72 max-w-[90vw] flex-col gap-1 overflow-y-auto bg-surface p-2 text-xs"
      :style="{ top: `${at.top}px`, right: `${at.right}px` }"
      role="dialog"
      :aria-label="t('paneV93.orbit.title')"
      data-pane-orbit-pop
    >
      <p class="px-1 text-2xs font-semibold text-ink-faint">{{ t('paneV93.orbit.title') }}</p>
      <ul class="flex flex-col">
        <li v-for="e in entries" :key="e.id" class="flex min-w-0 items-center gap-2 rounded-inner px-1 py-1" data-pane-orbit-row>
          <span class="size-1.5 shrink-0 rounded-full" :class="statusMeta(e.status).bar" aria-hidden="true" />
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="truncate font-medium text-ink">{{ e.name || e.type || t('paneV93.orbit.unnamed') }}</span>
            <span class="truncate text-2xs text-ink-muted">
              <span v-if="e.type && e.name">{{ e.type }} · </span>{{ e.tool ?? t('paneV93.orbit.idleTool') }}
            </span>
          </span>
          <span v-if="since(e.since)" class="shrink-0 text-2xs text-ink-faint tabular-nums">{{ since(e.since) }}</span>
        </li>
      </ul>
      <p v-if="hidden" class="px-1 text-2xs text-ink-faint">{{ t('paneV93.orbit.more', { n: hidden }) }}</p>
    </div>
  </Teleport>
</template>

<style scoped>
.ccm-orbit-spin {
  animation: ccm-orbit-turn 14s linear infinite;
}
@keyframes ccm-orbit-turn {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .ccm-orbit-spin {
    animation: none;
  }
}
</style>
