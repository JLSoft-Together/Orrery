<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { useI18n } from 'vue-i18n'
import { loadSettingsDeepSeek } from '../../features/deepseek'
import { hasDeepSeek } from '../../lib/desktop'
import { hasGitHub } from '../../lib/github'
import { useUiStore } from '../../stores/ui'
import GitHubAccountsSettings from '../GitHubAccountsSettings.vue'
import SettingEmbed from './SettingEmbed.vue'
import SettingPart from './SettingPart.vue'
import SettingsIntegrations from './SettingsIntegrations.vue'

const SettingsDeepSeek = defineAsyncComponent(loadSettingsDeepSeek)
const { t } = useI18n()
const ui = useUiStore()
</script>

<template>
  <div>
    <SettingPart v-if="hasGitHub" id="github" :title="t('v107gon.settings.parts.github')">
      <SettingEmbed id="github"><GitHubAccountsSettings :open="ui.settingsOpen" /></SettingEmbed>
    </SettingPart>
    <SettingPart id="tools" :title="t('v107gon.settings.parts.tools')"><SettingsIntegrations /></SettingPart>
    <SettingPart v-if="hasDeepSeek" id="deepseek" :title="t('v107gon.settings.parts.deepseek')"><SettingsDeepSeek /></SettingPart>
  </div>
</template>
