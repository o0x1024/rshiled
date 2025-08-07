<template>
  <div class="command-shell-container">
    <div class="shell-header">
      <div class="shell-title">
        <icon-command style="margin-right: 8px;" />
        {{ title || '命令行Shell' }}
      </div>
      <div class="shell-info">
        <a-tag>{{ shellType || 'bash' }}</a-tag>
        <a-tag color="arcoblue" v-if="sessionId">会话ID: {{ sessionId.substring(0, 8) }}...</a-tag>
      </div>
    </div>
    
    <div 
      ref="terminalRef"
      class="terminal-window" 
      :class="{ 'dark-theme': isDarkTheme }"
      @click="focusInput"
    >
      <!-- 历史命令和输出 -->
      <div v-for="(item, index) in historyItems" :key="index" class="terminal-item">
        <!-- 命令提示符和命令 -->
        <div class="command-prompt">
          <span class="prompt-text">{{ getPromptByOS() }}</span>
          <span class="command-text">{{ item.command }}</span>
        </div>
        
        <!-- 命令输出 -->
        <div class="command-output" v-if="item.output">
          <pre>{{ item.output }}</pre>
        </div>
      </div>
      
      <!-- 当前命令输入行 -->
      <div class="command-input-line">
        <span class="prompt-text">{{ getPromptByOS() }}</span>
        <input
          ref="commandInputRef"
          v-model="currentCommand"
          class="command-input"
          @keydown.enter="executeCommand"
          @keydown.up="navigateHistory(-1)" 
          @keydown.down="navigateHistory(1)"
          @keydown.tab.prevent="handleTabCompletion"
          :disabled="isExecuting"
          :placeholder="isExecuting ? '执行中...' : '输入命令...'"
          spellcheck="false"
          autocomplete="off"
        />
      </div>
    </div>
    
    <div class="shell-footer">
      <div class="shell-status">
        <a-badge 
          status="processing" 
          :color="isConnected ? 'green' : 'red'" 
          text=""
        />
        {{ isConnected ? '已连接' : '未连接' }}
      </div>
      
      <div class="shell-actions">
        <a-button-group size="mini">
          <a-button @click="clearTerminal" :disabled="isExecuting">
            清空
          </a-button>
          <a-button @click="toggleTheme">
            {{ isDarkTheme ? '浅色' : '深色' }}
          </a-button>
        </a-button-group>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { ref, onMounted, watch, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';
import { IconCommand } from '@arco-design/web-vue/es/icon';

// interface CommandResult {
//   command: string;
//   output: string;
//   timestamp?: number;
// }

interface HistoryItem {
  command: string;
  output: string;
}

// 组件属性
const props = defineProps({
  // 连接参数
  target: {
    type: String,
    required: true
  },
  cmdParam: {
    type: String,
    required: true
  },
  method: {
    type: String,
    default: 'GET'
  },
  prefix: {
    type: String,
    default: ''
  },
  suffix: {
    type: String,
    default: ''
  },
  shellType: {
    type: String,
    default: 'bash'
  },
  proxyUrl: {
    type: String,
    default: ''
  },
  // UI定制
  title: {
    type: String,
    default: '命令行Shell'
  },
  darkTheme: {
    type: Boolean,
    default: true
  },
  // 输出事件
  onOutput: {
    type: Function,
    default: null
  }
});

// 事件
const emit = defineEmits(['command-executed', 'session-created', 'connection-status']);

// 会话和连接状态
const sessionId = ref('');
const isConnected = ref(false);
const isExecuting = ref(false);

// 命令历史和当前命令
const historyItems = ref<HistoryItem[]>([]);
const commandHistory = ref<string[]>([]);
const currentCommand = ref('');
const historyIndex = ref(-1);

// UI相关
const isDarkTheme = ref(props.darkTheme);
const terminalRef = ref<HTMLElement | null>(null);
const commandInputRef = ref<HTMLInputElement | null>(null);

// 获取OS对应的命令提示符
const getPromptByOS = () => {
  switch (props.shellType) {
    case 'powershell':
      return 'PS> ';
    case 'cmd':
      return 'C:\\> ';
    default:
      return '$ ';
  }
};

// 执行命令
const executeCommand = async () => {
  if (!currentCommand.value.trim() || isExecuting.value) return;
  
  isExecuting.value = true;
  const command = currentCommand.value.trim();
  
  // 添加到命令历史
  commandHistory.value.push(command);
  historyIndex.value = commandHistory.value.length;
  
  // 清空当前命令
  currentCommand.value = '';
  
  try {
    // 添加命令到显示历史，但不含输出
    historyItems.value.push({
      command,
      output: ''
    });
    // 准备请求参数
    const params = {
      plugin_name: "命令执行交互式Shell",
      plugin_type: "vuln",
      target: props.target,
      custom_params: {
        cmd_param: props.cmdParam,
        method: props.method,
        prefix: props.prefix,
        suffix: props.suffix,
        shell_type: props.shellType,
        proxy_url: props.proxyUrl,
        command: command,
        session_id: sessionId.value || undefined,
        history: sessionId.value ? [] : undefined // 只在第一次请求时传历史记录
      }
    };
    console.log("params: " , params);

    // 调用后端执行插件
    const result = await invoke('execute_rhai_plugin', {
      params: params
    }) as any;
    
    if (result && result.data) {
      // 更新会话ID
      if (!sessionId.value && result.data.session_id) {
        sessionId.value = result.data.session_id;
        emit('session-created', sessionId.value);
      }
      
      // 更新输出
      const lastIndex = historyItems.value.length - 1;
      if (lastIndex >= 0) {
        historyItems.value[lastIndex].output = result.data.output || '命令执行成功，但没有输出';
      }
      
      // 更新连接状态
      isConnected.value = true;
      emit('connection-status', true);
      
      // 触发命令执行完成事件
      emit('command-executed', {
        command,
        output: result.data.output,
        success: result.success
      });
      
      // 如果设置了输出回调则调用
      if (props.onOutput && typeof props.onOutput === 'function') {
        props.onOutput(result.data.output);
      }
    } else {
      // 处理错误
      const lastIndex = historyItems.value.length - 1;
      if (lastIndex >= 0) {
        historyItems.value[lastIndex].output = `错误: ${result.details || '未知错误'}`;
      }
      
      // 更新连接状态
      isConnected.value = false;
      emit('connection-status', false);
    }
  } catch (error: any) {
    // 处理异常
    const lastIndex = historyItems.value.length - 1;
    if (lastIndex >= 0) {
      historyItems.value[lastIndex].output = `执行错误: ${error.message || String(error)}`;
    }
    
    // 更新连接状态
    isConnected.value = false;
    emit('connection-status', false);
    
    Message.error(`命令执行失败: ${error.message || String(error)}`);
  } finally {
    isExecuting.value = false;
    
    // 滚动到底部
    await nextTick();
    scrollToBottom();
    
    // 重新聚焦输入框
    focusInput();
  }
};

// 浏览命令历史
const navigateHistory = (direction: number) => {
  if (commandHistory.value.length === 0) return;
  
  // 如果当前不在历史中，先保存当前命令
  if (historyIndex.value === commandHistory.value.length) {
    // 保存当前输入以便返回
  }
  
  // 计算新的历史索引
  const newIndex = historyIndex.value + direction;
  
  if (newIndex >= 0 && newIndex <= commandHistory.value.length) {
    historyIndex.value = newIndex;
    
    // 设置命令
    if (newIndex === commandHistory.value.length) {
      // 回到最新状态，显示空命令
      currentCommand.value = '';
    } else {
      currentCommand.value = commandHistory.value[newIndex] || '';
    }
    
    // 将光标移到命令末尾
    nextTick(() => {
      if (commandInputRef.value) {
        commandInputRef.value.setSelectionRange(
          currentCommand.value.length,
          currentCommand.value.length
        );
      }
    });
  }
};

// Tab自动补全（简化版）
const handleTabCompletion = () => {
  // 这里可以实现自动补全逻辑
  // 简单示例：补全基本的Linux命令
  const commonCommands = ['ls', 'cd', 'pwd', 'cat', 'echo', 'grep', 'find', 'ps', 'whoami', 'ifconfig', 'netstat'];
  
  if (currentCommand.value) {
    const matchedCommands = commonCommands.filter(cmd => cmd.startsWith(currentCommand.value));
    if (matchedCommands.length === 1) {
      currentCommand.value = matchedCommands[0] + ' ';
    } else if (matchedCommands.length > 1) {
      // 多个匹配项，显示可能的补全选项
      const output = '\n可能的命令: ' + matchedCommands.join('  ');
      historyItems.value.push({
        command: currentCommand.value,
        output
      });
      scrollToBottom();
    }
  }
};

// 清空终端
const clearTerminal = () => {
  historyItems.value = [];
};

// 切换主题
const toggleTheme = () => {
  isDarkTheme.value = !isDarkTheme.value;
};

// 聚焦输入框
const focusInput = () => {
  nextTick(() => {
    if (commandInputRef.value) {
      commandInputRef.value.focus();
    }
  });
};

// 滚动到底部
const scrollToBottom = () => {
  nextTick(() => {
    if (terminalRef.value) {
      terminalRef.value.scrollTop = terminalRef.value.scrollHeight;
    }
  });
};

// 测试连接
const testConnection = async () => {
  try {
    isExecuting.value = true;
    
    // 执行一个简单的命令来测试连接
    const testCommand = props.shellType === 'cmd' ? 'ver' : 'echo "Connection test"';
    
    console.log("props: " , props.cmdParam);
    // 调用后端执行插件
    const result = await invoke('execute_rhai_plugin', {
      params: {
        plugin_name: "命令执行交互式Shell",
        plugin_type: "vuln",
        target: props.target,
        custom_params: {
          cmd_param: props.cmdParam,
          method: props.method,
          prefix: props.prefix,
          suffix: props.suffix,
          shell_type: props.shellType,
          proxy_url: props.proxyUrl,
          command: testCommand
        }
      }
    }) as any;

    if (result && result.success) {
      // 连接成功
      isConnected.value = true;
      emit('connection-status', true);
      
      // 更新会话ID
      if (result.data && result.data.session_id) {
        sessionId.value = result.data.session_id;
        emit('session-created', sessionId.value);
      }
      
      // 添加欢迎消息
      historyItems.value.push({
        command: testCommand,
        output: result.data?.output || '连接成功'
      });
      
      Message.success('连接成功');
    } else {
      // 连接失败
      isConnected.value = false;
      emit('connection-status', false);
      
      historyItems.value.push({
        command: '连接测试',
        output: `连接失败: ${result?.details || '未知错误'}`
      });
      
      Message.error(`连接失败: ${result?.details || '未知错误'}`);
    }
  } catch (error: any) {
    // 处理异常
    isConnected.value = false;
    emit('connection-status', false);
    
    historyItems.value.push({
      command: '连接测试',
      output: `连接错误: ${error.message || String(error)}`
    });
    
    Message.error(`连接错误: ${error.message || String(error)}`);
  } finally {
    isExecuting.value = false;
    scrollToBottom();
    focusInput();
  }
};

// 组件挂载
onMounted(async () => {
  focusInput();
  await testConnection();
});

// 当连接参数改变时重新测试连接
watch(
  () => [
    props.target, 
    props.cmdParam, 
    props.method, 
    props.prefix, 
    props.suffix, 
    props.shellType, 
    props.proxyUrl
  ],
  async () => {
    // 清空历史，重置状态
    historyItems.value = [];
    commandHistory.value = [];
    historyIndex.value = -1;
    sessionId.value = '';
    isConnected.value = false;
    
    // 重新测试连接
    await testConnection();
  },
  { deep: true }
);
</script>

<style scoped>
.command-shell-container {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  overflow: hidden;
  height: 100%;
  min-height: 300px;
  background-color: var(--color-bg-2);
  font-family: 'Courier New', monospace;
}

.shell-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background-color: var(--color-bg-3);
  border-bottom: 1px solid var(--color-border);
}

.shell-title {
  display: flex;
  align-items: center;
  font-weight: bold;
}

.shell-info {
  display: flex;
  gap: 8px;
}

.terminal-window {
  flex: 1;
  padding: 12px;
  overflow-y: auto;
  background-color: #f5f5f5;
  color: #333;
  font-size: 14px;
  line-height: 1.5;
}

.terminal-window.dark-theme {
  background-color: #1e1e1e;
  color: #f0f0f0;
}

.terminal-item {
  margin-bottom: 8px;
}

.command-prompt {
  display: flex;
  margin-bottom: 4px;
}

.prompt-text {
  color: #0a8a0a;
  margin-right: 4px;
  font-weight: bold;
}

.dark-theme .prompt-text {
  color: #4caf50;
}

.command-text {
  word-break: break-all;
  font-weight: bold;
}

.command-output {
  margin-left: 12px;
  white-space: pre-wrap;
  word-break: break-all;
}

.command-output pre {
  margin: 0;
  font-family: 'Courier New', monospace;
}

.command-input-line {
  display: flex;
  align-items: center;
  margin-top: 8px;
}

.command-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: inherit;
  font-family: 'Courier New', monospace;
  font-size: 14px;
  padding: 0;
  margin: 0;
}

.shell-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background-color: var(--color-bg-3);
  border-top: 1px solid var(--color-border);
}

.shell-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}

.shell-actions {
  display: flex;
  gap: 8px;
}
</style> 