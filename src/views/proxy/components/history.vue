<template>
  <div class="history-container">
    <div class="history-header">
      <div >
        <a-space>
          <a-input-search
            size="small"
            v-model="searchKeyword"
            :placeholder="$t('proxy.history.search')"
            allow-clear
            @search="handleSearch"
          />
          <a-select
            size="small"
            v-model="filterMethod"
            :placeholder="$t('proxy.request.method')"
            allow-clear
            style="width: 100px"
          >
            <a-option value="GET">GET</a-option>
            <a-option value="POST">POST</a-option>
            <a-option value="PUT">PUT</a-option>
            <a-option value="DELETE">DELETE</a-option>
            <a-option value="PATCH">PATCH</a-option>
            <a-option value="OPTIONS">OPTIONS</a-option>
            <a-option value="HEAD">HEAD</a-option>
          </a-select>
          <a-select
            size="small"
            v-model="filterStatus"
            :placeholder="$t('proxy.request.status')"
            allow-clear
            style="width: 120px"
          >
            <a-option value="success">2xx</a-option>
            <a-option value="redirect">3xx</a-option>
            <a-option value="error">4xx/5xx</a-option>
          </a-select>
          <a-button status="danger" @click="clearHistory" size="small">
            {{ $t('proxy.history.clear') }}
          </a-button>
        </a-space>
      </div>
    </div>
    
    <!-- 上下分割布局 -->
    <a-split direction="vertical" :default-size="0.3"
      :style="{ height: '800px', width: '100%' }">
      <!-- 上方请求列表 -->
      <template #first>
        <div class="history-list">
          <a-table
            :data="filteredRequests"
            :bordered="false"
            :pagination="false"
            @row-click="(record) => viewRequestDetail(record)"
            size="mini"
            :scroll="{ y: '100%' }"
            :selected-keys="currentRequest ? [currentRequest.id.toString()] : []"
            :column-resizable="true"
            :sortable="{ sortDirections: ['ascend', 'descend'], sorter: (a:any, b:any) => Number(a.id) - Number(b.id) }"
          >
            <template #columns>
              <a-table-column 
                title="ID" 
                :width="80"
                data-index="id" 
                :ellipsis="true"
                :sortable="{
                  sortDirections: ['ascend', 'descend'],
                  sorter: (a, b) => Number(a.id) - Number(b.id)
                }" 
              />
              <a-table-column  :width="200" :title="$t('proxy.request.host')" :ellipsis="true">
                <template #cell="{ record }">
                  {{ getFullHost(record) }}
                </template>
              </a-table-column>
              <a-table-column :title="$t('proxy.request.method')" :width="100" data-index="method"  :ellipsis="true" :sortable="{ sortDirections: ['ascend', 'descend'] }">
                <template #cell="{ record }">
                  <a-tag :color="getMethodColor(record.method)" size="small">
                    {{ record.method }}
                  </a-tag>
                </template>
              </a-table-column>
              <a-table-column :title="$t('proxy.request.url')"  :width="500" :ellipsis="true">
                <template #cell="{ record }">
                  {{ getFullUrl(record) }}
                </template>
              </a-table-column>
              <a-table-column 
                :title="$t('proxy.request.status')" 
                data-index="status" 
                :width="100"
                :ellipsis="true"
                :sortable="{
                  sortDirections: ['ascend', 'descend'],
                  sorter: (a, b) => a.status - b.status
                }">
                <template #cell="{ record }">
                  <a-tag :color="getStatusColor(record.status)" size="small">
                    {{ record.status }}
                  </a-tag>
                </template>
              </a-table-column>
              <a-table-column 
                title="MIME Type" 
                :ellipsis="true"
                :width="100"
                :sortable="{
                  sortDirections: ['ascend', 'descend'],
                  sorter: (a: any, b: any) => {
                    const mimeA = getMimeType(a as RequestRecord).fullType;
                    const mimeB = getMimeType(b as RequestRecord).fullType;
                    return mimeA.localeCompare(mimeB);
                  }
                }">
                <template #cell="{ record }">
                  <a-tag v-if="getMimeType(record).value !== '-'" :color="getMimeType(record).color" size="small">
                    {{ getMimeType(record).value }}
                  </a-tag>
                  <span v-else>-</span>
                </template>
              </a-table-column>
              <a-table-column 
                :title="$t('proxy.request.size')" 
                data-index="size" 
                :width="100"
                :ellipsis="true"
                :sortable="{
                  sortDirections: ['ascend', 'descend'],
                  sorter: (a: any, b: any) => {
                    // 提取数值部分并转换为字节
                    const getSizeInBytes = (sizeStr: string): number => {
                      const match = sizeStr.match(/^([\d.]+)\s*(\w+)$/);
                      if (!match) return 0;
                      
                      const [, size, unit] = match;
                      const numSize = parseFloat(size);
                      
                      switch (unit) {
                        case 'B': return numSize;
                        case 'KB': return numSize * 1024;
                        case 'MB': return numSize * 1024 * 1024;
                        case 'GB': return numSize * 1024 * 1024 * 1024;
                        default: return 0;
                      }
                    };
                    
                    return getSizeInBytes(a.size) - getSizeInBytes(b.size);
                  }
                }" 
              />
              <a-table-column 
                :title="$t('proxy.request.time')" 
                data-index="time" 
                :ellipsis="true"
                :sortable="{
                  sortDirections: ['ascend', 'descend'],
                  sorter: (a: any, b: any) => {
                    // 按照timestamp排序，而不是格式化后的时间字符串
                    return a.timestamp - b.timestamp;
                  }
                }" 
              />
            </template>
          </a-table>
        </div>
      </template>
      
      <!-- 下方详情区域：左右分割 -->
      <template #second>
        <div v-if="currentRequest" class="details-container">
          <a-split class="split-details" :default-size="0.5" @moveend="handleMoveSplit">
            <!-- 左侧请求详情 -->
            <template #first>
              <div class="request-panel">
                <div class="panel-header">
                  <h3>{{ $t('proxy.request.title') }}</h3>
                  <a-radio-group v-model="requestViewMode" size="small" type="button">
                    <a-radio value="raw">Raw</a-radio>
                    <a-radio value="hex">Hex</a-radio>
                  </a-radio-group>
                </div>
                
                <!-- 请求内容 -->
                <div class="request-content">
                  <!-- Raw 模式 -->
                  <div v-if="requestViewMode === 'raw'" class="body-content">
                    <Codemirror
                      v-model="requestContent"
                      :extensions="[requestExtension]"
                      :indent-with-tab="false"
                      :tab-size="2"
                      :style="{ height: '100%', width: '100%' }"
                      readonly
                    />
                  </div>
                  
                  <!-- Hex 模式 -->
                  <div v-if="requestViewMode === 'hex'" class="hex-view">
                    <div v-for="(chunk, index) in getHexView(formatRawRequest(currentRequest))" :key="index" class="hex-line">
                      <span class="hex-offset">{{ formatOffset(index * 16) }}</span>
                      <span class="hex-bytes">{{ chunk.hex }}</span>
                      <span class="hex-ascii">{{ chunk.ascii }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>
            
            <!-- 右侧响应详情 -->
            <template #second>
              <div class="response-panel">
                <div class="panel-header">
                  <h3>
                    {{ $t('proxy.response.title') }}
                    <a-tag :color="getStatusColor(currentRequest.status)">
                      {{ currentRequest.status }}
                    </a-tag>
                  </h3>
                  <a-radio-group v-model="responseViewMode" size="small" type="button">
                    <a-radio value="pretty">Pretty</a-radio>
                    <a-radio value="raw">Raw</a-radio>
                    <a-radio value="hex">Hex</a-radio>
                    <a-radio value="render">Render</a-radio>
                  </a-radio-group>
                </div>
                
                <!-- 响应内容 -->
                <div class="response-content">
                  <!-- Raw 模式 -->
                  <div v-if="responseViewMode === 'raw'" class="body-content">
                    <Codemirror
                      v-model="responseContent"
                      :extensions="[responseExtension]"
                      :indent-with-tab="false"
                      :tab-size="2"
                      :style="{ height: '100%', width: '100%' }"
                      readonly
                    />
                  </div>
                  
                  <!-- Hex 模式 -->
                  <div v-if="responseViewMode === 'hex'" class="hex-view">
                    <div v-for="(chunk, index) in getHexView(formatRawResponse(currentRequest))" :key="index" class="hex-line">
                      <span class="hex-offset">{{ formatOffset(index * 16) }}</span>
                      <span class="hex-bytes">{{ chunk.hex }}</span>
                      <span class="hex-ascii">{{ chunk.ascii }}</span>
                    </div>
                  </div>
                  
                  <!-- Pretty 模式 (自动格式化 JSON/XML/HTML) -->
                  <div v-if="responseViewMode === 'pretty'" class="pretty-view">
                    <Codemirror
                      v-model="prettyResponseWithHeader"
                      :extensions="[isJson ? jsonExtension : isXml ? xmlExtension : isHtml ? htmlExtension : textExtension]"
                      :indent-with-tab="false"
                      :tab-size="2"
                      :style="{ height: '100%', width: '100%' }"
                      readonly
                    />
                  </div>
                  
                  <!-- Render 模式 (用于 HTML) -->
                  <div v-if="responseViewMode === 'render'" class="render-view">
                    <iframe v-if="hasHtmlContent" class="render-frame" :srcdoc="currentRequest.response_body"></iframe>
                    <div v-else class="render-fallback">
                      {{ $t('proxy.response.not_renderable') }}
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </a-split>
          
          <!-- 请求操作按钮 -->
          <div class="action-buttons">
            <a-space>
              <a-button type="primary" @click="sendToRepeater">
                {{ $t('menu.repeater') }}
              </a-button>
            </a-space>
          </div>
        </div>
        <div v-else class="no-selection">
          {{ $t('proxy.history.select_request') }}
        </div>
      </template>
    </a-split>
  </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import dayjs from 'dayjs';
import { useI18n } from 'vue-i18n';
import { Codemirror } from 'vue-codemirror';
import { json } from '@codemirror/lang-json';
import { xml } from '@codemirror/lang-xml';
import { html } from '@codemirror/lang-html';
import { EditorView } from '@codemirror/view';

const { t } = useI18n();

interface RequestRecord {
  id: number;
  method: string;
  host: string;
  path: string;
  status: number;
  size: string;
  time: string;
  timestamp: number;
  request_headers: Record<string, string>;
  request_body: string;
  response_headers: Record<string, string>;
  response_body: string;
  url: string;
}

const searchKeyword = ref('');
const filterMethod = ref('');
const filterStatus = ref('');
const requests = ref<RequestRecord[]>([]);
const currentRequest = ref<RequestRecord | null>(null);
let unlisten: (() => void) | null = null;

// 视图模式
const requestViewMode = ref<'raw' | 'hex'>('raw');
const responseViewMode = ref<'raw' | 'hex' | 'pretty' | 'render'>('pretty');

// 请求和响应内容
const requestContent = ref('');
const responseContent = ref('');

// CodeMirror 扩展
const baseTheme = EditorView.theme({
  "&": {
    height: "100%",
  },
  ".cm-scroller": {
    overflow: "auto",
    fontFamily: "monospace"
  },
  ".cm-content": {
    padding: "8px"
  }
});

const requestExtension = computed(() => [baseTheme]);
const responseExtension = computed(() => [baseTheme]);
const jsonExtension = computed(() => [json(), baseTheme]);
const xmlExtension = computed(() => [xml(), baseTheme]);
const htmlExtension = computed(() => [html(), baseTheme]);




const textExtension = computed(() => [baseTheme]);

// 判断内容类型
const contentType = computed(() => {
  if (!currentRequest.value) return '';
  
  const headers = currentRequest.value.response_headers || {};
  return (
    headers['content-type'] || 
    headers['Content-Type'] || 
    ''
  ).toLowerCase();
});

const isJson = computed(() => {
  if (!currentRequest.value?.response_body) return false;
  const body = currentRequest.value.response_body.trim();
  
  // 检查 Content-Type
  if (contentType.value.includes('application/json')) return true;
  
  // 检查内容是否类似 JSON
  return (body.startsWith('{') && body.endsWith('}')) || 
         (body.startsWith('[') && body.endsWith(']'));
});

const isXml = computed(() => {
  if (!currentRequest.value?.response_body) return false;
  const body = currentRequest.value.response_body.trim();
  
  // 检查 Content-Type
  if (contentType.value.includes('application/xml') || 
      contentType.value.includes('text/xml')) return true;
  
  // 检查内容是否类似 XML
  return body.startsWith('<?xml') || 
         (body.startsWith('<') && body.endsWith('>'));
});

const isHtml = computed(() => {
  if (!currentRequest.value?.response_body) return false;
  
  // 检查 Content-Type
  return contentType.value.includes('text/html');
});


const hasHtmlContent = computed(() => {
  if (!currentRequest.value?.response_body) return false;
  
  // 检查 Content-Type
  if (contentType.value.includes('text/html')) return true;
  
  // 检查内容是否包含 HTML 标签
  const body = currentRequest.value.response_body.trim();
  return body.includes('<html') || body.includes('<body') || body.includes('<div');
});

// 格式化 XML (简单实现)
function formatXml(xml: string) {
  let formatted = '';
  let indent = '';
  const tab = '  '; // 2个空格的缩进
  
  xml.split(/>\s*</).forEach(function(node) {
    if (node.match(/^\/\w/)) { // 结束标签
      indent = indent.substring(tab.length);
    }
    
    formatted += indent + '<' + node + '>\n';
    
    if (node.match(/^<?\w[^>]*[^\/]$/) && !node.startsWith("?")) { // 开始标签，不是自闭合的
      indent += tab;
    }
  });
  
  return formatted.substring(1, formatted.length - 2);
}

// 获取十六进制视图数据
function getHexView(content: string) {
  if (!content) return [];
  
  const chunks = [];
  const bytes = new TextEncoder().encode(content);
  
  // 将字节数组分成16字节一组
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hexValues = Array.from(chunk).map(b => b.toString(16).padStart(2, '0')).join(' ');
    
    // 生成ASCII表示
    const asciiValues = Array.from(chunk).map(b => {
      if (b >= 32 && b <= 126) { // 可打印字符
        return String.fromCharCode(b);
      }
      return '.'; // 不可打印字符用点表示
    }).join('');
    
    chunks.push({
      hex: hexValues.padEnd(48, ' '), // 确保16个字节的显示空间
      ascii: asciiValues
    });
  }
  
  return chunks;
}

// 格式化偏移量
function formatOffset(offset: number) {
  return offset.toString(16).padStart(8, '0');
}

onMounted(async () => {
  // 加载历史记录
  try {
    const history = await invoke('get_proxy_history');
    requests.value = (history as RequestRecord[]) || [];
  } catch (error) {
    console.error('获取历史记录失败:', error);
  }
  
  // 监听新请求
  const unlistenRequest = await listen('proxy-request-received', (event) => {
    const request = event.payload as RequestRecord;
    
    console.log('收到新请求:', request);
    // 添加到历史记录顶部
    requests.value.unshift({
      ...request,
      time: formatTime(request.timestamp),
      size: formatSize(request.response_body ? request.response_body.length : 0),
    });
  });
  
  // 监听请求更新 (用于拦截修改的请求)
  const unlistenUpdated = await listen('proxy-request-updated', (event) => {
    const updatedRequest = event.payload as RequestRecord;
    
    // 更新已存在的请求
    const index = requests.value.findIndex(req => req.id === updatedRequest.id);
    if (index !== -1) {
      console.log('更新已存在的请求, 原URL:', requests.value[index].url, '新URL:', updatedRequest.url);
      
      // 更新已有请求，但保留格式化的时间和大小
      requests.value[index] = {
        ...updatedRequest,
        time: formatTime(updatedRequest.timestamp),
        size: formatSize(updatedRequest.response_body ? updatedRequest.response_body.length : 0),
      };
      
      // 如果当前显示的是这个请求，则更新它
      if (currentRequest.value && currentRequest.value.id === updatedRequest.id) {
        currentRequest.value = requests.value[index];
      }
    } else {
      console.warn('收到未找到的请求更新:', updatedRequest.id);
    }
  });
  
  // 监听请求完成
  const unlistenCompleted = await listen('proxy-request-completed', (event) => {
    const completedRequest = event.payload as RequestRecord;
    
    // 更新已存在的请求
    const index = requests.value.findIndex(req => req.id === completedRequest.id);
    if (index !== -1) {
      // 更新已有请求
      requests.value[index] = {
        ...completedRequest,
        time: formatTime(completedRequest.timestamp),
        size: formatSize(completedRequest.response_body ? completedRequest.response_body.length : 0),
      };
      
      // 如果当前显示的是这个请求，则更新它
      if (currentRequest.value && currentRequest.value.id === completedRequest.id) {
        currentRequest.value = requests.value[index];
      }
    } else {
      // 如果不存在则添加
      requests.value.unshift({
        ...completedRequest,
        time: formatTime(completedRequest.timestamp),
        size: formatSize(completedRequest.response_body ? completedRequest.response_body.length : 0),
      });
    }
  });
  
  // 合并清理函数
  unlisten = () => {
    unlistenRequest();
    unlistenUpdated();
    unlistenCompleted();
  };
});

onUnmounted(async () => {
  // 清理工作
  if (unlisten) {
    unlisten();
  }
});

// 根据过滤条件筛选请求
const filteredRequests = computed(() => {
  let result = [...requests.value];
  
  // 关键字搜索
  if (searchKeyword.value) {
    const keyword = searchKeyword.value.toLowerCase();
    result = result.filter(
      (item) =>
        item.host.toLowerCase().includes(keyword) ||
        item.path.toLowerCase().includes(keyword)
    );
  }
  
  // 方法过滤
  if (filterMethod.value) {
    result = result.filter((item) => item.method === filterMethod.value);
  }
  
  // 状态码过滤
  if (filterStatus.value) {
    switch (filterStatus.value) {
      case 'success':
        result = result.filter((item) => item.status >= 200 && item.status < 300);
        break;
      case 'redirect':
        result = result.filter((item) => item.status >= 300 && item.status < 400);
        break;
      case 'error':
        result = result.filter((item) => item.status >= 400);
        break;
    }
  }
  
  return result;
});

// 清空历史记录
const clearHistory = () => {
  Modal.confirm({
    title: t('proxy.history.clear_confirm_title') as string,
    content: t('proxy.history.clear_confirm_content') as string,
    onOk: async () => {
      try {
        await invoke('clear_proxy_history');
        requests.value = [];
        currentRequest.value = null;
        Message.success(t('proxy.history.clear_success') as string);
      } catch (error) {
        Message.error(`${t('proxy.history.clear_error')}: ${error}`);
      }
    },
  });
};

// 监听分割条移动
const handleMoveSplit = (newSize: number) => {
  console.log('Split size changed:', newSize);
};

// 查看请求详情
const viewRequestDetail = (record: any) => {
  currentRequest.value = record as RequestRecord;
};

// 发送到重放器
const sendToRepeater = async () => {
  if (!currentRequest.value) return;
  
  try {
    await invoke('send_to_repeater', {
      request: currentRequest.value
    });
    
    Message.success(t('proxy.history.sent_to_repeater') as string);
  } catch (error) {
    Message.error(`${t('proxy.history.send_error')}: ${error}`);
  }
};

// 搜索处理
const handleSearch = () => {
  // 搜索功能通过计算属性 filteredRequests 自动处理
};

// 获取HTTP方法对应的颜色
const getMethodColor = (method: string) => {
  const methodColors: Record<string, string> = {
    GET: 'blue',
    POST: 'green',
    PUT: 'orange',
    DELETE: 'red',
    PATCH: 'purple',
    OPTIONS: 'gray',
    HEAD: 'gray',
  };
  
  return methodColors[method] || 'default';
};

// 获取状态码对应的颜色
const getStatusColor = (status: number) => {
  if (status >= 200 && status < 300) return 'green';
  if (status >= 300 && status < 400) return 'blue';
  if (status >= 400 && status < 500) return 'orange';
  if (status >= 500) return 'red';
  return 'default';
};

// 格式化时间戳
const formatTime = (timestamp: number) => {
  return dayjs(timestamp).format('YYYY-MM-DD HH:mm:ss');
};

// 格式化大小
const formatSize = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

// 获取MIME类型
const getMimeType = (record: RequestRecord): { value: string; color: string; fullType: string } => {
  if (!record.response_headers) return { value: '-', color: 'gray', fullType: '-' };
  
  // 从响应头中查找Content-Type
  const contentType = record.response_headers['content-type'] || 
                     record.response_headers['Content-Type'] || 
                     '';
  
  // 提取MIME类型（去掉charset等参数）
  const mimeMatch = contentType.match(/^([^;]+)/);
  let mimeType = mimeMatch ? mimeMatch[1].trim() : '-';
  
  // 提取MIME类型的简短部分
  let shortType = '-';
  let color = 'gray';
  
  if (mimeType !== '-') {
    // 分割MIME类型，取最后一部分
    const parts = mimeType.split('/');
    if (parts.length > 1) {
      shortType = parts[1];
      // 处理如application/json, text/javascript等格式
      if (shortType.includes('+')) {
        shortType = shortType.split('+')[1];
      }
    } else {
      shortType = mimeType;
    }

    // 处理特定类型，使用更简短的标识符
    if (shortType.includes('json')) {
      shortType = 'json';
      color = 'blue';
    } else if (shortType.includes('html')) {
      shortType = 'html';
      color = 'green';
    } else if (shortType.includes('xml')) {
      shortType = 'xml';
      color = 'orange';
    } else if (shortType.includes('javascript')) {
      shortType = 'js';
      color = 'purple';
    } else if (shortType.includes('css')) {
      shortType = 'css';
      color = 'magenta';
    } else if (shortType.includes('csv')) {
      shortType = 'csv';
      color = 'blue-green';
    } else if (shortType.includes('pdf')) {
      shortType = 'pdf';
      color = 'red';
    } else if (shortType === 'plain') {
      shortType = 'text';
      color = 'lime';
    } else if (shortType === 'octet-stream') {
      shortType = 'bin';
      color = 'gray';
    } else if (shortType === 'form-data' || shortType === 'x-www-form-urlencoded') {
      shortType = 'form';
      color = 'gold';
    } else if (shortType.includes('zip') || shortType.includes('tar') || shortType.includes('gzip')) {
      shortType = 'zip';
      color = 'orange';
    } else if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'tiff'].some(ext => shortType.includes(ext))) {
      // 提取图片格式
      const imgType = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'tiff'].find(ext => shortType.includes(ext));
      shortType = imgType || 'img';
      color = 'cyan';
    } else if (['mp4', 'mpeg', 'webm', 'ogg', 'mp3', 'wav', 'avi'].some(ext => shortType.includes(ext))) {
      // 提取媒体格式
      const mediaType = ['mp4', 'mpeg', 'webm', 'ogg', 'mp3', 'wav', 'avi'].find(ext => shortType.includes(ext));
      shortType = mediaType || 'media';
      color = 'purple';
    } else if (mimeType.startsWith('text/')) {
      shortType = 'text';
      color = 'lime';
    } else {
      color = 'red'; // 其他类型
    }
  }
  
  return { value: shortType, color, fullType: mimeType };
};

// 格式化原始请求
const formatRawRequest = (request: RequestRecord | null): string => {
  if (!request) return '';

  // 解析URL，获取 path+search
  let fullPath = '';
  try {
    const u = new URL(request.url, `http://${request.host}/`);
    fullPath = u.pathname + (u.search || '');
  } catch {
    fullPath = request.path || request.url;
  }

  // 请求行（只显示 path+参数）
  let formattedRequest = `${request.method} ${fullPath} HTTP/1.1\n`;

  // Host头
  formattedRequest += `Host: ${request.host}\n`;

  // 其他请求头
  for (const [key, value] of Object.entries(request.request_headers)) {
    if (key.toLowerCase() !== 'host') {
      formattedRequest += `${key}: ${value}\n`;
    }
  }

  // 请求体
  if (request.request_body && request.request_body.trim()) {
    formattedRequest += `\n${request.request_body}`;
  }

  return formattedRequest;
};

// 格式化原始响应
const formatRawResponse = (request: RequestRecord | null): string => {
  if (!request) return '';
  
  // 状态行（根据实际情况调整）
  let result = `HTTP/1.1 ${request.status} ${getStatusText(request.status)}\n`;
  
  // 响应头
  const headers = request.response_headers || {};
  Object.entries(headers).forEach(([key, value]) => {
    result += `${key}: ${value}\n`;
  });
  
  // 空行分隔头部和主体
  result += '\n';
  
  // 响应体
  if (request.response_body) {
    result += request.response_body;
  }
  
  return result;
};

/**
 * 根据状态码获取状态文本
 */
function getStatusText(status: number): string {
  const statusMap: Record<number, string> = {
    200: 'OK',
    201: 'Created',
    204: 'No Content',
    301: 'Moved Permanently',
    302: 'Found',
    304: 'Not Modified',
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    405: 'Method Not Allowed',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
    504: 'Gateway Timeout'
  };
  
  return statusMap[status] || 'Unknown';
}


// 监听当前请求变化，更新CodeMirror内容
watch(() => currentRequest.value, (newRequest) => {
  if (newRequest) {
    requestContent.value = formatRawRequest(newRequest);
    responseContent.value = formatRawResponse(newRequest);
  } else {
    requestContent.value = '';
    responseContent.value = '';
  }
}, { immediate: true });

// 获取完整Host（带协议）
function getFullHost(record: RequestRecord): string {
  // 假设有 scheme 字段，否则根据端口推断
  let scheme = (record as any).scheme;
  if (!scheme) {
    scheme = record.url.startsWith('https://') || record.host.endsWith(':443') ? 'https' : 'http';
  }
  return `${scheme}://${record.host.replace(/^https?:\/\//, '')}`;
}

// 获取完整URL（path+参数）
function getFullUrl(record: RequestRecord): string {
  // record.url 通常是完整URL，去掉host部分
  try {
    const u = new URL(record.url, `${getFullHost(record)}/`);
    return u.pathname + (u.search || '');
  } catch {
    // 兜底
    return record.path || record.url;
  }
}

const prettyResponseWithHeader = computed(() => {
  if (!currentRequest.value) return '';
  // 状态行
  let result = `HTTP/1.1 ${currentRequest.value.status} ${getStatusText(currentRequest.value.status)}\n`;
  // 响应头
  const headers = currentRequest.value.response_headers || {};
  Object.entries(headers).forEach(([key, value]) => {
    result += `${key}: ${value}\n`;
  });
  result += '\n';
  // 格式化body
  if (isJson.value) {
    try {
      const obj = JSON.parse(currentRequest.value.response_body);
      result += JSON.stringify(obj, null, 2);
    } catch {
      result += currentRequest.value.response_body;
    }
  } else if (isXml.value) {
    result += formatXml(currentRequest.value.response_body);
  } else {
    result += currentRequest.value.response_body || '';
  }
  return result;
});
</script>

<style scoped lang="less">
.history-container {
  height: 100%;
  font-size: 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.history-header {
  margin-bottom: 5px;
  flex-shrink: 0;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  margin-bottom: 5px;
}

.split-container {
  flex: 1;
  overflow: hidden;
  min-height: 0; /* 确保Flex布局正确计算高度 */
}

.history-list {
  height: 100%;
  overflow: hidden; /* 让表格自己管理滚动 */
}

.details-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}

.split-details {
  height: calc(100% - 40px);
  width: 100%;
  min-height: 0; /* 确保Flex布局正确计算高度 */
}

/* 添加针对拖动手柄的样式 */
:deep(.arco-split-trigger) {
  background-color: var(--color-fill-2);
  transition: background-color 0.2s;
}

:deep(.arco-split-trigger:hover) {
  background-color: var(--color-fill-3);
}

:deep(.arco-split-trigger-horizontal),
:deep(.arco-split-trigger-vertical) {
  z-index: 10;
}

.request-panel,
.response-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 8px;
  overflow: hidden;
  min-height: 0; /* 确保Flex布局正确计算高度 */
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  
  h3 {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
}

.request-content,
.response-content {
  flex: 1;
  overflow: auto;
  background-color: var(--color-fill-2);
  border-radius: 4px;
  position: relative;
  height: calc(100% - 40px);
  display: flex;
  flex-direction: column;
}

.body-content,
.hex-view,
.pretty-view,
.render-view {
  flex: 1;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: monospace;
  line-height: 1.5;
  padding: 8px;
  margin: 0;
  min-height: 0; /* 确保flex子元素正确缩放 */
}

.hex-line {
  display: flex;
  margin-bottom: 4px;
}

.hex-offset {
  width: 80px;
  color: var(--color-text-3);
}

.hex-bytes {
  flex: 1;
  margin-right: 16px;
}

.hex-ascii {
  width: 16ch;
  background-color: var(--color-fill-3);
  padding: 0 4px;
}

.pretty-json,
.pretty-xml,
.pretty-code {
  white-space: pre-wrap;
  font-family: monospace;
  line-height: 1.5;
  padding: 8px;
  margin: 0;
}

.render-view {
  height: 100%;
  position: relative;
}

.render-frame {
  width: 100%;
  height: 100%;
  border: none;
  background-color: white;
}

.render-fallback {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  color: var(--color-text-3);
}

.action-buttons {
  display: flex;
  justify-content: flex-end;
  padding: 8px;
}

.no-selection {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  color: var(--color-text-3);
}

/* 添加表格单元格溢出处理 */
:deep(.arco-table-td) {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

:deep(.arco-table-th) {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 调整拖拽列宽的样式 */
:deep(.arco-table-resize-button) {
  position: absolute;
  top: 0;
  right: 0;
  width: 10px;
  height: 100%;
  cursor: col-resize;
}

/* 确保拖动列宽时内容不会自动换行 */
:deep(.arco-table-cell) {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 修复拖动列宽与排序按钮冲突的问题 */
:deep(.arco-table-cell-with-sorter) {
  padding-right: 30px !important;
}

:deep(.arco-table-sorter) {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
}

/* 调整CodeMirror相关容器样式 */
.body-content,
.pretty-json,
.pretty-xml,
.pretty-code {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

/* 添加CodeMirror相关样式 */
:deep(.cm-editor) {
  height: 100%;
  width: 100%;
  border-radius: 4px;
  overflow: hidden;
  background-color: var(--color-fill-2);
}

:deep(.cm-scroller) {
  font-family: monospace;
  font-size: 12px;
  line-height: 1.5;
  overflow: auto;
}

:deep(.cm-content) {
  padding: 8px;
}

/* 语法高亮样式 */
:deep(.cm-keyword) {
  color: #c678dd;
}

:deep(.cm-string) {
  color: #98c379;
}

:deep(.cm-number) {
  color: #d19a66;
}

:deep(.cm-property) {
  color: #61afef;
}

:deep(.cm-comment) {
  color: #5c6370;
  font-style: italic;
}

:deep(.cm-tag) {
  color: #e06c75;
}

:deep(.cm-attribute) {
  color: #d19a66;
}

.raw-headers {
  font-family: monospace;
  font-size: 12px;
  color: #888;
  margin-bottom: 8px;
  .header-line {
    line-height: 1.5;
  }
  .header-key {
    color: #555;
    font-weight: bold;
    margin-right: 4px;
  }
  .header-value {
    color: #222;
  }
}
</style> 