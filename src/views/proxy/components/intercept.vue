<template>
  <div class="intercept-container">
    <a-space>
      <a-button style="width: 120px;" size="small" @click="forward" :disabled="!currentRequest && !currentResponse"
        class="equal-width-btn">
        {{ $t('proxy.intercept.forward') }}
      </a-button>
      <a-button style="width: 120px;" size="small" @click="drop" :disabled="!currentRequest && !currentResponse"
        class="equal-width-btn">
        {{ $t('proxy.intercept.drop') }}
      </a-button>
      <a-button :type="interceptEnabled ? 'primary' : 'outline'" size="small"
        @click="toggleIntercept(!interceptEnabled)" class="equal-width-btn">
        {{ interceptEnabled ? $t('proxy.intercept.on') : $t('proxy.intercept.off') }}
      </a-button>
      <a-dropdown>
        <a-button :disabled="!currentRequest" style="width: 120px;" size="small">
          {{ $t('proxy.intercept.action') }}
        </a-button>
        <template #content>
          <a-doption @click="sendToRepeater">{{ $t('proxy.intercept.send_to_repeater') }}</a-doption>
          <a-doption @click="copyAsRaw">{{ $t('proxy.intercept.copy_as_raw') }}</a-doption>
        </template>
      </a-dropdown>
    </a-space>

    <div style="margin-top: 10px;">
      <codemirror v-if="currentRequest" v-model="rawRequest" :extensions="extensions" :style="{ height: '350px' }"
        @change="onRawRequestChange" />

      <codemirror v-if="currentResponse" v-model="rawResponse" :extensions="extensions" :style="{ height: '350px' }"
        @change="onRawResponseChange" />

    </div>


    <!-- <a-tabs type="card">


      <a-tab-pane key="response" :title="$t('proxy.intercept.response') || '响应拦截'">
        <div class="intercept-section">
          <div class="intercept-header">
            <div class="intercept-controls">
              <a-space>
                <a-button style="width: 120px;" size="small" @click="forwardResponse" :disabled="!currentResponse"
                  class="equal-width-btn">
                  {{ $t('proxy.intercept.forward_response') || '转发响应' }}
                </a-button>
                <a-button style="width: 120px;" size="small" @click="dropResponse" :disabled="!currentResponse"
                  class="equal-width-btn">
                  {{ $t('proxy.intercept.drop_response') || '丢弃响应' }}
                </a-button>
                <a-button :type="interceptResponseEnabled ? 'primary' : 'outline'" size="small"
                  @click="toggleResponseIntercept(!interceptResponseEnabled)" class="equal-width-btn">
                  {{ interceptResponseEnabled ? $t('proxy.intercept.response_on') || '拦截开' :
                    $t('proxy.intercept.response_off') || '拦截关' }}
                </a-button>
              </a-space>
            </div>
          </div>

          <div  class="response-editor">
            <div class="editor-container">
              <div class="editor-header">
                <span class="status-code">{{ $t('proxy.intercept.status') || '状态码' }}: {{ currentResponse.status ||
                  $t('proxy.intercept.pending') || '处理中' }}</span>
                <span v-if="currentResponse.request_id" class="related-request">{{ $t('proxy.intercept.related_request')
                  ||
                  '关联请求' }}: {{ currentResponse.request_id }}</span>
              </div>

            </div>
          </div>

          <div v-else class="waiting-message">
            <a-empty>
              <template #description>
                <p>{{ interceptResponseEnabled ? $t('proxy.intercept.response_waiting') || '等待响应...' :
                  $t('proxy.intercept.response_disabled') || '响应拦截已禁用' }}</p>
              </template>
            </a-empty>
          </div>
        </div>
      </a-tab-pane>
    </a-tabs> -->
  </div>
</template>

<script lang="ts" setup>
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Codemirror } from 'vue-codemirror';
import { oneDark } from '@codemirror/theme-one-dark';
import { json } from '@codemirror/lang-json';
import { useI18n } from 'vue-i18n';

interface Request {
  id: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string;
}

interface Response {
  id: string;
  request_id?: string;
  status: number;
  headers: Record<string, string>;
  body: string;
}

const { t } = useI18n();

const interceptEnabled = ref(false);
const interceptResponseEnabled = ref(false);
const currentRequest = ref<Request | null>(null);
const currentResponse = ref<Response | null>(null);
const rawRequest = ref('');
const rawResponse = ref('');
const currentHandle = ref<string>('');

let unlisten: (() => void) | null = null;
let unlistenStatus: (() => void) | null = null;
let unlistenResponse: (() => void) | null = null;

// CodeMirror 扩展
const extensions = [json(), oneDark];



// 添加这些方法来处理原始请求和响应文本
const formatRawRequest = (request: Request) => {
  if (!request) return '';

  // 获取URL的路径部分
  let path = '';
  try {
    const urlObj = new URL(request.url);
    path = urlObj.pathname + urlObj.search; // 路径 + 查询参数
  } catch (e) {
    path = request.url; // 解析失败时使用完整URL
    console.warn('无法解析URL:', e);
  }

  // 请求行，使用路径替代完整URL
  let raw = `${request.method} ${path} HTTP/1.1\n`;

  // 从URL中提取host
  let hostValue = '';
  try {
    const urlObj = new URL(request.url);
    hostValue = urlObj.host;
  } catch (e) {
    console.warn('无法从URL中提取host:', e);
  }

  // 确保Host头在第一行
  if (hostValue) {
    raw += `Host: ${hostValue}\n`;
  }

  // 请求头（跳过Host头，因为已经添加）
  if (request.headers) {
    Object.entries(request.headers).forEach(([name, value]) => {
      if (name.toLowerCase() !== 'host') {
        raw += `${name}: ${value}\n`;
      }
    });
  }

  // 请求体 (如果有)
  if (request.body && request.body.trim()) {
    raw += `\n${request.body}`;
  }

  return raw;
};

const formatRawResponse = (response: Response) => {
  let raw = `HTTP/1.1 ${response.status} ${getStatusText(response.status)}\n`;

  // 添加头部
  if (response.headers) {
    Object.entries(response.headers).forEach(([name, value]) => {
      raw += `${name}: ${value}\n`;
    });
  }

  // 添加空行和响应体
  raw += `\n${response.body || ''}`;
  return raw;
};

const getStatusText = (status: number): string => {
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
    500: 'Internal Server Error'
  };
  return statusMap[status] || 'Unknown';
};

const parseRawRequest = (raw: string) => {
  const lines = raw.split('\n');

  // 第一行应该是请求行: METHOD URL HTTP/1.1
  if (lines.length < 1) return null;

  const requestLine = lines[0].split(' ');
  if (requestLine.length < 2) return null;

  const method = requestLine[0];
  const pathPart = requestLine[1];

  // 解析头部和请求体
  const headers: Record<string, string> = {};
  let bodyStartIndex = -1;
  let hostHeader = '';

  for (let i = 1; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line === '') {
      bodyStartIndex = i + 1;
      break;
    }

    const separatorIndex = line.indexOf(':');
    if (separatorIndex > 0) {
      const name = line.substring(0, separatorIndex).trim();
      const value = line.substring(separatorIndex + 1).trim();
      headers[name] = value;

      // 记录Host头，用于重建完整URL
      if (name.toLowerCase() === 'host') {
        hostHeader = value;
      }
    }
  }

  // 从路径和Host头重建完整URL
  let url = '';
  if (hostHeader && pathPart.startsWith('/')) {
    // 如果有Host头且路径以/开头，重建完整URL
    url = `https://${hostHeader}${pathPart}`;
  } else if (pathPart.includes('://')) {
    // 如果路径已经是完整URL，直接使用
    url = pathPart;
  } else if (hostHeader) {
    // 否则尝试拼接
    url = `https://${hostHeader}/${pathPart}`;
  } else {
    // 无法重建，使用原始路径
    url = pathPart;
  }

  // 提取请求体
  let body = '';
  if (bodyStartIndex > 0 && bodyStartIndex < lines.length) {
    body = lines.slice(bodyStartIndex).join('\n');
  }

  return {
    method,
    url,
    headers,
    body
  };
};

const parseRawResponse = (raw: string) => {
  const lines = raw.split('\n');

  // 第一行应该是状态行: HTTP/1.1 STATUS STATUS_TEXT
  if (lines.length < 1) return null;

  const statusLine = lines[0].split(' ');
  if (statusLine.length < 2) return null;

  const status = parseInt(statusLine[1], 10);

  // 解析头部和响应体
  const headers: Record<string, string> = {};
  let bodyStartIndex = -1;

  for (let i = 1; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line === '') {
      bodyStartIndex = i + 1;
      break;
    }

    const separatorIndex = line.indexOf(':');
    if (separatorIndex > 0) {
      const name = line.substring(0, separatorIndex).trim();
      const value = line.substring(separatorIndex + 1).trim();
      headers[name] = value;
    }
  }

  // 提取响应体
  let body = '';
  if (bodyStartIndex > 0 && bodyStartIndex < lines.length) {
    body = lines.slice(bodyStartIndex).join('\n');
  }

  return {
    status,
    headers,
    body
  };
};

// 原始请求变更处理
const onRawRequestChange = () => {
  if (!currentRequest.value) return;

  const parsed = parseRawRequest(rawRequest.value);
  if (parsed) {
    // 更新当前请求对象
    currentRequest.value.method = parsed.method;
    currentRequest.value.url = parsed.url;
    currentRequest.value.headers = parsed.headers;
    currentRequest.value.body = parsed.body;
  }
};

// 原始响应变更处理
const onRawResponseChange = () => {
  if (!currentResponse.value) return;

  const parsed = parseRawResponse(rawResponse.value);
  if (parsed) {
    // 更新当前响应对象
    currentResponse.value.status = parsed.status;
    currentResponse.value.headers = parsed.headers;
    currentResponse.value.body = parsed.body;
  }
};

onMounted(async () => {
  // 检查请求拦截状态
  try {
    const status = await invoke('get_proxy_intercept_status');
    interceptEnabled.value = !!status;
  } catch (error) {
    console.error('获取拦截状态失败:', error);
  }

  // 检查响应拦截状态
  try {
    const responseStatus = await invoke('get_proxy_intercept_response_status');
    interceptResponseEnabled.value = !!responseStatus;
  } catch (error) {
    console.error('获取响应拦截状态失败:', error);
  }

  // 监听拦截的请求
  unlisten = await listen('proxy-request-intercepted', (event) => {
    currentHandle.value = 'request';
    const request = event.payload as Request;
    console.log('收到拦截请求: ID=', request.id);

    // 始终显示最新的请求，不再检查是否已有请求
    currentRequest.value = request;
    rawRequest.value = formatRawRequest(request);
  });

  // 监听拦截的响应，独立处理响应
  unlistenResponse = await listen('proxy-response-intercepted', (event) => {
    currentHandle.value = 'response';
    const response = event.payload as Response;
    console.log('收到拦截响应:', response.id);

    // 始终显示最新的响应，不再检查请求关联
    currentResponse.value = response;
    rawResponse.value = formatRawResponse(response);
  });

  // 监听拦截状态变更事件
  unlistenStatus = await listen('proxy-intercept-status-change', (event: any) => {
    if (event.payload && typeof event.payload.enabled === 'boolean') {
      interceptEnabled.value = event.payload.enabled;
    }
  });

  // 监听响应拦截状态变更事件
  const unlistenResponseStatus = await listen('proxy-intercept-response-status-change', (event: any) => {
    if (event.payload && typeof event.payload.enabled === 'boolean') {
      interceptResponseEnabled.value = event.payload.enabled;
    }
  });

  // 记得在onUnmounted中取消监听
  if (unlistenResponseStatus) {
    onUnmounted(() => {
      unlistenResponseStatus();
    });
  }
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
  }
  if (unlistenStatus) {
    unlistenStatus();
  }
  if (unlistenResponse) {
    unlistenResponse();
  }
});

// 切换拦截开关
const toggleIntercept = async (value: boolean) => {

  try {

    await invoke('set_proxy_intercept_status', { enabled: value });
    if (currentRequest.value) {
      await forwardRequest();
    }
    if (currentResponse.value) {
      await forwardResponse();
    }
    Message.success(value ? t('proxy.intercept.on') : t('proxy.intercept.off'));
  } catch (error) {
    Message.error(`设置拦截状态失败: ${error}`);
    interceptEnabled.value = !value; // 恢复状态
  }
};

const forward = async () => {
  console.log('forward', currentHandle.value);
  if (currentHandle.value === 'request') {
    await forwardRequest()
  }

  if (currentHandle.value === 'response') {
    await forwardResponse();
  }

}

const drop = async () => {
  if (currentHandle.value === 'request') {
    await dropRequest();
  }

  if (currentHandle.value === 'response') {
    await dropResponse();
  }
}

// 转发请求
const forwardRequest = async () => {
  if (!currentRequest.value) {
    Message.error(t('proxy.intercept.no_request'));
    return;
  }

  try {
    const parsedRequest = parseRawRequest(rawRequest.value);

    if (!parsedRequest) {
      throw new Error('无法解析请求格式');
    }

    // 检查请求是否被编辑过
    const methodChanged = parsedRequest.method !== currentRequest.value.method;
    const urlChanged = parsedRequest.url !== currentRequest.value.url;
    const headersChanged = JSON.stringify(parsedRequest.headers) !== JSON.stringify(currentRequest.value.headers);
    const bodyChanged = parsedRequest.body !== currentRequest.value.body;

    console.log(`请求转发检查: 方法变更=${methodChanged}, URL变更=${urlChanged}, 头部变更=${headersChanged}, 内容变更=${bodyChanged}`);

    // 存储当前请求ID并先清除当前请求，防止UI卡住
    const requestId = currentRequest.value.id;
    console.log('转发请求ID:', requestId);

    // 先清除当前请求和响应的引用，避免页面卡死
    const requestBeingForwarded = currentRequest.value;
    currentRequest.value = null;
    currentResponse.value = null;
    rawRequest.value = '';
    rawResponse.value = '';

    // 创建请求转发参数
    const forwardParams: {
      requestId: string;
      method: string | null;
      url: string | null;
      headers: Record<string, string> | null;
      body: string | null;
      responseHeaders: Record<string, string> | null;
      responseBody: string | null;
    } = {
      requestId: requestId,
      method: methodChanged ? parsedRequest.method : null,
      url: urlChanged ? parsedRequest.url : null,
      headers: headersChanged ? parsedRequest.headers : null,
      body: bodyChanged ? parsedRequest.body : null,
      responseHeaders: null,
      responseBody: null
    };

    // 如果有响应，添加响应参数
    if (requestBeingForwarded) {
      try {
        const parsedResponse = parseRawResponse(rawResponse.value);
        if (parsedResponse) {
          // 直接使用解析后的响应内容
          forwardParams.responseHeaders = parsedResponse.headers;
          forwardParams.responseBody = parsedResponse.body;
          console.log('添加响应数据到转发请求');
        }
      } catch (error) {
        console.error('响应解析错误:', error);
        // 继续处理请求部分，忽略响应解析错误
      }
    }

    // 创建一个不阻塞UI的异步执行
    setTimeout(async () => {
      try {
        // 设置超时
        const timeout = 10000;
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeout);

        console.log('开始转发请求:', requestId);
        await invoke('forward_intercepted_request', forwardParams);
        clearTimeout(timeoutId);

        console.log('请求转发成功:', requestId);
      } catch (error: any) {
        console.error('转发请求失败:', error);

        // 如果是超时或连接错误，自动重试一次
        if (error?.message && (error.message.includes('连接') || error.message.includes('超时'))) {
          console.log('尝试自动重试转发请求:', requestId);

          try {
            // 简化的重试参数
            await invoke('forward_intercepted_request', {
              requestId: requestId,
              method: null,
              url: null,
              headers: null,
              body: null
            });
            console.log('重试转发成功:', requestId);
          } catch (retryError) {
            console.error('重试转发失败:', retryError);
          }
        }
      }
    }, 0);


  } catch (error: any) {
    // 解析错误时恢复UI状态
    let errorMessage = `${t('proxy.intercept.forward_failed')}: ${error?.message || String(error)}`;
    console.error('请求准备转发失败:', error);
    Message.error(errorMessage);
  }
};

// 丢弃请求
const dropRequest = async () => {
  if (!currentRequest.value) {
    Message.error(t('proxy.intercept.no_request'));
    return;
  }

  try {
    const requestId = currentRequest.value.id;
    console.log('丢弃请求ID:', requestId);

    // 先清除当前请求和响应的引用，这样即使后端处理失败，前端UI也可以继续处理新请求
    currentRequest.value = null;
    currentResponse.value = null;
    rawRequest.value = '';
    rawResponse.value = '';

    // 显示处理中的消息
    Message.info(t('proxy.intercept.dropping'));

    // 使用非阻塞方式执行丢弃
    setTimeout(async () => {
      try {
        console.log('开始丢弃请求:', requestId);
        await invoke('drop_intercepted_request', { requestId });
        console.log('成功丢弃请求:', requestId);
        Message.success(t('proxy.intercept.request_dropped'));
      } catch (error) {
        console.error('丢弃请求失败:', error);
        // 错误只记录在控制台，不影响UI
      }
    }, 0);

  } catch (error: any) {
    let errorMessage = `${t('proxy.intercept.drop_failed')}: ${error?.message || String(error)}`;
    console.error('准备丢弃请求失败:', error);
    Message.error(errorMessage);
  }
};

// 发送到重放器
const sendToRepeater = async () => {
  if (!currentRequest.value) return;

  try {
    const parsedRequest = parseRawRequest(rawRequest.value);
    if (!parsedRequest) {
      throw new Error('无法解析请求格式');
    }

    await invoke('send_to_repeater', {
      request: {
        method: parsedRequest.method,
        url: parsedRequest.url,
        request_headers: parsedRequest.headers,
        request_body: parsedRequest.body
      }
    });

    Message.success('已发送到重放器');
  } catch (error) {
    Message.error(`发送到重放器失败: ${error}`);
  }
};

// 复制为原始文本
const copyAsRaw = () => {
  if (!currentRequest.value) return;

  // 直接使用当前的原始请求文本
  navigator.clipboard.writeText(rawRequest.value).then(() => {
    Message.success('已复制到剪贴板');
  }, (err) => {
    Message.error(`复制失败: ${err}`);
  });
};

// 转发响应
const forwardResponse = async () => {
  if (!currentResponse.value) {
    Message.error(t('proxy.intercept.no_response') || '没有响应可转发');
    return;
  }

  try {
    const parsedResponse = parseRawResponse(rawResponse.value);

    if (!parsedResponse) {
      throw new Error('无法解析响应格式');
    }

    // 检查响应是否被编辑过
    const statusChanged = parsedResponse.status !== currentResponse.value.status;
    const headersChanged = JSON.stringify(parsedResponse.headers) !== JSON.stringify(currentResponse.value.headers);
    const bodyChanged = parsedResponse.body !== currentResponse.value.body;

    console.log(`响应转发检查: 状态码变更=${statusChanged}, 头部变更=${headersChanged}, 内容变更=${bodyChanged}`);

    // 存储当前响应ID
    const responseId = currentResponse.value.id;
    console.log('转发响应ID:', responseId);

    // 先清除当前请求和响应的引用，避免页面卡死
    const responseBeingForwarded = currentResponse.value;
    currentRequest.value = null;
    currentResponse.value = null;
    rawRequest.value = '';
    rawResponse.value = '';

    // 创建响应转发参数
    const forwardParams: {
      responseId: string;
      status: number | null;
      headers: Record<string, string> | null;
      body: string | null;
    } = {
      responseId: responseId,
      status: statusChanged ? parsedResponse.status : null,
      headers: headersChanged ? parsedResponse.headers : null,
      body: bodyChanged ? parsedResponse.body : null
    };

    // 使用非阻塞方式执行转发
    setTimeout(async () => {
      try {
        console.log('开始转发响应:', responseId);
        await invoke('forward_intercepted_response', {
          responseId: responseId,  // 修改参数名以匹配后端API
          status: statusChanged ? parsedResponse.status : null,
          headers: headersChanged ? parsedResponse.headers : null,
          body: bodyChanged ? parsedResponse.body : null
        });
        console.log('响应转发成功:', responseId);
      } catch (error) {
        console.error('转发响应失败:', error);
        Message.error(`转发响应失败: ${error}`);
      }
    }, 0);

  } catch (error: any) {
    // 解析错误时显示错误消息
    let errorMessage = `${t('proxy.intercept.forward_response_failed') || '转发响应失败'}: ${error?.message || String(error)}`;
    console.error('响应准备转发失败:', error);
    Message.error(errorMessage);
  }
};

// 丢弃响应
const dropResponse = async () => {
  if (!currentResponse.value) {
    Message.error(t('proxy.intercept.no_response') || '没有响应可丢弃');
    return;
  }

  try {
    const responseId = currentResponse.value.id;
    console.log('丢弃响应ID:', responseId);

    // 先清除当前请求和响应的引用，这样即使后端处理失败，前端UI也可以继续处理新请求/响应
    currentRequest.value = null;
    currentResponse.value = null;
    rawRequest.value = '';
    rawResponse.value = '';

    // 显示处理中的消息
    Message.info(t('proxy.intercept.dropping_response') || '正在丢弃响应');

    // 使用非阻塞方式执行丢弃
    setTimeout(async () => {
      try {
        console.log('开始丢弃响应:', responseId);
        await invoke('drop_intercepted_response', { responseId });
        console.log('成功丢弃响应:', responseId);
        Message.success(t('proxy.intercept.response_dropped') || '响应已丢弃');
      } catch (error) {
        console.error('丢弃响应失败:', error);
        Message.error(`丢弃响应失败: ${error}`);
      }
    }, 0);

  } catch (error: any) {
    let errorMessage = `${t('proxy.intercept.drop_response_failed') || '丢弃响应失败'}: ${error?.message || String(error)}`;
    console.error('准备丢弃响应失败:', error);
    Message.error(errorMessage);
  }
};

</script>

<style scoped lang="less">
.intercept-container {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.intercept-header {
  margin-bottom: 16px;

  .intercept-controls {
    display: flex;
    justify-content: space-between;
    margin-bottom: 16px;
  }
}

.response-controls {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 16px;
  padding: 5px 0;
  border-bottom: 1px dashed var(--color-border-2);
}

.equal-width-btn {
  min-width: 90px;
  text-align: center;
}

.request-response-editor {
  flex: 1;
  overflow: auto;
}

.editor-container {
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  padding: 12px;
  background-color: var(--color-bg-2);
}

.editor-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--color-border-2);

  .method-url,
  .status-code,
  .related-request {
    font-weight: bold;
    font-family: monospace;
  }

  .related-request {
    font-size: 0.85em;
    color: var(--color-text-3);
    margin-left: 10px;
  }
}

.editor-section {
  margin-bottom: 16px;

  .section-header {
    font-weight: bold;
    margin-bottom: 4px;
    color: var(--color-text-2);
  }
}

.waiting-message {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 40px;
  color: var(--color-text-3);
}
</style>