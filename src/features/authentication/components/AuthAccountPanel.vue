<script setup lang="ts">
import { LogOut, UserRound } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AuthAccount } from "../contracts";

const props = defineProps<{ account: AuthAccount; pending: boolean }>();
const emit = defineEmits<{ logout: [] }>();
const { t } = useI18n();
const avatarLoaded = ref(true);

const displayName = computed(() => props.account.name.trim() || t("auth.accountFallback"));
const avatarUrl = computed(() => {
  if (!props.account.avatarUrl || !avatarLoaded.value) return null;
  try {
    const url = new URL(props.account.avatarUrl);
    const allowed = ["hdslb.com", "biliimg.com"].some(
      (host) => url.hostname === host || url.hostname.endsWith(`.${host}`),
    );
    return url.protocol === "https:" && allowed ? url.toString() : null;
  } catch {
    return null;
  }
});

watch(() => props.account.avatarUrl, () => { avatarLoaded.value = true; });
</script>

<template>
  <section class="auth-tool account-panel" aria-labelledby="account-panel-title">
    <div class="account-avatar">
      <img v-if="avatarUrl" :src="avatarUrl" alt="" referrerpolicy="no-referrer" @error="avatarLoaded = false" />
      <UserRound v-else data-testid="avatar-fallback" :size="42" :stroke-width="1.5" aria-hidden="true" />
    </div>
    <div class="account-copy">
      <span class="account-success">{{ t("auth.signedIn") }}</span>
      <h3 id="account-panel-title">{{ displayName }}</h3>
      <span v-if="account.mid" class="account-uid">UID {{ account.mid }}</span>
    </div>
    <button
      class="secondary-button account-logout"
      data-testid="logout-button"
      type="button"
      :disabled="pending"
      @click="emit('logout')"
    >
      <LogOut :size="17" aria-hidden="true" />{{ t("auth.logout") }}
    </button>
  </section>
</template>
