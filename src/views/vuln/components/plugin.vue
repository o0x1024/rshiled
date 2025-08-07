<template>
  <div class="plugin-manager-container">
    <RhaiPluginManager />
  </div>
</template>

<script setup>
// 添加组件名称，用于keep-alive识别
defineOptions({
  name: 'plugin'
});

import RhaiPluginManager from './RhaiPluginManager.vue';
</script>

<style scoped>

</style> 