<template>
  <div class="repeater-container">
    <div class="top-bar">
      <div class="action-left">
        <a-button type="primary" size="small" @click="sendRequest" :loading="isLoading" :disabled="isLoading">
          {{ isLoading ? $t('repeater.sending') : $t('repeater.send') }}
        </a-button>
        <a-button type="outline" size="small" @click="cancelRequest" :disabled="!isLoading" style="margin-left: 8px;">
          {{ $t('repeater.cancel') }}
          </a-button>
        </div>
      <div class="right-controls">
        <div class="socket-settings">
            <a-input size="small" v-model="targetHost" :placeholder="$t('repeater.target_host')" style="width: 150px;" />
            <a-input-number size="small" v-model="targetPort" :min="1" :max="65535" :placeholder="$t('repeater.port')"
              style="width: 70px; " />
            <a-checkbox v-model="useHttps">
              {{ $t('repeater.https') }}
            </a-checkbox>
        </div>
        <div class="action-buttons">
          <a-button @click="openSettings" size="small">
            <icon-settings />
          </a-button>
      </div>
    </div>
                  </div>
    <a-split :default-size="0.5" min="0.2" max="0.8" class="repeater-split">
      <template #first>
        <a-card class="request-panel" size="small">
          <a-tabs position="top" size="small" class="full-height-tabs">
            <template #extra>
              <a-space size="small">
                <a-tooltip :content="$t('repeater.line_wrapping')">
                  <a-button shape="circle" size="mini" :type="requestLineWrapping ? 'primary' : 'outline'"
                    @click="requestLineWrapping = !requestLineWrapping">
                    <icon-line-height />
                  </a-button>
                </a-tooltip>
              </a-space>
            </template>
            <a-tab-pane key="raw" :title="$t('repeater.raw')">
              <codemirror v-model="rawRequest" :placeholder="$t('repeater.raw_request_placeholder')"
                :style="{ height: 'calc(100vh - 280px)', fontSize: editorFontSize, lineHeight: editorLineHeight }"
                :autofocus="true" :indent-with-tab="true" :tab-size="2" :extensions="requestEditorExtensions"
                @ready="handleReady" class="raw-editor" @paste="handleRequestPaste" />
              </a-tab-pane>
            <a-tab-pane key="pretty" :title="$t('repeater.pretty')">
              <codemirror v-model="prettyRequest" :placeholder="$t('repeater.pretty_request_placeholder')"
                :style="{ height: 'calc(100vh - 280px)', fontSize: editorFontSize, lineHeight: editorLineHeight }"
                :indent-with-tab="true" :tab-size="2" :extensions="requestEditorExtensions" class="pretty-editor" />
              </a-tab-pane>
            <a-tab-pane key="hex" :title="$t('repeater.hex')">
              <div class="hex-view"
                :style="{ height: 'calc(100vh - 280px)', fontSize: editorFontSize, lineHeight: editorLineHeight, overflow: 'auto' }"
                v-if="rawRequest">
                <pre>{{ requestHexView }}</pre>
                </div>
              </a-tab-pane>
            </a-tabs>
        </a-card>
      </template>
      <template #second>
        <a-card size="small" class="response-panel" style="height: 700px;">
          <a-tabs position="top" size="small" class="full-height-tabs">
            <template #extra>
              <a-space size="small">
                <a-tooltip content="自动换行">
                  <a-button shape="circle" size="mini" :type="responseLineWrapping ? 'primary' : 'outline'"
                    @click="responseLineWrapping = !responseLineWrapping">
                    <icon-line-height />
                  </a-button>
                </a-tooltip>
              </a-space>
            </template>
            <a-tab-pane key="1" :title="$t('repeater.pretty')">
              <div class="beautified-response" v-if="response">
                <codemirror v-model="formattedFullResponse" :placeholder="$t('repeater.pretty_response_placeholder')"
                  :style="{ height: 'calc(100vh - 280px)', fontSize: editorFontSize, lineHeight: editorLineHeight }"
                  :extensions="formattedFullResponseExtensions" :readonly="true" class="formatted-editor" />
              </div>
                  </a-tab-pane>
            <a-tab-pane key="2" :title="$t('repeater.response')">
              <div style="display: flex; flex-direction: column; height: 100%; overflow: hidden">
                    <codemirror v-model="rawResponse" :placeholder="$t('repeater.raw_response_placeholder')"
                      :style="{ height: 'calc(100vh - 250px)', fontSize: editorFontSize, lineHeight: editorLineHeight }"
                  :extensions="rawResponseEditorExtensions" :readonly="true" @ready="handleRawResponseEditorReady"
                      class="raw-response-editor" />
              </div>
            </a-tab-pane>
            <a-tab-pane key="3" :title="$t('repeater.render')">
              <div class="rendered-response" v-if="response && responseBody">
                <iframe v-if="isHtmlResponse" :srcdoc="responseBody" style="width: 100%; height: 650px; border: none;"
                  sandbox="allow-same-origin allow-scripts" referrerpolicy="no-referrer"></iframe>
                <pre v-else-if="isJsonResponse" :style="formattedContentStyle">{{ formattedJson }}</pre>
                <pre v-else :style="formattedContentStyle">{{ responseBody }}</pre>
              </div>
            </a-tab-pane>
            <a-tab-pane key="4" :title="$t('repeater.hex')">
              <div class="hex-view"
                :style="{ height: 'calc(100vh - 250px)', fontSize: editorFontSize, lineHeight: editorLineHeight }"
                v-if="response && responseBody">
                <pre>{{ responseHexView }}</pre>
              </div>
            </a-tab-pane>
          </a-tabs>
        </a-card>
      </template>
    </a-split>
    <div class="panel-header">
      <div class="panel-info" v-if="response">
        <a-tag size="small" color="green" v-if="response.status >= 200 && response.status < 300">
          {{ response.status }}
        </a-tag>
        <a-tag size="small" color="orange" v-else-if="response.status >= 300 && response.status < 400">
          {{ response.status }}
        </a-tag>
        <a-tag size="small" color="red" v-else>
          {{ response.status }}
        </a-tag>
        <span>{{ responseTime }}ms</span>
      </div>
    </div>
  </div>
  <RepeaterSettings ref="settingsRef" @confirm="applySettings" />
</template>

<script lang="ts" setup>
import { ref, computed, watch, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';
import RepeaterSettings from './repeater-settings.vue';
import { IconSettings, IconLineHeight } from '@arco-design/web-vue/es/icon';
import { Codemirror } from 'vue-codemirror';
import { json } from '@codemirror/lang-json';
import { html } from '@codemirror/lang-html';
import { xml } from '@codemirror/lang-xml';
import { javascript } from '@codemirror/lang-javascript';
import { css } from '@codemirror/lang-css';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorView } from '@codemirror/view';
import { StreamLanguage } from '@codemirror/language';
import { LanguageSupport } from '@codemirror/language';

// 只保留Socket模式相关变量
const editorFontSize = ref('12px');
const editorLineHeight = ref(1.5);
const rawRequest = ref('');
const prettyRequest = ref('');
const targetHost = ref('');
const targetPort = ref(443);
const useHttps = ref(true);
const response = ref<any>(null);
const responseBody = ref('');
const rawResponse = ref('');
const responseTime = ref(0);
const isDarkMode = ref(false);
const isLoading = ref(false);
const abortController = ref<AbortController | null>(null);

// 请求编辑器设置
const requestLineWrapping = ref(true);

// 响应编辑器设置
const responseLineWrapping = ref(true);

// 添加双向同步功能，让原始和美化tabs之间的内容能够相互同步
const isSyncingFromPretty = ref(false);

// 创建基础的HTTP头部高亮
const createHttpHeaderHighlighter = () => {
  const parser = {
    token(stream: any, state: any) {
      // 如果已经进入body区域，不处理
      if (state.inBody) {
        stream.skipToEnd();
        return null;
      }

      // 检测空行（HTTP头与正文的分隔）
      if (stream.sol() && stream.match(/^$/)) {
        state.inBody = true;
        return null;
      }
      
      // HTTP请求行或响应行高亮
      if (stream.sol() && stream.match(/^(GET|POST|PUT|DELETE|PATCH|OPTIONS|HEAD|CONNECT|TRACE|HTTP\/[0-9.]+) /)) {
        return "keyword";
      }
      if (stream.sol() && stream.match(/^HTTP\/[0-9.]+ [0-9]+ /)) {
        return "keyword";
      }
      
      // HTTP头部名称高亮
      if (stream.sol() && stream.match(/^[A-Za-z0-9-]+:/)) {
        return "def";
      }
      
      stream.skipToEnd();
      return null;
    },
    startState() {
      return {
        inBody: false
      };
    }
  };
  
  return StreamLanguage.define(parser);
};

// 请求编辑器设置
const requestEditorExtensions = computed(() => {
  const base = [isDarkMode.value ? oneDark : []];
  
  // 添加HTTP头部高亮
  base.push(new LanguageSupport(createHttpHeaderHighlighter()));
  
  // 直接添加JSON高亮，CodeMirror会自动在合适的区域应用
  base.push(json());
  
  if (requestLineWrapping.value) {
    base.push(EditorView.lineWrapping);
  }
  return base;
});

const rawResponseEditorExtensions = computed(() => {
  const base = [isDarkMode.value ? oneDark : []];
  
  // 添加HTTP头部高亮  
  base.push(new LanguageSupport(createHttpHeaderHighlighter()));
  
  // 直接添加JSON高亮，CodeMirror会自动在合适的区域应用
  base.push(json());
  
  if (responseLineWrapping.value) {
    base.push(EditorView.lineWrapping);
  }
  return base;
});

const formattedFullResponseExtensions = computed(() => {
  const base = [isDarkMode.value ? oneDark : []];
  
  // 添加HTTP头部高亮
  base.push(new LanguageSupport(createHttpHeaderHighlighter()));
  
  // 根据内容类型添加特定语言支持
  const contentType = response.value?.headers?.['content-type']?.toLowerCase() || '';
  
  // 直接添加JSON高亮，CodeMirror会自动在合适的区域应用
  base.push(json());
  
  // 添加其他类型的高亮支持
  if (contentType.includes('html')) {
    base.push(html());
  } else if (contentType.includes('xml')) {
    base.push(xml());
  } else if (contentType.includes('javascript')) {
    base.push(javascript());
  } else if (contentType.includes('css')) {
    base.push(css());
  }
  
  if (responseLineWrapping.value) {
    base.push(EditorView.lineWrapping);
  }
  
  return base;
});

const responseType = computed(() => {
  const contentType = response.value?.headers?.['content-type'] || '';
  if (contentType.includes('application/json')) return 'json';
  if (contentType.includes('text/html')) return 'html';
  if (contentType.includes('text/xml') || contentType.includes('application/xml')) return 'xml';
  if (contentType.includes('application/javascript') || contentType.includes('text/javascript')) return 'javascript';
  if (contentType.includes('text/css')) return 'css';
  return 'text';
});

const isHtmlResponse = computed(() => responseType.value === 'html');
const isJsonResponse = computed(() => responseType.value === 'json');

const formattedJson = computed(() => {
  if (!responseBody.value || !isJsonResponse.value) return '';
  try {
    const jsonObj = JSON.parse(responseBody.value);
    return JSON.stringify(jsonObj, null, 2);
  } catch (e) {
    return responseBody.value;
  }
});

const formattedContentStyle = computed(() => ({
  fontSize: editorFontSize.value,
  lineHeight: editorLineHeight.value,
  fontFamily: "Maple Mono NF CN, Menlo, Consolas, Maple UI, PingFang, 'Microsoft YaHei', monospace",
  margin: 0,
  overflow: 'auto',
  height: 'auto',
  maxHeight: '500px'
}));

const handleReady = (_view: any) => { };
const handleRawResponseEditorReady = (_payload: { view: EditorView; state: any; container: HTMLDivElement }) => { };
const settingsRef = ref<InstanceType<typeof RepeaterSettings> | null>(null);

const applySettings = (settings: any) => {
  editorFontSize.value = `${settings.fontSize}px`;
  editorLineHeight.value = settings.lineHeight;
};

const openSettings = () => {
  if (settingsRef.value) {
    settingsRef.value.openModal();
  }
};

const sendRequest = async () => {
  if (isLoading.value) return;
  response.value = null;
  responseBody.value = '';
  rawResponse.value = '';
  responseTime.value = 0;
  abortController.value = new AbortController();
  isLoading.value = true;
        await nextTick();
  try {
  if (!targetHost.value) {
    Message.error('请输入目标主机');
      isLoading.value = false; return;
  }
  if (!targetPort.value || targetPort.value < 1 || targetPort.value > 65535) {
    Message.error('请输入有效的端口号(1-65535)');
      isLoading.value = false; return;
  }
  if (!rawRequest.value) {
    Message.error('请输入原始HTTP请求');
      isLoading.value = false; return;
  }
    const startTime = Date.now();
    if (!isLoading.value || !abortController.value) return;
    const result: any = await invoke('repeater_send_request', {
      method: 'GET',
      url: `${useHttps.value ? 'https' : 'http'}://${targetHost.value}:${targetPort.value}/`,
      headers: {},
      body: '',
      useSocket: true,
      targetHost: targetHost.value,
      targetPort: targetPort.value,
      useHttps: useHttps.value,
      rawRequest: rawRequest.value,
    });
    if (!isLoading.value || !abortController.value) return;
    await nextTick();
    responseTime.value = Date.now() - startTime;
    if (result && result.body) {
    responseBody.value = result.body;
    }
    response.value = result;
    console.log('Response received:', result);
    console.log('Content-Type:', result?.headers?.['content-type']);
    console.log('Response body sample:', result?.body?.substring(0, 100));
  } catch (error: any) {
    if (isLoading.value) {
      console.error('Request failed:', error);
    Message.error(`请求失败: ${error}`);
    response.value = null;
    responseBody.value = `Error: ${error}`;
    }
  } finally {
    isLoading.value = false;
    abortController.value = null;
  }
};

const cancelRequest = () => {
  if (isLoading.value) {
    console.log('Cancelling request...');
    abortController.value = null;
    isLoading.value = false;
    Message.info('请求已取消');
  }
};

const buildRawResponse = () => {
  if (!response.value) return '';
  let raw = `HTTP/1.1 ${response.value.status}\n`;
  Object.entries(response.value.headers).forEach(([name, value]) => {
    raw += `${name}: ${value}\n`;
  });
  raw += `\n${response.value.body ?? ''}`;
  return raw;
};

watch(response, (newValue) => {
  if (newValue) {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        rawResponse.value = buildRawResponse();
      });
    });
  } else {
    rawResponse.value = '';
  }
});

const requestHexView = computed(() => {
  if (!rawRequest.value) return '';
  return stringToHex(rawRequest.value);
});

function stringToHex(str: string): string {
  let result = '';
  let ascii = '';
  let hexRow = '';
  for (let i = 0; i < str.length; i++) {
    const charCode = str.charCodeAt(i);
    const hex = charCode.toString(16).padStart(2, '0');
    hexRow += hex + ' ';
    ascii += (charCode >= 32 && charCode <= 126) ? str[i] : '.';
    if ((i + 1) % 16 === 0 || i === str.length - 1) {
      result += hexRow.padEnd(48, ' ') + ' | ' + ascii + '\n';
      hexRow = '';
      ascii = '';
    }
  }
  return result;
}

// 修改原有的从rawRequest到prettyRequest的监听器，增加同步状态检查
watch(rawRequest, (newValue) => {
  // 如果是从prettyRequest同步过来的，不再触发更新
  if (isSyncingFromPretty.value) return;
  
  if (newValue) {
    try {
      const lines = newValue.split('\n');
      let formatted = '';
      let inBody = false;
      for (const line of lines) {
        if (!inBody) {
          if (line.trim() === '') {
            inBody = true;
            formatted += '\n';
            continue;
          }
          if (!formatted) {
            formatted += line + '\n';
          } else {
            const colonIndex = line.indexOf(':');
            if (colonIndex > 0) {
              const headerName = line.substring(0, colonIndex).trim();
              const headerValue = line.substring(colonIndex + 1).trim();
              if (headerName.toLowerCase() === 'host') {
                targetHost.value = headerValue;
              }
              formatted += `${headerName}: ${headerValue}\n`;
            } else {
              formatted += line + '\n';
            }
          }
        } else {
          try {
            const bodyContent = lines.slice(lines.indexOf('') + 1).join('\n');
            const jsonBody = JSON.parse(bodyContent);
            formatted += JSON.stringify(jsonBody, null, 2);
            break;
          } catch (e) {
            formatted += line + '\n';
          }
        }
      }
      prettyRequest.value = formatted;
    } catch (e) {
      prettyRequest.value = newValue;
    }
  } else {
    prettyRequest.value = '';
  }
});

// 修改prettyRequest的监听器，处理内容被完全删除的情况
watch(prettyRequest, (newValue) => {
  try {
    // 标记正在从prettyRequest同步到rawRequest，避免循环更新
    isSyncingFromPretty.value = true;
    
    // 如果美化内容被完全删除，也删除原始内容
    if (!newValue || newValue.trim() === '') {
      rawRequest.value = '';
      return;
    }
    
    const lines = newValue.split('\n');
    let rawOutput = '';
    let headerSection = true;
    let jsonBody = null;
    
    // 尝试解析美化后的JSON请求体
    const emptyLineIndex = lines.findIndex(line => line.trim() === '');
    if (emptyLineIndex !== -1 && emptyLineIndex < lines.length - 1) {
      try {
        const bodyLines = lines.slice(emptyLineIndex + 1);
        const bodyText = bodyLines.join('\n');
        if (bodyText.trim().startsWith('{') || bodyText.trim().startsWith('[')) {
          jsonBody = JSON.parse(bodyText);
        }
      } catch (e) {
        console.log('Not a valid JSON body');
      }
    }
    
    // 重构原始请求
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      
      if (line.trim() === '' && headerSection) {
        headerSection = false;
        rawOutput += '\n';
        
        // 如果有解析到的JSON请求体，添加压缩后的JSON
        if (jsonBody) {
          rawOutput += JSON.stringify(jsonBody);
          break; // JSON已添加，跳过后面的行
        }
        continue;
      }
      
      if (headerSection) {
        // 保留头部格式但去除多余缩进
        rawOutput += line.trim() + '\n';
      } else if (!jsonBody) {
        // 如果没有解析到JSON，直接添加原始行
        rawOutput += line + '\n';
      }
    }
    
    // 更新rawRequest，但避免操作时自动触发更新prettyRequest
    rawRequest.value = rawOutput.trim();
  } catch (e) {
    console.error('Error syncing from pretty to raw:', e);
  } finally {
    // 重置同步标记
    setTimeout(() => {
      isSyncingFromPretty.value = false;
    }, 0);
  }
});

const responseHexView = computed(() => {
  if (!responseBody.value) return '';
  return stringToHex(responseBody.value);
});

const handleRequestPaste = (event: ClipboardEvent) => {
  const clipboardData = event.clipboardData;
  if (clipboardData) {
    const pastedText = clipboardData.getData('text');
    if (pastedText) {
      const lines = pastedText.split('\n');
      for (const line of lines) {
        if (line.toLowerCase().includes('host:')) {
          const hostMatch = line.match(/host: (.+)/i);
          if (hostMatch) {
            targetHost.value = hostMatch[1].trim();
          }
        }
      }
    }
  }
};

const formattedFullResponse = computed(() => {
  if (!response.value) return '';
  let formatted = `HTTP/1.1 ${response.value.status}\n`;
  Object.entries(response.value.headers).forEach(([name, value]) => {
    formatted += `${name}: ${value}\n`;
  });
  formatted += '\n';
  if (responseBody.value) {
    const contentType = response.value?.headers?.['content-type'] || '';
    console.log('Content-Type:', contentType);
    console.log('Response body length:', responseBody.value.length);
    if (contentType.toLowerCase().includes('application/json') ||
      contentType.toLowerCase().includes('json') ||
      isValidJson(responseBody.value)) {
      try {
        console.log('Formatting as JSON');
        const jsonObj = JSON.parse(responseBody.value);
        formatted += JSON.stringify(jsonObj, null, 2);
        console.log('JSON formatting successful');
      } catch (e) {
        console.error('JSON parse error:', e);
        formatted += responseBody.value;
      }
    } else if (contentType.toLowerCase().includes('xml') || responseBody.value.trim().startsWith('<?xml')) {
      try {
        console.log('Formatting as XML');
        let xmlFormatted = '';
        let indent = 0;
        const lines = responseBody.value.replace(/></g, '>\n<').split('\n');
        for (const line of lines) {
          if (line.match(/<\/[^>]+>/)) {
            indent = Math.max(0, indent - 2);
            xmlFormatted += ' '.repeat(indent) + line + '\n';
          } else if (line.match(/<[^>]+\/>/)) {
            xmlFormatted += ' '.repeat(indent) + line + '\n';
          } else if (line.match(/<[^>]+>/)) {
            xmlFormatted += ' '.repeat(indent) + line + '\n';
            if (!line.includes('</')) {
              indent += 2;
            }
          } else {
            xmlFormatted += ' '.repeat(indent) + line + '\n';
          }
        }
        formatted += xmlFormatted;
        console.log('XML formatting successful');
      } catch (e) {
        console.error('XML format error:', e);
        formatted += responseBody.value;
      }
    } else if (contentType.toLowerCase().includes('html') || responseBody.value.trim().startsWith('<!DOCTYPE html>') || responseBody.value.trim().startsWith('<html>')) {
      try {
        console.log('Formatting as HTML');
        let htmlFormatted = '';
        let indent = 0;
        const lines = responseBody.value.replace(/></g, '>\n<').split('\n');
        for (const line of lines) {
          if (line.match(/<\/[^>]+>/)) {
            indent = Math.max(0, indent - 2);
            htmlFormatted += ' '.repeat(indent) + line + '\n';
          } else if (line.match(/<[^>]+\/>/)) {
            htmlFormatted += ' '.repeat(indent) + line + '\n';
          } else if (line.match(/<[^>]+>/)) {
            htmlFormatted += ' '.repeat(indent) + line + '\n';
            if (!line.includes('</')) {
              indent += 2;
            }
          } else {
            htmlFormatted += ' '.repeat(indent) + line + '\n';
          }
        }
        formatted += htmlFormatted;
        console.log('HTML formatting successful');
      } catch (e) {
        console.error('HTML format error:', e);
        formatted += responseBody.value;
      }
    } else {
      console.log('No formatting applied, keeping original');
      formatted += responseBody.value;
    }
  }
  return formatted;
});

function isValidJson(str: string): boolean {
  try {
    const trimmed = str.trim();
    if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
      JSON.parse(trimmed);
      return true;
    }
    return false;
  } catch (e) {
    return false;
  }
}
</script>

<style scoped>
.repeater-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 16px;
  overflow: hidden;
}

.repeater-split {
  height: calc(100% - 100px);
  flex: 1;
  overflow: hidden;
}

.top-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.action-left {
  display: flex;
  align-items: center;
  min-width: 80px;
}

.right-controls {
  display: flex;
  align-items: center;
  flex: 1;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
}

.action-buttons {
  display: flex;
  align-items: center;
  white-space: nowrap;
  margin-right: 16px;
}

.request-panel,
.response-panel {
  height: 700px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  display: flex;
  justify-content: end;
  align-items: center;
  width: 100%;
}

.panel-actions {
  display: flex;
  align-items: center;
}

.panel-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.socket-settings {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.socket-info {
  margin-bottom: 16px;
}

.socket-info-alert {
  font-size: 12px;
}

.beautified-response {
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
  height: calc(100vh - 280px);
}

.response-headers,
.header-line,
.header-name,
.header-value,
.header-body-separator {
  display: none;
}

.format-actions {
  display: flex;
  margin-bottom: 10px;
  position: relative;
  z-index: 1;
}

.formatted-content {
  flex: 1;
  overflow: auto;
  margin: 0;
  padding: 10px;
  background-color: #f5f7fa;
  border-bottom-left-radius: 4px;
  border-bottom-right-radius: 4px;
  font-family: Maple Mono NF CN, Menlo, Consolas, Maple UI, PingFang, 'Microsoft YaHei', monospace;
  white-space: pre-wrap;
  position: relative;
  height: calc(100% - 200px);
  min-height: 200px;
  contain: content;
  transform: translateZ(0);
}

.full-height-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.full-height-tabs :deep(.arco-tabs-content) {
  flex: 1;
  overflow: auto;
  height: 100%;
}

.full-height-tabs :deep(.arco-tabs-content-list) {
  height: 100%;
}

.full-height-tabs :deep(.arco-tabs-pane) {
  height: 100%;
}

.hex-view {
  font-family: Maple Mono NF CN, Menlo, Consolas, Maple UI, PingFang, 'Microsoft YaHei', monospace;
  white-space: pre;
  padding: 10px;
  background-color: #f5f7fa;
  border-radius: 4px;
}

.rendered-response {
  height: 600px;
  min-height: 400px;
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}
</style>