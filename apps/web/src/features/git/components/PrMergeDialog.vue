<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { GitMerge, LoaderCircle, X } from 'lucide-vue-next'
import type { GhFail, GhMergeMethod, GhPr } from '@ccm/shared'
import { ghPr } from '../../../core/lib/preview'
import { MERGE_METHODS, errorKey, permissionLabel, readMethod, writeMethod, type MergeOutcome } from '../lib/prView'

const props = defineProps<{ root: string; pr: GhPr }>()
const emit = defineEmits<{ merged: [outcome: MergeOutcome]; close: [] }>()
const { t } = useI18n()

const method = ref<GhMergeMethod>(readMethod(props.root))
const deleteBranch = ref(true)
const busy = ref(false)
const failure = ref<GhFail | null>(null)
const sheet = ref<HTMLElement | null>(null)
const submitBtn = ref<HTMLButtonElement | null>(null)

const errorText = computed(() => {
  const f = failure.value
  if (!f) return ''
  const main = f.error === 'gh.scope' && f.permission ? t('v111pr.scope', { permission: permissionLabel(f.permission) }) : t(errorKey(f.error))
  return f.detail && f.error !== 'gh.scope' ? `${main} ${f.detail}` : main
})

async function submit(): Promise<void> {
  if (busy.value) return
  busy.value = true
  failure.value = null
  writeMethod(props.root, method.value)
  try {
    const res = await ghPr({ op: 'merge', root: props.root, number: props.pr.number, method: method.value, sha: props.pr.headSha, base: props.pr.base, head: props.pr.head, deleteBranch: deleteBranch.value })
    if (!res.ok) {
      if (res.error !== 'cancelled') failure.value = res
      return
    }
    emit('merged', { merged: res.merged, branchDeleted: res.branchDeleted, deleteError: res.deleteError as MergeOutcome['deleteError'] })
  } finally {
    busy.value = false
  }
}

function close(): void {
  if (!busy.value) emit('close')
}

function trap(e: KeyboardEvent): void {
  if (e.key !== 'Tab') return
  const items = [...(sheet.value?.querySelectorAll<HTMLElement>('button:not([disabled]), input') ?? [])]
  if (!items.length) return
  const at = items.indexOf(document.activeElement as HTMLElement)
  e.preventDefault()
  items[(at + (e.shiftKey ? -1 : 1) + items.length) % items.length]?.focus()
}

onMounted(async () => {
  await nextTick()
  submitBtn.value?.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[8vh] sm:px-4 sm:pt-[12vh]" @mousedown.self="close">
      <div ref="sheet" role="dialog" aria-modal="true" aria-labelledby="pr-merge-title" class="ccm-sheet flex max-h-[80vh] w-full max-w-[460px] flex-col overflow-hidden" data-pr-merge @keydown.esc.stop="close" @keydown="trap">
        <header class="flex items-center gap-2 border-b border-line px-4 py-3">
          <GitMerge :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <h2 id="pr-merge-title" class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">{{ t('v111pr.mergeDialog.title', { number: pr.number }) }}</h2>
          <button type="button" class="ccm-btn-plain shrink-0" :disabled="busy" :title="t('v111pr.mergeDialog.cancel')" :aria-label="t('v111pr.mergeDialog.cancel')" @click="close"><X :size="14" aria-hidden="true" /></button>
        </header>
        <form class="flex flex-col gap-3 px-4 py-3" novalidate @submit.prevent="submit">
          <p class="truncate text-xs text-ink-muted" :title="pr.title">{{ pr.title }} <span class="font-mono text-ink-faint">({{ pr.head }} -> {{ pr.base }})</span></p>
          <fieldset class="flex flex-col gap-1.5" :disabled="busy">
            <legend class="mb-1 text-xs font-medium text-ink-muted">{{ t('v111pr.mergeDialog.method') }}</legend>
            <label v-for="m in MERGE_METHODS" :key="m" class="flex cursor-pointer items-center gap-2 text-sm text-ink">
              <input v-model="method" type="radio" name="pr-merge-method" :value="m" class="accent-[var(--accent)]" :data-pr-method="m" />
              {{ t(`v111pr.mergeDialog.methods.${m}`) }}
            </label>
          </fieldset>
          <label class="flex cursor-pointer items-center gap-2 text-sm text-ink">
            <input v-model="deleteBranch" type="checkbox" :disabled="busy" class="accent-[var(--accent)]" data-pr-delete-branch />
            {{ t('v111pr.mergeDialog.deleteBranch') }}
          </label>
          <p v-if="errorText" class="text-xs text-st-error" role="alert" data-pr-error>{{ errorText }}</p>
          <footer class="flex items-center justify-end gap-2 pt-1">
            <button type="button" class="ccm-btn" :disabled="busy" data-pr-cancel @click="close">{{ t('v111pr.mergeDialog.cancel') }}</button>
            <button ref="submitBtn" type="submit" class="ccm-btn ccm-btn-primary" :disabled="busy" :aria-busy="busy" data-pr-merge-submit>
              <LoaderCircle v-if="busy" :size="13" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
              {{ busy ? t('v111pr.mergeDialog.busy') : t('v111pr.mergeDialog.submit') }}
            </button>
          </footer>
        </form>
      </div>
    </div>
  </Teleport>
</template>
