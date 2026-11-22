<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { LoaderCircle, ShieldAlert, X } from 'lucide-vue-next'
import type { SecretBlock } from '../../core/lib/preview'
import { groupHits, kindKey } from './secretGuard'

const props = defineProps<{ block: SecretBlock | null; mode: 'commit' | 'push'; unstaging?: string | null; note?: string | null }>()
const emit = defineEmits<{ unstage: [path: string]; proceed: []; cancel: [] }>()
const { t } = useI18n()

const sheet = ref<HTMLElement | null>(null)
const cancelBtn = ref<HTMLButtonElement | null>(null)
const files = computed(() => groupHits(props.block?.hits ?? []))
const count = computed(() => props.block?.hits.length ?? 0)
const title = computed(() => (props.block?.scanError ? t('secretGuard.titleError') : t('secretGuard.title', { n: count.value }, count.value)))
const skipped = computed(() => (props.block?.skipped ?? []).join(', '))

watch(
  () => !!props.block,
  async (open) => {
    if (!open) return
    await nextTick()
    cancelBtn.value?.focus()
  },
  { immediate: true },
)

function trap(e: KeyboardEvent): void {
  if (e.key !== 'Tab' || !sheet.value) return
  const items = [...sheet.value.querySelectorAll<HTMLElement>('button:not([disabled])')]
  const at = items.indexOf(document.activeElement as HTMLElement)
  e.preventDefault()
  items[(at + (e.shiftKey ? -1 : 1) + items.length) % items.length]?.focus()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="block" class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[8vh] sm:px-4 sm:pt-[12vh]" @mousedown.self="emit('cancel')">
      <div
        ref="sheet"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="secret-guard-title"
        aria-describedby="secret-guard-sub"
        class="ccm-sheet flex max-h-[80vh] w-full max-w-[560px] flex-col overflow-hidden"
        data-secret-guard
        :data-secret-mode="mode"
        @keydown.esc.stop="emit('cancel')"
        @keydown="trap"
      >
        <header class="flex items-center gap-2 border-b border-line px-4 py-3">
          <ShieldAlert :size="16" class="shrink-0 text-warning" aria-hidden="true" />
          <div class="min-w-0 flex-1">
            <h2 id="secret-guard-title" class="truncate text-sm font-semibold text-ink" data-secret-title>{{ title }}</h2>
            <p id="secret-guard-sub" class="text-xs text-ink-muted">
              {{ block.scanError ? t('secretGuard.scanError', { reason: block.scanError }) : mode === 'push' ? t('secretGuard.subtitlePush') : t('secretGuard.subtitleCommit') }}
            </p>
          </div>
          <button type="button" class="ccm-btn-plain shrink-0" :title="t('secretGuard.cancel')" :aria-label="t('secretGuard.cancel')" @click="emit('cancel')">
            <X :size="14" aria-hidden="true" />
          </button>
        </header>

        <div class="ccm-scroll flex min-h-0 flex-1 flex-col gap-2 overflow-x-hidden overflow-y-auto px-4 py-3">
          <section v-for="f in files" :key="f.path" class="flex flex-col rounded-inner border border-line" data-secret-file :data-path="f.path">
            <div class="flex min-w-0 items-center gap-2 border-b border-line px-2.5 py-1.5">
              <span class="min-w-0 flex-1 truncate font-mono text-xs text-ink" :title="f.path">{{ f.path }}</span>
              <button
                v-if="mode === 'commit'"
                type="button"
                class="ccm-btn h-6 shrink-0 gap-1 px-2 text-2xs"
                :disabled="!!unstaging"
                data-secret-unstage
                @click="emit('unstage', f.path)"
              >
                <LoaderCircle v-if="unstaging === f.path" :size="11" class="ccm-spin" aria-hidden="true" />
                {{ unstaging === f.path ? t('secretGuard.unstaging') : t('secretGuard.unstage') }}
              </button>
            </div>
            <ul class="flex flex-col py-0.5">
              <li v-for="h in f.hits" :key="`${h.commit ?? ''}:${h.line}:${h.kind}`" class="flex min-w-0 items-center gap-2 px-2.5 py-1 text-xs" data-secret-hit>
                <span class="max-w-40 shrink-0 truncate font-mono text-ink-faint tabular" :title="t('secretGuard.location', { path: f.path, line: h.line })" data-secret-where>{{
                  t('secretGuard.location', { path: f.path.split('/').pop(), line: h.line })
                }}</span>
                <span class="ccm-chip ccm-chip-warning shrink-0 text-2xs">{{ t(kindKey(h.kind)) }}</span>
                <code class="min-w-0 flex-1 truncate font-mono text-ink-muted" data-secret-masked>{{ h.masked }}</code>
                <span v-if="h.commit" class="shrink-0 font-mono text-2xs text-ink-faint">{{ t('secretGuard.inCommit', { hash: h.commit }) }}</span>
              </li>
            </ul>
          </section>
          <p v-if="block.truncated" class="text-2xs text-ink-faint">{{ t('secretGuard.truncated') }}</p>
          <p v-if="skipped" class="truncate text-2xs text-ink-faint" :title="skipped">{{ t('secretGuard.skipped', { files: skipped }) }}</p>
          <p v-if="note" class="text-2xs text-ink-muted" role="status" data-secret-note>{{ note }}</p>
          <p v-if="!block.scanError" class="text-2xs text-ink-faint">{{ t('secretGuard.allowHint') }}</p>
        </div>

        <footer class="flex items-center justify-end gap-2 border-t border-line px-4 py-3">
          <button ref="cancelBtn" type="button" class="ccm-btn ccm-btn-primary" data-secret-cancel @click="emit('cancel')">{{ t('secretGuard.cancel') }}</button>
          <button type="button" class="ccm-btn text-st-error" :disabled="!!unstaging" data-secret-proceed @click.stop="emit('proceed')">
            {{ mode === 'push' ? t('secretGuard.pushAnyway') : t('secretGuard.commitAnyway') }}
          </button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>
