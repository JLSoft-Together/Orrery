<script setup lang="ts">
import { hasDiskMap } from '../lib/diskMap'
import { SettingEmbed } from '@features/settings'
import SettingsDiskMap from './SettingsDiskMap.vue'
</script>

<template>
  <div data-settings-disk>
    <SettingEmbed v-if="hasDiskMap" id="diskMap"><SettingsDiskMap /></SettingEmbed>
  </div>
</template>
