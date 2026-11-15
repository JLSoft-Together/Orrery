<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { dragPaths, hasDragPath } from '../../../lib/explorer/dragPath'
import { compassTiles, dropFit, zoneRect, DOCK_PERCENT, type CompassZone } from '../../../lib/next/dropCompass'
import DropOverlay from './DropOverlay.vue'

const props = defineProps<{ canvas: HTMLElement | null; enabled: boolean }>()
const emit = defineEmits<{ drop: [key: string, zone: CompassZone, paths: string[]] }>()
const { t } = useI18n()

interface Area {
  left: number
  top: number
  width: number
  height: number
}

const target = ref<{ key: string; area: Area } | null>(null)
const hot = ref<CompassZone | null>(null)
let paths: string[] = []

const nameOf = (p: string): string => p.split(/[\\/]/).filter(Boolean).pop() ?? p

function paneAt(x: number, y: number): { key: string; area: Area } | null {
  const el = props.canvas
  if (!el) return null
  const box = el.getBoundingClientRect()
  if (x < box.left || x > box.right || y < box.top || y > box.bottom) return null
  for (const pane of el.querySelectorAll<HTMLElement>('[data-pane]')) {
    const r = pane.getBoundingClientRect()
    if (!r.width || !r.height || x < r.left || x > r.right || y < r.top || y > r.bottom) continue
    return { key: pane.dataset.pane!, area: { left: r.left - box.left, top: r.top - box.top, width: r.width, height: r.height } }
  }
  return null
}

function reset(): void {
  target.value = null
  hot.value = null
}

function onDragStart(e: DragEvent): void {
  paths = hasDragPath(e.dataTransfer) ? dragPaths(e.dataTransfer) : []
}

function onDocOver(e: DragEvent): void {
  if (!props.enabled || !hasDragPath(e.dataTransfer)) return
  if ((e.target as Element | null)?.closest?.('[data-file-compass]')) return
  const hit = paneAt(e.clientX, e.clientY)
  if (!hit) return reset()
  if (target.value?.key !== hit.key) hot.value = null
  target.value = hit
}

function onEnd(): void {
  reset()
  paths = []
}

const tiles = computed(() => {
  const a = target.value?.area
  if (!a) return []
  return compassTiles({ left: a.left, top: a.top, right: a.left + a.width, bottom: a.top + a.height })
})

const fit = computed(() => (target.value && hot.value ? dropFit(target.value.area, hot.value) : null))

const preview = computed(() => {
  const a = target.value?.area
  const z = hot.value
  if (!a || !z) return null
  const r = zoneRect(a, z)
  const f = fit.value
  return { ...r, zone: z, target: a, edge: z === 'center' ? undefined : z, bad: f && !f.ok ? { left: f.left, min: f.min } : undefined }
})

const label = computed(() => {
  const p = preview.value
  if (!p) return ''
  if (p.bad) return t('mist.drag.tooNarrow', { left: p.bad.left, min: p.bad.min })
  const name = paths.length === 1 ? nameOf(paths[0]!) : t('mist.drag.files', { n: paths.length || 1 }, paths.length || 1)
  if (p.zone === 'center') return t('mist.drag.openTab', { name })
  return t('mist.drag.openFile', { name, edge: t(`mist.drag.edge.${p.zone}`), pct: DOCK_PERCENT })
})

function onTileOver(e: DragEvent, zone: CompassZone): void {
  if (!hasDragPath(e.dataTransfer)) return
  e.preventDefault()
  e.stopPropagation()
  hot.value = zone
  if (e.dataTransfer) e.dataTransfer.dropEffect = fit.value && !fit.value.ok ? 'none' : 'copy'
}

function onTileLeave(zone: CompassZone): void {
  if (hot.value === zone) hot.value = null
}

function onTileDrop(e: DragEvent, zone: CompassZone): void {
  e.preventDefault()
  e.stopPropagation()
  const key = target.value?.key
  const list = dragPaths(e.dataTransfer)
  const ok = !fit.value || fit.value.ok
  reset()
  if (key && list.length && ok) emit('drop', key, zone, list)
}

onMounted(() => {
  document.addEventListener('dragstart', onDragStart)
  document.addEventListener('dragover', onDocOver, true)
  document.addEventListener('dragend', onEnd, true)
  document.addEventListener('drop', onEnd)
})
onBeforeUnmount(() => {
  document.removeEventListener('dragstart', onDragStart)
  document.removeEventListener('dragover', onDocOver, true)
  document.removeEventListener('dragend', onEnd, true)
  document.removeEventListener('drop', onEnd)
})
</script>

<template>
  <template v-if="enabled && target">
    <DropOverlay :preview="preview" :label="label" />
    <div class="pointer-events-none absolute inset-0 z-40" data-file-compass>
      <span
        v-for="tile in tiles"
        :key="tile.zone"
        class="mist-compass-tile mist-compass-drop pointer-events-auto absolute top-0 left-0"
        :class="hot === tile.zone ? (fit && !fit.ok ? 'mist-compass-bad' : 'mist-compass-hot') : ''"
        :style="{ transform: `translate(${tile.left}px, ${tile.top}px)` }"
        :data-compass-zone="tile.zone"
        aria-hidden="true"
        @dragenter="onTileOver($event, tile.zone)"
        @dragover="onTileOver($event, tile.zone)"
        @dragleave="onTileLeave(tile.zone)"
        @drop="onTileDrop($event, tile.zone)"
      >
        <span class="mist-compass-half" :data-side="tile.zone" />
      </span>
    </div>
  </template>
</template>
