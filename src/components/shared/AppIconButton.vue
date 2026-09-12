<script setup lang="ts">
import type { Component } from "vue";

withDefaults(
  defineProps<{
    label: string;
    icon: Component;
    disabled?: boolean;
  }>(),
  { disabled: false },
);

defineEmits<{ click: [event: MouseEvent] }>();
</script>

<template>
  <button
    class="icon-button"
    type="button"
    :aria-label="label"
    :title="label"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <component :is="icon" :size="19" :stroke-width="1.8" aria-hidden="true" />
  </button>
</template>
