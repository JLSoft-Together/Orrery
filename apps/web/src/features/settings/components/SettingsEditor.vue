<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { clampShellDoneSeconds, SHELL_DONE_SECONDS_MAX, SHELL_DONE_SECONDS_MIN } from '../../shellDone/shellDone'
import { AUTO_SAVE_DELAYS, AUTO_SAVE_MODES } from '@features/files'
import { useSettingsStore } from '../../../core/stores/settings'
import { useUiGenStore } from '../../next/stores/uiGen'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'
import SegmentedControl from '../../../core/components/ui/SegmentedControl.vue'
import SettingRow from './SettingRow.vue'
import SettingSection from './SettingSection.vue'

const { t } = useI18n()
const settings = useSettingsStore()
const uiGen = useUiGenStore()
const autoSaveModes = computed(() => AUTO_SAVE_MODES.map((mode) => ({ value: mode, label: t(`settings.autoSaveMode.${mode}`) })))
const autoSaveDelays = computed(() => AUTO_SAVE_DELAYS.map((ms) => ({ value: ms, label: t('settings.seconds', { n: ms / 1000 }) })))
</script>

<template>
  <div>
    <SettingSection id="editor">
      <SettingRow :label="t('settings.autoSave')" :info="t('settings.autoSaveHint')" stacked data-setting-row="autoSave">
        <SegmentedControl v-model="settings.autoSave" :options="autoSaveModes" :label="t('settings.autoSave')" fill class="w-full" data-setting="auto-save" />
        <SegmentedControl
          v-if="settings.autoSave === 'afterDelay'"
          v-model="settings.autoSaveDelay"
          :options="autoSaveDelays"
          :label="t('settings.autoSaveDelay')"
          fill
          class="mt-2 w-full"
          data-setting="auto-save-delay"
        />
      </SettingRow>
      <ToggleSwitch v-model="settings.minimap" :label="t('settings.minimap')" :info="t('settings.minimapHint')" class="px-3" data-setting-row="minimap" />
      <ToggleSwitch v-if="!uiGen.isNext" v-model="settings.dimInactive" :label="t('workspace.dim.setting')" :info="t('workspace.dim.settingHint')" class="px-3" data-setting-row="dim" />
      <ToggleSwitch
        v-model="settings.shellIntegration"
        :label="t('shellMarks.setting')"
        :info="t('shellMarks.settingHint')"
        class="px-3"
        data-setting="shell-integration"
        data-setting-row="shell"
      />
      <ToggleSwitch
        v-model="settings.shellDoneNotify"
        :label="t('shellDone.setting')"
        :info="t('shellDone.settingHint')"
        :disabled="!settings.shellIntegration"
        class="px-3"
        data-setting="shell-done"
        data-setting-row="shellDone"
      />
      <SettingRow v-if="settings.shellIntegration && settings.shellDoneNotify" :label="t('shellDone.seconds')" label-for="set-shell-done-seconds" data-setting-row="shellDoneSeconds">
        <div class="ccm-field flex w-20 shrink-0 items-center">
          <input
            id="set-shell-done-seconds"
            :value="settings.shellDoneSeconds"
            type="number"
            :min="SHELL_DONE_SECONDS_MIN"
            :max="SHELL_DONE_SECONDS_MAX"
            step="1"
            inputmode="numeric"
            class="ccm-bare-input h-full min-w-0 flex-1 bg-transparent tabular outline-none"
            data-setting="shell-done-seconds"
            @change="settings.shellDoneSeconds = clampShellDoneSeconds(Number(($event.target as HTMLInputElement).value))"
          />
        </div>
      </SettingRow>
      <ToggleSwitch
        v-if="!uiGen.isNext"
        v-model="settings.agentShim"
        :label="t('shellMarks.agentShim')"
        :info="t('shellMarks.agentShimHint')"
        class="px-3"
        data-setting="agent-shim"
        data-setting-row="agentShim"
      />
      <ToggleSwitch
        v-if="!uiGen.isNext"
        v-model="settings.compactAuto"
        :label="t('workspace.compact.auto.setting')"
        :info="t('workspace.compact.auto.settingHint')"
        class="px-3"
        data-setting="compact-auto"
        data-setting-row="compactAuto"
      />
      <ToggleSwitch
        v-if="!uiGen.isNext"
        v-model="settings.clickMenu"
        :label="t('clickMenu.setting')"
        :info="t('clickMenu.settingHint')"
        class="px-3"
        data-setting="click-menu"
        data-setting-row="clickMenu"
      />
      <ToggleSwitch
        v-if="!uiGen.isNext"
        v-model="settings.menuToast"
        :label="t('clickMenu.toast')"
        :info="t('clickMenu.toastHint')"
        class="px-3"
        data-setting="menu-toast"
        data-setting-row="menuToast"
      />
    </SettingSection>
  </div>
</template>
