<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, CircleAlert, CircleCheck, CircleMinus, ClipboardCopy, Download, RefreshCw, X } from 'lucide-vue-next'
import { retireOverlay } from '../lib/motion'
import { useDiagnosticsReport } from '../composables/useDiagnosticsReport'
import { desktop } from '../lib/desktop'
import { useConnectionStore } from '../stores/connection'
import { useUiStore } from '../stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const connection = useConnectionStore()

const dialog = ref<HTMLElement | null>(null)
const copied = ref(false)
let restoreFocus: HTMLElement | null = null
let poll: number | undefined

const ICON = { ok: CircleCheck, warn: CircleAlert, off: CircleMinus } as const
const TONE = { ok: 'text-success', warn: 'text-warning', off: 'text-ink-faint' } as const

const { rows, report } = useDiagnosticsReport()

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(report())
    copied.value = true
    window.setTimeout(() => (copied.value = false), 1500)
  } catch {
    copied.value = false
  }
}

const logs = desktop?.logs
const logDone = ref<'copy' | 'save' | null>(null)

async function exportLog(action: 'copy' | 'save'): Promise<void> {
  const res = await logs?.(action, report()).catch(() => null)
  if (!res?.ok) return
  logDone.value = action
  window.setTimeout(() => (logDone.value = null), 1500)
}

function close(): void {
  ui.diagnosticsOpen = false
}

function trap(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
    return
  }
  if (e.key !== 'Tab' || !dialog.value) return
  const focusables = [...dialog.value.querySelectorAll<HTMLElement>('button:not([disabled])')]
  const first = focusables[0]
  const last = focusables.at(-1)
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last?.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first?.focus()
  }
}

watch(
  () => ui.diagnosticsOpen,
  async (open) => {
    window.clearInterval(poll)
    if (open) {
      restoreFocus = document.activeElement as HTMLElement | null
      connection.loadDiagnostics()
      poll = window.setInterval(() => connection.loadDiagnostics(), 5_000)
      await nextTick()
      dialog.value?.querySelector<HTMLElement>('button')?.focus()
    } else {
      if (restoreFocus?.isConnected) restoreFocus.focus()
      restoreFocus = null
    }
  },
  { immediate: true },
)
onBeforeUnmount(() => window.clearInterval(poll))
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" @before-leave="(el) => retireOverlay(el.firstElementChild!)">
      <div v-if="ui.diagnosticsOpen" class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto ccm-scrim px-4 py-[6vh]" @mousedown.self="close">
        <div
          ref="dialog"
          role="dialog"
          aria-modal="true"
          aria-labelledby="diag-title"
          class="w-full max-w-[600px] ccm-sheet"
          @keydown="trap"
        >
          <div class="flex items-center gap-2 border-b border-line px-5 py-4">
            <div class="mr-auto min-w-0">
              <h2 id="diag-title" class="text-base font-semibold">{{ t('diag.title') }}</h2>
              <p class="text-2xs text-ink-muted">{{ t('diag.subtitle') }}</p>
            </div>
            <button
              type="button"
              class="ccm-btn-plain"
              :aria-label="t('diag.refresh')"
              :title="t('diag.refresh')"
              @click="connection.loadDiagnostics()"
            >
              <RefreshCw :size="15" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="ccm-btn-plain"
              :aria-label="t('projects.close')"
              @click="close"
            >
              <X :size="16" aria-hidden="true" />
            </button>
          </div>

          <p v-if="connection.state !== 'connected'" class="px-5 py-6 text-sm text-st-error">{{ t('diag.offline') }}</p>
          <p v-else-if="!rows.length" class="px-5 py-6 text-sm text-ink-muted">{{ t('diag.loading') }}</p>
          <ul v-else class="divide-y divide-line px-5">
            <li v-for="r in rows" :key="r.key" class="flex gap-3 py-3">
              <component :is="ICON[r.level]" :size="18" class="mt-0.5 shrink-0" :class="TONE[r.level]" aria-hidden="true" />
              <div class="min-w-0 flex-1">
                <p class="flex flex-wrap items-baseline justify-between gap-x-3 text-sm">
                  <span class="font-medium text-ink">{{ t(`diag.row.${r.key}`) }}</span>
                  <span class="sr-only">{{ t(`diag.level.${r.level}`) }}</span>
                  <span class="min-w-0 break-all text-ink-muted tabular">{{ r.value }}</span>
                </p>
                <p v-if="r.detail" class="mt-0.5 break-all text-2xs text-ink-faint">{{ r.detail }}</p>
              </div>
            </li>
          </ul>

          <div class="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-3">
            <template v-if="logs">
              <p class="mr-auto min-w-0 basis-full text-2xs text-ink-faint sm:basis-auto sm:flex-1">{{ t('diag.logHint') }}</p>
              <button type="button" class="ccm-btn" @click="exportLog('save')">
                <Check v-if="logDone === 'save'" :size="14" class="ccm-pop text-success" aria-hidden="true" /><Download v-else :size="14" aria-hidden="true" />
                {{ logDone === 'save' ? t('diag.savedLog') : t('diag.saveLog') }}
              </button>
              <button type="button" class="ccm-btn" @click="exportLog('copy')">
                <Check v-if="logDone === 'copy'" :size="14" class="ccm-pop text-success" aria-hidden="true" /><ClipboardCopy v-else :size="14" aria-hidden="true" />
                {{ logDone === 'copy' ? t('limits.copied') : t('diag.copyLog') }}
              </button>
            </template>
            <button
              v-else
              type="button"
              class="ccm-btn"
              :disabled="!rows.length"
              @click="copy"
            >
              <Check v-if="copied" :size="14" class="ccm-pop text-success" aria-hidden="true" /><ClipboardCopy v-else :size="14" aria-hidden="true" />
              {{ copied ? t('limits.copied') : t('diag.copy') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
