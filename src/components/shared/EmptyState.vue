<script setup lang="ts">
import type { Component } from "vue";

defineProps<{
  icon: Component;
  title: string;
  message: string;
}>();
</script>

<template>
  <section class="empty-state">
    <component :is="icon" class="empty-icon" :size="28" :stroke-width="1.6" aria-hidden="true" />
    <h2 data-testid="page-heading">{{ title }}</h2>
    <p>{{ message }}</p>
    <slot />
  </section>
</template>
