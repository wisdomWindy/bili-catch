<script setup lang="ts">
import { ref, watch } from "vue";
import { Film } from "@lucide/vue";
import type { ParseVideoResult } from "../contracts";

const props = defineProps<{ result: ParseVideoResult }>();
const coverFailed = ref(false);

watch(() => props.result.coverUrl, () => { coverFailed.value = false; });

function duration(value: number): string {
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  const seconds = value % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
    : `${minutes}:${String(seconds).padStart(2, "0")}`;
}
</script>

<template>
  <section class="video-summary" aria-labelledby="video-title">
    <div class="cover-frame">
      <img v-if="result.coverUrl && !coverFailed" :src="result.coverUrl" alt="" @error="coverFailed = true" />
      <Film v-else :size="36" aria-hidden="true" />
    </div>
    <div class="video-copy">
      <h2 id="video-title">{{ result.title || "—" }}</h2>
      <p>{{ result.ownerName || "—" }}</p>
      <dl>
        <div><dt>BV</dt><dd>{{ result.bvid }}</dd></div>
        <div><dt>{{ $t("download.duration") }}</dt><dd>{{ duration(result.durationSeconds) }}</dd></div>
        <div><dt>{{ $t("download.partsCount") }}</dt><dd>{{ result.parts.length }}</dd></div>
      </dl>
    </div>
  </section>
</template>
