<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { Radio, Send } from 'lucide-vue-next'
import { useTelegramStore } from '../stores/telegram'
import { useToastsStore } from '../../../core/stores/toasts'
import { useUiStore } from '../../../core/stores/ui'

const props = withDefaults(defineProps<{ iconOnly?: boolean }>(), { iconOnly: false })
const { t } = useI18n()
const tg = useTelegramStore()
const ui = useUiStore()
onMounted(() => tg.start())

const toasts = useToastsStore()
const FAIL_ID = 'telegram-toggle-fail'

const on = computed(() => tg.enabled)
const title = computed(() => (on.value ? t('telegram.top.onTitle') : t('telegram.top.offTitle')))

async function toggle(): Promise<void> {
  const res = await tg.toggle()
  if (res.ok) {
    toasts.remove(FAIL_ID)
    return
  }
  toasts.push({
    id: FAIL_ID,
    kind: 'phoneFail',
    title: t('telegram.failTitle'),
    text: t(`telegram.setError.${res.error}`),
    attrs: { 'data-telegram-toggle-fail': '' },
  })
}

async function stopReceiving(): Promise<void> {
  tg.busy = true
  try {
    await tg.setReceiving(false)
  } finally {
    tg.busy = false
  }
}
</script>

<template>
  <Transition name="ccm-fade">
    <button
      v-if="tg.ready"
      type="button"
      role="switch"
      :aria-checked="on"
      :aria-label="title"
      :title="title"
      :disabled="tg.busy"
      class="ccm-btn-plain gap-1 text-xs font-medium transition-colors duration-(--ccm-dur-base)"
      :class="[props.iconOnly ? 'relative size-8 rounded-control' : 'mr-1 h-6 rounded-full px-2', on ? 'bg-fill text-accent-ink' : 'text-ink-muted']"
      data-telegram-toggle
      :data-on="on || undefined"
      @click="toggle"
    >
      <Send :size="14" class="shrink-0 transition-transform duration-(--ccm-dur-base) ease-spring" :class="on ? '' : '-rotate-45'" aria-hidden="true" />
      <Transition name="ccm-swap" mode="out-in">
        <span v-if="!ui.compact && !props.iconOnly" :key="on ? 'on' : 'off'" class="hidden sm:inline">{{ on ? t('telegram.top.on') : t('telegram.top.off') }}</span>
      </Transition>
      <span class="size-1.5 shrink-0 rounded-full transition-colors duration-(--ccm-dur-base)" :class="[on ? 'bg-success' : 'bg-fill-strong', props.iconOnly ? 'absolute top-1 right-1' : '']" aria-hidden="true" />
    </button>
  </Transition>
  <Transition name="ccm-fade">
    <button
      v-if="tg.receiving"
      type="button"
      :aria-label="t('telegram.receive.topTitle')"
      :title="t('telegram.receive.topTitle')"
      :disabled="tg.busy"
      class="ccm-btn-plain gap-1 bg-warning-soft text-xs font-medium text-warning"
      :class="props.iconOnly ? 'size-8 rounded-control' : 'mr-1 h-6 rounded-full px-2'"
      data-telegram-receiving-chip
      @click="stopReceiving"
    >
      <Radio :size="14" class="shrink-0" aria-hidden="true" />
      <span v-if="!ui.compact && !props.iconOnly" class="hidden sm:inline">{{ t('telegram.receive.top') }}</span>
    </button>
  </Transition>
</template>
