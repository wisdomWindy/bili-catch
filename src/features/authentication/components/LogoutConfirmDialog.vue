<script setup lang="ts">
import { LogOut } from "@lucide/vue";
import { NModal } from "naive-ui";
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  show: boolean;
  pending: boolean;
  accountName: string;
  errorMessage: string | null;
}>();
const emit = defineEmits<{ close: []; confirm: [] }>();
const { t } = useI18n();
const keepButton = ref<HTMLButtonElement | null>(null);
let restoreFocus: HTMLElement | null = null;

function focusSafeAction() {
  keepButton.value?.focus();
}

function focusAfterEnter() {
  window.setTimeout(focusSafeAction, 0);
}

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && !props.pending) {
    emit("close");
    return;
  }
  if (event.key !== "Tab") return;
  const dialog = event.currentTarget;
  if (!(dialog instanceof HTMLElement)) return;
  const buttons = Array.from(dialog.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"));
  if (buttons.length === 0) return;
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

watch(() => props.show, async (show, previous) => {
  if (show) {
    restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    await nextTick();
    focusSafeAction();
  } else if (previous) {
    await nextTick();
    restoreFocus?.focus();
    restoreFocus = null;
  }
}, { flush: "post" });

onBeforeUnmount(() => restoreFocus?.focus());
</script>

<template>
  <NModal
    :show="show"
    display-directive="show"
    :auto-focus="false"
    :trap-focus="false"
    :mask-closable="!pending"
    :close-on-esc="!pending"
    @after-enter="focusAfterEnter"
    @update:show="!$event && emit('close')"
  >
    <div class="logout-dialog" role="alertdialog" aria-modal="true" aria-labelledby="logout-dialog-title" @keydown="handleDialogKeydown">
      <div class="dialog-icon"><LogOut :size="22" aria-hidden="true" /></div>
      <h3 id="logout-dialog-title">{{ t("auth.logoutTitle") }}</h3>
      <p>{{ t("auth.logoutMessage", { name: accountName }) }}</p>
      <p v-if="errorMessage" class="dialog-error" role="alert">{{ errorMessage }}</p>
      <div class="dialog-actions">
        <button ref="keepButton" class="secondary-button" data-testid="keep-login" type="button" :disabled="pending" @click="emit('close')">
          {{ t("auth.keepLogin") }}
        </button>
        <button class="danger-button" type="button" :disabled="pending" @click="emit('confirm')">
          <LogOut :class="{ spin: pending }" :size="16" aria-hidden="true" />
          {{ pending ? t("auth.loggingOut") : t("auth.confirmLogout") }}
        </button>
      </div>
    </div>
  </NModal>
</template>
