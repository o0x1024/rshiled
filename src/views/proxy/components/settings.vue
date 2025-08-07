<template>
    <a-typography>
    <a-typography-title :heading="4">
      {{ $t('proxy.settings.proxy_listeners') }}
    </a-typography-title>
    </a-typography>

  <a-row>
    <a-col :span="2">
      <a-space direction="vertical">
        <a-button style="width: 100px;" size="small" @click="openProxyDialog('add')">
          {{ $t('proxy.settings.add') }}
        </a-button>
        <a-button style="width: 100px;" :disabled="!selectedProxy" size="small" @click="openProxyDialog('edit')">
          {{ $t('proxy.settings.edit') }}
        </a-button>
        <a-button style="width: 100px;" :disabled="!selectedProxy" size="small" @click="confirmDelete">
          {{ $t('proxy.settings.remove') }}
        </a-button>
      </a-space>
    </a-col>
    <a-col :span="21">
      <a-table :data="proxyList" :loading="loading" size="small" :scroll="{ y: 200 }" :pagination="false"
        @row-click="onSelectRow" :selected-keys="selectedKeys">
        <template #columns>
          <a-table-column :title="$t('proxy.proxy.status')" :width="100">
            <template #cell="{ record }">
              <a-switch :model-value="!!record.running"
                @change="(value) => toggleProxyStatus(record as ProxyConfig, !!value)" :loading="!!record.loading" />
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.interface')" data-index="interface" />
          <a-table-column :title="$t('proxy.settings.port')" data-index="port" />
          <a-table-column :title="$t('proxy.settings.http_version')" align="center" :width="100">
            <template #cell="{ record }">
              <a-tag :color="Number(record.httpVersion) === 2 ? 'green' : 'gray'">
                {{ Number(record.httpVersion) === 2 ? '支持' : '不支持' }}
              </a-tag>
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.https')" align="center" :width="100">
            <template #cell="{ record }">
              <a-checkbox :model-value="!!record.httpsEnabled" disabled />
            </template>
          </a-table-column>
        </template>
      </a-table>
    </a-col>
  </a-row>


  <a-row style="margin-top: 10px;">
    <a-col :span="24">
      <a-typography>
          <a-typography-paragraph>
            {{ $t('proxy.settings.ca_certificate_description') }}
          </a-typography-paragraph>
        </a-typography>
    </a-col>
  </a-row>
  <a-row >
    <a-col :span="12">
      <a-space direction="vertical">

        <a-space>
          <a-button size="small" @click="generateCACertificate">
            {{ $t('proxy.settings.generate_ca') }}
          </a-button>
          <a-button size="small" @click="exportCACertificate">
            {{ $t('proxy.settings.export_ca') }}
          </a-button>
        </a-space>

      </a-space>
    </a-col>
  </a-row>


  <a-divider />

  <a-space direction="vertical"></a-space>
  <a-typography>
    <a-typography-title :heading="4">
      {{ $t('proxy.settings.request_rules_title') }}
    </a-typography-title>
    </a-typography>


  <a-checkbox style="margin-bottom: 10px;" :model-value="interceptRequestEnabled" @change="toggleInterceptRequest">{{
    $t('proxy.settings.enable_request_intercept') }}</a-checkbox>
  <a-tooltip :content="$t('proxy.settings.enable_request_intercept_description')">
    <icon-info-circle style="margin-left: 4px; cursor: help;" />
  </a-tooltip>
  <a-row>
    <a-col :span="2">
      <a-space direction="vertical">
        <a-button style="width: 100px;" size="small" @click="openRuleDialog('request', 'add')">
          <template #icon><icon-plus /></template>
          {{ $t('proxy.settings.add') }}
        </a-button>
        <a-button style="width: 100px;" size="small" :disabled="!selectedRequestRule"
          @click="openRuleDialog('request', 'edit')">
          <template #icon><icon-edit /></template>
          {{ $t('proxy.settings.edit') }}
        </a-button>
        <a-button style="width: 100px;" size="small" :disabled="!selectedRequestRule || isFirstRule('request')"
          @click="moveRule('request', 'up')">
          <template #icon><icon-up /></template>
          {{ $t('proxy.settings.move_up') }}
        </a-button>
        <a-button style="width: 100px;" size="small" :disabled="!selectedRequestRule || isLastRule('request')"
          @click="moveRule('request', 'down')">
          <template #icon><icon-down /></template>
          {{ $t('proxy.settings.move_down') }}
        </a-button>
      </a-space>
    </a-col>
    <a-col :span="21">
      <a-table :data="requestRules" :pagination="false" @row-click="(record) => onSelectRule('request', record)"
        :selected-keys="selectedRequestRuleKeys">
        <template #columns>
          <a-table-column :title="$t('proxy.settings.enable')" :width="80" align="center">
            <template #cell="{ record }">
              <a-checkbox :model-value="record.enabled"
                @change="(value) => toggleRuleStatus('request', record.id, value)" />
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.operator')" data-index="operator" :width="120">
            <template #cell="{ record }">
              {{ record.operator === 'and' ? 'AND' : 'OR' }}
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.match_type')" data-index="matchType" :width="150">
            <template #cell="{ record }">
              {{ getMatchTypeLabel(record.matchType) }}
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.match_relationship')" data-index="matchRelationship" :width="150">
            <template #cell="{ record }">
              {{ record.matchRelationship === 'matches' ? '匹配' : '不匹配' }}
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.match_condition')" data-index="matchCondition" />
        </template>
      </a-table>
    </a-col>
  </a-row>

  <a-divider style="margin: 24px 0" />

  <a-typography>
    <a-typography-title :heading="4">
      {{ $t('proxy.settings.response_rules_title') }}
    </a-typography-title>
  </a-typography>
  <a-checkbox style="margin-bottom: 10px;" :model-value="interceptResponseEnabled" @change="toggleInterceptResponse">{{
    $t('proxy.settings.enable_response_intercept') }}</a-checkbox>
  <a-tooltip :content="$t('proxy.settings.enable_response_intercept_description')">
    <icon-info-circle style="margin-left: 4px; cursor: help;" />
  </a-tooltip>
  <a-row>
    <a-col :span="2">
      <a-space direction="vertical">
        <a-button size="small" style="width: 100px;" @click="openRuleDialog('response', 'add')">
          <template #icon><icon-plus /></template>
          {{ $t('proxy.settings.add') }}
        </a-button>
        <a-button size="small" style="width: 100px;" :disabled="!selectedResponseRule"
          @click="openRuleDialog('response', 'edit')">
          <template #icon><icon-edit /></template>
          {{ $t('proxy.settings.edit') }}
        </a-button>
        <a-button size="small" style="width: 100px;" :disabled="!selectedResponseRule || isFirstRule('response')"
          @click="moveRule('response', 'up')">
          <template #icon><icon-up /></template>
          {{ $t('proxy.settings.move_up') }}
        </a-button>
        <a-button size="small" style="width: 100px;" :disabled="!selectedResponseRule || isLastRule('response')"
          @click="moveRule('response', 'down')">
          <template #icon><icon-down /></template>
          {{ $t('proxy.settings.move_down') }}
        </a-button>
      </a-space>
    </a-col>
    <a-col :span="21">
      <a-table :data="responseRules" :pagination="false" @row-click="(record) => onSelectRule('response', record)"
        :selected-keys="selectedResponseRuleKeys">
        <template #columns>
          <a-table-column :title="$t('proxy.settings.enable')" :width="80" align="center">
            <template #cell="{ record }">
              <a-checkbox :model-value="record.enabled"
                @change="(value) => toggleRuleStatus('response', record.id, value)" />
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.operator')" data-index="operator" :width="120">
            <template #cell="{ record }">
              {{ record.operator === 'and' ? 'AND' : 'OR' }}
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.match_type')" data-index="matchType" :width="150">
            <template #cell="{ record }">
              {{ getMatchTypeLabel(record.matchType) }}
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.match_relationship')" data-index="matchRelationship" :width="150">
            <template #cell="{ record }">
              {{ record.matchRelationship === 'matches' ? $t('proxy.settings.matches') : $t('proxy.settings.not_matches') }}
            </template>
          </a-table-column>
          <a-table-column :title="$t('proxy.settings.match_condition')" data-index="matchCondition" />
        </template>
      </a-table>
    </a-col>
  </a-row>


  <a-divider />

  <div class="proxy-list">
    <div class="help-info">
      <a-card class="help-card" :title="$t('proxy.settings.browser_setup')">
        <p>{{ $t('proxy.settings.browser_instruction') }}</p>
        <a-typography-paragraph v-if="selectedProxy" copyable>
          {{ `http${selectedProxy.httpsEnabled ? 's' : ''}://${selectedProxy.interface === '0.0.0.0' ? 'localhost'
            :
            selectedProxy.interface}:${selectedProxy.port}` }}
        </a-typography-paragraph>
      </a-card>
    </div>
  </div>


  <!-- 添加/编辑代理对话框 -->
  <a-modal v-model:visible="dialogVisible"
    :title="dialogType === 'add' ? $t('proxy.settings.add_proxy') : $t('proxy.settings.edit_proxy')" @ok="saveProxy"
    @cancel="dialogVisible = false" :ok-loading="dialogLoading">
    <a-form :model="proxyForm" auto-label-width>
      <a-form-item field="interface" :label="$t('proxy.settings.interface')">
        <a-select v-model="proxyForm.interface">
          <a-option value="0.0.0.0">{{ $t('proxy.settings.all_interfaces') }}</a-option>
          <a-option value="127.0.0.1">{{ $t('proxy.settings.localhost') }}</a-option>
        </a-select>
      </a-form-item>

      <a-form-item field="port" :label="$t('proxy.settings.port')">
        <a-input-number v-model="proxyForm.port" :min="1" :max="65535" style="width: 100%" />
      </a-form-item>

      <a-form-item field="httpVersion" :label="$t('proxy.settings.http_version')">
        <a-radio-group v-model="proxyForm.httpVersion">
          <a-radio :value="1">HTTP/1.1</a-radio>
          <a-radio :value="2">HTTP/2</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item field="options" :label="$t('proxy.settings.proxy_options')">
        <a-space direction="vertical" style="width: 100%">
          <a-checkbox v-model="proxyForm.httpsEnabled">
            {{ $t('proxy.settings.https') }}
            <template #extra>
              <div class="option-description">{{ $t('proxy.settings.https_description') }}</div>
            </template>
          </a-checkbox>
        </a-space>
      </a-form-item>
    </a-form>
  </a-modal>

  <!-- 添加/编辑规则对话框 -->
  <a-modal v-model:visible="ruleDialogVisible"
    :title="ruleDialogType === 'add' ? $t('proxy.settings.add_rule') : $t('proxy.settings.edit_rule')" @ok="saveRule"
    @cancel="ruleDialogVisible = false" :ok-loading="ruleDialogLoading">
    <a-form :model="ruleForm" auto-label-width>
      <a-form-item field="enabled" :label="$t('proxy.settings.enable_rule')">
        <a-switch v-model="ruleForm.enabled" />
      </a-form-item>

      <a-form-item field="operator" :label="$t('proxy.settings.operator')">
        <a-radio-group v-model="ruleForm.operator">
          <a-radio value="and">{{ $t('proxy.settings.and') }}</a-radio>
          <a-radio value="or">{{ $t('proxy.settings.or') }}</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item field="matchType" :label="$t('proxy.settings.match_type')">
        <a-select v-model="ruleForm.matchType">
          <a-option value="domain">{{ $t('proxy.settings.domain') }}</a-option>
          <a-option value="ip">{{ $t('proxy.settings.ip') }}</a-option>
          <a-option value="protocol">{{ $t('proxy.settings.protocol') }}</a-option>
          <a-option value="method">{{ $t('proxy.settings.method') }}</a-option>
          <a-option value="extension">{{ $t('proxy.settings.extension') }}</a-option>
          <a-option value="path">{{ $t('proxy.settings.path') }}</a-option>
          <a-option value="header">{{ $t('proxy.settings.header') }}</a-option>
          <a-option value="statusCode">{{ $t('proxy.settings.status_code') }}</a-option>
        </a-select>
      </a-form-item>

      <a-form-item field="matchRelationship" :label="$t('proxy.settings.match_relationship')">
        <a-radio-group v-model="ruleForm.matchRelationship">
          <a-radio value="matches">{{ $t('proxy.settings.matches') }}</a-radio>
          <a-radio value="not_matches">{{ $t('proxy.settings.not_matches') }}</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item field="matchCondition" :label="$t('proxy.settings.match_condition')">
        <a-input v-model="ruleForm.matchCondition" :placeholder="$t('proxy.settings.match_condition_placeholder')" />
        <template #help>
          <span>
            {{ $t('proxy.settings.match_condition_description') }}
            <template v-if="ruleForm.matchType === 'header'">
              {{ $t('proxy.settings.header_condition_description') }}
            </template>
          </span>
        </template>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted, computed,onUnmounted} from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { save } from '@tauri-apps/plugin-dialog';
import { TableData } from '@arco-design/web-vue/es/table/interface';

// 定义代理配置接口
interface ProxyConfig {
  id: string;
  interface: string;
  port: number;
  httpVersion: 1 | 2;
  httpsEnabled: boolean;
  running: boolean;
  loading: boolean;
}

// 定义拦截规则接口
interface InterceptionRule {
  id: string;
  enabled: boolean;
  operator: 'and' | 'or';
  matchType: 'domain' | 'ip' | 'protocol' | 'method' | 'extension' | 'path' | 'header' | 'statusCode';
  matchRelationship: 'matches' | 'not_matches';
  matchCondition: string;
}

const { t } = useI18n();
const loading = ref(false);
const selectedKeys = ref<string[]>([]);
const dialogVisible = ref(false);
const dialogLoading = ref(false);
const dialogType = ref<'add' | 'edit'>('add');

// 代理列表
const proxyList = ref<ProxyConfig[]>([]);

// 选中的代理
const selectedProxy = computed(() => {
  if (selectedKeys.value.length === 0) return null;
  const id = selectedKeys.value[0];
  return proxyList.value.find(proxy => proxy.id === id);
});

// 代理表单
const proxyForm = reactive<ProxyConfig>({
  id: '',
  interface: '127.0.0.1',
  port: 8080,
  httpVersion: 2,
  httpsEnabled: true,
  running: false,
  loading: false
});

// 拦截状态
const interceptRequestEnabled = ref(true);
const interceptResponseEnabled = ref(false);

// 规则列表
const requestRules = ref<InterceptionRule[]>([]);
const responseRules = ref<InterceptionRule[]>([]);

// 选中的规则
const selectedRequestRuleKeys = ref<string[]>([]);
const selectedResponseRuleKeys = ref<string[]>([]);

// 规则对话框控制
const ruleDialogVisible = ref(false);
const ruleDialogLoading = ref(false);
const ruleDialogType = ref<'add' | 'edit'>('add');
const currentRuleType = ref<'request' | 'response'>('request');

// 规则表单
const ruleForm = reactive<InterceptionRule>({
  id: '',
  enabled: true,
  operator: 'and',
  matchType: 'domain',
  matchRelationship: 'matches',
  matchCondition: ''
});

// 计算属性：选中的规则
const selectedRequestRule = computed(() => {
  if (selectedRequestRuleKeys.value.length === 0) return null;
  const id = selectedRequestRuleKeys.value[0];
  return requestRules.value.find(rule => rule.id === id);
});

const selectedResponseRule = computed(() => {
  if (selectedResponseRuleKeys.value.length === 0) return null;
  const id = selectedResponseRuleKeys.value[0];
  return responseRules.value.find(rule => rule.id === id);
});

// 添加一个通用转换函数
const convertRuleToBackend = (rule: InterceptionRule) => ({
  id: rule.id,
  enabled: rule.enabled,
  operator: rule.operator,
  match_type: rule.matchType,
  match_relationship: rule.matchRelationship,
  match_condition: rule.matchCondition
});

// 添加从后端转换到前端的函数
const convertRuleToFrontend = (rule: any): InterceptionRule => ({
  id: rule.id,
  enabled: rule.enabled,
  operator: rule.operator,
  matchType: rule.match_type,
  matchRelationship: rule.match_relationship,
  matchCondition: rule.match_condition
});

onUnmounted(() => {
  window.removeEventListener('proxy-intercept-status-change', () => {});
  window.removeEventListener('proxy-intercept-response-status-change', () => {});
  window.removeEventListener('proxy-status-change', () => {});
  window.removeEventListener('proxy-error', () => {});
});

onMounted(async () => {
  await loadProxyList();

  window.addEventListener('proxy-intercept-status-change', (event: any) => {
    if (event.detail && typeof event.detail.enabled === 'boolean') {
      interceptRequestEnabled.value = event.detail.enabled;
    }
  });

  window.addEventListener('proxy-intercept-response-status-change', (event: any) => {
    if (event.detail && typeof event.detail.enabled === 'boolean') {
      interceptResponseEnabled.value = event.detail.enabled;
    }
  });

  window.addEventListener('proxy-status-change', (event: any) => {
    if (event.detail && event.detail.id) {
      const id = event.detail.id;
      const proxy = proxyList.value.find(p => p.id === id);
      if (proxy) {
        proxy.running = event.detail.status;
        proxy.loading = false;
      }
    }
  });

  // 监听代理错误事件
  window.addEventListener('proxy-error', (event: any) => {
    if (event.detail && event.detail.message) {
      Message.error(event.detail.message);
      // 发生错误时强制刷新状态
      setTimeout(loadProxyList, 1000);
    }
  });

  // 加载拦截状态和规则
  try {
    // 加载拦截状态
    interceptRequestEnabled.value = await invoke('get_proxy_intercept_request_status');

    // 加载响应拦截状态
    const settings = await invoke<{ intercept_response?: boolean }>('get_proxy_settings');
    interceptResponseEnabled.value = !!settings.intercept_response;

    // 加载拦截规则
    const requestRulesData = await invoke<any[]>('get_request_rules');
    if (Array.isArray(requestRulesData)) {
      requestRules.value = requestRulesData.map(convertRuleToFrontend);
    }

    const responseRulesData = await invoke<any[]>('get_response_rules');
    if (Array.isArray(responseRulesData)) {
      responseRules.value = responseRulesData.map(convertRuleToFrontend);
    }
  } catch (error) {
    console.error('加载拦截规则失败:', error);
    Message.error(`加载拦截规则失败: ${error}`);
  }
});

// 加载代理列表
const loadProxyList = async () => {
  loading.value = true;

  try {
    // 加载代理配置列表
    const configList = await invoke<any[]>('get_proxy_configs');
    if (configList && Array.isArray(configList)) {
      proxyList.value = configList.map(config => {
        // 确保httpVersion的值是1或2
        const httpVer = Number(config.http_version) || 1;
        const version = httpVer === 2 ? 2 : 1;

        return {
          id: config.id || `proxy_${Date.now()}`,
          interface: config.interface || '127.0.0.1',
          port: Number(config.port) || 8080,
          httpVersion: version as (1 | 2),
          httpsEnabled: Boolean(config.https_enabled),
          loading: false,
          running: false
        };
      });

      // 检查各个代理的运行状态
      for (const proxy of proxyList.value) {
        const status = await invoke<boolean>('get_proxy_status', { id: proxy.id });
        proxy.running = !!status;
      }
    }
  } catch (error) {
    Message.error(`${t('proxy.settings.load_error')}: ${error}`);
    proxyList.value = [];
  } finally {
    loading.value = false;
  }
};

// 选择行
const onSelectRow = (record: TableData) => {
  const proxyRecord = record as unknown as ProxyConfig;
  selectedKeys.value = [proxyRecord.id];
};

// 打开代理对话框
const openProxyDialog = (type: 'add' | 'edit') => {
  dialogType.value = type;
  dialogVisible.value = true;

  if (type === 'add') {
    // 重置表单
    Object.assign(proxyForm, {
      id: '',
      interface: '127.0.0.1',
      port: 8080,
      httpVersion: 2,
      httpsEnabled: true,
      running: false,
      loading: false
    });
  } else if (type === 'edit' && selectedProxy.value) {
    // 填充表单
    Object.assign(proxyForm, { ...selectedProxy.value });
  }
};

// 保存代理
const saveProxy = async () => {
  dialogLoading.value = true;

  try {
    // 生成唯一ID（如果是添加新代理）
    if (dialogType.value === 'add') {
      proxyForm.id = `proxy_${Date.now()}`;
    }

    // 调用后端保存代理配置
    await invoke('save_proxy_config_with_id', {
      id: proxyForm.id,
      config: {
        interface: proxyForm.interface,
        port: proxyForm.port,
        http_version: proxyForm.httpVersion,
        https_enabled: proxyForm.httpsEnabled,
      }
    });

    Message.success(dialogType.value === 'add'
      ? t('proxy.settings.add_success')
      : t('proxy.settings.edit_success'));

    // 刷新代理列表
    await loadProxyList();

    // 关闭对话框
    dialogVisible.value = false;
  } catch (error) {
    Message.error(`${dialogType.value === 'add'
      ? t('proxy.settings.add_error')
      : t('proxy.settings.edit_error')}: ${error}`);
  } finally {
    dialogLoading.value = false;
  }
};

// 确认删除
const confirmDelete = () => {
  if (!selectedProxy.value) return;

  Modal.warning({
    title: t('proxy.settings.delete_confirm_title'),
    content: t('proxy.settings.delete_confirm_content'),
    okText: t('proxy.settings.delete'),
    cancelText: t('common.cancel'),
    onOk: deleteProxy
  });
};

// 删除代理
const deleteProxy = async () => {
  if (!selectedProxy.value) return;

  loading.value = true;

  try {
    // 如果代理正在运行，需要先停止
    if (selectedProxy.value.running) {
      await invoke('stop_proxy_by_id', { id: selectedProxy.value.id });
    }

    // 删除代理配置
    await invoke('delete_proxy_config', { id: selectedProxy.value.id });

    Message.success(t('proxy.settings.delete_success'));

    // 刷新代理列表
    await loadProxyList();

    // 清空选择
    selectedKeys.value = [];
  } catch (error) {
    Message.error(`${t('proxy.settings.delete_error')}: ${error}`);
  } finally {
    loading.value = false;
  }
};

// 切换代理状态
const toggleProxyStatus = async (proxy: ProxyConfig, status: boolean) => {
  // 标记为加载中
  proxy.loading = true;

  try {
    if (status) {
      // 启动代理
      await invoke('start_proxy_by_id', { id: proxy.id });

      // 等待一点时间让代理启动
      await new Promise(resolve => setTimeout(resolve, 1000));

      // 验证代理是否成功启动
      const runningStatus = await invoke<boolean>('get_proxy_status_by_id', { id: proxy.id });
      proxy.running = !!runningStatus;

      if (!proxy.running) {
        throw new Error(t('proxy.settings.start_error'));
      }

      Message.success(t('proxy.settings.started_success'));
    } else {
      // 停止代理
      await invoke('stop_proxy_by_id', { id: proxy.id });

      // 等待一点时间让代理停止
      await new Promise(resolve => setTimeout(resolve, 6000));

      // 验证代理是否成功停止
      const runningStatus = await invoke<boolean>('get_proxy_status_by_id', { id: proxy.id });
      proxy.running = !!runningStatus;

      if (proxy.running) {
        throw new Error(t('proxy.settings.stop_error'));
      }

      Message.success(t('proxy.settings.stopped_success'));
    }
  } catch (error) {
    Message.error(`${status ? t('proxy.settings.start_error') : t('proxy.settings.stop_error')}: ${error}`);

    // 回滚UI状态
    proxy.running = !status;
  } finally {
    proxy.loading = false;
  }
};

// 生成CA证书
const generateCACertificate = async () => {
  try {
    await invoke('generate_ca_certificate');
    Message.success(t('proxy.settings.ca_generated'));
  } catch (error) {
    Message.error(`${t('proxy.settings.ca_generation_error')}: ${error}`);
  }
};

// 导出CA证书
const exportCACertificate = async () => {
  try {
    const path = await save({
      filters: [{
        name: 'Certificate',
        extensions: ['crt', 'key']
      }],
      defaultPath: 'rshield-ca.crt'
    });

    if (path) {
      await invoke('export_ca_certificate', { path });
      Message.success(t('proxy.settings.ca_exported'));
    }
  } catch (error) {
    Message.error(`${t('proxy.settings.ca_export_error')}: ${error}`);
  }
};

// 切换请求拦截状态
const toggleInterceptRequest = async (value: boolean | (string | number | boolean)[]) => {
  const enabled = typeof value === 'boolean' ? value : Boolean(value[0]);
  try {
    await invoke('set_proxy_intercept_request_status', { enabled });
    interceptRequestEnabled.value = enabled;
    Message.success(enabled ? '已启用请求拦截规则' : '已禁用请求拦截规则');
  } catch (error) {
    Message.error(`设置拦截状态失败: ${error}`);
    interceptRequestEnabled.value = !enabled;
  }
};

// 切换响应拦截状态
const toggleInterceptResponse = async (value: any) => {
  try {
    const result = await invoke<boolean>('set_proxy_intercept_response_status', { enabled: value });
    if (result) {
      interceptResponseEnabled.value = value;
      Message.success(value ? '已启用响应拦截规则' : '已禁用响应拦截规则');
    } else {
      throw new Error('操作未成功完成');
    }
  } catch (error) {
    Message.error(`设置响应拦截状态失败: ${error}`);
    interceptResponseEnabled.value = !value; // 恢复状态
  }
};

// 获取匹配类型标签
const getMatchTypeLabel = (type: string): string => {
  const typeMap: Record<string, string> = {
    'domain': '域名',
    'ip': 'IP地址',
    'protocol': '协议',
    'method': 'HTTP方法',
    'extension': '文件扩展名',
    'path': '路径',
    'header': '请求头',
    'statusCode': '状态码'
  };

  return typeMap[type] || type;
};

// 选择规则
const onSelectRule = (type: 'request' | 'response', record: TableData) => {
  const ruleRecord = record as unknown as InterceptionRule;
  if (type === 'request') {
    selectedRequestRuleKeys.value = [ruleRecord.id];
    selectedResponseRuleKeys.value = [];
  } else {
    selectedResponseRuleKeys.value = [ruleRecord.id];
    selectedRequestRuleKeys.value = [];
  }
};

// 打开规则对话框
const openRuleDialog = (type: 'request' | 'response', action: 'add' | 'edit') => {
  currentRuleType.value = type;
  ruleDialogType.value = action;
  ruleDialogVisible.value = true;

  if (action === 'add') {
    // 重置表单
    Object.assign(ruleForm, {
      id: '',
      enabled: true,
      operator: 'and',
      matchType: 'domain',
      matchRelationship: 'matches',
      matchCondition: ''
    });
  } else if (action === 'edit') {
    // 填充表单
    const selectedRule = type === 'request' ? selectedRequestRule.value : selectedResponseRule.value;
    if (selectedRule) {
      Object.assign(ruleForm, { ...selectedRule });
    }
  }
};

// 保存规则
const saveRule = async () => {
  ruleDialogLoading.value = true;

  try {
    // 生成唯一ID（如果是添加新规则）
    if (ruleDialogType.value === 'add') {
      ruleForm.id = `rule_${Date.now()}`;
    }

    // 保存规则
    if (currentRuleType.value === 'request') {
      // 添加新规则或更新现有规则
      if (ruleDialogType.value === 'add') {
        requestRules.value.push({ ...ruleForm });
      } else {
        const index = requestRules.value.findIndex(r => r.id === ruleForm.id);
        if (index >= 0) {
          requestRules.value[index] = { ...ruleForm };
        }
      }

      // 调用后端API保存规则
      await invoke('set_request_rules', {
        rules: requestRules.value.map(convertRuleToBackend)
      });
    } else {
      // 添加新规则或更新现有规则
      if (ruleDialogType.value === 'add') {
        responseRules.value.push({ ...ruleForm });
      } else {
        const index = responseRules.value.findIndex(r => r.id === ruleForm.id);
        if (index >= 0) {
          responseRules.value[index] = { ...ruleForm };
        }
      }

      // 调用后端API保存规则
      await invoke('set_response_rules', {
        rules: responseRules.value.map(convertRuleToBackend)
      });
    }

    Message.success(ruleDialogType.value === 'add' ? t('proxy.settings.rule_added') : t('proxy.settings.rule_updated'));
    ruleDialogVisible.value = false;
  } catch (error) {
    Message.error(`保存规则失败: ${error}`);
  } finally {
    ruleDialogLoading.value = false;
  }
};


// 切换规则状态
const toggleRuleStatus = async (type: 'request' | 'response', ruleId: string, enabled: any) => {
  try {
    if (type === 'request') {
      const rule = requestRules.value.find(r => r.id === ruleId);
      if (rule) {
        rule.enabled = enabled;
        // 调用后端API保存规则
        await invoke('set_request_rules', {
          rules: requestRules.value.map(convertRuleToBackend)
        });
        Message.success(`规则已${enabled ? '启用' : '禁用'}`);
      }
    } else {
      const rule = responseRules.value.find(r => r.id === ruleId);
      if (rule) {
        rule.enabled = enabled;
        // 调用后端API保存规则
        await invoke('set_response_rules', {
          rules: responseRules.value.map(convertRuleToBackend)
        });
        Message.success(`规则已${enabled ? '启用' : '禁用'}`);
      }
    }
  } catch (error) {
    Message.error(`更新规则状态失败: ${error}`);
  }
};

// 上移或下移规则
const moveRule = async (type: 'request' | 'response', direction: 'up' | 'down') => {
  try {
    const rules = type === 'request' ? requestRules.value : responseRules.value;
    const selectedRule = type === 'request' ? selectedRequestRule.value : selectedResponseRule.value;

    if (!selectedRule) return;

    const index = rules.findIndex(r => r.id === selectedRule.id);
    if (index < 0) return;

    if (direction === 'up' && index > 0) {
      // 上移
      [rules[index], rules[index - 1]] = [rules[index - 1], rules[index]];
    } else if (direction === 'down' && index < rules.length - 1) {
      // 下移
      [rules[index], rules[index + 1]] = [rules[index + 1], rules[index]];
    }

    // 调用后端API保存规则
    if (type === 'request') {
      await invoke('set_request_rules', {
        rules: requestRules.value.map(convertRuleToBackend)
      });
    } else {
      await invoke('set_response_rules', {
        rules: responseRules.value.map(convertRuleToBackend)
      });
    }

    Message.success(t('proxy.settings.rule_order_updated'));
  } catch (error) {
    Message.error(`更新规则排序失败: ${error}`);
  }
};

// 检查是否为第一条规则
const isFirstRule = (type: 'request' | 'response'): boolean => {
  const rules = type === 'request' ? requestRules.value : responseRules.value;
  const selectedRule = type === 'request' ? selectedRequestRule.value : selectedResponseRule.value;

  if (!selectedRule || rules.length === 0) return true;

  const index = rules.findIndex(r => r.id === selectedRule.id);
  return index === 0;
};

// 检查是否为最后一条规则
const isLastRule = (type: 'request' | 'response'): boolean => {
  const rules = type === 'request' ? requestRules.value : responseRules.value;
  const selectedRule = type === 'request' ? selectedRequestRule.value : selectedResponseRule.value;

  if (!selectedRule || rules.length === 0) return true;

  const index = rules.findIndex(r => r.id === selectedRule.id);
  return index === rules.length - 1;
};
</script>

<style scoped lang="less">
.settings-section {
  margin-bottom: 24px;
}

.section-header {
  margin-bottom: 16px;

  h3 {
    margin-bottom: 8px;
  }

  p {
    color: var(--color-text-3);
  }
}

.rules-container {
  margin-top: 16px;
}

.settings-header {
  margin-bottom: 8px;

  h3 {
    margin-bottom: 8px;
  }

  p {
    color: var(--color-text-3);
  }
}

.proxy-settings-layout {
  display: flex;
  gap: 24px;

  .proxy-actions {
    width: 250px;
    flex-shrink: 0;
  }

  .proxy-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
}

.ca-card,
.help-card {
  width: 100%;
}

.ca-description {
  margin-bottom: 16px;

  p {
    margin-bottom: 8px;
  }
}

.help-info {
  margin-top: 16px;
}

.option-description {
  color: var(--color-text-3);
  font-size: 12px;
  margin-top: 4px;
  margin-left: 24px;
}

.settings-title {
  margin-bottom: 16px;

  h2 {
    margin-bottom: 0;
    font-size: 18px;
    font-weight: 500;
    color: var(--color-text-1);
  }
}
</style>