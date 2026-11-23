<script setup lang="ts">
import { computed, onMounted, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bell, CircleAlert, CircleCheck, FolderOpen, Gauge, LoaderCircle, MessageSquareMore, Volume2, Webhook } from 'lucide-vue-next'
import { desktop } from '../../../core/lib/desktop'
import { SETUP_ORDER, useSetupChecklistStore, type SetupItemId } from '../lib/setupChecklist'
import { useSettingsStore } from '../../../core/stores/settings'
import SettingSection from './SettingSection.vue'

const { t } = useI18n()
const setup = useSetupChecklistStore()
const settings = useSettingsStore()

const ICONS: Record<SetupItemId, Component> = { notify: Bell, toast: MessageSquareMore, sound: Volume2, statusLine: Gauge, hooks: Webhook, folder: FolderOpen }

onMounted(() => setup.ensureLoaded())

const rows = computed(() =>
  SETUP_ORDER.filter((id) => setup.available(id)).map((id) => {
    const on = setup.isOn(id)
    let note: string | null = null
    let blocked = false
    if (id === 'notify' && settings.notifyPermission === 'denied') {
      note = t('setup.notify.blocked')
      blocked = true
    } else if (id === 'statusLine' && setup.statusLine === 'unreadable') {
      note = t('setup.statusLine.unreadable')
      blocked = true
    } else if (id === 'statusLine' && setup.statusLineNote) note = t(`setup.statusLine.${setup.statusLineNote}`)
    else if (id === 'statusLine' && on) note = t('setup.statusLine.newSessions')
    else if (id === 'hooks' && setup.hooks === 'unreadable') {
      note = t('setup.hooks.unreadable')
      blocked = true
    } else if (id === 'hooks' && setup.hooksNote) note = t(`setup.hooks.${setup.hooksNote}`)
    else if (id === 'hooks' && setup.hooks === 'partial') note = t('setup.hooks.partial')
    else if (id === 'hooks' && on) note = t('setup.hooks.newSessions')
    else if (id === 'folder' && setup.folder) note = t('setup.folder.granted', { path: setup.folder })
    return { id, icon: ICONS[id], title: t(`setup.${id}.title`), body: t(`setup.${id}.body`), on, note, blocked }
  }),
)
const left = computed(() => setup.pending.length)
</script>

<template>
  <SettingSection v-if="desktop && rows.length" id="setup" :title="t('setup.checklist')">
    <div class="flex items-center gap-2 px-3 py-2.5 text-xs text-ink-muted" data-setting-row="setup" data-setup-summary>
      <CircleCheck v-if="left === 0" :size="14" class="shrink-0 text-success" aria-hidden="true" />
      <span class="min-w-0 flex-1">{{ left === 0 ? t('setup.allDone') : t('setup.left', { n: left }) }}</span>
      <button v-if="left > 0 && !setup.hidden" type="button" class="ccm-btn" data-setup-hide @click="setup.hideChip">{{ t('setup.hideChip') }}</button>
      <button v-else-if="left > 0" type="button" class="ccm-btn" data-setup-show @click="setup.showChip">{{ t('setup.showChip') }}</button>
    </div>
    <ul class="divide-y divide-line border-t border-line">
      <li v-for="r in rows" :key="r.id" class="flex gap-3 px-3 py-3" :data-setup-row="r.id">
        <span class="mt-0.5 grid size-8 shrink-0 place-items-center rounded-inner bg-fill" aria-hidden="true">
          <component :is="r.icon" :size="16" :class="r.on ? 'text-success' : 'text-ink-muted'" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium">{{ r.title }}</p>
          <p class="mt-0.5 text-xs leading-relaxed text-ink-muted">{{ r.body }}</p>
          <p
            v-if="r.note"
            class="mt-1 flex items-start gap-1 text-2xs leading-relaxed break-all"
            :class="r.blocked || ((r.id === 'statusLine' || r.id === 'hooks') && !r.on) ? 'text-warning' : 'text-ink-faint'"
            :role="r.blocked ? 'alert' : undefined"
          >
            <CircleAlert v-if="r.blocked" :size="12" class="mt-0.5 shrink-0" aria-hidden="true" />{{ r.note }}
          </p>
        </div>
        <div class="flex shrink-0 items-start gap-2 pt-0.5">
          <button
            v-if="r.on && r.id === 'hooks'"
            type="button"
            class="ccm-btn"
            :disabled="setup.busy !== null"
            :aria-busy="setup.busy === r.id || undefined"
            :aria-label="`${t('setup.turnOff')}: ${r.title}`"
            data-setup-off="hooks"
            @click="setup.disableHooks"
          >
            <LoaderCircle v-if="setup.busy === r.id" :size="13" class="animate-spin" aria-hidden="true" />
            {{ t('setup.turnOff') }}
          </button>
          <span v-if="r.on && r.id !== 'folder'" class="inline-flex h-7 items-center gap-1 text-xs font-medium text-success" :data-setup-on="r.id">
            <CircleCheck :size="14" aria-hidden="true" />{{ t('setup.on') }}
          </span>
          <button
            v-else
            type="button"
            class="ccm-btn"
            :class="r.on ? '' : 'ccm-btn-primary'"
            :disabled="r.blocked || setup.busy !== null"
            :aria-busy="setup.busy === r.id || undefined"
            :aria-label="`${r.id === 'folder' ? t('setup.choose') : t('setup.turnOn')}: ${r.title}`"
            :data-setup-action="r.id"
            @click="setup.enable(r.id)"
          >
            <LoaderCircle v-if="setup.busy === r.id" :size="13" class="animate-spin" aria-hidden="true" />
            {{ r.id === 'folder' ? t('setup.choose') : t('setup.turnOn') }}
          </button>
        </div>
      </li>
    </ul>
  </SettingSection>
</template>
