<script setup>
import { ref } from 'vue'
const name = ref('')
</script>

<template>
  <main style="font-family: sans-serif; padding: 24px">
    <h1>Sign in</h1>
    <form id="login">
      <input v-model="name" placeholder="Name" style="padding: 6px" />
      <button id="go" type="submit" style="color: rgb(255, 255, 255); background-color: rgb(0, 113, 227); font-size: 18px; font-weight: 600; padding: 10px 24px; margin: 8px 0 0 0; border: 0; border-radius: 12px">Sign in</button>
    </form>
  </main>
</template>
