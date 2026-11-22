<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Sparkles, X } from 'lucide-vue-next'
import { hasTerminals } from '../../../core/lib/desktop'
import { tourShownThisSession } from '../../settings/lib/onboardingGate'
import { tourDone, tourFor } from '../../settings/lib/tour'
import { UI_GENERATION_KEY } from '../../../core/lib/uiGeneration'
import { canOfferNextInvite, readEverUsedNext, readInvite, writeInvite, type InviteState } from '../lib/invite'
import { useUiStore } from '../../../core/stores/ui'
import { useUiGenStore } from '../stores/uiGen'

const { t } = useI18n()
const ui = useUiStore()
const uiGen = useUiGenStore()
const shown = ref(false)
let decided = false
let timer = 0

function tourDue(): boolean {
  if (ui.compact) return false
  const id = tourFor(ui.view, hasTerminals)
  return !!id && !tourDone(id)
}

const allowed = (): boolean =>
  canOfferNextInvite({
    isNext: uiGen.isNext,
    everUsedNext: readEverUsedNext(UI_GENERATION_KEY),
    alreadyAnswered: readInvite() !== null,
    desktop: hasTerminals,
    setupOpen: ui.setupOpen,
    whatsNewOpen: ui.whatsNewOpen,
    tourOpen: !!ui.tourOpen,
    tourDue: tourDue(),
    tourShownThisSession: tourShownThisSession(),
    compact: ui.compact,
  })

function offer(): void {
  window.clearTimeout(timer)
  if (decided || shown.value) return
  timer = window.setTimeout(() => {
    if (decided || shown.value || !allowed()) return
    shown.value = true
  }, 1500)
}

function answer(state: InviteState): void {
  decided = true
  writeInvite(state)
  shown.value = false
}

function tryNow(): void {
  answer('tried')
  uiGen.setGeneration('next')
}

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Escape' && shown.value) answer('closed')
}

watch(() => [ui.setupOpen, ui.whatsNewOpen, ui.tourOpen, ui.view, ui.compact], offer)
watch(
  () => uiGen.isNext,
  (next) => {
    if (next && shown.value) answer('tried')
  },
)
onMounted(() => {
  window.addEventListener('keydown', onKey)
  offer()
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  window.clearTimeout(timer)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-scrim">
      <section
        v-if="shown && !uiGen.isNext"
        class="ccm-sheet fixed right-4 bottom-10 z-[55] flex w-[320px] max-w-[calc(100vw-32px)] flex-col gap-2 p-4"
        role="complementary"
        aria-labelledby="mist-invite-title"
        data-mist-invite
      >
        <div class="flex items-center gap-2">
          <Sparkles :size="14" class="shrink-0 text-accent-ink" aria-hidden="true" />
          <h2 id="mist-invite-title" class="min-w-0 flex-1 text-sm font-semibold">{{ t('mist.invite.title') }}</h2>
          <button type="button" class="ccm-btn-plain -mr-1 shrink-0" :aria-label="t('mist.invite.close')" :title="t('mist.invite.close')" @click="answer('closed')">
            <X :size="14" aria-hidden="true" />
          </button>
        </div>
        <svg class="h-14 w-full rounded-control bg-fill" viewBox="0 0 120 40" preserveAspectRatio="none" aria-hidden="true">
          <rect x="4" y="4" width="22" height="32" rx="4" class="fill-line-strong" opacity="0.6" />
          <rect x="30" y="4" width="48" height="32" rx="4" class="fill-accent" opacity="0.4" />
          <rect x="82" y="4" width="34" height="32" rx="4" class="fill-line-strong" opacity="0.6" />
        </svg>
        <p class="text-xs leading-relaxed text-ink-muted">{{ t('mist.invite.body') }}</p>
        <div class="flex justify-end gap-2 pt-1">
          <button type="button" class="ccm-btn" @click="answer('later')">{{ t('mist.invite.later') }}</button>
          <button type="button" class="ccm-btn ccm-btn-primary" @click="tryNow">{{ t('mist.invite.try') }}</button>
        </div>
      </section>
    </Transition>
  </Teleport>
</template>
