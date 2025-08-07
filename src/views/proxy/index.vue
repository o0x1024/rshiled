<template>
  <div class="container">
      <a-tabs v-model:activeKey="activeTab" >
        <a-tab-pane key="intercept" :title="$t('proxy.tabs.intercept')">
          <intercept />
        </a-tab-pane>
        <a-tab-pane key="history" :title="$t('proxy.tabs.history')">
          <history />
        </a-tab-pane>
        <a-tab-pane key="settings" :title="$t('proxy.tabs.settings')">
          <settings />
        </a-tab-pane>
      </a-tabs>
      
      <!-- 状态指示器 -->
      <!-- <div class="status-indicator">
        <a-tag :color="proxyStatus ? 'green' : 'red'" size="small">
          {{ proxyStatus ? $t('proxy.status.running') : $t('proxy.status.stopped') }}
        </a-tag>
      </div> -->
  </div>
</template>

<script lang="ts" setup>
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import intercept from './components/intercept.vue';
import history from './components/history.vue';
import settings from './components/settings.vue';


const activeTab = ref('intercept');
const proxyStatus = ref(false);

// 定期检查代理状态
let statusCheckInterval: number | null = null;

// 检查代理状态函数
const checkProxyStatus = async () => {
  try {
    const status = await invoke('get_proxy_status');
    proxyStatus.value = !!status;
    
    // 触发全局事件
    const event = new CustomEvent('proxy-status-change', {
      detail: { status: proxyStatus.value }
    });
    window.dispatchEvent(event);
  } catch (error) {
    console.error('获取代理状态失败:', error);
    // 发生错误时将状态设置为false
    proxyStatus.value = false;
  }
};

onMounted(async () => {
  // 初始获取代理状态
  await checkProxyStatus();

  // 设置状态检查定时器（每5秒检查一次）
  statusCheckInterval = window.setInterval(checkProxyStatus, 5000);

  // 监听代理状态变化的事件
  window.addEventListener('proxy-status-change', (event: any) => {
    if (event.detail) {
      proxyStatus.value = event.detail.status;
    }
  });
  
  // 监听代理错误事件
  window.addEventListener('proxy-error', () => {
    // 发生错误时重新检查状态
    setTimeout(checkProxyStatus, 500);
  });
});

onUnmounted(() => {
  // 清除定时器
  if (statusCheckInterval !== null) {
    clearInterval(statusCheckInterval);
  }
});
</script>

<style scoped lang="less">
.container {
  padding: 16px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.nav-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--color-border);
}

.active {
  font-weight: bold;
  color: rgb(var(--primary-6));
}

.content {
  flex: 1;
  overflow: auto;
}

.status-indicator {
  position: absolute;
  bottom: 8px;
  right: 16px;
  z-index: 10;
}
</style> 