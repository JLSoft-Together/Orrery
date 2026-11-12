<template>
  <svg class="orbit" viewBox="0 0 120 120" width="120" height="120" aria-hidden="true">
    <circle cx="60" cy="60" r="54" fill="none" class="ring outer" stroke-width="1.5" />
    <circle cx="60" cy="60" r="36" fill="none" class="ring inner" stroke-width="1.5" />
    <circle cx="60" cy="60" r="12" fill="none" class="core-ring" stroke-width="2" />
    <circle cx="60" cy="60" r="6" class="core" />
    <circle cx="60" cy="6" r="5" class="dot-codex" />
    <circle cx="96" cy="60" r="4" class="dot-shell" />
  </svg>
</template>

<style scoped>
.orbit {
  color: var(--ccm-accent);
}

.ring.outer {
  stroke: color-mix(in srgb, var(--ccm-accent) 24%, transparent);
}

.ring.inner {
  stroke: color-mix(in srgb, var(--ccm-accent) 52%, transparent);
}

.core-ring {
  stroke: var(--ccm-accent);
}

.core {
  fill: var(--ccm-accent);
}

.dot-codex {
  fill: var(--ccm-agent-codex);
}

.dot-shell {
  fill: var(--ccm-agent-shell);
}
</style>
