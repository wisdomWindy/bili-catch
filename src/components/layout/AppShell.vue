<script setup lang="ts">
import Sidebar from "./Sidebar.vue";
import TopBar from "./TopBar.vue";
</script>

<template>
  <div class="app-shell" data-testid="app-shell">
    <Sidebar />
    <div class="workspace">
      <TopBar />
      <main class="content-scroll" tabindex="-1">
        <div class="content-container">
          <slot name="status" />
          <RouterView />
        </div>
      </main>
    </div>
  </div>
</template>
