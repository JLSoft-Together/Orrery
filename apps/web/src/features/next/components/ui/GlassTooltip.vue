<script setup lang="ts">
import { onBeforeUnmount, ref, useId } from 'vue'

export type TooltipSide = 'top' | 'bottom'

const props = withDefaults(defineProps<{ text: string; side?: TooltipSide; delay?: number; disabled?: boolean }>(), {
  side: 'top',
  delay: 450,
  disabled: false,
})

const id = useId()
const trigger = ref<HTMLElement | null>(null)
const shown = ref(false)
const pos = ref({ left: 0, top: 0 })
let timer: number | undefined

function place(): void {
  const r = trigger.value?.getBoundingClientRect()
  if (!r) return
  const vw = window.innerWidth
  const left = Math.max(8, Math.min(r.left + r.width / 2, vw - 8))
  pos.value = { left, top: props.side === 'top' ? r.top - 6 : r.bottom + 6 }
}

function show(now: boolean): void {
  if (props.disabled || !props.text) return
  window.clearTimeout(timer)
  if (now) {
    place()
    shown.value = true
    return
  }
  timer = window.setTimeout(() => {
    place()
    shown.value = true
  }, props.delay)
}

function hide(): void {
  window.clearTimeout(timer)
  shown.value = false
}

function onFocus(e: FocusEvent): void {
  const el = e.target as HTMLElement | null
  if (el?.matches?.(':focus-visible')) show(true)
}

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape' && shown.value) hide()
}

onBeforeUnmount(() => window.clearTimeout(timer))
</script>

<template>
  <span
    ref="trigger"
    class="inline-flex min-w-0"
    :aria-describedby="shown ? id : undefined"
    @mouseenter="show(false)"
    @mouseleave="hide"
    @focusin="onFocus"
    @focusout="hide"
    @keydown="onKey"
    @pointerdown="hide"
  >
    <slot />
  </span>
  <Teleport to="body">
    <Transition name="ccm-next-fade">
      <div
        v-if="shown"
        :id="id"
        role="tooltip"
        class="glass-pop pointer-events-none fixed z-[60] max-w-[320px] rounded-inner px-2 py-1.5 text-2xs font-medium text-ink"
        :class="side === 'top' ? '-translate-x-1/2 -translate-y-full' : '-translate-x-1/2'"
        :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
      >
        {{ text }}
      </div>
    </Transition>
  </Teleport>
</template>
