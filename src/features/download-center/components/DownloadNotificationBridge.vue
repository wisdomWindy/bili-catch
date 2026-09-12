<script setup lang="ts">
import { provide } from "vue";
import { useMessage } from "naive-ui";
import { downloadNotifierKey } from "../injection";

const message = useMessage();
provide(downloadNotifierKey, {
  success: (text: string) => message.success(text),
  info: (text: string) => message.info(text),
});
</script>

<template><slot /></template>
