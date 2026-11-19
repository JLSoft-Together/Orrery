<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatPercent, formatSize, GROUP_VAR, groupOf, topSlices, type DiskEntry } from '../../lib/diskMap'
import { arcAt, layoutRings, type SunburstArc, type SunburstGeometry } from '../../lib/sunburst'
import DiskTip from './DiskTip.vue'
import { useCanvas } from './useCanvas'

const MAX_ARCS = 80
const MAX_SUB = 12

const props = defineProps<{ items: DiskEntry[]; total: number; selected: Set<string>; name: string; sub: Map<string, DiskEntry[]> }>()
const emit = defineEmits<{ open: [path: string]; pick: [path: string, range: boolean]; need: [paths: string[]] }>()

const { t, locale } = useI18n()
const host = ref<HTMLElement | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)
const hover = ref<{ arc: SunburstArc; x: number; y: number } | null>(null)

const slices = computed(() => topSlices(props.items, MAX_ARCS, props.total))
const dirPaths = computed(() =>
  slices.value
    .filter((s) => s.entry?.dir)
    .slice(0, MAX_SUB)
    .map((s) => s.entry!.path),
)

const subOf = (i: number): DiskEntry[] => {
  const e = slices.value[i]?.entry
  return e ? (props.sub.get(e.path) ?? []) : []
}

const arcs = computed(() =>
  layoutRings(
    slices.value.map((s) => s.size),
    slices.value.map((_, i) => subOf(i).map((c) => c.size)),
    slices.value.map((s) => s.size),
  ),
)

const entryOf = (a: SunburstArc): DiskEntry | null => (a.ring === 0 ? (slices.value[a.index]?.entry ?? null) : (subOf(a.parent ?? -1)[a.index] ?? null))

let geo: SunburstGeometry = { cx: 0, cy: 0, inner: 0, ringWidth: 0 }

function draw({ ctx, w, h, color, font }: Parameters<Parameters<typeof useCanvas>[2]>[0]): void {
  const radius = Math.max(10, Math.min(w, h) / 2 - 6)
  const inner = radius * 0.3
  const ringWidth = (radius - inner) / 2
  geo = { cx: w / 2, cy: h / 2, inner, ringWidth }
  const hv = hover.value?.arc
  for (const a of arcs.value) {
    const e = entryOf(a)
    const g = e ? groupOf(e) : 'rest'
    const r0 = inner + a.ring * ringWidth
    const r1 = r0 + ringWidth - 1
    if (a.end - a.start < 0.002) continue
    const picked = !!e && props.selected.has(e.path)
    const hot = hv && hv.ring === a.ring && hv.index === a.index && hv.parent === a.parent
    ctx.beginPath()
    ctx.arc(geo.cx, geo.cy, r1, a.start, a.end)
    ctx.arc(geo.cx, geo.cy, r0, a.end, a.start, true)
    ctx.closePath()
    ctx.fillStyle = color(GROUP_VAR[g], picked ? 0.55 : hot ? 0.42 : a.ring === 0 ? 0.34 : 0.24)
    ctx.fill()
    ctx.strokeStyle = picked ? color('--ccm-accent') : color(GROUP_VAR[g])
    ctx.lineWidth = picked ? 2 : 1
    ctx.stroke()
  }
  ctx.fillStyle = color('--ccm-ink')
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.font = `600 12px ${font}`
  const label = props.name.length > 16 ? `${props.name.slice(0, 15)}...` : props.name
  ctx.fillText(label, geo.cx, geo.cy - 8, inner * 1.8)
  ctx.font = `11px ${font}`
  ctx.fillStyle = color('--ccm-ink-muted')
  ctx.fillText(formatSize(props.total, locale.value), geo.cx, geo.cy + 9, inner * 1.8)
}

const { size, redraw } = useCanvas(host, canvas, draw)
watch([arcs, () => props.selected, hover, () => props.name, locale], redraw)
watch(dirPaths, (p) => p.length && emit('need', p), { immediate: true })

function local(e: MouseEvent): { x: number; y: number } {
  const b = canvas.value!.getBoundingClientRect()
  return { x: e.clientX - b.left, y: e.clientY - b.top }
}

function onMove(e: MouseEvent): void {
  const p = local(e)
  const arc = arcAt(arcs.value, geo, p.x, p.y)
  hover.value = arc ? { arc, x: p.x, y: p.y } : null
}

function onClick(e: MouseEvent): void {
  const p = local(e)
  const arc = arcAt(arcs.value, geo, p.x, p.y)
  const entry = arc ? entryOf(arc) : null
  if (!entry) return
  if (e.shiftKey || e.ctrlKey || e.metaKey || !entry.dir) emit('pick', entry.path, e.shiftKey)
  else emit('open', entry.path)
}

const tip = computed(() => {
  const hv = hover.value
  if (!hv) return null
  const e = entryOf(hv.arc)
  const size_ = e ? e.size : (slices.value[hv.arc.index]?.size ?? 0)
  const parent = hv.arc.ring === 0 ? props.total : (slices.value[hv.arc.parent ?? -1]?.size ?? props.total)
  const lines = [`${formatSize(size_, locale.value)}, ${formatPercent(size_, parent)}`]
  if (e) lines.push(t(`diskMap.group.${groupOf(e)}`))
  return { title: e ? e.name : t('diskMap.rest', { n: slices.value[hv.arc.index]?.more ?? 0 }), lines }
})

</script>

<template>
  <div ref="host" class="relative h-full min-h-[14rem] w-full overflow-hidden" data-disk-sunburst>
    <canvas ref="canvas" class="block cursor-pointer" role="img" :aria-label="t('diskMap.chart.sunburst')" @mousemove="onMove" @mouseleave="hover = null" @click="onClick" />
    <DiskTip v-if="tip && hover" :x="hover.x" :y="hover.y" :w="size.w" :h="size.h" :title="tip.title" :lines="tip.lines" />
  </div>
</template>
