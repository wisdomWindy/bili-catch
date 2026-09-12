<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps<{ open: boolean; title: string; message: string }>();
const emit = defineEmits<{ cancel: []; confirm: [] }>();
const { t } = useI18n();
const cancelButton = ref<HTMLButtonElement>();
let returnFocus: HTMLElement | null = null;
watch(() => props.open, async (open) => {
  if (open) {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    await nextTick();
    cancelButton.value?.focus();
  } else {
    await nextTick();
    returnFocus?.focus();
    returnFocus = null;
  }
});
</script>

<template>
  <div v-if="open" class="task-dialog-backdrop" role="presentation" @click.self="emit('cancel')" @keydown.esc="emit('cancel')">
    <section class="task-dialog" role="alertdialog" aria-modal="true" aria-labelledby="task-confirm-title">
      <h2 id="task-confirm-title">{{ title }}</h2>
      <p>{{ message }}</p>
      <div>
        <button ref="cancelButton" type="button" @click="emit('cancel')">{{ t('tasks.actions.keep') }}</button>
        <button class="danger-button" type="button" @click="emit('confirm')">{{ t('tasks.actions.confirm') }}</button>
      </div>
    </section>
  </div>
</template>
