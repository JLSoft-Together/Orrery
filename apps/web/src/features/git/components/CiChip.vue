<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, LoaderCircle, X } from 'lucide-vue-next'
import { ciLook } from '../lib/prView'
import { useCiStore } from '../stores/ci'

const props = defineProps<{ root: string }>()
const { t } = useI18n()
const ci = useCiStore()

const state = computed(() => ci.chip(props.root))
const look = computed(() => (state.value ? ciLook(state.value) : null))
const label = computed(() => {
  const entry = ci.entry(props.root)
  const text = look.value ? t(look.value.text) : ''
  return entry ? `${text}: ${entry.branch}` : text
})
</script>

<template>
  <span v-if="look" class="inline-flex size-4 shrink-0 items-center justify-center" :class="look.cls" :title="label" role="img" :aria-label="label" :data-ci-chip="state">
    <LoaderCircle v-if="look.icon === 'spin'" :size="13" class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
    <Check v-else-if="look.icon === 'check'" :size="13" aria-hidden="true" />
    <X v-else :size="13" aria-hidden="true" />
  </span>
</template>
