<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { GitPullRequest, LoaderCircle, X } from 'lucide-vue-next'
import type { GhFail, GhPr } from '@ccm/shared'
import AppSelect from '../../../core/components/AppSelect.vue'
import { ghPr } from '../../../core/lib/preview'
import { errorKey, permissionLabel } from '../lib/prView'

const props = defineProps<{
  root: string
  branch: string
  subject: string
  defaultBase: string
  bases: string[]
  loadBody: () => Promise<string | null>
}>()
const emit = defineEmits<{ created: [pr: GhPr]; close: [] }>()
const { t } = useI18n()

const base = ref(props.defaultBase)
const title = ref(props.subject)
const body = ref('')
const bodyState = ref<'loading' | 'ready' | 'none'>('loading')
const busy = ref(false)
const failure = ref<GhFail | null>(null)
const sheet = ref<HTMLElement | null>(null)
const titleInput = ref<HTMLInputElement | null>(null)
let restore: HTMLElement | null = null

const options = computed(() => {
  const all = new Set([props.defaultBase, ...props.bases])
  all.delete(props.branch)
  return [...all].map((b) => ({ value: b, label: b }))
})
const sameBranch = computed(() => base.value === props.branch)
const can = computed(() => !busy.value && !sameBranch.value && title.value.trim().length > 0 && !!base.value)
const errorText = computed(() => {
  const f = failure.value
  if (!f) return ''
  const main = f.error === 'gh.scope' && f.permission ? t('v111pr.scope', { permission: permissionLabel(f.permission) }) : t(errorKey(f.error))
  return f.detail && f.error !== 'gh.scope' ? `${main} ${f.detail}` : main
})

async function submit(): Promise<void> {
  if (!can.value) return
  busy.value = true
  failure.value = null
  try {
    const res = await ghPr({ op: 'create', root: props.root, base: base.value, head: props.branch, title: title.value.trim(), body: body.value })
    if (res.ok) emit('created', res.pr)
    else if (res.error !== 'cancelled') failure.value = res
  } finally {
    busy.value = false
  }
}

function close(): void {
  if (!busy.value) emit('close')
}

function trap(e: KeyboardEvent): void {
  if (e.key !== 'Tab') return
  const items = [...(sheet.value?.querySelectorAll<HTMLElement>('button:not([disabled]), input, textarea, [role=combobox]') ?? [])]
  if (!items.length) return
  const at = items.indexOf(document.activeElement as HTMLElement)
  e.preventDefault()
  items[(at + (e.shiftKey ? -1 : 1) + items.length) % items.length]?.focus()
}

onMounted(async () => {
  restore = document.activeElement instanceof HTMLElement ? document.activeElement : null
  await nextTick()
  titleInput.value?.focus()
  titleInput.value?.select()
  const text = await props.loadBody().catch(() => null)
  if (text) {
    if (!body.value) body.value = text
    bodyState.value = 'ready'
  } else bodyState.value = 'none'
})

onBeforeUnmount(() => {
  if (restore?.isConnected) restore.focus()
})
</script>

<template>
  <Teleport to="body">
    <div class="ccm-scrim fixed inset-0 z-50 flex items-start justify-center px-3 pt-[8vh] sm:px-4 sm:pt-[12vh]" @mousedown.self="close">
      <div ref="sheet" role="dialog" aria-modal="true" aria-labelledby="pr-create-title" class="ccm-sheet flex max-h-[84vh] w-full max-w-[560px] flex-col overflow-hidden" data-pr-create @keydown.esc.stop="close" @keydown="trap">
        <header class="flex items-center gap-2 border-b border-line px-4 py-3">
          <GitPullRequest :size="16" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <h2 id="pr-create-title" class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">{{ t('v111pr.createDialog.title') }}</h2>
          <button type="button" class="ccm-btn-plain shrink-0" :disabled="busy" :title="t('v111pr.createDialog.cancel')" :aria-label="t('v111pr.createDialog.cancel')" @click="close"><X :size="14" aria-hidden="true" /></button>
        </header>
        <form class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-4 py-3" novalidate @submit.prevent="submit">
          <div class="flex min-w-0 items-end gap-3">
            <div class="flex shrink-0 flex-col gap-1 text-xs font-medium text-ink-muted">
              <span>{{ t('v111pr.createDialog.base') }}</span>
              <AppSelect v-model="base" :options="options" :label="t('v111pr.createDialog.base')" size="sm" data-pr-base />
            </div>
            <p class="min-w-0 flex-1 truncate pb-1 text-xs text-ink-muted" :title="branch">{{ t('v111pr.createDialog.head') }}: <span class="font-mono text-ink">{{ branch }}</span></p>
          </div>
          <p v-if="sameBranch" class="text-xs text-warning" role="status">{{ t('v111pr.createDialog.sameBranch', { branch }) }}</p>
          <label class="flex flex-col gap-1 text-xs font-medium text-ink-muted">
            {{ t('v111pr.createDialog.titleLabel') }}
            <input ref="titleInput" v-model="title" type="text" maxlength="250" spellcheck="false" autocomplete="off" class="ccm-field h-8 text-sm" :disabled="busy" data-pr-title />
          </label>
          <label class="flex min-h-0 flex-col gap-1 text-xs font-medium text-ink-muted">
            <span class="flex items-center gap-2">
              {{ t('v111pr.createDialog.body') }}
              <LoaderCircle v-if="bodyState === 'loading'" :size="12" class="animate-spin text-ink-faint motion-reduce:animate-none" aria-hidden="true" />
              <span v-if="bodyState === 'loading'" class="font-normal text-ink-faint">{{ t('v111pr.createDialog.bodyLoading') }}</span>
            </span>
            <textarea v-model="body" rows="9" spellcheck="false" class="ccm-field min-h-[120px] resize-y font-mono text-xs" :disabled="busy" data-pr-body />
            <span class="font-normal text-ink-faint">{{ bodyState === 'none' ? t('v111pr.createDialog.bodyNone') : t('v111pr.createDialog.bodyHint') }}</span>
          </label>
          <p v-if="errorText" class="text-xs text-st-error" role="alert" data-pr-error>{{ errorText }}</p>
          <footer class="flex items-center justify-end gap-2 pt-1">
            <button type="button" class="ccm-btn" :disabled="busy" data-pr-cancel @click="close">{{ t('v111pr.createDialog.cancel') }}</button>
            <button type="submit" class="ccm-btn ccm-btn-primary" :disabled="!can" :aria-busy="busy" data-pr-submit>
              <LoaderCircle v-if="busy" :size="13" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
              {{ busy ? t('v111pr.createDialog.busy') : t('v111pr.createDialog.submit') }}
            </button>
          </footer>
        </form>
      </div>
    </div>
  </Teleport>
</template>
