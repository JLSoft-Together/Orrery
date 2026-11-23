<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { desktop, hasTerminals, officeNeedsNode } from '../../../core/lib/desktop'
import { hasAwake } from '../lib/capture'
import { isMac } from '../../../core/lib/platform'
import { RESTORE_MODES, restoreMode } from '../../workspace/lib/workspaceRestore'
import { useSettingsStore } from '../../../core/stores/settings'
import { useTipsStore } from '@features/update'
import { useUiStore } from '../../../core/stores/ui'
import AwakeSettings from './AwakeSettings.vue'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'
import SegmentedControl from '../../../core/components/ui/SegmentedControl.vue'
import SettingEmbed from './SettingEmbed.vue'
import SetupChecklist from './SetupChecklist.vue'
import SettingRow from './SettingRow.vue'
import SettingSection from './SettingSection.vue'

const { t } = useI18n()
const settings = useSettingsStore()
const tips = useTipsStore()
const ui = useUiStore()
const advancedOpen = ref(false)
const tipsOn = computed({ get: () => !tips.state.off, set: (on: boolean) => tips.setOff(!on) })

const loginItem = ref<boolean | null>(null)
async function setLoginItem(on: boolean): Promise<void> {
  loginItem.value = on
  const now = await desktop?.loginItem?.(on)
  loginItem.value = typeof now === 'boolean' ? now : null
}
onMounted(() => void desktop?.loginItem?.().then((on) => (loginItem.value = typeof on === 'boolean' ? on : null)))

const locales = [
  { value: 'vi' as const, label: 'Tiếng Việt', lang: 'vi' },
  { value: 'en' as const, label: 'English', lang: 'en' },
]
const restoreModes = computed(() => RESTORE_MODES.map((mode) => ({ value: mode, label: t(`wsRestore.modes.${mode}`) })))
const terminalApps = computed(() => (['terminal', 'iterm'] as const).map((app) => ({ value: app, label: t(`settings.launchTerminalApp.${app}`) })))
</script>

<template>
  <div>
    <SetupChecklist />
    <SettingSection id="general" class="mt-4">
      <SettingRow :label="t('settings.language')" data-setting-row="language">
        <SegmentedControl v-model="settings.locale" :options="locales" :label="t('settings.language')" />
      </SettingRow>
      <ToggleSwitch v-model="settings.keyHints" :label="t('keys.hints')" :info="t('keys.hintsHint')" class="px-3" data-setting-row="keyHints" />
      <ToggleSwitch v-if="hasTerminals" v-model="tipsOn" :label="t('tips.setting')" :info="t('tips.settingHint')" class="px-3" data-setting-row="tips" />
      <SettingRow v-if="isMac" :label="t('settings.launchTerminal')" :info="t('settings.launchTerminalHint')" data-setting-row="launch">
        <SegmentedControl v-model="ui.launchTerminal" :options="terminalApps" :label="t('settings.launchTerminal')" />
      </SettingRow>
    </SettingSection>

    <SettingSection v-if="loginItem !== null || hasTerminals" id="startup" :title="t('settings.section.startup')">
      <SettingRow v-if="hasTerminals" :label="t('wsRestore.mode')" :info="t('wsRestore.modeHint')" stacked data-setting-row="restore">
        <SegmentedControl v-model="restoreMode" :options="restoreModes" :label="t('wsRestore.mode')" fill class="w-full" data-setting="restore-mode" />
      </SettingRow>
      <ToggleSwitch
        v-if="loginItem !== null"
        :model-value="loginItem"
        :label="t(isMac ? 'settings.loginItemMac' : 'settings.loginItem')"
        :info="t(isMac ? 'settings.loginItemHintMac' : 'settings.loginItemHint')"
        class="px-3"
        data-setting-row="loginItem"
        @update:model-value="setLoginItem"
      />
    </SettingSection>

    <SettingEmbed v-if="hasAwake" id="awake"><AwakeSettings /></SettingEmbed>

    <SettingSection id="advanced-toggle">
      <div class="flex items-center gap-3 px-3 py-2" data-general-more-row>
        <div class="min-w-0 flex-1">
          <p class="text-sm">{{ t('v107gon.settings.advanced') }}</p>
          <p class="mt-0.5 text-2xs text-ink-faint">{{ t('v107gon.settings.advancedHint') }}</p>
        </div>
        <button type="button" class="ccm-btn" :aria-expanded="advancedOpen" aria-controls="general-advanced" data-general-more @click="advancedOpen = !advancedOpen">
          {{ advancedOpen ? t('v107gon.settings.advancedClose') : t('v107gon.settings.advancedOpen') }}
        </button>
      </div>
    </SettingSection>
    <div v-show="advancedOpen" id="general-advanced" class="mt-4" data-general-advanced>
      <SettingSection id="advanced">
        <ToggleSwitch v-model="settings.officeEnabled" :label="t('settings.office')" :info="t('settings.officeHint')" class="px-3" data-setting-row="office" />
        <p v-if="settings.officeEnabled && officeNeedsNode" role="status" class="px-3 py-1 text-sm text-ink-muted" data-office-needs-node>{{ t('settings.officeNeedsNode') }}</p>
      </SettingSection>
    </div>
  </div>
</template>
