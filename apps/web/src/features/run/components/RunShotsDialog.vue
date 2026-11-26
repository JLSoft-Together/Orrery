<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Camera, Check, CircleCheck, CircleSlash, CircleX, Clock, Download, LoaderCircle, Play, Plus, Send, Smartphone, TabletSmartphone, TriangleAlert, X } from 'lucide-vue-next'
import { retireOverlay } from '../../../core/lib/motion'
import SegmentedControl from '../../../core/components/ui/SegmentedControl.vue'
import { useShotsHandoff } from '../composables/useShotsHandoff'
import { MULTI_MAX, VARIANTS, type MultiStage, type Shot, type VariantId } from '../lib/multiRun'
import { pickToggle, ratioOf, saveShot, shotFileName, stageTone, type StageTone } from '../lib/multiRunUi'
import { useMultiRunStore } from '../stores/multiRun'
import { useRunStore } from '../stores/run'

const props = defineProps<{ open: boolean; root: string }>()
const emit = defineEmits<{ close: [] }>()

const { t, te } = useI18n()
const run = useRunStore()
const multi = useMultiRunStore()
const handoff = useShotsHandoff(() => props.root)

const selected = ref<string[]>([])
const variant = ref<VariantId>('light')
const note = ref<{ tone: 'ok' | 'warn'; text: string } | null>(null)
const sheet = ref<HTMLElement | null>(null)

const config = computed(() => {
  const c = run.selectedConfig(props.root)
  return c?.type === 'android' ? c : null
})
const ready = computed(() => run.devicesFor(config.value).filter((d) => d.state === 'ready'))
const state = computed(() => multi.stateFor(props.root))
const running = computed(() => !!state.value?.running)
const rows = computed(() => Object.entries(state.value?.stages ?? {}))
const shots = computed(() => (state.value?.outcome?.shots ?? []).filter((s): s is Shot & { png: string } => s.stage === 'done' && !!s.png))
const restoreFailed = computed(() => state.value?.outcome?.restoreFailed ?? [])
const canRun = computed(() => !!config.value && !running.value && selected.value.length > 0)

const nameOf = (id: string): string => run.devices.find((d) => d.id === id)?.name ?? id
const variantText = (id: VariantId): string => t(`runMulti.variants.${id}`)
const variantOptions = computed(() => VARIANTS.map((v) => ({ value: v.id, label: variantText(v.id) })))

watch(
  () => props.open,
  (open) => {
    if (!open) return
    note.value = null
    selected.value = selected.value.filter((id) => ready.value.some((d) => d.id === id))
    if (!selected.value.length) selected.value = ready.value.slice(0, 2).map((d) => d.id)
    void run.refreshDevices()
  },
  { immediate: true },
)

function toggle(id: string): void {
  if (running.value) return
  selected.value = pickToggle(selected.value, id, MULTI_MAX)
}

async function start(): Promise<void> {
  if (!config.value || !canRun.value) return
  note.value = null
  await multi.startMany(props.root, config.value.id, selected.value, variant.value)
}

function close(): void {
  if (running.value) multi.cancelMany(props.root)
  emit('close')
}

function errorText(code?: string): string {
  if (!code) return ''
  return te(`runMulti.err.${code}`) ? t(`runMulti.err.${code}`) : t('runMulti.err.unknown', { code })
}

async function send(list: (Shot & { png: string })[]): Promise<void> {
  const r = await handoff.attach(list.map((s) => ({ png: s.png, device: nameOf(s.deviceId), variant: s.variant })))
  note.value = r === 'ok' ? { tone: 'ok', text: t('runMulti.sent') } : { tone: 'warn', text: t(r === 'no-agent' ? 'runMulti.noAgent' : 'runMulti.sendFailed') }
}

const save = (s: Shot & { png: string }): void => saveShot(s.png, shotFileName(nameOf(s.deviceId), s.variant))

const TONE: Record<StageTone, string> = { busy: 'text-st-working', wait: 'text-ink-muted', ok: 'text-success', error: 'text-st-error', muted: 'text-st-stale' }
const stageOf = (s: MultiStage): string => t(`runMulti.stage.${s}`)
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" @before-leave="(el) => retireOverlay(el.firstElementChild!)">
      <div v-if="open" class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[6vh] sm:px-4" @mousedown.self="close">
        <div ref="sheet" role="dialog" aria-modal="true" aria-labelledby="run-multi-title" class="ccm-sheet flex max-h-[88vh] w-full max-w-[820px] flex-col overflow-hidden" data-run-multi @keydown.esc.stop="close">
          <header class="flex items-center gap-2 border-b border-line px-4 py-3">
            <Camera :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
            <h2 id="run-multi-title" class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">{{ t('runMulti.title') }}</h2>
            <button type="button" class="ccm-btn-plain shrink-0" :title="t('runMulti.close')" :aria-label="t('runMulti.close')" data-multi-close @click="close">
              <X :size="14" aria-hidden="true" />
            </button>
          </header>

          <div class="ccm-scroll flex min-h-0 flex-1 flex-col gap-4 overflow-x-hidden overflow-y-auto p-4">
            <p class="text-2xs leading-relaxed text-ink-faint">{{ t('runMulti.hint') }}</p>

            <section class="flex flex-col gap-2" :aria-label="t('runMulti.devices')">
              <h3 class="text-2xs font-semibold text-ink-faint">{{ t('runMulti.devices') }}</h3>
              <p v-if="ready.length < 2" class="text-xs text-warning" data-multi-need>{{ t('runMulti.needTwo') }}</p>
              <div class="flex flex-wrap gap-1.5" role="group" :aria-label="t('runMulti.devices')">
                <button
                  v-for="d in ready"
                  :key="d.id"
                  type="button"
                  class="ccm-btn"
                  :class="selected.includes(d.id) ? 'bg-accent text-on-accent' : ''"
                  :aria-pressed="selected.includes(d.id)"
                  :disabled="running || (!selected.includes(d.id) && selected.length >= MULTI_MAX)"
                  :title="d.id"
                  :data-multi-device="d.id"
                  @click="toggle(d.id)"
                >
                  <Check v-if="selected.includes(d.id)" :size="12" aria-hidden="true" />
                  <Plus v-else :size="12" aria-hidden="true" />
                  <TabletSmartphone v-if="d.emulator" :size="13" aria-hidden="true" />
                  <Smartphone v-else :size="13" aria-hidden="true" />
                  <span class="max-w-40 truncate">{{ d.name }}</span>
                </button>
              </div>
              <p v-if="selected.length >= MULTI_MAX" class="text-2xs text-ink-faint">{{ t('runMulti.pickMax') }}</p>
            </section>

            <div class="flex flex-wrap items-center gap-3">
              <SegmentedControl v-model="variant" :options="variantOptions" :label="t('runMulti.variant')" :disabled="running" data-multi-variants />
              <button v-if="!running" type="button" class="ccm-btn" :disabled="!canRun" data-multi-run @click="start">
                <Play :size="13" aria-hidden="true" />{{ state?.outcome ? t('runMulti.again') : t('runMulti.run') }}
              </button>
              <button v-else type="button" class="ccm-btn" :disabled="state?.cancelling" data-multi-cancel @click="multi.cancelMany(root)">
                <LoaderCircle v-if="state?.cancelling" :size="13" class="ccm-spin" aria-hidden="true" /><X v-else :size="13" aria-hidden="true" />{{ state?.cancelling ? t('runMulti.cancelling') : t('runMulti.cancel') }}
              </button>
            </div>

            <ul v-if="rows.length" class="ccm-group divide-y divide-line" :aria-label="t('runMulti.progress')" aria-live="polite" data-multi-status>
              <li v-for="[id, s] in rows" :key="id" class="flex min-w-0 items-center gap-2 px-3 py-2 text-xs" :data-multi-row="id" :data-multi-stage="s.stage">
                <span class="flex w-36 shrink-0 items-center gap-1.5 font-medium" :class="TONE[stageTone(s.stage)]">
                  <LoaderCircle v-if="stageTone(s.stage) === 'busy'" :size="13" class="ccm-spin shrink-0" aria-hidden="true" />
                  <CircleCheck v-else-if="s.stage === 'done'" :size="13" class="shrink-0" aria-hidden="true" />
                  <CircleX v-else-if="s.stage === 'failed'" :size="13" class="shrink-0" aria-hidden="true" />
                  <CircleSlash v-else-if="s.stage === 'cancelled'" :size="13" class="shrink-0" aria-hidden="true" />
                  <Clock v-else :size="13" class="shrink-0" aria-hidden="true" />
                  <span class="truncate">{{ stageOf(s.stage) }}</span>
                </span>
                <span class="min-w-0 flex-1 truncate text-sm text-ink" :title="id">{{ nameOf(id) }}</span>
                <span v-if="s.error" class="min-w-0 max-w-[55%] truncate text-2xs text-st-error" :title="errorText(s.error)">{{ errorText(s.error) }}</span>
              </li>
            </ul>

            <p v-if="restoreFailed.length" class="flex items-start gap-1.5 text-xs text-warning" role="alert" data-multi-restore-failed>
              <TriangleAlert :size="13" class="mt-0.5 shrink-0" aria-hidden="true" />{{ t('runMulti.restoreFailed', { names: restoreFailed.map(nameOf).join(', ') }) }}
            </p>
            <p v-if="note" class="text-xs" :class="note.tone === 'ok' ? 'text-success' : 'text-warning'" role="status" data-multi-note>{{ note.text }}</p>
            <p v-else-if="state?.outcome && !restoreFailed.length" class="text-2xs text-ink-faint" role="status" data-multi-back>{{ t('runMulti.settingsBack') }}</p>

            <section v-if="shots.length" class="flex flex-col gap-2" :aria-label="t('runMulti.shots')">
              <div class="flex items-center gap-2">
                <h3 class="min-w-0 flex-1 text-2xs font-semibold text-ink-faint">{{ t('runMulti.shots') }}</h3>
                <button type="button" class="ccm-btn shrink-0" data-multi-send-all @click="send(shots)"><Send :size="13" aria-hidden="true" />{{ t('runMulti.sendAll') }}</button>
              </div>
              <ul class="grid gap-3" style="grid-template-columns: repeat(auto-fit, minmax(160px, 1fr))" data-multi-grid>
                <li v-for="s in shots" :key="s.deviceId" class="flex min-w-0 flex-col gap-1.5" :data-multi-shot="s.deviceId">
                  <img :src="s.png" :alt="t('runMulti.shotAlt', { name: nameOf(s.deviceId), variant: variantText(s.variant) })" class="w-full rounded-inner border border-line bg-well object-contain" :style="{ aspectRatio: ratioOf(s.width, s.height), maxHeight: '420px' }" />
                  <div class="flex min-w-0 items-center gap-1.5">
                    <span class="min-w-0 flex-1 truncate text-xs text-ink" :title="s.deviceId">{{ nameOf(s.deviceId) }}</span>
                    <span class="shrink-0 rounded-full bg-fill px-1.5 text-2xs text-ink-muted">{{ variantText(s.variant) }}</span>
                  </div>
                  <div class="flex items-center gap-1">
                    <button type="button" class="ccm-btn-plain" :title="t('runMulti.send')" :aria-label="`${t('runMulti.send')}: ${nameOf(s.deviceId)}`" data-multi-send @click="send([s])"><Send :size="13" aria-hidden="true" /></button>
                    <button type="button" class="ccm-btn-plain" :title="t('runMulti.save')" :aria-label="`${t('runMulti.save')}: ${nameOf(s.deviceId)}`" data-multi-save @click="save(s)"><Download :size="13" aria-hidden="true" /></button>
                  </div>
                </li>
              </ul>
            </section>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
