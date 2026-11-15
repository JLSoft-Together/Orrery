<script setup lang="ts" generic="T extends string | number">
import { onBeforeUnmount, onMounted, ref, watch, type Component } from 'vue'

export interface NextSegment<V> {
  value: V
  label: string
  icon?: Component
  title?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{ options: NextSegment<T>[]; label: string; fill?: boolean; iconOnly?: boolean; size?: 'sm' | 'md'; disabled?: boolean }>(), {
  fill: false,
  iconOnly: false,
  size: 'md',
  disabled: false,
})
const model = defineModel<T>({ required: true })

const root = ref<HTMLElement | null>(null)
const thumb = ref<{ x: number; w: number; h: number; y: number } | null>(null)
const ready = ref(false)

function measure(): void {
  const el = root.value?.querySelector<HTMLElement>('[aria-checked="true"]')
  if (!el || !el.offsetWidth) return void (thumb.value = el ? thumb.value : null)
  thumb.value = { x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight }
  if (!ready.value) requestAnimationFrame(() => (ready.value = true))
}
const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(() => measure())
watch([model, () => props.options.map((o) => o.label).join()], measure, { flush: 'post' })
onMounted(() => {
  measure()
  if (root.value) observer?.observe(root.value)
})
onBeforeUnmount(() => observer?.disconnect())

function onKey(e: KeyboardEvent): void {
  const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
  if (!step || props.disabled) return
  e.preventDefault()
  const open = props.options.map((o, i) => (o.disabled ? -1 : i)).filter((i) => i >= 0)
  if (!open.length) return
  const at = open.indexOf(props.options.findIndex((o) => o.value === model.value))
  const next = open[(at + step + open.length) % open.length]!
  model.value = props.options[next]!.value
  root.value?.querySelectorAll<HTMLButtonElement>('[role="radio"]')[next]?.focus()
}
</script>

<template>
  <div
    ref="root"
    class="relative rounded-full bg-(--ccm-seg-track) p-0.5"
    :class="[fill ? 'grid auto-cols-fr grid-flow-col' : 'inline-flex', size === 'md' ? 'h-8' : 'h-7', disabled ? 'text-ink-faint' : '']"
    role="radiogroup"
    :aria-label="label"
    :aria-disabled="disabled || undefined"
    data-next-seg
    @keydown="onKey"
  >
    <span
      v-if="thumb"
      aria-hidden="true"
      class="pointer-events-none absolute top-0 left-0 rounded-full bg-(--ccm-seg-thumb) shadow-(--ccm-seg-thumb-shadow)"
      :class="ready ? 'transition-transform duration-(--ccm-dur-base) ease-(--ccm-ease-standard)' : ''"
      :style="{ width: `${thumb.w}px`, height: `${thumb.h}px`, transform: `translate(${thumb.x}px, ${thumb.y}px)` }"
    />
    <button
      v-for="o in options"
      :key="String(o.value)"
      type="button"
      role="radio"
      :data-value="String(o.value)"
      :aria-checked="model === o.value"
      :aria-label="iconOnly ? o.label : undefined"
      :title="o.title ?? (iconOnly ? o.label : undefined)"
      :tabindex="model === o.value ? 0 : -1"
      :disabled="disabled || o.disabled"
      class="relative inline-flex h-full min-w-0 cursor-pointer items-center justify-center gap-1.5 rounded-full px-3 text-2xs font-semibold whitespace-nowrap transition-colors duration-(--ccm-dur-fast) outline-none focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus disabled:cursor-default"
      :class="model === o.value ? 'text-(--ccm-seg-on-ink)' : 'text-ink-muted hover:text-ink disabled:text-ink-faint'"
      @click="model = o.value"
    >
      <component :is="o.icon" v-if="o.icon" :size="14" aria-hidden="true" />
      <span v-if="!iconOnly" class="truncate">{{ o.label }}</span>
    </button>
  </div>
</template>
