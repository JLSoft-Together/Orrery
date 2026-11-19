<script setup lang="ts">
defineProps<{ modelValue: boolean; label: string; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <button type="button" role="switch" class="switch" :class="{ on: modelValue }" :aria-checked="modelValue" :aria-label="label" :disabled="disabled" @click="emit('update:modelValue', !modelValue)">
    <span class="thumb" />
  </button>
</template>

<style scoped>
.switch {
  position: relative;
  flex: none;
  width: 51px;
  height: 31px;
  padding: 0;
  border: 0;
  border-radius: 16px;
  background: var(--ccm-fill-strong);
  transition: background-color var(--ccm-dur-base) var(--ccm-ease-standard);
}

.switch::after {
  content: '';
  position: absolute;
  inset: -6px -0px;
}

.switch.on {
  background: var(--ccm-accent);
}

.switch:disabled {
  opacity: 0.45;
}

.thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 27px;
  height: 27px;
  border-radius: 50%;
  background: #ffffff;
  box-shadow: 0 2px 6px rgb(0 0 0 / 0.2);
  transition: transform var(--ccm-dur-base) var(--ccm-ease-enter);
}

.on .thumb {
  transform: translateX(20px);
}
</style>
