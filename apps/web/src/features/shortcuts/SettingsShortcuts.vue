<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { isMac } from '../../lib/platform'
import { keyCombo } from '../../lib/shortcuts'
import SettingSection from '../../components/settings/SettingSection.vue'
import { availableCommands, effectiveKeys, isOverridden, overrides, recording, resetAllBindings, resetBinding, setBinding } from './bindings'
import { chordFromEvent, chordKeys } from './keys'

const { t } = useI18n()

const active = ref<string | null>(null)
const error = ref<{ id: string; text: string } | null>(null)

const rows = computed(() => {
  void overrides.value
  return availableCommands(isMac).map((id) => ({
    id,
    name: t(`keys.item.${id}`),
    text: keyCombo(effectiveKeys(id, isMac), isMac),
    changed: isOverridden(id),
  }))
})

const anyChanged = computed(() => Object.keys(overrides.value).length > 0)

function start(id: string): void {
  active.value = id
  error.value = null
  recording.value = true
}

function stop(): void {
  active.value = null
  recording.value = false
}

function nameOf(id: string): string {
  return t(`keys.item.${id}`)
}

function onKey(e: KeyboardEvent, id: string): void {
  if (active.value !== id) return
  if (e.key === 'Tab' && !e.ctrlKey && !e.altKey && !e.metaKey) return
  e.preventDefault()
  e.stopPropagation()
  if (e.isComposing) return
  if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey) {
    stop()
    error.value = null
    return
  }
  const chord = chordFromEvent(e, isMac)
  if (!chord) return
  const res = setBinding(id, chordKeys(chord), isMac)
  if (res.ok) {
    error.value = null
    stop()
    return
  }
  const text = res.reason === 'conflict' ? t('shortcutsV109.reason.conflict', { name: nameOf(res.with) }) : t(`shortcutsV109.reason.${res.reason}`)
  error.value = { id, text }
}

function onBlur(id: string): void {
  if (active.value === id) stop()
}

function reset(id: string): void {
  resetBinding(id)
  if (error.value?.id === id) error.value = null
}

function resetAll(): void {
  resetAllBindings()
  error.value = null
}

onBeforeUnmount(() => {
  recording.value = false
})
</script>

<template>
  <div data-settings-shortcuts data-setting-row="shortcuts">
    <SettingSection id="shortcuts">
      <div class="flex items-center justify-between gap-3 px-3 py-2">
        <p class="min-w-0 flex-1 text-2xs text-ink-muted">{{ t('shortcutsV109.hint') }}</p>
        <button v-if="anyChanged" type="button" class="ccm-btn-plain shrink-0 text-xs" data-shortcut-reset-all @click="resetAll">{{ t('shortcutsV109.resetAll') }}</button>
      </div>
      <div v-for="r in rows" :key="r.id" class="border-t border-line first:border-t-0" :data-shortcut-row="r.id">
        <div class="flex items-center justify-between gap-3 px-3 py-2">
          <p class="min-w-0 flex-1 truncate text-sm">{{ r.name }}</p>
          <span v-if="r.changed" class="shrink-0 text-2xs text-accent-ink" data-shortcut-changed>{{ t('shortcutsV109.changed') }}</span>
          <button
            v-if="r.changed"
            type="button"
            class="ccm-btn-plain shrink-0 text-2xs"
            :aria-label="t('shortcutsV109.resetRow', { name: r.name })"
            :data-shortcut-reset="r.id"
            @click="reset(r.id)"
          >{{ t('shortcutsV109.reset') }}</button>
          <button
            type="button"
            class="inline-flex h-7 min-w-24 shrink-0 items-center justify-center rounded-inner px-2.5 font-sans text-xs font-medium"
            :class="active === r.id ? 'bg-accent text-on-accent' : 'bg-fill text-ink-muted'"
            :aria-label="t('shortcutsV109.change', { name: r.name })"
            :data-shortcut-key="r.id"
            @click="start(r.id)"
            @keydown="onKey($event, r.id)"
            @blur="onBlur(r.id)"
          >{{ active === r.id ? t('shortcutsV109.press') : r.text }}</button>
        </div>
        <p v-if="error?.id === r.id" class="px-3 pb-2 text-2xs text-st-error" role="alert" data-shortcut-error>{{ error.text }}</p>
      </div>
    </SettingSection>
  </div>
</template>
