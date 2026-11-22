<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { CheckCheck, ListRestart } from "@lucide/vue";
import type { ParseContainer, VideoPart, VideoSource } from "../contracts";
import { formatDuration } from "../format";

const props = defineProps<{
  parts: VideoPart[];
  selectedCids: number[];
  source: VideoSource;
  container: ParseContainer | null;
}>();
const emit = defineEmits<{
  toggle: [cid: number];
  toggleGroup: [cids: number[]];
  selectAll: [];
  invert: [];
}>();

const { t } = useI18n();
const listRef = ref<HTMLElement | null>(null);

watch(
  () => props.selectedCids,
  async () => {
    await nextTick();
    const cid = props.selectedCids[0];
    if (cid === undefined || !listRef.value) return;
    const row = listRef.value.querySelector<HTMLElement>(`[data-cid="${cid}"]`);
    row?.scrollIntoView?.({ block: "nearest" });
  },
  { immediate: true },
);

interface DisplayRow {
  key: string;
  kind: "group" | "unit" | "child";
  label: string;
  part: VideoPart | null;
  children: VideoPart[];
}

const isSeason = computed(() => props.container?.kind === "season");

const displayRows = computed<DisplayRow[]>(() => {
  const groups = new Map<number, VideoPart[]>();
  for (const part of props.parts) {
    const bucket = groups.get(part.groupIndex);
    if (bucket) {
      bucket.push(part);
    } else {
      groups.set(part.groupIndex, [part]);
    }
  }
  const rows: DisplayRow[] = [];
  for (const [groupIndex, units] of [...groups.entries()].sort((left, right) => left[0] - right[0])) {
    const ordered = [...units].sort((left, right) => left.page - right.page);
    if (isSeason.value && ordered.length > 1) {
      rows.push({
        key: `group-${groupIndex}`,
        kind: "group",
        label: `EP${groupIndex}`,
        part: null,
        children: ordered,
      });
      for (const child of ordered) {
        rows.push({
          key: `child-${child.cid}`,
          kind: "child",
          label: `P${child.page}`,
          part: child,
          children: [],
        });
      }
      continue;
    }
    for (const unit of ordered) {
      rows.push({
        key: `unit-${unit.cid}`,
        kind: "unit",
        label: isSeason.value ? `EP${groupIndex}` : `P${unit.page}`,
        part: unit,
        children: [],
      });
    }
  }
  return rows;
});

function isPlayable(part: VideoPart | null): boolean {
  return part?.availability === "playable";
}

function selectableCids(children: VideoPart[]): number[] {
  return children.filter((child) => child.availability === "playable").map((child) => child.cid);
}

function isGroupChecked(children: VideoPart[]): boolean {
  const cids = selectableCids(children);
  return cids.length > 0 && cids.every((cid) => props.selectedCids.includes(cid));
}

function isGroupIndeterminate(children: VideoPart[]): boolean {
  const cids = selectableCids(children);
  const selected = cids.filter((cid) => props.selectedCids.includes(cid)).length;
  return selected > 0 && selected < cids.length;
}

function unavailableReason(part: VideoPart | null): string {
  if (!part || part.availability === "playable") return "";
  return t(`download.unavailableReasons.${part.unavailableReason ?? "unknown"}`);
}
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
    <div ref="listRef" class="parts-list">
      <label
        v-for="row in displayRows"
        :key="row.key"
        class="part-row"
        :data-cid="row.part?.cid"
        :class="{
          'part-group': row.kind === 'group',
          'part-child': row.kind === 'child',
          'is-disabled': row.kind !== 'group' && !isPlayable(row.part),
        }"
      >
        <template v-if="row.kind === 'group'">
          <input
            type="checkbox"
            :checked="isGroupChecked(row.children)"
            :indeterminate="isGroupIndeterminate(row.children)"
            :disabled="selectableCids(row.children).length === 0"
            @change="emit('toggleGroup', selectableCids(row.children))"
          />
          <span class="part-index">{{ row.label }}</span>
          <span class="part-title">{{ row.children.length }} P</span>
        </template>
        <template v-else>
          <input
            type="checkbox"
            :checked="selectedCids.includes(row.part!.cid)"
            :disabled="!isPlayable(row.part)"
            :aria-describedby="isPlayable(row.part) ? undefined : `part-reason-${row.part!.cid}`"
            @change="emit('toggle', row.part!.cid)"
          />
          <span class="part-index">{{ row.label }}</span>
          <span class="part-title">{{ row.part!.title || row.label }}</span>
          <span v-if="row.part!.durationSeconds > 0" class="part-duration">
            {{ formatDuration(row.part!.durationSeconds) }}
          </span>
          <span
            v-if="!isPlayable(row.part)"
            :id="`part-reason-${row.part!.cid}`"
            class="part-reason"
          >
            {{ unavailableReason(row.part) }}
          </span>
        </template>
      </label>
    </div>
  </section>
</template>
