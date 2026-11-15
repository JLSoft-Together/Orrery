<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch, type Component } from 'vue'
import { Check } from 'lucide-vue-next'
import { captureFocus, wrapIndex } from './focus'

export interface GlassMenuItem {
  id: string
  label: string
  icon?: Component
  kbd?: string
  hint?: string
  checked?: boolean
  disabled?: boolean
  danger?: boolean
  separator?: boolean
}

const props = withDefaults(defineProps<{ open: boolean; items: GlassMenuItem[]; label: string; width?: number; x?: number; y?: number }>(), {
  width: 240,
  x: undefined,
  y: undefined,
})
const emit = defineEmits<{ select: [id: string]; close: [] }>()

const root = ref<HTMLElement | null>(null)
const active = ref(-1)
const w = computed(() => Math.min(280, Math.max(220, props.width)))
const enabled = computed(() => props.items.map((it, i) => (!it.separator && !it.disabled ? i : -1)).filter((i) => i >= 0))
const style = computed(() => {
  const s: Record<string, string> = { width: `${w.value}px` }
  if (props.x !== undefined && props.y !== undefined) {
    const vw = typeof window === 'undefined' ? Infinity : window.innerWidth
    s.position = 'fixed'
    s.left = `${Math.max(8, Math.min(props.x, vw - w.value - 8))}px`
    s.top = `${props.y}px`
  }
  return s
})
let restore: (() => void) | null = null

function focusRow(i: number): void {
  active.value = i
  root.value?.querySelector<HTMLElement>(`[data-menu-index="${i}"]`)?.focus()
}

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      restore?.()
      restore = null
      return
    }
    restore = captureFocus()
    await nextTick()
    const first = enabled.value[0]
    if (first !== undefined) focusRow(first)
    else root.value?.focus()
  },
  { immediate: true },
)
onBeforeUnmount(() => restore?.())

function move(step: number): void {
  const list = enabled.value
  const at = list.indexOf(active.value)
  const next = list[wrapIndex(at, step, list.length)]
  if (next !== undefined) focusRow(next)
}

function pick(i: number): void {
  const it = props.items[i]
  if (!it || it.disabled || it.separator) return
  emit('select', it.id)
  emit('close')
}

function onKey(e: KeyboardEvent): void {
  const list = enabled.value
  if (e.key === 'ArrowDown') move(1)
  else if (e.key === 'ArrowUp') move(-1)
  else if (e.key === 'Home' && list.length) focusRow(list[0]!)
  else if (e.key === 'End' && list.length) focusRow(list[list.length - 1]!)
  else if (e.key === 'Enter' || e.key === ' ') pick(active.value)
  else if (e.key === 'Escape' || e.key === 'Tab') emit('close')
  else return
  e.preventDefault()
  e.stopPropagation()
}
</script>

<template>
  <Transition name="ccm-next-pop">
    <div
      v-if="open"
      ref="root"
      role="menu"
      :aria-label="label"
      tabindex="-1"
      class="glass-pop z-50 flex flex-col rounded-[14px] p-1 text-ink outline-none"
      :style="style"
      data-glass-menu
      @keydown="onKey"
    >
      <template v-for="(it, i) in items" :key="it.id">
        <div v-if="it.separator" role="separator" class="mx-2 my-1 h-px bg-line" />
        <button
          v-else
          type="button"
          role="menuitem"
          :data-menu-index="i"
          :data-menu-id="it.id"
          :aria-disabled="it.disabled || undefined"
          :aria-checked="it.checked === undefined ? undefined : it.checked"
          :tabindex="active === i ? 0 : -1"
          class="flex h-8 w-full min-w-0 cursor-pointer items-center gap-2 rounded-control px-2 text-left text-xs font-medium outline-none transition-colors duration-(--ccm-dur-instant) aria-disabled:cursor-default aria-disabled:text-ink-faint"
          :class="[active === i && !it.disabled ? 'bg-accent-soft' : '', it.danger ? 'text-st-error' : '']"
          @mouseenter="!it.disabled && (active = i)"
          @focus="active = i"
          @click="pick(i)"
        >
          <span class="inline-grid size-4 shrink-0 place-items-center" aria-hidden="true">
            <Check v-if="it.checked" :size="14" />
            <component :is="it.icon" v-else-if="it.icon" :size="14" />
          </span>
          <span class="min-w-0 flex-1 truncate">{{ it.label }}</span>
          <span v-if="it.hint" class="shrink-0 truncate text-2xs text-ink-faint">{{ it.hint }}</span>
          <kbd v-if="it.kbd" class="shrink-0 rounded-[5px] border border-line-strong px-1 font-mono text-[10px] font-semibold text-ink-muted">{{ it.kbd }}</kbd>
        </button>
      </template>
    </div>
  </Transition>
</template>
