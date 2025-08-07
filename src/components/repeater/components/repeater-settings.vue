<template>
  <a-modal 
    v-model:visible="visible" 
    :title="$t('repeater.repeater_settings')" 
    @cancel="handleCancel" 
    @ok="handleConfirm"
    :mask-closable="false"
    :width="500"
  >
    <a-form :model="form" layout="vertical">
      <a-form-item :label="$t('repeater.editor_settings')">
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item :label="$t('repeater.font_size')" field="fontSize">
              <a-input-number 
                v-model="form.fontSize" 
                :placeholder="$t('repeater.font_size_placeholder')" 
                :min="8" 
                :max="32" 
                :step="1"
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item :label="$t('repeater.line_height')" field="lineHeight">
              <a-input-number 
                v-model="form.lineHeight" 
                :placeholder="$t('repeater.line_height_placeholder')" 
                :min="1" 
                :max="3" 
                :step="0.1"
                :precision="1"
              />
            </a-form-item>
          </a-col>
        </a-row>
      </a-form-item>

      <a-form-item :label="$t('repeater.http_version_settings')">
        <a-radio-group v-model="form.httpVersion">
          <a-radio value="Http1">HTTP/1.1</a-radio>
          <a-radio value="Http2">HTTP/2</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item :label="$t('repeater.proxy_settings')">
        <a-switch v-model="form.useProxy" style="margin-bottom: 16px;">
          <template #checked>{{ $t('repeater.enable_proxy') }}</template>
          <template #unchecked>{{ $t('repeater.disable_proxy') }}</template>
        </a-switch>
        
        <div v-if="form.useProxy" class="proxy-settings-container">
          <!-- 基本代理设置 -->
          <div class="proxy-section">
            <ADivider orientation="left" :size="4">{{ $t('repeater.basic_settings') }}</ADivider>
            <a-row :gutter="16">
              <a-col :span="16">
                <a-form-item :label="$t('repeater.proxy_host')" field="proxyHost">
                  <a-input v-model="form.proxyHost" :placeholder="$t('repeater.proxy_host_placeholder')" />
                </a-form-item>
              </a-col>
              <a-col :span="8">
                <a-form-item :label="$t('repeater.proxy_port')" field="proxyPort">
                  <a-input-number 
                    v-model="form.proxyPort" 
                    :placeholder="$t('repeater.proxy_port_placeholder')" 
                    :min="1" 
                    :max="65535" 
                    :step="1"
                  />
                </a-form-item>
              </a-col>
            </a-row>
          
            <a-row :gutter="16">
              <a-col :span="12">
                <a-form-item :label="$t('repeater.proxy_type')" field="proxyType">
                  <a-select v-model="form.proxyType">
                    <a-option value="http">HTTP</a-option>
                    <a-option value="https">HTTPS</a-option>
                    <a-option value="socks5">SOCKS5</a-option>
                  </a-select>
                </a-form-item>
              </a-col>
              <a-col :span="12">
                <a-form-item :label="$t('repeater.timeout')" field="timeout">
                  <a-input-number 
                    v-model="form.timeout" 
                    :placeholder="$t('repeater.timeout_placeholder')" 
                    :min="1" 
                    :max="120" 
                    :step="1"
                  />
                </a-form-item>
              </a-col>
            </a-row>
          </div>
          
          <!-- 认证设置 -->
          <div class="proxy-section">
            <ADivider orientation="left" :size="4">{{ $t('repeater.proxy_auth') }}</ADivider>
            <a-row :gutter="16">
              <a-col :span="12">
                <a-form-item :label="$t('repeater.proxy_user')" field="proxyUser">
                  <a-input v-model="form.proxyUser" :placeholder="$t('repeater.proxy_user_placeholder')" />
                </a-form-item>
              </a-col>
              <a-col :span="12">
                <a-form-item :label="$t('repeater.proxy_password')" field="proxyPassword">
                  <a-input-password v-model="form.proxyPassword" :placeholder="$t('repeater.proxy_password_placeholder')" />
                </a-form-item>
              </a-col>
            </a-row>
          </div>
        </div>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script lang="ts" setup>
import { ref, reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';

const emit = defineEmits(['confirm', 'cancel']);

// 控制模态框显示
const visible = ref(false);

// 表单数据
const form = reactive({
  fontSize: 14,
  lineHeight: 1.5,
  useProxy: false,
  proxyHost: '',
  proxyPort: 8080,
  proxyType: 'http',
  proxyUser: '',
  proxyPassword: '',
  timeout: 30,
  httpVersion: 'Http1',
});

// 默认设置，用于重置
const defaultSettings = {
  fontSize: 14,
  lineHeight: 1.5,
  useProxy: false,
  proxyHost: '',
  proxyPort: 8080,
  proxyType: 'http',
  proxyUser: '',
  proxyPassword: '',
  timeout: 30,
  httpVersion: 'Http1',
};

// 加载设置
const loadSettings = async () => {
  try {
    const backendSettings = await invoke('repeater_get_settings') as any;
    if (backendSettings) {
      // 转换为前端格式
      form.fontSize = backendSettings.font_size;
      form.lineHeight = backendSettings.line_height;
      form.useProxy = backendSettings.use_proxy;
      form.proxyHost = backendSettings.proxy_host;
      form.proxyPort = Number(backendSettings.proxy_port);
      form.proxyType = backendSettings.proxy_type;
      form.proxyUser = backendSettings.proxy_user;
      form.proxyPassword = backendSettings.proxy_password;
      form.timeout = Number(backendSettings.timeout);
      // 处理HTTP版本设置
      if (backendSettings.default_http_version) {
        form.httpVersion = backendSettings.default_http_version;
      }
    }
  } catch (error) {
    console.error('加载设置失败', error);
    // 加载默认设置
    Object.assign(form, defaultSettings);
  }
};

// 打开模态框
const openModal = async () => {
  // 每次打开模态框时加载最新设置
  await loadSettings();
  visible.value = true;
};

// 暴露给父组件的方法
defineExpose({
  openModal,
});

// 保存设置
const saveSettings = async () => {
  try {
    // 转换为后端格式
    const backendSettings = {
      font_size: form.fontSize,
      line_height: form.lineHeight,
      use_proxy: form.useProxy,
      proxy_host: form.proxyHost,
      proxy_port: form.proxyPort,
      proxy_type: form.proxyType,
      proxy_user: form.proxyUser,
      proxy_password: form.proxyPassword,
      timeout: form.timeout,
      default_http_version: form.httpVersion
    };
    
    await invoke('repeater_save_settings', { settings: backendSettings });
    Message.success('设置已保存');
  } catch (error) {
    Message.error(`保存设置失败: ${error}`);
  }
};

// 取消
const handleCancel = () => {
  visible.value = false;
  emit('cancel');
};

// 确认
const handleConfirm = async () => {
  await saveSettings();
  visible.value = false;
  emit('confirm', form);
};
</script>

<style scoped>
.form-row {
  display: flex;
  gap: 16px;
}

.proxy-settings-container {
  margin-top: 8px;
  border: 1px solid var(--color-neutral-3);
  border-radius: 4px;
  padding: 0 8px;
  background-color: var(--color-fill-2);
}

.proxy-section {
  margin-bottom: 16px;
}

.proxy-section:last-child {
  margin-bottom: 8px;
}
</style> 