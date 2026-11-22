<script setup lang="ts">
import { onBeforeUnmount, toRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Expand, History, Shrink } from 'lucide-vue-next'
import KeyHint from '../../../core/components/KeyHint.vue'
import { useOfficeStore } from '../stores/office'
import { useOfficeControls } from './useOfficeControls'
import OfficeOptions from './OfficeOptions.vue'

const TOOL = 'ccm-btn-plain ccm-press text-xs'

const props = defineProps<{ canvas: HTMLElement | null }>()
const emit = defineEmits<{ 'update:fullscreen': [on: boolean] }>()

const { t } = useI18n()
const office = useOfficeStore()
const { officeControls, volumePct, volumeIcon, onVolumeWheel, officeFullscreen, toggleOfficeFullscreen } = useOfficeControls(toRef(props, 'canvas'))
watch(officeFullscreen, (on) => emit('update:fullscreen', on), { immediate: true })
onBeforeUnmount(() => emit('update:fullscreen', false))
</script>

<template>
  <button
    v-for="c in officeControls"
    :key="c.key"
    type="button"
    :class="TOOL"
    :aria-label="c.label"
    :title="c.label"
    :aria-keyshortcuts="c.hint"
    :disabled="!office.world"
    @click="c.run()"
  >
    <component :is="c.icon" :size="16" aria-hidden="true" />
    <KeyHint v-if="c.hint" :keys="c.hint" />
  </button>
  <div class="flex items-center" data-office-sound @wheel.prevent="onVolumeWheel">
    <button
      type="button"
      :class="TOOL"
      :aria-pressed="office.sound"
      :aria-label="t('officeV60.sound')"
      :title="t('officeV60.sound')"
      @click="office.setSound(!office.sound)"
    >
      <component :is="volumeIcon" :size="16" aria-hidden="true" />
    </button>
    <input
      type="range"
      class="ccm-slider mr-1.5 ml-0.5 w-20"
      min="0"
      max="100"
      step="5"
      :value="volumePct"
      :style="{ '--ccm-slider-value': `${volumePct}%` }"
      :data-muted="office.sound ? undefined : ''"
      :aria-label="t('officeV60.volume')"
      :aria-valuetext="t('officeV60.volumeValue', { n: volumePct })"
      :title="t('officeV60.volumeHint', { n: volumePct })"
      data-office-volume
      @input="office.setVolume(Number(($event.target as HTMLInputElement).value) / 100)"
    />
  </div>
  <button
    type="button"
    :class="TOOL"
    :aria-pressed="office.timelapse"
    :aria-label="office.timelapse ? t('officeV60.timelapse.close') : t('officeV60.timelapse.open')"
    :title="office.timelapse ? t('officeV60.timelapse.close') : t('officeV60.timelapse.open')"
    aria-keyshortcuts="L"
    :disabled="!office.world && !office.timelapse"
    data-office-timelapse-toggle
    @click="office.toggleTimelapse()"
  >
    <History :size="16" aria-hidden="true" />
    <KeyHint keys="L" />
  </button>
  <OfficeOptions />
  <button
    type="button"
    :class="TOOL"
    :aria-pressed="officeFullscreen"
    :aria-label="officeFullscreen ? t('officeV62.fullscreen.exit') : t('officeV62.fullscreen.enter')"
    :title="officeFullscreen ? t('officeV62.fullscreen.exitHint') : t('officeV62.fullscreen.enterHint')"
    aria-keyshortcuts="Shift+F"
    data-office-fullscreen-toggle
    @click="toggleOfficeFullscreen()"
  >
    <Transition name="ccm-swap" mode="out-in">
      <component :is="officeFullscreen ? Shrink : Expand" :key="officeFullscreen ? 'exit' : 'enter'" :size="16" aria-hidden="true" />
    </Transition>
    <KeyHint keys="⇧F" />
  </button>
</template>
