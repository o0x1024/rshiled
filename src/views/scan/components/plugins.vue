<template>
  <div class="scan-plugins-container">
    <ScanPluginManager />
  </div>
</template>

<script setup>
import { defineComponent } from 'vue';
import ScanPluginManager from './ScanPluginManager.vue';
</script>

<style scoped>
.scan-plugins-container {
  width: 100%;
  height: 100%;
}
</style> 