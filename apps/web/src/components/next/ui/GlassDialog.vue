<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { X } from 'lucide-vue-next'
import { captureFocus, focusables, trapTab } from './focus'

export type GlassDialogSize = 'sm' | 'md' | 'lg'

const props = withDefaults(defineProps<{ open: boolean; title?: string; size?: GlassDialogSize; blocking?: boolean; closeButton?: boolean; initialFocus?: string }>(), {
  title: undefined,
  size: 'sm',
  blocking: false,
  closeButton: true,
  initialFocus: undefined,
})
const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()

const WIDTH: Record<GlassDialogSize, number> = { sm: 480, md: 640, lg: 880 }
const width = computed(() => `min(${WIDTH[props.size]}px, calc(100vw - 32px))`)
const panel = ref<HTMLElement | null>(null)
const titleId = useId()
let restore: (() => void) | null = null

watch(
  () => props.open,
  async (open) => {
    if (open) {
      restore = captureFocus()
      await nextTick()
      const el = panel.value
      if (!el) return
      const pick = props.initialFocus ? el.querySelector<HTMLElement>(props.initialFocus) : null
      ;(pick ?? focusables(el)[0] ?? el).focus()
    } else {
      restore?.()
      restore = null
    }
  },
  { immediate: true },
)
onBeforeUnmount(() => restore?.())

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape' && !props.blocking) {
    e.stopPropagation()
    e.preventDefault()
    emit('close')
    return
  }
  if (panel.value) trapTab(e, panel.value)
}

function onScrim(): void {
  if (!props.blocking) emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-next-dialog">
      <div v-if="open" class="fixed inset-0 z-50 grid place-items-center p-4" data-glass-dialog>
        <div class="ccm-next-scrim absolute inset-0 bg-(--ccm-scrim)" aria-hidden="true" @click="onScrim" />
        <section
          ref="panel"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="title || $slots.title ? titleId : undefined"
          tabindex="-1"
          class="glass-pop ccm-next-panel relative flex max-h-[calc(100dvh-32px)] min-h-0 flex-col rounded-[20px] text-ink outline-none"
          :style="{ width }"
          @keydown="onKey"
        >
          <header v-if="title || $slots.title || closeButton" class="flex min-h-12 shrink-0 items-center gap-2 px-5 pt-4 pb-2">
            <h2 :id="titleId" class="min-w-0 flex-1 truncate text-base font-bold">
              <slot name="title">{{ title }}</slot>
            </h2>
            <slot name="actions" />
            <button
              v-if="closeButton && !blocking"
              type="button"
              class="inline-grid size-7 shrink-0 cursor-pointer place-items-center rounded-control text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
              :aria-label="t('mist.ui.close')"
              @click="emit('close')"
            >
              <X :size="16" aria-hidden="true" />
            </button>
          </header>
          <div class="min-h-0 flex-1 overflow-auto px-5 pb-5">
            <slot />
          </div>
          <footer v-if="$slots.footer" class="flex shrink-0 items-center justify-end gap-2 border-t border-line px-5 py-3">
            <slot name="footer" />
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>
