<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { LoaderCircle, Smartphone } from 'lucide-vue-next'
import type { RunSession } from '@ccm/shared'
import { hasPushPhone, useRemoteStore } from '@features/remote'
import { offerSession } from '../lib/apkOffer'
import { apkOf, autoApkOn, setAutoApk } from '../lib/runApk'

const props = defineProps<{ session: RunSession | null; root: string }>()
const { t } = useI18n()
const remote = useRemoteStore()
const apk = computed(() => apkOf(props.session))
const available = computed(() => hasPushPhone(remote.devices))
const sending = ref(false)
const auto = computed(() => autoApkOn(props.root))

async function send(): Promise<void> {
  if (!props.session || sending.value) return
  sending.value = true
  try {
    await offerSession(props.session.id)
  } finally {
    sending.value = false
  }
}
</script>

<template>
  <div v-if="available" class="flex shrink-0 items-center gap-2" data-run-apk>
    <button v-if="apk" type="button" class="ccm-btn-plain shrink-0" :disabled="sending" :title="t('runApk.send')" :aria-label="t('runApk.send')" data-run-apk-send @click="send">
      <LoaderCircle v-if="sending" :size="13" class="ccm-spin" aria-hidden="true" />
      <Smartphone v-else :size="13" aria-hidden="true" />
      <span class="hidden text-xs @2xl:inline">{{ t('runApk.send') }}</span>
    </button>
    <label class="flex cursor-pointer items-center gap-1 text-2xs text-ink-muted" :title="t('runApk.autoHint')">
      <input type="checkbox" class="size-3.5 accent-(--ccm-accent,currentColor)" :checked="auto" data-run-apk-auto @change="setAutoApk(root, ($event.target as HTMLInputElement).checked)" />
      <span class="hidden @3xl:inline">{{ t('runApk.auto') }}</span>
      <span class="sr-only @3xl:hidden">{{ t('runApk.auto') }}</span>
    </label>
  </div>
</template>
