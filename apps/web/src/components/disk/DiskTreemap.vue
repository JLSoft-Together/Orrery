<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatPercent, formatSize, GROUP_VAR, groupOf, topSlices, type DiskEntry } from '../../lib/diskMap'
import { cellAt, inset, squarify, type TreemapCell } from '../../lib/treemap'
import DiskTip from './DiskTip.vue'
import { useCanvas } from './useCanvas'

const MAX_CELLS = 2000

const props = defineProps<{ items: DiskEntry[]; total: number; selected: Set<string> }>()
const emit = defineEmits<{ open: [path: string]; pick: [path: string, range: boolean] }>()

const { t, locale } = useI18n()
const host = ref<HTMLElement | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)
const hover = ref<{ cell: TreemapCell; x: number; y: number } | null>(null)

const slices = computed(() => topSlices(props.items, MAX_CELLS, props.total))
let cells: TreemapCell[] = []

function draw({ ctx, w, h, color, font }: Parameters<Parameters<typeof useCanvas>[2]>[0]): void {
  cells = squarify(
    slices.value.map((s) => s.size),
    { x: 0, y: 0, w, h },
  )
  const ink = color('--ccm-ink')
  const accent = color('--ccm-accent')
  const hoverIndex = hover.value?.cell.index ?? -1
  for (const c of cells) {
    const s = slices.value[c.index]
    if (!s) continue
    const g = s.entry ? groupOf(s.entry) : 'rest'
    const base = color(GROUP_VAR[g])
    const r = inset(c, 1)
    if (r.w < 1 || r.h < 1) continue
    const picked = !!s.entry && props.selected.has(s.entry.path)
    ctx.fillStyle = color(GROUP_VAR[g], picked ? 0.5 : c.index === hoverIndex ? 0.38 : 0.3)
    ctx.fillRect(r.x, r.y, r.w, r.h)
    ctx.strokeStyle = picked ? accent : base
    ctx.lineWidth = picked ? 2 : 1
    ctx.strokeRect(r.x + 0.5, r.y + 0.5, Math.max(0, r.w - 1), Math.max(0, r.h - 1))
    if (r.w > 56 && r.h > 30) {
      ctx.save()
      ctx.beginPath()
      ctx.rect(r.x + 4, r.y + 2, r.w - 8, r.h - 4)
      ctx.clip()
      ctx.fillStyle = ink
      ctx.font = `600 11px ${font}`
      ctx.textBaseline = 'top'
      ctx.fillText(s.entry ? s.entry.name : t('diskMap.rest', { n: s.more }), r.x + 6, r.y + 5)
      if (r.h > 46) {
        ctx.font = `11px ${font}`
        ctx.globalAlpha = 0.7
        ctx.fillText(formatSize(s.size, locale.value), r.x + 6, r.y + 20)
      }
      ctx.restore()
    }
  }
}

const { size, redraw } = useCanvas(host, canvas, draw)
watch([slices, () => props.selected, hover, locale], redraw)

function local(e: MouseEvent): { x: number; y: number } {
  const b = canvas.value!.getBoundingClientRect()
  return { x: e.clientX - b.left, y: e.clientY - b.top }
}

function onMove(e: MouseEvent): void {
  const p = local(e)
  const cell = cellAt(cells, p.x, p.y)
  hover.value = cell ? { cell, x: p.x, y: p.y } : null
}

function onClick(e: MouseEvent): void {
  const p = local(e)
  const cell = cellAt(cells, p.x, p.y)
  const entry = cell ? (slices.value[cell.index]?.entry ?? null) : null
  if (!entry) return
  if (e.shiftKey || e.ctrlKey || e.metaKey || !entry.dir) emit('pick', entry.path, e.shiftKey)
  else emit('open', entry.path)
}

const tip = computed(() => {
  const hv = hover.value
  if (!hv) return null
  const s = slices.value[hv.cell.index]
  if (!s) return null
  const lines = [`${formatSize(s.size, locale.value)}, ${formatPercent(s.size, props.total)}`]
  if (s.entry) {
    lines.push(t(`diskMap.group.${groupOf(s.entry)}`))
    if (s.entry.dir && s.entry.files !== undefined) lines.push(t('diskMap.files', { n: s.entry.files }))
  }
  return { title: s.entry ? s.entry.name : t('diskMap.rest', { n: s.more }), lines }
})
</script>

<template>
  <div ref="host" class="relative h-full min-h-[14rem] w-full overflow-hidden" data-disk-treemap>
    <canvas ref="canvas" class="block cursor-pointer" role="img" :aria-label="t('diskMap.chart.treemap')" @mousemove="onMove" @mouseleave="hover = null" @click="onClick" />
    <DiskTip v-if="tip && hover" :x="hover.x" :y="hover.y" :w="size.w" :h="size.h" :title="tip.title" :lines="tip.lines" />
  </div>
</template>
