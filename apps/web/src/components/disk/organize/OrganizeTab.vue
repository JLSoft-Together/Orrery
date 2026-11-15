<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, FolderPlus, X } from 'lucide-vue-next'
import { formatSize } from '../../../lib/diskMap'
import { baseName, ORGANIZE_KINDS, type OrganizeMode } from '../../../lib/organize'
import { useOrganizeStore } from '../../../stores/organize'
import SegmentedControl from '../../ui/SegmentedControl.vue'
import ToggleSwitch from '../../ToggleSwitch.vue'
import OrganizeRules from './OrganizeRules.vue'
import OrganizeHistory from './OrganizeHistory.vue'
import OrganizeJunk from './OrganizeJunk.vue'

const { t, te, locale } = useI18n()
const store = useOrganizeStore()
const detail = ref(false)

const minutes = computed(() => Math.max(1, Math.round((store.state?.stableMs ?? 120_000) / 60_000)))
const modes = computed(() => (['type', 'month'] as OrganizeMode[]).map((m) => ({ value: m, label: t(`diskOrganize.mode.${m}`) })))
const current = computed(() => store.sources.find((s) => s.path === store.settings?.source) ?? null)
const hasDownloads = computed(() => store.sources.some((s) => s.label === 'downloads'))
const hasDesktop = computed(() => store.sources.some((s) => s.label === 'desktop'))
const allOn = computed(() => !!store.plan && store.off.size === 0)

const sourceName = (p: string, label: string): string => (label === 'custom' ? baseName(p) : t(`diskOrganize.sourceLabel.${label}`))
const errText = (r: string): string => (te(`diskOrganize.error.${r}`) ? t(`diskOrganize.error.${r}`) : r)
const skipText = (r: string): string => (te(`diskOrganize.skip.${r}`) ? t(`diskOrganize.skip.${r}`, { min: minutes.value }) : r)
const sep = computed(() => (store.plan?.source.includes('\\') ? '\\' : '/'))

onMounted(() => void store.init())
onBeforeUnmount(() => store.dispose())
</script>

<template>
  <section class="mt-4 flex flex-col gap-4" data-organize>
    <p class="max-w-[80ch] text-xs leading-relaxed text-ink-muted">{{ t('diskOrganize.hint') }}</p>

    <div class="flex flex-col gap-3 rounded-card border border-line p-3">
      <div class="flex flex-wrap items-center gap-2">
        <span class="ccm-label shrink-0">{{ t('diskOrganize.source') }}</span>
        <div class="flex min-w-0 flex-1 flex-wrap gap-1.5" role="radiogroup" :aria-label="t('diskOrganize.source')" data-organize-sources>
          <span
            v-for="s in store.sources"
            :key="s.path"
            class="flex min-w-0 max-w-full items-center rounded-control border"
            :class="s.path === store.settings?.source ? 'border-accent bg-accent-soft' : 'border-line'"
          >
            <button
              type="button"
              role="radio"
              class="min-w-0 truncate px-2 py-1 text-xs"
              :class="s.ok ? '' : 'text-ink-muted line-through'"
              :aria-checked="s.path === store.settings?.source"
              :title="s.ok ? s.path : `${s.path}\n${errText(s.reason ?? '')}`"
              :data-organize-source="s.label"
              @click="store.setSource(s.path)"
            >
              {{ sourceName(s.path, s.label) }}
            </button>
            <button type="button" class="ccm-btn-plain -ml-1 shrink-0 px-1" :aria-label="t('diskOrganize.removeSource', { name: sourceName(s.path, s.label) })" :title="t('diskOrganize.removeSource', { name: sourceName(s.path, s.label) })" @click="store.removeSource(s.path)">
              <X :size="12" aria-hidden="true" />
            </button>
          </span>
          <span v-if="!store.sources.length" class="text-xs text-ink-muted">{{ t('diskOrganize.noSource') }}</span>
        </div>
        <div class="flex flex-wrap gap-1.5">
          <button v-if="!hasDownloads" type="button" class="ccm-btn" @click="store.addSource('downloads')">{{ t('diskOrganize.addDownloads') }}</button>
          <button v-if="!hasDesktop" type="button" class="ccm-btn" @click="store.addSource('desktop')">{{ t('diskOrganize.addDesktop') }}</button>
          <button type="button" class="ccm-btn" data-organize-add @click="store.addSource('pick')"><FolderPlus :size="14" aria-hidden="true" />{{ t('diskOrganize.addFolder') }}</button>
        </div>
      </div>
      <p v-if="store.sourceError" class="flex items-center gap-1.5 text-xs text-warning" role="alert" data-organize-source-error>
        <CircleAlert :size="13" aria-hidden="true" />{{ t('diskOrganize.sourceBad', { reason: errText(store.sourceError) }) }}
      </p>
      <p v-else-if="current && !current.ok" class="flex items-center gap-1.5 text-xs text-warning" role="alert">
        <CircleAlert :size="13" aria-hidden="true" />{{ t('diskOrganize.sourceBad', { reason: errText(current.reason ?? '') }) }}
      </p>

      <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
        <div class="flex items-center gap-2">
          <span class="ccm-label">{{ t('diskOrganize.mode.label') }}</span>
          <SegmentedControl
            v-if="store.settings"
            :model-value="store.settings.mode"
            :options="modes"
            :label="t('diskOrganize.mode.label')"
            data-organize-mode
            @update:model-value="(m: OrganizeMode) => store.setMode(m)"
          />
        </div>
        <div v-if="store.settings?.mode === 'type'" class="flex min-w-0 flex-wrap items-center gap-1.5" role="group" :aria-label="t('diskOrganize.kindsLabel')" data-organize-kinds>
          <button
            v-for="k in ORGANIZE_KINDS"
            :key="k"
            type="button"
            class="ccm-btn"
            :class="store.settings.kinds.includes(k) ? 'ccm-btn-primary' : ''"
            :aria-pressed="store.settings.kinds.includes(k)"
            @click="store.toggleKind(k)"
          >
            {{ t(`diskOrganize.kind.${k}`) }}
          </button>
        </div>
      </div>

      <OrganizeRules v-if="store.settings" :rules="store.settings.rules" @save="store.saveRules" />

      <ToggleSwitch
        v-if="store.settings"
        :model-value="store.settings.auto"
        :label="t('diskOrganize.auto.label')"
        :hint="t('diskOrganize.auto.hint', { min: minutes })"
        data-organize-auto
        @update:model-value="(on: boolean) => store.setAuto(on)"
      />
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <button type="button" class="ccm-btn" :disabled="!current?.ok || store.planning || store.running" data-organize-preview @click="store.preview()">
        {{ store.planning ? t('diskOrganize.previewing') : store.plan ? t('diskOrganize.previewAgain') : t('diskOrganize.preview') }}
      </button>
      <button
        v-if="store.plan && store.plan.items.length"
        type="button"
        class="ccm-btn ccm-btn-primary"
        :disabled="!store.chosen.length || store.running"
        data-organize-run
        @click="store.run()"
      >
        {{ store.running ? t('diskOrganize.running') : t('diskOrganize.run', { n: store.chosen.length }) }}
      </button>
      <span v-if="store.plan && store.chosen.length" class="text-xs text-ink-muted tabular-nums">{{ formatSize(store.chosenBytes, locale) }}</span>
    </div>

    <div v-if="store.result" class="flex flex-wrap items-center gap-2 rounded-card border border-line px-3 py-2 text-xs" role="status" data-organize-result>
      <template v-if="store.result.error">
        <CircleAlert :size="14" class="text-warning" aria-hidden="true" /><span class="flex-1">{{ errText(store.result.error) }}</span>
      </template>
      <template v-else>
        <CircleCheck :size="14" class="text-success" aria-hidden="true" />
        <span class="min-w-0 flex-1">
          {{ t('diskOrganize.result', { n: store.result.moved, size: formatSize(store.result.bytes, locale) }) }}
          <span v-if="store.result.failed.length" class="text-warning">, {{ t('diskOrganize.resultFailed', { n: store.result.failed.length }) }}</span>
        </span>
        <button v-if="store.result.failed.length" type="button" class="ccm-btn-plain" @click="detail = !detail">{{ t('diskOrganize.details') }}</button>
      </template>
      <button type="button" class="ccm-btn-plain" :aria-label="t('diskOrganize.dismiss')" @click="store.dismiss()"><X :size="13" aria-hidden="true" /></button>
      <ul v-if="detail && store.result.failed.length" class="w-full space-y-0.5 text-ink-muted">
        <li v-for="f in store.result.failed" :key="f.name" class="truncate">{{ f.name }}: {{ skipText(f.reason) }}</li>
      </ul>
    </div>

    <div v-if="store.plan" class="flex flex-col gap-2" data-organize-plan>
      <p v-if="store.plan.error" class="flex items-center gap-1.5 text-xs text-warning" role="alert"><CircleAlert :size="13" aria-hidden="true" />{{ errText(store.plan.error) }}</p>
      <p v-else-if="!store.plan.items.length" class="rounded-card border border-dashed border-line px-4 py-6 text-center text-sm text-ink-muted" data-organize-nothing>{{ t('diskOrganize.nothing') }}</p>
      <div v-else class="ccm-panel overflow-hidden rounded-card border border-line">
        <div class="flex items-center gap-2 border-b border-line px-3 py-1.5 text-2xs text-ink-muted">
          <input type="checkbox" class="shrink-0" :checked="allOn" :aria-label="allOn ? t('diskOrganize.selectNone') : t('diskOrganize.selectAll')" data-organize-all @change="store.setAll(!allOn)" />
          <span class="min-w-0 flex-1">{{ t('diskOrganize.colFile') }}</span>
          <span class="hidden min-w-0 flex-1 sm:block">{{ t('diskOrganize.colTarget') }}</span>
          <span class="w-20 shrink-0 text-right">{{ t('diskOrganize.colSize') }}</span>
        </div>
        <ul class="max-h-[22rem] divide-y divide-line overflow-y-auto overflow-x-hidden">
          <li v-for="i in store.plan.items" :key="i.id" class="flex items-center gap-2 px-3 py-1.5 text-xs" :data-organize-item="i.name">
            <input type="checkbox" class="shrink-0" :checked="!store.off.has(i.id)" :aria-label="i.name" @change="store.toggleItem(i.id)" />
            <span class="min-w-0 flex-1 truncate" :title="i.name">{{ i.name }}</span>
            <span class="hidden min-w-0 flex-1 truncate text-ink-muted sm:block" :title="`${i.folder}${sep}${i.target}`">{{ i.folder }}{{ sep }}{{ i.target }}</span>
            <span class="w-20 shrink-0 text-right tabular-nums text-ink-muted">{{ formatSize(i.size, locale) }}</span>
          </li>
        </ul>
      </div>
      <details v-if="store.plan.skipped.length" class="text-xs text-ink-muted" data-organize-skipped>
        <summary class="cursor-pointer select-none">{{ t('diskOrganize.skippedTitle', { n: store.plan.skipped.length }) }}</summary>
        <ul class="mt-1 max-h-40 space-y-0.5 overflow-y-auto overflow-x-hidden">
          <li v-for="s in store.plan.skipped" :key="s.name" class="truncate">{{ s.name }}: {{ skipText(s.reason) }}</li>
        </ul>
      </details>
    </div>

    <OrganizeHistory />
    <OrganizeJunk />
  </section>
</template>
