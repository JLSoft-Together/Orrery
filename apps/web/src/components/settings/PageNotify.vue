<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { hasTelegram } from '../../lib/telegram'
import { useUiStore } from '../../stores/ui'
import TelegramSettings from '../TelegramSettings.vue'
import SettingEmbed from './SettingEmbed.vue'
import SettingPart from './SettingPart.vue'
import SettingsNotify from './SettingsNotify.vue'

const { t } = useI18n()
const ui = useUiStore()
</script>

<template>
  <div>
    <SettingsNotify />
    <SettingPart v-if="hasTelegram" id="telegram" :title="t('v107gon.settings.parts.telegram')">
      <SettingEmbed id="telegram"><TelegramSettings :open="ui.settingsOpen" /></SettingEmbed>
    </SettingPart>
  </div>
</template>
