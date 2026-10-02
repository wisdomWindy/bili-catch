<script setup lang="ts">
import { Download, LoaderCircle } from "@lucide/vue";
import { NModal } from "naive-ui";
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  show: boolean;
  pending: boolean;
  currentVersion: string;
  latestVersion: string;
  errorMessage: string | null;
}>();
const emit = defineEmits<{ close: []; confirm: [] }>();
const { t } = useI18n();
const laterButton = ref<HTMLButtonElement | null>(null);
let restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;

function requestClose() {
  if (!props.pending) emit("close");
}

function focusSafeAction() {
  laterButton.value?.focus();
}

function restorePreviousFocus() {
  restoreFocus?.focus();
  restoreFocus = null;
}

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    requestClose();
    return;
  }
  if (event.key !== "Tab") return;
  const dialog = event.currentTarget;
  if (!(dialog instanceof HTMLElement)) return;
  const buttons = Array.from(dialog.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"));
  if (buttons.length === 0) {
    event.preventDefault();
    dialog.focus();
    return;
  }
  const first = buttons[0];
  const last = buttons[buttons.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

watch(() => props.show, async (show) => {
  if (show) {
    const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (active && !active.closest(".n-modal-body-wrapper")) restoreFocus = active;
    await nextTick();
    await nextTick();
    focusSafeAction();
  }
}, { flush: "sync" });

onBeforeUnmount(restorePreviousFocus);
</script>

<template>
  <NModal
    :show="show"
    display-directive="show"
    :auto-focus="true"
    :trap-focus="true"
    :mask-closable="!pending"
    :close-on-esc="!pending"
    @after-enter="focusSafeAction"
    @after-leave="restorePreviousFocus"
    @update:show="!$event && requestClose()"
  >
    <div
      class="update-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="startup-update-title"
      tabindex="-1"
      @keydown="handleDialogKeydown"
    >
      <div class="update-dialog__icon"><Download :size="22" aria-hidden="true" /></div>
      <h3 id="startup-update-title">{{ t("settings.updateDialog.title") }}</h3>
      <p>{{ t("settings.updateDialog.message") }}</p>
      <dl class="update-dialog__versions">
        <div>
          <dt>{{ t("settings.updateDialog.currentVersion") }}</dt>
          <dd>{{ currentVersion }}</dd>
        </div>
        <div>
          <dt>{{ t("settings.updateDialog.latestVersion") }}</dt>
          <dd class="update-dialog__latest">{{ latestVersion }}</dd>
        </div>
      </dl>
      <p v-if="errorMessage" class="update-dialog__error" role="alert">{{ errorMessage }}</p>
      <div class="update-dialog__actions">
        <button
          ref="laterButton"
          data-testid="update-later"
          type="button"
          tabindex="0"
          class="secondary-button"
          :disabled="pending"
          @click="requestClose"
        >
          {{ t("settings.updateDialog.later") }}
        </button>
        <button
          data-testid="install-startup-update"
          type="button"
          tabindex="0"
          class="primary-button"
          :disabled="pending"
          @click="emit('confirm')"
        >
          <LoaderCircle v-if="pending" :size="16" class="spin" aria-hidden="true" />
          <Download v-else :size="16" aria-hidden="true" />
          <span>{{ pending ? t("settings.updateDialog.installing") : t("settings.updateDialog.install") }}</span>
        </button>
      </div>
    </div>
  </NModal>
</template>
