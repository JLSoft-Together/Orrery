<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { hasTerminals } from '../../../core/lib/desktop'
import SettingPart from './SettingPart.vue'
import SettingsShortcuts from '../../shortcuts/SettingsShortcuts.vue'
import SettingsEditor from './SettingsEditor.vue'
import SettingsWorkspace from './SettingsWorkspace.vue'

const { t } = useI18n()
</script>

<template>
  <div>
    <SettingPart v-if="hasTerminals" id="editor" :title="t('v107gon.settings.parts.editor')"><SettingsEditor /></SettingPart>
    <SettingPart id="workspace" :title="t('v107gon.settings.parts.workspace')"><SettingsWorkspace /></SettingPart>
    <SettingPart v-if="hasTerminals" id="shortcuts" :title="t('shortcutsV109.title')"><SettingsShortcuts /></SettingPart>
  </div>
</template>
