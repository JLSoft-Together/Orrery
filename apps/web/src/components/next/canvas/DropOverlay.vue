<script setup lang="ts">
import { computed } from 'vue'
import { ArrowLeftRight } from 'lucide-vue-next'
import { COMPASS_GAP, COMPASS_TILE, type CompassZone } from '../../../lib/next/dropCompass'

interface Area {
  left: number
  top: number
  width: number
  height: number
}

const props = defineProps<{
  preview: (Area & { edge?: string; zone?: CompassZone; target?: Area; from?: Area; bad?: { left: number; min: number } }) | null
  label: string
  compass?: boolean
}>()

const swap = computed(() => props.preview?.zone === 'center')
const target = computed(() => props.preview?.target ?? null)
const center = computed(() => (target.value ? { x: target.value.left + target.value.width / 2, y: target.value.top + target.value.height / 2 } : null))
const step = COMPASS_TILE + COMPASS_GAP
const TILES: Array<{ zone: CompassZone; dx: number; dy: number }> = [
  { zone: 'top', dx: 0, dy: -1 },
  { zone: 'left', dx: -1, dy: 0 },
  { zone: 'center', dx: 0, dy: 0 },
  { zone: 'right', dx: 1, dy: 0 },
  { zone: 'bottom', dx: 0, dy: 1 },
]
const box = (a: Area): Record<string, string> => ({ transform: `translate(${a.left}px, ${a.top}px)`, width: `${a.width}px`, height: `${a.height}px` })
</script>

<template>
  <div v-if="preview" class="pointer-events-none absolute inset-0 z-30" aria-hidden="true" data-next-drop>
    <div v-if="swap && preview.from" class="mist-drop-zone mist-drop-swap" :style="box(preview.from)" data-drop-swap-from />
    <div
      class="mist-drop-zone"
      :class="preview.bad ? 'mist-drop-bad' : swap ? 'mist-drop-swap' : 'mist-drop-split'"
      :style="box(preview)"
      data-drop-preview
      :data-edge="preview.edge ?? 'swap'"
      :data-bad="preview.bad ? '' : undefined"
    >
      <span v-if="label" class="mist-drop-label">{{ label }}</span>
    </div>
    <div
      v-if="compass && center"
      class="mist-compass"
      :style="{ transform: `translate(${center.x}px, ${center.y}px)` }"
      data-drop-compass
    >
      <span
        v-for="tile in TILES"
        :key="tile.zone"
        class="mist-compass-tile"
        :class="preview.zone === tile.zone ? (preview.bad ? 'mist-compass-bad' : 'mist-compass-hot') : ''"
        :style="{ transform: `translate(${tile.dx * step - COMPASS_TILE / 2}px, ${tile.dy * step - COMPASS_TILE / 2}px)` }"
        :data-compass-zone="tile.zone"
        :data-hot="preview.zone === tile.zone || undefined"
      >
        <ArrowLeftRight v-if="tile.zone === 'center'" :size="16" />
        <span v-else class="mist-compass-half" :data-side="tile.zone" />
      </span>
    </div>
  </div>
</template>
