<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { FolderX, Link2Off, Search, Trash2 } from 'lucide-vue-next'
import { useOrganizeStore } from '../../stores/organize'
import { useDiskMapStore } from '../../stores/diskMap'

const { t } = useI18n()
const store = useOrganizeStore()
const diskMap = useDiskMapStore()
const removed = ref<number | null>(null)

async function remove(): Promise<void> {
  removed.value = null
  const n = await store.removeJunk()
  if (n) removed.value = n
  await diskMap.loadHistory()
}
</script>

<template>
  <section class="rounded-card border border-line" :aria-label="t('diskOrganize.junk.title')" data-organize-junk>
    <header class="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2">
      <div class="min-w-0 flex-1">
        <h3 class="text-xs font-semibold">{{ t('diskOrganize.junk.title') }}</h3>
        <p class="text-2xs text-ink-muted">{{ t('diskOrganize.junk.hint') }}</p>
      </div>
      <button type="button" class="ccm-btn" :disabled="store.junkScanning" data-organize-junk-scan @click="store.scanJunk()">
        <Search :size="13" aria-hidden="true" />{{ store.junkScanning ? t('diskOrganize.junk.scanning') : t('diskOrganize.junk.scan') }}
      </button>
      <button v-if="store.junk.length" type="button" class="ccm-btn ccm-btn-danger" :disabled="!store.junkChosen.length" data-organize-junk-remove @click="remove">
        <Trash2 :size="13" aria-hidden="true" />{{ t('diskOrganize.junk.remove', { n: store.junkChosen.length }) }}
      </button>
    </header>
    <p v-if="removed" class="border-b border-line px-3 py-1.5 text-xs" role="status">{{ t('diskOrganize.junk.removed', { n: removed }) }}</p>
    <template v-if="store.junkScanned">
      <p v-if="!store.junkIndexed" class="px-3 py-4 text-center text-xs text-ink-muted" data-organize-junk-noindex>{{ t('diskOrganize.junk.notIndexed') }}</p>
      <p v-else-if="!store.junk.length" class="px-3 py-4 text-center text-xs text-ink-muted" data-organize-junk-none>{{ t('diskOrganize.junk.none') }}</p>
      <ul v-else class="max-h-72 divide-y divide-line overflow-y-auto overflow-x-hidden">
        <li v-for="i in store.junk" :key="i.path" class="flex items-center gap-2 px-3 py-1.5 text-xs" :data-organize-junk-item="i.kind" :data-state="i.state">
          <input type="checkbox" class="shrink-0" :checked="store.junkPicked.has(i.path)" :aria-label="i.name" @change="store.toggleJunk(i.path)" />
          <component :is="i.kind === 'empty' ? FolderX : Link2Off" :size="13" class="shrink-0 text-ink-muted" aria-hidden="true" />
          <span class="min-w-0 flex-1 truncate" :title="i.target ? `${i.path}\n${i.target}` : i.path">{{ i.path }}</span>
          <span class="shrink-0 text-2xs" :class="i.state === 'unknown' ? 'text-ink-muted' : 'text-warning'">{{ t(`diskOrganize.junk.${i.state}`) }}</span>
        </li>
      </ul>
      <p v-if="store.junkTruncated" class="border-t border-line px-3 py-1.5 text-2xs text-ink-muted">{{ t('diskOrganize.junk.truncated') }}</p>
    </template>
  </section>
</template>
