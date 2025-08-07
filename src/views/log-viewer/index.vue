<template>
  <div class="log-viewer-container">
    <div class="log-header">
      <div class="log-info">
        <div class="log-path">{{ $t('settings.log.path') }}: {{ logPath }}</div>
      </div>
      <div class="log-actions">
        <a-switch v-model="autoRefresh" size="small">
          <template #checked>{{ $t('settings.log.autoRefresh') }}</template>
          <template #unchecked>{{ $t('settings.log.autoRefresh') }}</template>
        </a-switch>
        <a-button type="primary" size="small" @click="refreshLogContent">
          {{ $t('settings.log.refresh') }}
        </a-button>
      </div>
    </div>
    <a-divider style="margin: 8px 0" />
    <a-spin :loading="logLoading">
      <div class="log-container">
        <div class="code-editor-container" :class="{ 'dark-mode': isDarkMode }">
          <codemirror 
            :options="cmOptions" 
            v-model="logContent" 
            :style="{ height: 'calc(100vh - 120px)', width: '100%', fontSize: '12px' }"
            :extensions="logExtensions" 
            :readonly="true"
            @ready="handleLogEditorReady" 
          />
        </div>
      </div>
    </a-spin>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
// import { useI18n } from 'vue-i18n';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { Codemirror } from 'vue-codemirror';
import { rust } from '@codemirror/lang-rust';
import { oneDark } from '@codemirror/theme-one-dark';
import { listen } from '@tauri-apps/api/event';
import {  getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

// const { t } = useI18n();
const logContent = ref('');
const logLoading = ref(false);
const logPath = ref('');
const isDarkMode = ref(false);
const logEditorRef = ref<any>(null);
const autoRefresh = ref(true);
const refreshInterval = ref<number | null>(null);

// 设置CodeMirror扩展
const logExtensions = computed(() => [
  rust(),
  isDarkMode.value ? oneDark : []
]);

// 处理编辑器就绪
const handleLogEditorReady = (editor: any) => {
  logEditorRef.value = editor;
  // 编辑器准备好后立即滚动到底部
  scrollToBottom();
  // 再次尝试滚动到底部确保成功
  setTimeout(scrollToBottom, 100);
  setTimeout(scrollToBottom, 300);
};

const cmOptions = {
      mode: "log",
      theme: "default",
    };
// 滚动到底部
const scrollToBottom = () => {
  if (!logEditorRef.value) return;
  
  try {
    const view = logEditorRef.value.view;
    if (!view) return;
    
    const doc = view.state.doc;
    const lineCount = doc.lines;
    
    if (lineCount > 0) {
      const lastLine = doc.line(lineCount);
      const scrollEffect = view.scrollToPos(lastLine.to);
      
      view.dispatch({
        effects: scrollEffect
      });
      
      // 额外的滚动方法，通过DOM直接滚动
      const editorDOM = view.dom;
      if (editorDOM && editorDOM.scrollHeight) {
        editorDOM.scrollTop = editorDOM.scrollHeight;
      }
    }
  } catch (err) {
    console.error('滚动到底部失败:', err);
  }
};

// 刷新日志内容
async function refreshLogContent() {
  logLoading.value = true;
  try {
    // const _prevLength = logContent.value.length;
    logContent.value = await invoke('read_log_file');
    
    // 无论内容是否更新，都尝试滚动到底部
    // 设置较长的延时确保内容完全渲染后再滚动
    setTimeout(scrollToBottom, 200);
  } catch (error) {
    Message.error(`刷新日志失败: ${error}`);
  } finally {
    logLoading.value = false;
  }
}

// 监听自动刷新开关
function startAutoRefresh() {
  if (autoRefresh.value) {
    stopAutoRefresh();
    refreshInterval.value = window.setInterval(refreshLogContent, 5000);
  }
}

function stopAutoRefresh() {
  if (refreshInterval.value) {
    clearInterval(refreshInterval.value);
    refreshInterval.value = null;
  }
}

// 监听主窗口传递的数据
async function setupEventListeners() {
  try {
    const logDataUnlisten = await listen('log-data', (event) => {
      const { logPath: path, isDark } = event.payload as any;
      logPath.value = path;
      isDarkMode.value = isDark;
      refreshLogContent();
      // 额外增加一次延迟滚动确保首次加载时能滚动到底部
      setTimeout(scrollToBottom, 500);
    });
    
    const themeChangedUnlisten = await listen('theme-changed', (event) => {
      const { isDark } = event.payload as any;
      isDarkMode.value = isDark;
    });
    
    return () => {
      logDataUnlisten();
      themeChangedUnlisten();
    };
  } catch (error) {
    console.error('设置事件监听失败:', error);
    Message.error(`设置事件监听失败: ${error}`);
    return () => {};
  }
}

// 组件挂载后初始化
onMounted(async () => {
  try {
    // 监听主窗口传递的事件
    const unlisten = await setupEventListeners();
    
    // 确保窗口获得焦点
    try {
      const appWindow = getCurrentWebviewWindow();
      await appWindow.setFocus();
      await appWindow.show();
    } catch (err) {
      console.error('设置窗口焦点失败:', err);
    }
    
    // 获取日志文件路径
    try {
      logPath.value = await invoke('get_log_file_path');
    } catch (error) {
      console.error('获取日志路径失败:', error);
      logPath.value = '获取日志路径失败';
    }
    
    // 读取日志内容
    await refreshLogContent();
    
    // 增加多次尝试滚动到底部，确保成功
    const scrollTimers = [
      setTimeout(scrollToBottom, 200),
      setTimeout(scrollToBottom, 500),
      setTimeout(scrollToBottom, 1000)
    ];
    
    // 开始自动刷新
    startAutoRefresh();
    
    // 组件卸载时清理
    onUnmounted(() => {
      // 停止自动刷新
      stopAutoRefresh();
      // 清理事件监听
      unlisten();
      // 清理所有滚动定时器
      scrollTimers.forEach(timer => clearTimeout(timer));
      
      // 确保所有状态被重置
      logEditorRef.value = null;
      refreshInterval.value = null;
      
      console.log('日志查看器已清理');
    });
  } catch (error) {
    Message.error(`无法初始化日志查看器: ${error}`);
  }
});

// 监听自动刷新开关变化
watch(autoRefresh, (newVal) => {
  if (newVal) {
    startAutoRefresh();
  } else {
    stopAutoRefresh();
  }
});
</script>

<style scoped lang="less">
.log-viewer-container {
  padding: 16px;
  height: 100vh;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-1);
}

.log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.log-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.log-path {
  font-size: 12px;
  color: var(--color-text-2);
}

.log-container {
  flex: 1;
  overflow: hidden;

  .code-editor-container {
    border: 1px solid var(--color-border-2);
    border-radius: 4px;
    overflow: hidden;
    height: 100%;

    :deep(.cm-editor) {
      height: 100%;
      font-size: 12px;
      background-color: var(--color-bg-2);
      color: var(--color-text-1);
    }

    :deep(.cm-gutters) {
      border: none;
      background-color: var(--color-bg-2);
      color: var(--color-text-3);
    }

    :deep(.cm-activeLineGutter) {
      background-color: var(--color-fill-2);
    }

    :deep(.cm-activeLine) {
      background-color: var(--color-fill-2);
    }

    :deep(.cm-content) {
      font-family: monospace;
    }

    :deep(.cm-lineNumbers) {
      color: var(--color-text-3);
    }

    :deep(.cm-cursor) {
      border-left: 2px solid var(--color-text-1);
    }

    :deep(.cm-selectionBackground) {
      background-color: var(--color-fill-2);
    }

    &.dark-mode {
      :deep(.cm-editor) {
        background-color: #1e1e1e;
        color: #d4d4d4;
      }

      :deep(.cm-gutters) {
        background-color: #262626;
        border-right: 1px solid #333;
      }

      :deep(.cm-activeLineGutter) {
        background-color: #2c2c2c;
      }

      :deep(.cm-activeLine) {
        background-color: #2c2c2c;
      }

      :deep(.cm-lineNumbers) {
        color: #666;
      }

      :deep(.cm-cursor) {
        border-left: 2px solid #fff;
      }

      :deep(.cm-selectionBackground) {
        background-color: #2c2c2c;
      }

      :deep(.cm-content) {
        color: #d4d4d4;
      }
    }
  }
}
</style> 