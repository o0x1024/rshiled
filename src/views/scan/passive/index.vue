<template>
  <div class="passive-scan-container">
    <div class="scan-header">
      <h2>{{ $t('scan.passive_scan') }}</h2>
      <a-space>
        <a-button type="primary" @click="startScan">
          <template #icon><icon-play /></template>
          {{ $t('scan.start_scan') }}
        </a-button>
        <a-button @click="stopScan">
          <template #icon><icon-stop /></template>
          {{ $t('scan.stop_scan') }}
        </a-button>
      </a-space>
    </div>
    
    <div class="scan-content">
      <div class="scan-main">
        <!-- 这里可以添加扫描的主要内容和配置 -->
        <a-card class="scan-config">
          <template #title>{{ $t('scan.scan_config') }}</template>
          <a-form :model="scanConfig" layout="vertical">
            <a-form-item :label="$t('scan.target_url')">
              <a-input v-model="scanConfig.targetUrl" placeholder="https://example.com" />
            </a-form-item>
            <a-form-item :label="$t('scan.scan_depth')">
              <a-input-number v-model="scanConfig.depth" :min="1" :max="5" />
            </a-form-item>
            <a-form-item :label="$t('scan.max_pages')">
              <a-input-number v-model="scanConfig.maxPages" :min="1" :max="1000" />
            </a-form-item>
          </a-form>
        </a-card>
      </div>
      
      <div class="scan-sidebar">
        <url-list ref="urlListRef" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { Message } from '@arco-design/web-vue';
import UrlList from '../components/UrlList.vue';
import { invoke } from '@tauri-apps/api/core';

const urlListRef = ref();

const scanConfig = ref({
  targetUrl: '',
  depth: 2,
  maxPages: 100,
});

// 开始扫描
const startScan = async () => {
  if (!scanConfig.value.targetUrl) {
    Message.warning('请输入目标URL');
    return;
  }
  
  try {
    // 添加初始URL到列表
    urlListRef.value?.addUrl(scanConfig.value.targetUrl, 'scanning');
    
    // 调用后端开始扫描
    await invoke('start_passive_scan', {
      config: scanConfig.value,
    });
    
    Message.success('扫描已开始');
  } catch (err) {
    Message.error('启动扫描失败');
    urlListRef.value?.updateUrlStatus(scanConfig.value.targetUrl, 'error');
  }
};

// 停止扫描
const stopScan = async () => {
  try {
    await invoke('stop_passive_scan');
    Message.success('扫描已停止');
  } catch (err) {
    Message.error('停止扫描失败');
  }
};
</script>

<style scoped>
.passive-scan-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.scan-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.scan-content {
  flex: 1;
  display: flex;
  gap: 16px;
  overflow: hidden;
}

.scan-main {
  flex: 1;
  overflow-y: auto;
}

.scan-sidebar {
  width: 300px;
  overflow: hidden;
}

.scan-config {
  margin-bottom: 16px;
}
</style> 