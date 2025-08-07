<template>
  <a-tabs default-active-key="1" lazy-load>
    <a-tab-pane key="1">
      <template #title>
        <icon-settings /> {{ $t('setting.asm.basicSettings') }}
      </template>
      <a-card class="general-card">
        <a-form :model="config" layout="vertical" @submit="handleSubmit">
          <a-row :gutter="16">
            <a-col :span="12">
              <a-form-item field="dns_collection_brute_status" :label="$t('setting.asm.dns_collection_brute')">
                <a-switch 
                  v-model="config.dns_collection_brute_status" 
                  type="round"
                  :checked-text="$t('setting.asm.enabled')"
                  :unchecked-text="$t('setting.asm.disabled')"
                />
              </a-form-item>
            </a-col>
            <a-col :span="12">
              <a-form-item field="dns_collection_plugin_status" :label="$t('setting.asm.dns_collection_plugin')">
                <a-switch 
                  v-model="config.dns_collection_plugin_status" 
                  type="round"
                  :checked-text="$t('setting.asm.enabled')"
                  :unchecked-text="$t('setting.asm.disabled')"
                />
              </a-form-item>
            </a-col>
          </a-row>
          
          <a-form-item field="is_buildin" :label="$t('setting.asm.use_buildin_dict')">
            <a-switch 
              v-model="config.is_buildin" 
              type="round"
              :checked-text="$t('setting.asm.enabled')"
              :unchecked-text="$t('setting.asm.disabled')"
            />
          </a-form-item>
          

          <a-form-item field="file_dict" :label="$t('setting.asm.file_dict')">
            <a-input 
              v-model="config.file_dict" 
              :placeholder="$t('setting.asm.file_dict_placeholder')" 
              :disabled="config.is_buildin"
              style="width: 500px"
            />
            <a-button @click="selectFileDictFile">
                {{ $t('setting.asm.select_file') }}
              </a-button>
            <template #help>
              {{ $t('setting.asm.file_dict_help') }}
            </template>
          </a-form-item>
          
          <a-form-item field="subdomain_dict" :label="$t('setting.asm.subdomain_dict')">
            <a-input 
              v-model="config.subdomain_dict" 
              :placeholder="$t('setting.asm.subdomain_dict_placeholder')" 
              :disabled="config.is_buildin"
              style="width: 500px"
            />
            <a-button @click="selectSubdomainDictFile">
                {{ $t('setting.asm.select_file') }}
              </a-button>
            <template #help>
              {{ $t('setting.asm.subdomain_dict_help') }}
            </template>
          </a-form-item>

          
          
          <a-form-item field="thread_num" :label="$t('setting.asm.thread_num')">
            <a-input-number
              v-model="config.thread_num"
              :min="1"
              :max="100"
              :step="1"
              style="width: 160px"
              mode="button"
            />
          </a-form-item>
          
          <a-form-item field="subdomain_level" :label="$t('setting.asm.subdomain_level')">
            <a-input-number
              v-model="config.subdomain_level"
              :min="3"
              :max="5"
              :step="1"
              style="width: 160px"
              mode="button"
            />
            <template #help>
              {{ $t('setting.asm.subdomain_level_help') }}
            </template>
          </a-form-item>
          
          <a-form-item field="http_timeout" :label="$t('setting.asm.http_timeout')">
            <a-input-number
              v-model="config.http_timeout"
              :min="1"
              :max="300"
              :step="1"
              style="width: 160px"
              mode="button"
            />
          </a-form-item>
          
          <a-form-item field="proxy" :label="$t('setting.asm.proxy')">
            <a-input 
              v-model="config.proxy" 
              placeholder="http://127.0.0.1:8080" 
              allow-clear
            />
            <template #help>
              格式: http://ip:port 或 socks5://ip:port
            </template>
          </a-form-item>
          
          <a-form-item field="user_agent" :label="$t('setting.asm.user_agent')">
            <a-input 
              v-model="config.user_agent" 
              placeholder="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
              allow-clear
            />
          </a-form-item>
          
          <a-divider>{{ $t('setting.asm.http_headers') }}</a-divider>
          
          <a-space direction="vertical" style="width: 100%" size="large">
            <div v-for="(header, index) in headers" :key="index" class="header-item">
              <a-row :gutter="8">
                <a-col :span="10">
                  <a-input v-model="header.key" placeholder="Header Name" allow-clear />
                </a-col>
                <a-col :span="10">
                  <a-input v-model="header.value" placeholder="Header Value" allow-clear />
                </a-col>
                <a-col :span="4">
                  <a-button status="danger" shape="circle" @click="removeHeader(index)">
                    <template #icon><icon-delete /></template>
                  </a-button>
                </a-col>
              </a-row>
            </div>
            
            <a-button type="dashed" long @click="addHeader">
              <template #icon><icon-plus /></template>
              {{ $t('setting.asm.add_header') }}
            </a-button>
          </a-space>
          
          <div style="margin-top: 24px">
            <a-space>
              <a-button type="primary" html-type="submit" :loading="loading">
                {{ $t('setting.asm.save') }}
              </a-button>
              <a-button @click="loadConfig">
                {{ $t('setting.asm.reset') }}
              </a-button>
            </a-space>
          </div>
        </a-form>
      </a-card>
    </a-tab-pane>

    <a-tab-pane key="2">
      <template #title>
        <icon-bug /> 风险扫描配置
      </template>
      <a-card class="general-card">
        <asm-risk />
      </a-card>
    </a-tab-pane>

    <a-tab-pane key="3">
      <template #title>
        <icon-calendar /> 组件识别配置
      </template>
      <a-card class="general-card">
        <a-empty description="此功能尚在开发中"/>
      </a-card>
    </a-tab-pane>
  </a-tabs>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from 'vue';
import { Message } from '@arco-design/web-vue';
import { useI18n } from 'vue-i18n';
import { AsmConfig, HeaderItem } from '../types/config';
import { getAsmConfig, updateAsmConfig } from '@/api/setting';
import { open } from '@tauri-apps/plugin-dialog';
import AsmRisk from './asm-risk.vue';
const { t } = useI18n();

const config = reactive<AsmConfig>({
  dns_collection_brute_status: false,
  dns_collection_plugin_status: false,
  is_buildin: true,
  proxy: '',
  user_agent: '',
  http_headers: [],
  http_timeout: 30,
  thread_num: 10,
  file_dict: '',
  subdomain_dict: '',
  subdomain_level: 3
});

const headers = ref<HeaderItem[]>([]);
const loading = ref(false);

const selectSubdomainDictFile = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{
        name: 'Text Files',
        extensions: ['txt', 'dict', 'list']
      }]
    });
    
    if (selected && !Array.isArray(selected)) {
      config.subdomain_dict = selected;
    }
  } catch (error) {
    console.error('选择文件错误:', error);
    Message.error(t('setting.asm.file_select_error'));
  }
};

const selectFileDictFile = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{
        name: 'Text Files',
        extensions: ['txt', 'dict', 'list']
      }]
    });

    if (selected && !Array.isArray(selected)) {
      config.file_dict = selected;
    }
  } catch (error) {
    console.error('选择文件错误:', error);
    Message.error(t('setting.asm.file_select_error'));
  }
};


const parseHeaders = (headerArray?: Array<[string, string]>) => {
  headers.value = [];
  if (!headerArray || headerArray.length === 0) {
    return;
  }
  
  headers.value = headerArray.map(([key, value]) => ({ key, value }));
};

const formatHeaders = (): Array<[string, string]> => {
  return headers.value
    .filter(h => h.key && h.value)
    .map(h => [h.key, h.value]);
};

const addHeader = () => {
  headers.value.push({ key: '', value: '' });
};

const removeHeader = (index: number) => {
  headers.value.splice(index, 1);
};

const loadConfig = async () => {
  loading.value = true;
  try {
    const data = await getAsmConfig();
    Object.assign(config, data);
    parseHeaders(data.http_headers);
  } catch (error) {
    console.error('加载配置失败:', error);
    Message.error(t('setting.asm.load_error'));
  } finally {
    loading.value = false;
  }
};

const handleSubmit = async () => {
  loading.value = true;
  try {
    config.http_headers = formatHeaders();
    
    const success = await updateAsmConfig(config);
    if (success) {
      Message.success(t('setting.asm.save_success'));
    } else {
      Message.error(t('setting.asm.save_error'));
    }
  } catch (error) {
    console.error('保存配置失败:', error);
    Message.error(t('setting.asm.save_error'));
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  loadConfig();
});
</script>

<style scoped lang="less">
.general-card {
  margin-bottom: 16px;
}

.header-item {
  margin-bottom: 8px;
}

:deep(.arco-form-item-label-col) {
  font-weight: 500;
}

:deep(.arco-divider-text) {
  font-weight: 500;
  font-size: 14px;
}
</style>