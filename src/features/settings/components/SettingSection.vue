<script setup lang="ts">
defineProps<{ id: string; title: string }>();
</script>

<template>
  <section class="settings-section" :aria-labelledby="`${id}-title`">
    <header class="settings-section__header"><h2 :id="`${id}-title`">{{ title }}</h2></header>
    <div class="settings-section__body"><slot /></div>
  </section>
</template>
