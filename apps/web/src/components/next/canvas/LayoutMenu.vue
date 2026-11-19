<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { LayoutGrid } from 'lucide-vue-next'
import { useDismiss } from '../../../composables/useDismiss'
import { LAYOUT_PRESETS, type LayoutPreset } from '../../../lib/layoutPresets'

const props = defineProps<{
  disabled: Partial<Record<LayoutPreset | 'even', string>>
  current?: LayoutPreset | null
}>()
const emit = defineEmits<{ preset: [id: LayoutPreset]; even: [] }>()
const { t } = useI18n()

const open = ref(false)
const root = ref<HTMLElement | null>(null)
const button = ref<HTMLButtonElement | null>(null)
const panel = ref<HTMLElement | null>(null)
const place = ref<{ top: string; right: string }>({ top: '0px', right: '0px' })

function measure(): void {
  const r = button.value?.getBoundingClientRect()
  if (!r) return
  place.value = { top: `${Math.round(r.bottom + 8)}px`, right: `${Math.max(12, Math.round(window.innerWidth - r.right))}px` }
}

const CELLS: Record<LayoutPreset, { cols: string; rows?: string; cells: Array<Record<string, string>> }> = {
  one: { cols: '1fr', cells: [{}] },
  cols2: { cols: '1fr 1fr', cells: [{}, {}] },
  cols3: { cols: '1fr 1fr 1fr', cells: [{}, {}, {}] },
  grid2: { cols: '1fr 1fr', rows: '1fr 1fr', cells: [{}, {}, {}, {}] },
  mainCol: { cols: '1.6fr 1fr', rows: 'repeat(3, 1fr)', cells: [{ gridRow: 'span 3' }, {}, {}, {}] },
  topTwo: { cols: '1fr 1fr', rows: '1fr 1fr', cells: [{ gridColumn: 'span 2' }, {}, {}] },
}

function toggle(): void {
  if (!open.value) measure()
  open.value = !open.value
}

function close(focus = false): void {
  if (!open.value) return
  open.value = false
  if (focus) button.value?.focus()
}

function pick(id: LayoutPreset): void {
  if (props.disabled[id]) return
  emit('preset', id)
  close(true)
}

function even(): void {
  if (props.disabled.even) return
  emit('even')
  close(true)
}

useDismiss({ active: open, inside: () => [root.value, panel.value], dismiss: () => close() })

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    close(true)
    return
  }
  const keys = ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown']
  if (!keys.includes(e.key)) return
  const list = [...(panel.value?.querySelectorAll<HTMLButtonElement>('[data-layout-item]') ?? [])]
  const at = list.indexOf(document.activeElement as HTMLButtonElement)
  if (at < 0) return
  e.preventDefault()
  const step = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : e.key === 'ArrowUp' ? -3 : 3
  const next = Math.max(0, Math.min(list.length - 1, at + step))
  list[next]?.focus()
}

watch(open, async (on) => {
  if (on) {
    window.addEventListener('resize', measure)
    await nextTick()
    panel.value?.querySelector<HTMLButtonElement>('[data-layout-item]:not([aria-disabled])')?.focus()
  } else {
    window.removeEventListener('resize', measure)
  }
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', measure)
})
</script>

<template>
  <div ref="root" class="relative shrink-0" data-no-drag>
    <button
      ref="button"
      type="button"
      class="mist-layout-btn"
      :aria-expanded="open"
      aria-haspopup="dialog"
      :title="t('mist.drag.layout.title')"
      data-layout-button
      @click="toggle"
    >
      <LayoutGrid :size="14" aria-hidden="true" />
      <span class="mist-layout-word">{{ t('mist.drag.layout.button') }}</span>
    </button>
    <Teleport to="body">
    <Transition name="ccm-next-pop">
      <div
        v-if="open"
        ref="panel"
        class="mist-layout-pop glass-pop"
        :style="place"
        role="dialog"
        :aria-label="t('mist.drag.layout.title')"
        data-layout-menu
        @keydown="onKey"
      >
        <h4 class="mist-layout-title">{{ t('mist.drag.layout.title') }}</h4>
        <div class="mist-layout-grid">
          <button
            v-for="id in LAYOUT_PRESETS"
            :key="id"
            type="button"
            class="mist-layout-item"
            :class="current === id ? 'mist-layout-on' : ''"
            :aria-disabled="disabled[id] ? 'true' : undefined"
            :aria-pressed="current === id"
            :title="disabled[id] || t(`mist.drag.layout.${id}`)"
            :data-layout-item="id"
            @click="pick(id)"
          >
            <span class="mist-layout-thumb" :style="{ gridTemplateColumns: CELLS[id].cols, gridTemplateRows: CELLS[id].rows }" aria-hidden="true">
              <i v-for="(cell, n) in CELLS[id].cells" :key="n" :style="cell" />
            </span>
            {{ t(`mist.drag.layout.${id}`) }}
          </button>
        </div>
        <button type="button" class="mist-layout-even" :aria-disabled="disabled.even ? 'true' : undefined" :title="disabled.even" data-layout-item="even" @click="even">
          {{ t('mist.drag.layout.even') }}
        </button>
        <p class="mist-layout-hint">{{ t('mist.drag.layout.hint') }}</p>
      </div>
    </Transition>
    </Teleport>
  </div>
</template>
