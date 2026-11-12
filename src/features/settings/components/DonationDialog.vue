<script setup lang="ts">
import { NGrid, NGridItem, NImage, NModal, NSpace, NText } from "naive-ui";
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import alipayQrCodeUrl from "../../../assets/donation/alipay-donation-qr.jpg";
import wechatQrCodeUrl from "../../../assets/donation/wechat-donation-qr.png";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const closeButton = ref<HTMLButtonElement | null>(null);
let restoreFocus: HTMLElement | null = null;

function requestClose() {
  emit("close");
}

function focusCloseAction() {
  closeButton.value?.focus({ preventScroll: true });
}

function restorePreviousFocus() {
  restoreFocus?.focus();
  restoreFocus = null;
}

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") requestClose();
}

watch(() => props.show, async (isShown) => {
  if (!isShown) return;
  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  if (active && !active.closest(".n-modal-body-wrapper")) restoreFocus = active;
  await nextTick();
  await nextTick();
  focusCloseAction();
}, { flush: "sync", immediate: true });

onBeforeUnmount(restorePreviousFocus);
</script>

<template>
  <NModal
    :show="show"
    :auto-focus="false"
    :trap-focus="true"
    :mask-closable="true"
    :close-on-esc="true"
    @after-enter="focusCloseAction"
    @after-leave="restorePreviousFocus"
    @update:show="!$event && requestClose()"
  >
    <div
      class="settings-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="donation-dialog-title"
      tabindex="-1"
      @keydown="handleDialogKeydown"
    >
      <h3 id="donation-dialog-title">{{ t("settings.about.donation") }}</h3>
      <NGrid :cols="1" :y-gap="18">
        <NGridItem>
          <NSpace vertical align="center" :size="8">
            <NText strong>{{ t("settings.about.alipay") }}</NText>
            <NImage
              width="100%"
              object-fit="contain"
              preview-disabled
              :src="alipayQrCodeUrl"
              :alt="t('settings.about.alipayQrAlt')"
            />
          </NSpace>
        </NGridItem>
        <NGridItem>
          <NSpace vertical align="center" :size="8">
            <NText strong>{{ t("settings.about.wechatPay") }}</NText>
            <NImage
              width="100%"
              object-fit="contain"
              preview-disabled
              :src="wechatQrCodeUrl"
              :alt="t('settings.about.wechatQrAlt')"
            />
          </NSpace>
        </NGridItem>
      </NGrid>
      <NSpace justify="end">
        <button ref="closeButton" data-testid="close-donation" type="button" class="secondary-button" @click="requestClose">
          {{ t("settings.about.closeDonation") }}
        </button>
      </NSpace>
    </div>
  </NModal>
</template>
