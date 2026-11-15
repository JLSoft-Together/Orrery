<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSettingsStore } from '../../stores/settings'
import ToggleSwitch from '../ToggleSwitch.vue'
import SegmentedControl from '../ui/SegmentedControl.vue'
import SettingRow from './SettingRow.vue'
import SettingSection from './SettingSection.vue'

const { t } = useI18n()
const settings = useSettingsStore()
const fileIconStyles = computed(() => [
  { value: 'atom' as const, label: t('settings.fileIconsAtom') },
  { value: 'simple' as const, label: t('settings.fileIconsSimple') },
])
</script>

<template>
  <div data-settings-workspace>
    <SettingSection id="clickMenuGroup" :title="t('mist.settings.clickMenuGroup')">
      <ToggleSwitch
        v-model="settings.clickMenu"
        :label="t('clickMenu.setting')"
        :info="t('clickMenu.settingHint')"
        class="px-3"
        data-setting="click-menu"
        data-setting-row="clickMenu"
      />
      <ToggleSwitch
        v-model="settings.menuToast"
        :label="t('clickMenu.toast')"
        :info="t('clickMenu.toastHint')"
        class="px-3"
        data-setting="menu-toast"
        data-setting-row="menuToast"
      />
    </SettingSection>
    <SettingSection id="panes" :title="t('mist.settings.panesGroup')">
      <ToggleSwitch v-model="settings.dimInactive" :label="t('workspace.dim.setting')" :info="t('workspace.dim.settingHint')" class="px-3" data-setting-row="dim" />
      <ToggleSwitch
        v-model="settings.compactAuto"
        :label="t('workspace.compact.auto.setting')"
        :info="t('workspace.compact.auto.settingHint')"
        class="px-3"
        data-setting="compact-auto"
        data-setting-row="compactAuto"
      />
      <ToggleSwitch
        v-model="settings.agentShim"
        :label="t('shellMarks.agentShim')"
        :info="t('shellMarks.agentShimHint')"
        class="px-3"
        data-setting="agent-shim"
        data-setting-row="agentShim"
      />
      <ToggleSwitch v-model="settings.dustEffect" :label="t('settings.dust')" :info="t('settings.dustHint')" class="px-3" data-setting-row="dust" />
      <SettingRow :label="t('settings.fileIcons')" :info="t('settings.fileIconsHint')" data-setting-row="fileIcons">
        <SegmentedControl v-model="settings.fileIcons" :options="fileIconStyles" :label="t('settings.fileIcons')" data-setting="file-icons" />
      </SettingRow>
    </SettingSection>
  </div>
</template>
