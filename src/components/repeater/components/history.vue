<template>
  <div class="history-container">
    <a-card title="请求历史">
      <a-table :data="historyData" :pagination="false" :bordered="false">
        <template #columns>
          <a-table-column title="方法" data-index="method" :width="80" />
          <a-table-column title="URL" data-index="url" />
          <a-table-column title="状态" data-index="status" :width="80">
            <template #cell="{ record }">
              <a-tag color="green" v-if="record.status >= 200 && record.status < 300">{{ record.status }}</a-tag>
              <a-tag color="orange" v-if="record.status >= 300 && record.status < 400">{{ record.status }}</a-tag>
              <a-tag color="red" v-if="record.status >= 400">{{ record.status }}</a-tag>
            </template>
          </a-table-column>
          <a-table-column title="时间" data-index="time" :width="80">
            <template #cell="{ record }">{{ record.time }}ms</template>
          </a-table-column>
          <a-table-column title="操作" :width="150">
            <template #cell="{ record }">
              <a-button type="text" @click="loadRequest(record)">加载</a-button>
              <a-button type="text" status="danger" @click="removeHistoryItem(record.id)">删除</a-button>
            </template>
          </a-table-column>
        </template>
      </a-table>
    </a-card>
  </div>
</template>

<script lang="ts" setup>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';

// 添加组件名称，用于keep-alive识别
defineOptions({
  name: 'repeater-history'
});

interface HistoryItem {
  id: string;
  method: string;
  url: string;
  status: number;
  time: number;
  // 其他可能的属性
}

const historyData = ref<HistoryItem[]>([]);

// 从后端加载历史记录
const loadHistory = async () => {
  try {
    const result = await invoke<HistoryItem[]>('repeater_get_request_history');
    historyData.value = result;
  } catch (error) {
    Message.error(`加载历史记录失败: ${error}`);
  }
};

// 加载请求到Repeater
const loadRequest = (record: HistoryItem) => {
  emit('loadRequest', record);
};

// 删除历史记录项
const removeHistoryItem = async (id: string) => {
  try {
    await invoke('repeater_delete_history_item', { id });
    loadHistory();
  } catch (error) {
    Message.error(`删除历史记录失败: ${error}`);
  }
};

const emit = defineEmits(['loadRequest']);

onMounted(() => {
  loadHistory();
});
</script>

<style scoped>
.history-container {
  height: 100%;
}
</style> 