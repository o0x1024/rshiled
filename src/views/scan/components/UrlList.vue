<template>
  <div class="url-list-container">
    <div class="url-list-header">
      <span class="title">{{ $t('scan.url_list') }}</span>
      <a-space>
        <a-button type="text" @click="clearList">
          <template #icon><icon-delete /></template>
          {{ $t('scan.clear_list') }}
        </a-button>
      </a-space>
    </div>
    <div class="url-list-content" ref="listRef">
      <a-scrollbar style="height: 100%">
        <a-list :data="urlList" :bordered="false">
          <template #item="{ item }">
            <a-list-item>
              <div class="url-item">
                <div class="url-info">
                  <a-space>
                    <icon-link />
                    <span class="url-text">{{ item.url }}</span>
                  </a-space>
                  <div class="url-meta">
                    <a-space>
                      <a-tag :color="getStatusColor(item.status)">
                        {{ getStatusText(item.status) }}
                      </a-tag>
                      <span class="time">{{ formatTime(item.timestamp) }}</span>
                    </a-space>
                  </div>
                </div>
                <div class="url-actions">
                  <a-space>
                    <a-button type="text" @click="copyUrl(item.url)">
                      <template #icon><icon-copy /></template>
                    </a-button>
                    <a-button type="text" @click="openInBrowser(item.url)">
                      <template #icon><icon-external-link /></template>
                    </a-button>
                  </a-space>
                </div>
              </div>
            </a-list-item>
          </template>
        </a-list>
      </a-scrollbar>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref} from 'vue';
import { Message } from '@arco-design/web-vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

interface UrlItem {
  url: string;
  status: 'scanning' | 'completed' | 'error';
  timestamp: number;
}

const urlList = ref<UrlItem[]>([]);
const listRef = ref<HTMLElement | null>(null);

// 获取状态颜色
const getStatusColor = (status: string) => {
  switch (status) {
    case 'scanning':
      return 'blue';
    case 'completed':
      return 'green';
    case 'error':
      return 'red';
    default:
      return 'gray';
  }
};

// 获取状态文本
const getStatusText = (status: string) => {
  switch (status) {
    case 'scanning':
      return t('scan.status_scanning');
    case 'completed':
      return t('scan.status_completed');
    case 'error':
      return t('scan.status_error');
    default:
      return t('scan.status_unknown');
  }
};

// 格式化时间
const formatTime = (timestamp: number) => {
  return new Date(timestamp).toLocaleTimeString();
};

// 复制URL
const copyUrl = async (url: string) => {
  try {
    await navigator.clipboard.writeText(url);
    Message.success(t('scan.url_copied'));
  } catch (err) {
    Message.error(t('scan.copy_failed'));
  }
};

// 在浏览器中打开URL
const openInBrowser = async (url: string) => {
  try {
    await invoke('open_url', { url });
  } catch (err) {
    Message.error(t('scan.open_failed'));
  }
};

// 清空列表
const clearList = () => {
  urlList.value = [];
};

// 添加URL到列表
const addUrl = (url: string, status: 'scanning' | 'completed' | 'error' = 'scanning') => {
  urlList.value.unshift({
    url,
    status,
    timestamp: Date.now(),
  });
};

// 更新URL状态
const updateUrlStatus = (url: string, status: 'scanning' | 'completed' | 'error') => {
  const item = urlList.value.find(item => item.url === url);
  if (item) {
    item.status = status;
  }
};

// 暴露方法给父组件
defineExpose({
  addUrl,
  updateUrlStatus,
  clearList,
});
</script>

<style scoped>
.url-list-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-2);
  border-radius: 4px;
  overflow: hidden;
}

.url-list-header {
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--color-border);
}

.title {
  font-size: 16px;
  font-weight: 500;
  color: var(--color-text-1);
}

.url-list-content {
  flex: 1;
  overflow: hidden;
}

.url-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
}

.url-info {
  flex: 1;
  min-width: 0;
}

.url-text {
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.url-meta {
  margin-top: 4px;
}

.time {
  color: var(--color-text-3);
  font-size: 12px;
}

.url-actions {
  margin-left: 16px;
}

:deep(.arco-list-item) {
  padding: 8px 16px;
}

:deep(.arco-list-item:hover) {
  background-color: var(--color-fill-2);
}
</style> 