<script setup lang="ts">
import { CheckCheck, ListRestart } from "@lucide/vue";
import type { VideoPart } from "../contracts";

defineProps<{ parts: VideoPart[]; selectedCids: number[] }>();
const emit = defineEmits<{ toggle: [cid: number]; selectAll: []; invert: [] }>();
</script>

<template>
  <section class="parts-section" aria-labelledby="parts-title">
    <div class="section-heading">
      <div>
        <h2 id="parts-title">{{ $t("download.parts") }}</h2>
        <p>{{ $t("download.selectedParts", { count: selectedCids.length }) }}</p>
      </div>
      <div class="compact-actions">
        <button type="button" @click="emit('selectAll')">
          <CheckCheck :size="16" aria-hidden="true" />{{ $t("download.selectAll") }}
        </button>
        <button type="button" @click="emit('invert')">
          <ListRestart :size="16" aria-hidden="true" />{{ $t("download.invert") }}
        </button>
      </div>
    </div>
    <div class="parts-list">
      <label v-for="part in parts" :key="part.cid" class="part-row">
        <input
          type="checkbox"
          :checked="selectedCids.includes(part.cid)"
          @change="emit('toggle', part.cid)"
        />
        <span class="part-index">P{{ part.page }}</span>
        <span class="part-title">{{ part.title || `P${part.page}` }}</span>
      </label>
    </div>
  </section>
</template>
