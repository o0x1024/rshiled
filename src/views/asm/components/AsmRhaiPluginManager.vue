<template>
  <a-space direction="vertical" fill>
    <a-typography-text style="font-size: large; font-weight: 540;">{{ $t('asm_plugin.title') }}</a-typography-text>
    <a-row >
      <a-col :span="24">
        <a-button type="primary" size="small" @click="loadPlugins" :loading="loading">
          {{ $t('asm_plugin.refresh') }}
        </a-button>
        <a-button type="primary" style="margin-left: 8px" size="small" @click="reloadFromDisk" :loading="reloading">
          {{ $t('asm_plugin.reload') }}
        </a-button>
        <a-button type="primary" style="margin-left: 8px" size="small" @click="showUploadModal">
          {{ $t('asm_plugin.upload') }}
        </a-button>
        <a-button type="primary" style="margin-left: 8px" size="small" @click="showAddPluginModal">
          {{ $t('asm_plugin.addPlugin') }}
        </a-button>
      </a-col>
    </a-row>

    <a-table 
      :bordered="false" 
      :loading="loading" 
      :columns="columns" 
      :data="plugins" 
      :pagination="pagination"
      @page-change="onPageChange"
      @page-size-change="onPageSizeChange"
      size="small"
    >
      <template #name="{ record }">
        <a-link @click="viewPluginDetails(record)">{{ record.name }}</a-link>
      </template>

      <template #Operations="{ record }">
        <a-dropdown>
          <div class="clickable"><icon-more /></div>
          <template #content>
            <a-doption @click="viewPluginDetails(record)">
              <template #icon>
                <icon-search />
              </template>
              <template #default>{{ $t('asm_plugin.view') }}</template>
            </a-doption>
            <a-doption @click="editPlugin(record)">
              <template #icon>
                <icon-edit />
              </template>
              <template #default>{{ $t('asm_plugin.edit') }}</template>
            </a-doption>
            <a-doption @click="executePlugin(record)">
              <template #icon>
                <icon-play-circle />
              </template>
              <template #default>{{ $t('asm_plugin.execute') }}</template>
            </a-doption>
            <a-doption @click="confirmDelete(record)">
              <template #icon>
                <icon-delete />
              </template>
              <template #default>{{ $t('asm_plugin.delete') }}</template>
            </a-doption>
          </template>
        </a-dropdown>
      </template>
    </a-table>

    <!-- 上传插件模态框 -->
    <a-modal v-model:visible="uploadModalVisible" :title="$t('asm_plugin.upload_title')" @ok="handleUpload"
      :ok-button-props="{ disabled: !uploadForm.file }">
      <a-upload :file-list="fileList" :custom-request="() => { }" @change="handleFileChange" :multiple="false" :limit="1"
        accept=".rhai">
        <template #upload-button>
          <a-button>
            {{ $t('asm_plugin.select_file') }}
          </a-button>
        </template>
      </a-upload>
    </a-modal>

    <!-- 执行插件模态框 -->
    <a-modal v-model:visible="executeModalVisible" :title="$t('asm_plugin.execute_title')" @ok="handleExecute"
      :confirmLoading="executeLoading" width="700px">
      <a-form :model="executeForm" layout="vertical">
        <a-form-item :label="$t('asm_plugin.target')" name="target">
          <a-input v-model="executeForm.target" :placeholder="$t('asm_plugin.target_placeholder')" />
        </a-form-item>

        <a-form-item :label="$t('asm_plugin.task_id')" name="task_id">
          <a-input-number v-model="executeForm.task_id" :placeholder="$t('asm_plugin.task_id_placeholder')" />
        </a-form-item>

        <!-- 动态参数 -->
        <template v-if="currentPlugin && currentPlugin.params && currentPlugin.params.length > 0">
          <!-- 添加代理URL参数 -->
          <a-form-item :label="$t('asm_plugin.proxy_url')" :required="false">
            <a-input v-model="executeForm.customParams['proxy_url']"
              :placeholder="$t('asm_plugin.proxy_url_placeholder')" />
            <div class="parameter-help-text">
              {{ $t('asm_plugin.proxy_url_help') }}
            </div>
          </a-form-item>

          <!-- 其他参数 -->
          <a-divider>{{ $t('asm_plugin.plugin_parameters') }}</a-divider>

          <div class="parameter-section-description">
            {{ $t('asm_plugin.parameters_section_description') }}
          </div>

          <a-form-item v-for="param in currentPlugin.params.filter(p => p.key !== 'proxy_url')" :key="param.key"
            :label="param.name" :required="param.required">
            <a-input v-if="param.type === 'string'" v-model="executeForm.customParams[param.key]"
              :placeholder="param.description"
              :status="param.required && !executeForm.customParams[param.key] ? 'error' : ''" />
            <a-input-number v-else-if="param.type === 'number'" v-model="executeForm.customParams[param.key]"
              :placeholder="param.description"
              :status="param.required && executeForm.customParams[param.key] === undefined ? 'error' : ''" />
            <a-textarea v-else-if="param.type === 'textarea'" v-model="executeForm.customParams[param.key]"
              :placeholder="param.description"
              :status="param.required && !executeForm.customParams[param.key] ? 'error' : ''" />
            <a-switch v-else-if="param.type === 'boolean'" v-model="executeForm.customParams[param.key]" />
            <a-select v-else-if="param.type === 'select'" v-model="executeForm.customParams[param.key]"
              :placeholder="param.description"
              :status="param.required && executeForm.customParams[param.key] === undefined ? 'error' : ''">
              <a-option v-for="option in param.options || []" :key="option.value" :value="option.value">
                {{ option.label }}
              </a-option>
            </a-select>

            <div class="parameter-help-text">
              <template v-if="param.required">
                <a-tag color="blue" size="small" style="margin-right: 8px;">{{ $t('asm_plugin.required') }}</a-tag>
              </template>
              <span>{{ param.description }}</span>
              <template v-if="param.default !== undefined">
                <span class="parameter-default-value">
                  {{ $t('asm_plugin.default_value') }}: {{ param.default }}
                </span>
              </template>
            </div>
          </a-form-item>
        </template>
      </a-form>

      <a-divider />

      <div v-if="executeResult">
        <h3>{{ $t('asm_plugin.execute_results') }}</h3>
        <a-descriptions bordered>
          <a-descriptions-item :label="$t('asm_plugin.status')" :span="3">
            <a-tag :color="executeResult.success ? 'success' : 'error'">
              {{ executeResult.success ? $t('asm_plugin.success') : $t('asm_plugin.failure') }}
            </a-tag>
          </a-descriptions-item>
          <a-descriptions-item :label="$t('asm_plugin.message')" :span="3">
            {{ executeResult.message }}
          </a-descriptions-item>
          <a-descriptions-item v-if="executeResult.raw_output" :label="$t('asm_plugin.raw_output')" :span="3">
            {{ executeResult.raw_output }}
          </a-descriptions-item>
        </a-descriptions>

        <a-divider />

        <h3 v-if="executeResult.data">{{ $t('asm_plugin.data') }}</h3>
        <pre v-if="executeResult.data" class="result-data">{{ JSON.stringify(executeResult.data, null, 2) }}</pre>

        <template v-if="executeResult.found_domains && executeResult.found_domains.length > 0">
          <h3>{{ $t('asm_plugin.found_domains') }}</h3>
          <a-card>
            <template v-for="(domain, index) in executeResult.found_domains" :key="index">
              <a-tag style="margin: 4px">{{ domain }}</a-tag>
            </template>
          </a-card>
        </template>

        <template v-if="executeResult.found_risks && executeResult.found_risks.length > 0">
          <h3>{{ $t('asm_plugin.found_risks') }}</h3>
          <a-table :data="executeResult.found_risks" :pagination="false" bordered>
            <template #columns>
              <a-table-column title="风险名称" data-index="name" />
              <a-table-column title="风险类型" data-index="type" />
              <a-table-column title="风险等级" data-index="level" />
              <a-table-column title="风险描述" data-index="description" />
            </template>
          </a-table>
        </template>

        <template v-if="executeResult.found_fingerprints && executeResult.found_fingerprints.length > 0">
          <h3>{{ $t('asm_plugin.found_fingerprints') }}</h3>
          <a-table :data="executeResult.found_fingerprints" :pagination="false" bordered>
            <template #columns>
              <a-table-column title="指纹名称" data-index="name" />
              <a-table-column title="指纹版本" data-index="version" />
              <a-table-column title="指纹类型" data-index="type" />
            </template>
          </a-table>
        </template>

        <template v-if="executeResult.found_ports && executeResult.found_ports.length > 0">
          <h3>{{ $t('asm_plugin.found_ports') }}</h3>
          <a-table :data="executeResult.found_ports" :pagination="false" bordered>
            <template #columns>
              <a-table-column title="IP" data-index="ip" />
              <a-table-column title="端口" data-index="port" />
              <a-table-column title="服务" data-index="service" />
              <a-table-column title="状态" data-index="status" />
            </template>
          </a-table>
        </template>
      </div>
    </a-modal>

    <!-- 查看插件详情模态框 -->
    <a-modal v-model:visible="detailsModalVisible" :title="$t('asm_plugin.view_details')" width="800px" :footer="false">
      <div class="dark-mode-toggle">
        <span>{{ $t('asm_plugin.dark_mode') }}</span>
        <a-switch v-model="isDarkMode" @change="onToggleDarkMode" />
      </div>
      <a-descriptions :title="currentPlugin?.name || ''" bordered :column="1">
        <a-descriptions-item :label="$t('asm_plugin.plugin_name')">
          {{ currentPlugin?.name || '' }}
        </a-descriptions-item>
        <a-descriptions-item :label="$t('asm_plugin.plugin_author')">
          {{ currentPlugin?.author || '' }}
        </a-descriptions-item>
        <a-descriptions-item :label="$t('asm_plugin.plugin_type')">
          <a-tag>{{ currentPlugin?.plugin_type || '' }}</a-tag>
        </a-descriptions-item>
        <a-descriptions-item :label="$t('asm_plugin.plugin_version')">
          {{ currentPlugin?.version || '' }}
        </a-descriptions-item>
        <a-descriptions-item v-if="currentPlugin?.severity" :label="$t('asm_plugin.plugin_severity')">
          <a-tag :color="getSeverityColor(currentPlugin.severity)">{{ currentPlugin?.severity || '' }}</a-tag>
        </a-descriptions-item>
        <a-descriptions-item :label="$t('asm_plugin.plugin_description')">
          {{ currentPlugin?.description || '' }}
        </a-descriptions-item>
        <a-descriptions-item v-if="currentPlugin?.references && currentPlugin.references.length > 0"
          :label="$t('asm_plugin.plugin_references')">
          <div v-for="(ref, index) in currentPlugin?.references" :key="index">
            <a-link :href="ref" target="_blank">{{ ref }}</a-link>
          </div>
        </a-descriptions-item>
        <a-descriptions-item v-if="currentPlugin?.params && currentPlugin.params.length > 0"
          :label="$t('asm_plugin.params')">
          <div v-for="(param, index) in currentPlugin?.params || []" :key="index">
            <a-tag color="blue">{{ param.name }}</a-tag>
            <span class="param-key">({{ param.key }})</span>
            <span v-if="param.required" class="param-required">*</span>
            <div class="param-desc">{{ param.description }}</div>
          </div>
        </a-descriptions-item>
      </a-descriptions>

      <a-divider />

      <div class="code-section">
        <h3>{{ $t('asm_plugin.plugin_script') }}</h3>
        <div class="code-container" :class="{ 'dark-mode': isDarkMode }">
          <pre><code>{{ currentPlugin?.script || '' }}</code></pre>
        </div>
      </div>
    </a-modal>
    <!-- 添加/编辑插件模态框 -->
    <a-modal v-model:visible="addPluginModalVisible"
      :title="isEditingMode ? $t('asm_plugin.edit_title') : $t('asm_plugin.add_title')" @ok="handleAddPlugin"
      :ok-text="$t('common.save')" :cancel-text="$t('common.cancel')" :confirmLoading="addPluginLoading" width="90%">
      <div class="plugin-form-container">
        <!-- 脚本编辑器 -->
        <div class="script-editor-section">
          <div class="script-header">
            <h3>{{ $t('asm_plugin.script_editor_title') }}</h3>
            <div class="script-actions">
              <a-space>
                <a-button type="outline" @click="showTemplateModal">
                  <template #icon><icon-file /></template>
                  {{ $t('asm_plugin.load_template') }}
                </a-button>
                <a-button type="outline" @click="toggleDarkMode">
                  <template #icon><icon-moon /></template>
                  {{ $t('asm_plugin.dark_mode') }}
                </a-button>
                <a-button type="outline" @click="validateScript">
                  <template #icon><icon-check /></template>
                  {{ $t('asm_plugin.validate') }}
                </a-button>
              </a-space>
            </div>
          </div>

          <div class="code-editor-container" :class="{ 'dark-mode': isDarkMode }">
            <codemirror v-model="newPluginScript" :style="{ height: '600px', width: '100%', fontSize: '12px' }"
              :autofocus="true" :indent-with-tab="true" :tab-size="2" :extensions="extensions" @ready="handleReady" />
          </div>
        </div>
      </div>
    </a-modal>
  </a-space>
</template>

<script setup>
import { ref, reactive, onMounted, computed, h } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { Codemirror } from 'vue-codemirror';
import { rust } from '@codemirror/lang-rust';
import { oneDark } from '@codemirror/theme-one-dark';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

// 上传表单
const uploadForm = reactive({
  file: null
});

// 分页配置
const pagination = reactive({
  total: 0,
  current: 1,
  pageSize: 10,
  showTotal: true,
  showJumper: true,
  showPageSize: true,
  pageSizeOptions: [10, 20, 50, 100]
});

// 插件数据
const allPlugins = ref([]);
const plugins = ref([]);
const loading = ref(false);
const reloading = ref(false);
const currentPlugin = ref(null);
const uploadModalVisible = ref(false);
const executeModalVisible = ref(false);
const detailsModalVisible = ref(false);
const editModalVisible = ref(false);
const executeLoading = ref(false);
const executeResult = ref(null);
const fileList = ref([]);
const isDarkMode = ref(false);
const isEditing = ref(false);

// 执行表单
const executeForm = ref({
  target: '',
  task_id: null,
  customParams: {}
});

// 添加新的状态变量 - 多步骤向导
const addPluginModalVisible = ref(false);
const addPluginLoading = ref(false);
const newPlugin = ref({
  name: '',
  type: 'web',
  description: '',
  author: '',
  version: '1.0.0'
});
const newPluginScript = ref('');
const currentStep = ref(1);
const pluginParams = ref([]);
const parameterModalVisible = ref(false);
const editingParam = ref({
  key: '',
  name: '',
  type: 'string',
  required: false,
  description: ''
});
const editingParamIndex = ref(-1);
const validationResult = ref(null);
const validationCompleted = ref(false);
const editingPluginId = ref(null);
const isEditingMode = ref(false);

// 添加一个新的 ref 来控制模板选择模态框的可见性
const templateModalVisible = ref(false);

// 代码编辑器配置
const extensions = computed(() => [
  rust(),
  isDarkMode.value ? oneDark : []
]);

// 表格列定义
const columns = [
  {
    title: t('asm_plugin.name'),
    dataIndex: 'name',
    slotName: 'name',
  },
  {
    title: t('asm_plugin.type'),
    dataIndex: 'plugin_type',
  },
  {
    title: t('asm_plugin.description'),
    dataIndex: 'description',
  },
  {
    title: t('asm_plugin.author'),
    dataIndex: 'author',
  },
  {
    title: t('asm_plugin.version'),
    dataIndex: 'version',
  },
  {
    title: t('asm_plugin.operations'),
    slotName: 'Operations',
    width: 80,
  },
];

// 加载插件列表
const loadPlugins = async () => {
  loading.value = true;
  try {
    const result = await invoke('list_asm_plugins');
    allPlugins.value = result;
    pagination.total = result.length;
    Message.success('插件列表已刷新');
    updatePageData();
  } catch (error) {
    Message.error(t('asm_plugin.load_failed'));
    console.error('Failed to load plugins:', error);
  } finally {
    loading.value = false;
  }
};

// 更新当前页数据
const updatePageData = () => {
  const start = (pagination.current - 1) * pagination.pageSize;
  const end = start + pagination.pageSize;
  plugins.value = allPlugins.value.slice(start, end);
};

// 页码改变事件处理
const onPageChange = (page) => {
  pagination.current = page;
  updatePageData();
};

// 每页条数改变事件处理
const onPageSizeChange = (pageSize) => {
  pagination.pageSize = pageSize;
  pagination.current = 1;
  updatePageData();
};

// 从磁盘重新加载插件
const reloadFromDisk = async () => {
  reloading.value = true;
  try {
    await invoke('load_asm_plugins');
    Message.success(t('asm_plugin.reload_success'));
    await loadPlugins();
  } catch (e) {
    Message.error(t('asm_plugin.reload_failure'));
    console.error(t('asm_plugin.reload_failure'), e);
  } finally {
    reloading.value = false;
  }
};

// 查看插件详情
const viewPluginDetails = async (plugin) => {
  try {
    // 确保使用正确的插件ID格式获取插件
    const pluginId = plugin.id;

    // 检查ID格式，确保是type:name格式
    if (!pluginId.includes(':')) {
      Message.error(t('asm_plugin.invalid_plugin_id_format'));
      return;
    }

    console.log(t('asm_plugin.get_plugin_details'), pluginId);
    const result = await invoke('get_asm_plugin', { pluginId });
    currentPlugin.value = result;
    detailsModalVisible.value = true;
  } catch (e) {
    Message.error(t('asm_plugin.get_plugin_details_failed'));
    console.error(t('asm_plugin.get_plugin_details_error'), e);
  }
};

// 编辑插件
const editPlugin = async (plugin) => {
  isEditingMode.value = true;
  editingPluginId.value = plugin.id;
  newPlugin.value = {
    name: plugin.name,
    description: plugin.description,
    type: plugin.plugin_type || 'web',
    author: plugin.author || '',
    version: plugin.version || '1.0.0'
  };

  try {
    // 使用正确的插件ID格式获取插件
    const pluginId = plugin.id;

    // 检查ID格式，确保是type:name格式
    if (!pluginId.includes(':')) {
      Message.error(t('asm_plugin.invalid_plugin_id_format'));
      return;
    }

    console.log(t('asm_plugin.get_edit_plugin'), pluginId);
    const result = await invoke('get_asm_plugin', { pluginId });
    newPluginScript.value = result.script || '';

    // 尝试从脚本中提取参数
    pluginParams.value = extractParamsFromScript(result.script) || [];

    // 开启向导式修改模式
    currentStep.value = 1;
    validationResult.value = null;
    validationCompleted.value = false;
    addPluginModalVisible.value = true;
  } catch (e) {
    Message.error(t('asm_plugin.get_plugin_script_failed'));
    console.error(t('asm_plugin.get_plugin_script_error'), e);
  }
};

// 显示引导式添加插件模态框
const showAddPluginModal = () => {
  isEditingMode.value = false;
  newPlugin.value = {
    name: '',
    type: 'web',
    description: '',
    author: '',
    version: '1.0.0'
  };
  newPluginScript.value = example_script;

  // 从默认模板中提取基本信息和参数
  extractBasicInfoFromScript(newPluginScript.value);
  pluginParams.value = extractParamsFromScript(newPluginScript.value) || [];

  currentStep.value = 1;
  validationResult.value = null;
  validationCompleted.value = false;
  editingPluginId.value = null;
  addPluginModalVisible.value = true;
};

// 显示上传模态框
const showUploadModal = () => {
  fileList.value = [];
  uploadForm.file = null;
  uploadModalVisible.value = true;
};

// 执行插件
const executePlugin = async (plugin) => {
  try {
    const result = await invoke('test_asm_plugin', { pluginId: plugin.id });
    currentPlugin.value = result;

    // 重置执行表单
    executeForm.value = {
      target: '',
      task_id: null,
      customParams: {}
    };

    executeResult.value = null;
    // executeModalVisible.value = true;
  } catch (e) {
    Message.error(`准备执行插件失败：${e}`);
    console.error('准备执行插件错误:', e);
  }
};
const example_script = '';
// 处理执行插件



const handleExecute = async () => {
  if (!executeForm.value.target) {
    Message.error(t('asm_plugin.target_required'));
    return;
  }

  executeLoading.value = true;

  try {
    const result = await invoke('execute_asm_plugin', {
      pluginId: currentPlugin.value.id,
      target: executeForm.value.target,
      taskId: executeForm.value.task_id,
      customParams: executeForm.value.customParams || {}
    });

    executeResult.value = result;
  } catch (e) {
    Message.error(t('asm_plugin.execute_failed'));
    console.error(t('asm_plugin.execute_error'), e);
    executeResult.value = {
      success: false,
      message: `错误: ${e}`,
      data: null
    };
  } finally {
    executeLoading.value = false;
  }
};

// 处理添加插件
const handleAddPlugin = async () => {
  // 检查基本字段

  addPluginLoading.value = true;

  try {
    let result;

    // 首先更新脚本头部注释
    let scriptWithComments = updateScriptHeaderComments(newPluginScript.value);

    // 然后注入参数信息到manifest函数，同时更新基本信息
    const scriptWithParams = injectParamsIntoScript(scriptWithComments, pluginParams.value);

    if (isEditingMode.value) {
      // 更新插件
      await invoke('update_asm_plugin', {
        pluginId: editingPluginId.value,
        name: newPlugin.value.name,
        description: newPlugin.value.description,
        script: scriptWithParams
      });
      Message.success(t('asm_plugin.update_plugin_success'));
    } else {
      // 创建新插件
      const filename = `${newPlugin.value.type || 'web'}_${newPlugin.value.name.replace(/\s+/g, '_').toLowerCase()}.rhai`;
      await invoke('upload_asm_plugin_content', {
        filename,
        content: scriptWithParams,
      });
      Message.success(t('asm_plugin.add_plugin_success'));
    }

    addPluginModalVisible.value = false;

    // 重置表单和状态
    newPlugin.value = {
      name: '',
      type: 'web',
      description: '',
      author: '',
      version: '1.0.0'
    };
    newPluginScript.value = '';
    pluginParams.value = [];
    currentStep.value = 1;
    validationResult.value = null;
    validationCompleted.value = false;
    editingPluginId.value = null;
    isEditingMode.value = false;

    // 重新加载插件列表
    await loadPlugins();
  } catch (e) {
    Message.error(t('asm_plugin.save_plugin_failed'));
    console.error(t('asm_plugin.save_plugin_error'), e);
  } finally {
    addPluginLoading.value = false;
  }
};

// 加载脚本模板
const loadScriptTemplate = () => {
  templateModalVisible.value = true;
  Modal.confirm({
    title: t('asm_plugin.template_title'),
    content: h('div', { class: 'template-list' }, [
      h('div', { class: 'template-item', onClick: () => loadTemplate('risk') }, [
        h('h4', [t('asm_plugin.template_risk')]),
        h('p', [t('asm_plugin.template_risk_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('domain') }, [
        h('h4', [t('asm_plugin.template_domain')]),
        h('p', [t('asm_plugin.template_domain_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('component') }, [
        h('h4', [t('asm_plugin.template_component')]),
        h('p', [t('asm_plugin.template_component_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('website') }, [
        h('h4', [t('asm_plugin.template_website')]),
        h('p', [t('asm_plugin.template_website_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('port') }, [
        h('h4', [t('asm_plugin.template_port')]),
        h('p', [t('asm_plugin.template_port_desc')])
      ])
    ]),
    footer: false,
    width: 600,
    onCancel: () => {
      templateModalVisible.value = false;
    }
  });
};

// 加载具体模板
const loadTemplate = (type) => {
  const name = newPlugin.value.name || t('asm_plugin.new_plugin');
  const description = newPlugin.value.description || t('asm_plugin.please_add_description');
  const author = newPlugin.value.author || t('asm_plugin.user');
  const version = newPlugin.value.version || '1.0.0';

  let template = '';
  switch (type) {
    case 'risk':
      template = `// ${name} - 风险检测插件
// 描述: ${description}
// 作者: ${author} 
// 版本: ${version}

fn get_manifest() {
    let manifest = #{
        name: "${name}",
        description: "${description}",
        author: "${author}",
        version: "${version}",
        plugin_type: "risk_scanning",
        severity: "medium",
        references: [],
        params: [
            #{
                name: "超时时间（秒）",
                key: "timeout",
                type: "number",
                required: false,
                default_value: 30,
                description: "HTTP请求超时时间（秒）"
            },
            #{
                name: "自定义请求头",
                key: "custom_header",
                type: "string",
                required: false,
                default_value: "",
                description: "发送请求时附加的自定义HTTP请求头"
            }
        ],
        result_fields: [
            #{
                name: "风险名称",
                key: "risk_name",
                type: "string",
                description: "发现的风险名称"
            },
            #{
                name: "风险类型",
                key: "risk_type",
                type: "string",
                description: "风险的类型"
            },
            #{
                name: "风险等级",
                key: "risk_level",
                type: "string",
                description: "风险的严重程度"
            },
            #{
                name: "风险描述",
                key: "risk_description",
                type: "string",
                description: "风险的详细描述"
            }
        ]
    };
    return to_json(manifest);
}

fn analyze(request_json) {
    let request = parse_json(request_json);
    let target = request.target;
    let params = request.params || #{};
    
    // 获取超时参数
    let timeout = params.timeout || 30;
    
    print_info("正在检测目标: " + target);
    
    // 构造HTTP请求
    let http_params = #{
        url: target,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        },
        timeout: timeout,
        follow_redirects: true
    };
    
    // 发送请求
    let response = http_request(to_json(http_params));
    let parsed_response = parse_json(response);
    
    // 检查风险
    let risks = [];
    
    if parsed_response.body != () {
        let body = parsed_response.body;
        
        // 检查敏感信息泄露
        if body.contains("password") || body.contains("api_key") || body.contains("secret") {
            risks.push(#{
                name: "敏感信息泄露",
                type: "information_disclosure",
                level: "high",
                description: "页面中发现敏感信息泄露"
            });
        }
        
        // 检查错误信息泄露
        if body.contains("SQL syntax") || body.contains("ODBC Driver") {
            risks.push(#{
                name: "错误信息泄露",
                type: "information_disclosure",
                level: "medium",
                description: "页面中存在数据库错误信息泄露"
            });
        }
    }
    
    // 检查响应头
    if parsed_response.headers != () {
        let headers = parsed_response.headers;
        
        // 检查安全头部缺失
        if !headers.contains("X-Frame-Options") {
            risks.push(#{
                name: "缺少X-Frame-Options头",
                type: "security_headers",
                level: "low",
                description: "网站可能存在点击劫持风险"
            });
        }
    }
    
    return to_json(#{
        success: risks.len() > 0,
        message: risks.len() > 0 ? "发现安全风险" : "未发现风险",
        data: #{
            risks: risks
        },
        found_risks: risks
    });
}`;
      break;
    case 'domain':
      template = `// ${name} - 域名收集插件
// 描述: ${description}
// 作者: ${author} 
// 版本: ${version}

fn get_manifest() {
    let manifest = #{
        name: "${name}",
        description: "${description}",
        author: "${author}",
        version: "${version}",
        plugin_type: "domain_discovery",
        severity: "info",
        references: [],
        params: [
            #{
                name: "API密钥",
                key: "api_key",
                type: "string",
                required: true,
                description: "用于查询的API密钥"
            },
            #{
                name: "查询深度",
                key: "depth",
                type: "number",
                required: false,
                default_value: 2,
                description: "子域名查询深度"
            }
        ],
        result_fields: [
            #{
                name: "域名",
                key: "domain",
                type: "string",
                description: "发现的域名"
            },
            #{
                name: "DNS记录",
                key: "dns_records",
                type: "array",
                description: "域名的DNS记录信息"
            },
            #{
                name: "子域名数量",
                key: "subdomain_count",
                type: "number",
                description: "发现的子域名数量"
            },
            #{
                name: "查询状态",
                key: "status",
                type: "string",
                description: "域名查询的状态"
            }
        ]
    };
    return to_json(manifest);
}

fn analyze(request_json) {
    let request = parse_json(request_json);
    let target = request.target;
    let params = request.params;
    
    // 获取参数
    let api_key = if params != () && params.api_key != () { params.api_key } else { "" };
    let depth = if params != () && params.depth != () { params.depth } else { 2 };
    
    print_info("正在收集域名: " + target);
    
    // 模拟API查询结果
    let found_domains = [
        target,
        "www." + target,
        "api." + target,
        "mail." + target,
        "blog." + target
    ];
    
    return to_json(#{
        success: true,
        message: "发现 " + found_domains.len() + " 个域名",
        data: #{
            domains: found_domains,
            depth: depth,
            status: "completed"
        },
        found_domains: found_domains
    });
}`;
      break;
    case 'component':
      template = `// ${name} - 组件发现插件
// 描述: ${description}
// 作者: ${author} 
// 版本: ${version}

fn get_manifest() {
    let manifest = #{
        name: "${name}",
        description: "${description}",
        author: "${author}",
        version: "${version}",
        plugin_type: "fingerprint",
        severity: "info",
        references: [],
        params: [
            #{
                name: "超时时间",
                key: "timeout",
                type: "number",
                required: false,
                default_value: 30,
                description: "请求超时时间（秒）"
            }
        ],
        result_fields: [
            #{
                name: "组件名称",
                key: "component_name",
                type: "string",
                description: "发现的组件名称"
            },
            #{
                name: "组件版本",
                key: "component_version",
                type: "string",
                description: "组件的版本号"
            },
            #{
                name: "组件类型",
                key: "component_type",
                type: "string",
                description: "组件的类型（如：框架、库、CMS等）"
            },
            #{
                name: "置信度",
                key: "confidence",
                type: "number",
                description: "识别结果的置信度（0-100）"
            }
        ]
    };
    return to_json(manifest);
}

fn analyze(request_json) {
    let request = parse_json(request_json);
    let target = request.target;
    let params = request.params;
    let timeout = if params != () && params.timeout != () { params.timeout } else { 30 };
    
    print_info("正在识别组件: " + target);
    
    // 发送HTTP请求
    let http_params = #{
        url: target,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        },
        timeout: timeout
    };
    
    let response = http_request(to_json(http_params));
    let parsed_response = parse_json(response);
    let fingerprints = [];
    
    if parsed_response.headers != () {
        let headers = parsed_response.headers;
        
        // 检查服务器信息
        if headers["Server"] != () {
            fingerprints.push(#{
                name: "Web Server",
                version: headers["Server"],
                type: "server"
            });
        }
        
        // 检查框架信息
        if headers["X-Powered-By"] != () {
            fingerprints.push(#{
                name: "Framework",
                version: headers["X-Powered-By"],
                type: "framework"
            });
        }
    }
    
    if parsed_response.body != () {
        let body = parsed_response.body;
        
        // 检查常见框架特征
        if body.contains("jquery") {
            fingerprints.push(#{
                name: "jQuery",
                type: "javascript"
            });
        }
        
        if body.contains("bootstrap") {
            fingerprints.push(#{
                name: "Bootstrap",
                type: "css"
            });
        }
        
        if body.contains("react") {
            fingerprints.push(#{
                name: "React",
                type: "javascript"
            });
        }
        
        if body.contains("vue") {
            fingerprints.push(#{
                name: "Vue.js",
                type: "javascript"
            });
        }
    }
    
    return to_json(#{
        success: true,
        message: "发现 " + fingerprints.len() + " 个组件",
        data: #{
            fingerprints: fingerprints
        },
        found_fingerprints: fingerprints
    });
}`;
      break;
    case 'website':
      template = `// ${name} - 网站扫描插件
// 描述: ${description}
// 作者: ${author} 
// 版本: ${version}

fn get_manifest() {
    let manifest = #{
        name: "${name}",
        description: "${description}",
        author: "${author}",
        version: "${version}",
        plugin_type: "web",
        severity: "info",
        references: [],
        params: [
            #{
                name: "检查SSL",
                key: "check_ssl",
                type: "boolean",
                required: false,
                default_value: true,
                description: "是否检查SSL证书信息"
            },
            #{
                name: "检查网站标题",
                key: "check_title",
                type: "boolean",
                required: false,
                default_value: true,
                description: "是否检查网站标题"
            }
        ],
        result_fields: [
            #{
                name: "网站标题",
                key: "title",
                type: "string",
                description: "网站的标题"
            },
            #{
                name: "响应状态码",
                key: "status_code",
                type: "number",
                description: "HTTP响应状态码"
            },
            #{
                name: "服务器信息",
                key: "server",
                type: "string",
                description: "Web服务器信息"
            },
            #{
                name: "SSL证书信息",
                key: "ssl_info",
                type: "object",
                description: "SSL证书相关信息"
            },
            #{
                name: "响应头",
                key: "headers",
                type: "object",
                description: "HTTP响应头信息"
            }
        ]
    };
    return to_json(manifest);
}

fn analyze(request_json) {
    let request = parse_json(request_json);
    let target = request.target;
    let params = request.params;
    
    // 获取参数
    let check_ssl = if params != () && params.check_ssl != () { params.check_ssl } else { true };
    let check_title = if params != () && params.check_title != () { params.check_title } else { true };
    
    print_info("正在扫描网站: " + target);
    
    // 发送HTTP请求
    let http_params = #{
        url: target,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        }
    };
    
    let response = http_request(to_json(http_params));
    let parsed_response = parse_json(response);
    let info = #{};
    
    // 基本信息
    info.status_code = parsed_response.status;
    info.headers = parsed_response.headers;
    
    // 提取网站标题
    if check_title && parsed_response.body != () {
        let title_match = regex::match("<title>(.*?)</title>", parsed_response.body);
        if title_match != () {
            info.title = title_match[1];
        }
    }
    
    // 检查SSL证书
    if check_ssl && target.starts_with("https") {
        info.ssl = #{
            valid: true,
            issuer: "示例证书颁发机构",
            expires: "2024-12-31"
        };
    }
    
    return to_json(#{
        success: true,
        message: "网站扫描完成",
        data: info
    });
}`;
      break;
    case 'port':
      template = `// ${name} - 端口扫描插件
// 描述: ${description}
// 作者: ${author} 
// 版本: ${version}

fn get_manifest() {
    let manifest = #{
        name: "${name}",
        description: "${description}",
        author: "${author}",
        version: "${version}",
        plugin_type: "port_scanning",
        severity: "info",
        references: [],
        params: [
            #{
                name: "端口范围",
                key: "port_range",
                type: "string",
                required: false,
                default_value: "1-1000",
                description: "要扫描的端口范围，例如：80,443,8080-8090"
            },
            #{
                name: "超时时间",
                key: "timeout",
                type: "number",
                required: false,
                default_value: 5,
                description: "单个端口扫描超时时间（秒）"
            }
        ],
        result_fields: [
            #{
                name: "端口号",
                key: "port",
                type: "number",
                description: "开放的端口号"
            },
            #{
                name: "服务名称",
                key: "service",
                type: "string",
                description: "端口上运行的服务名称"
            },
            #{
                name: "服务版本",
                key: "version",
                type: "string",
                description: "服务的版本信息"
            },
            #{
                name: "服务状态",
                key: "status",
                type: "string",
                description: "端口的开放状态"
            },
            #{
                name: "服务Banner",
                key: "banner",
                type: "string",
                description: "服务的Banner信息"
            }
        ]
    };
    return to_json(manifest);
}

fn analyze(request_json) {
    let request = parse_json(request_json);
    let target = request.target;
    let params = request.params;
    
    // 获取参数
    let port_range = if params != () && params.port_range != () { params.port_range } else { "1-1000" };
    let timeout = if params != () && params.timeout != () { params.timeout } else { 5 };
    
    print_info("正在扫描端口: " + target);
    
    // 模拟端口扫描结果
    let open_ports = [
        #{
            port: 80,
            service: "http",
            version: "Apache/2.4.41",
            status: "open"
        },
        #{
            port: 443,
            service: "https",
            version: "nginx/1.18.0",
            status: "open"
        },
        #{
            port: 22,
            service: "ssh",
            version: "OpenSSH/8.2p1",
            status: "open"
        }
    ];
    
    return to_json(#{
        success: true,
        message: "发现 " + open_ports.len() + " 个开放端口",
        data: #{
            ports: open_ports
        },
        found_ports: open_ports
    });
}`;
      break;
  }

  if (template) {
    newPluginScript.value = template;
    Message.success('模板加载成功');
  }
};

// 插入代码片段
const insertCodeSnippet = (type) => {
  let snippet = '';

  switch (type) {
    case 'http':
      snippet = `// HTTP请求示例
let response = http::get(target + "/path");
print("状态: " + response.status_code);

if response.status_code == 200 {
  // 检查响应体
  if regex::match("sensitive_data", response.body) {
    result.success = true;
    result.data.test_results["found_sensitive_data"] = true;
  }
}`;
      break;
    case 'success':
      snippet = `// 返回成功结果示例
return {
  success: true,
  data: {
    type: "information_disclosure",
    risk_level: "medium",
    test_results: {
      "endpoint_accessible": true,
      "sensitive_data_found": "用户凭证暴露"
    },
    info: "应用程序暴露了敏感信息",
    cve: ["CVE-2023-12345"],
    references: [
      "https://example.com/reference1",
      "https://example.com/reference2"
    ]
  }
};`;
      break;
  }

  if (snippet) {
    // 在光标位置插入或附加到末尾
    newPluginScript.value += '\n\n' + snippet;
  }
};

// 参数管理功能
const showParameterModal = () => {
  editingParam.value = {
    key: '',
    name: '',
    type: 'string',
    required: false,
    description: ''
  };
  editingParamIndex.value = -1;
  parameterModalVisible.value = true;
};

const editParameter = (param, index) => {
  // 复制参数对象，避免直接修改原对象
  editingParam.value = {
    key: param.key || '',
    name: param.name || '',
    type: param.type || 'string',
    required: !!param.required,
    description: param.description || '',
    default_value: param.default_value
  };

  editingParamIndex.value = index;
  parameterModalVisible.value = true;
};

const saveParameter = () => {
  // 基本验证
  if (!editingParam.value.key || !editingParam.value.name) {
    Message.error('参数键和名称是必填项');
    return;
  }

  // 检查键中是否有空格
  if (/\s/.test(editingParam.value.key)) {
    Message.error('参数键不能包含空格');
    return;
  }

  // 验证键的唯一性（仅针对新参数）
  if (editingParamIndex.value === -1) {
    const keyExists = pluginParams.value.some(param => param.key === editingParam.value.key);
    if (keyExists) {
      Message.error('参数键必须是唯一的');
      return;
    }
  }

  // 类型特定的验证和格式化
  if (editingParam.value.type === 'number' && editingParam.value.default_value !== undefined && editingParam.value.default_value !== '') {
    // 确保数字默认值确实是数字
    const numValue = Number(editingParam.value.default_value);
    if (isNaN(numValue)) {
      Message.error('默认值必须是有效的数字');
      return;
    }
    editingParam.value.default_value = numValue;
  }

  // 创建干净的参数对象
  const newParam = {
    key: editingParam.value.key.trim(),
    name: editingParam.value.name.trim(),
    type: editingParam.value.type,
    required: editingParam.value.required,
    description: editingParam.value.description?.trim() || '',
    default_value: editingParam.value.default_value
  };

  if (editingParamIndex.value === -1) {
    // 添加新参数
    pluginParams.value.push(newParam);
  } else {
    // 更新现有参数
    pluginParams.value[editingParamIndex.value] = newParam;
  }

  Message.success(
    editingParamIndex.value === -1
      ? '参数已添加'
      : '参数已更新'
  );

  parameterModalVisible.value = false;
};

const deleteParameter = (index) => {
  pluginParams.value.splice(index, 1);
};

// 应用参数示例
const applyParamExample = (example) => {
  editingParam.value = { ...example.template };
  Message.success('已应用参数示例');
};

// 验证脚本
const validateScript = async () => {
  if (!newPlugin.value.name || !newPluginScript.value) {
    validationResult.value = {
      valid: false,
      message: '插件名称和脚本是必填项'
    };
    return;
  }

  validationResult.value = null;

  try {
    // 创建临时文件名和ID
    const tempTimestamp = Date.now();
    const tempName = `temp_validate_${tempTimestamp}`;
    const tempFilename = `${tempName}.rhai`;

    // 为验证脚本修正plugin_type字段
    let scriptForValidation = newPluginScript.value;
    let pluginType = "domain_discovery"; // 使用domain_discovery作为默认类型

    // 检查脚本中是否包含plugin_type字段，并提取它的值
    const pluginTypeRegex = /plugin_type\s*:\s*"([^"]+)"/;
    const pluginTypeMatch = scriptForValidation.match(pluginTypeRegex);

    if (pluginTypeMatch && pluginTypeMatch[1]) {
      pluginType = pluginTypeMatch[1];
    } else {
      // 如果没有找到plugin_type，尝试替换rtype或添加plugin_type字段
      if (scriptForValidation.includes('rtype:') || scriptForValidation.includes('rtype :')) {
        // 查找并提取rtype的值
        const rtypeRegex = /rtype\s*:\s*"([^"]+)"/;
        const rtypeMatch = scriptForValidation.match(rtypeRegex);
        if (rtypeMatch && rtypeMatch[1]) {
          pluginType = rtypeMatch[1];
        }

        // 替换rtype为plugin_type
        scriptForValidation = scriptForValidation.replace(/rtype\s*:/g, 'plugin_type:');
      } else {
        // 否则添加plugin_type字段
        pluginType = newPlugin.value.type || 'domain_discovery';

        const manifestRegex = /(let\s+manifest\s*=\s*#{[^}]*?)(};)/;
        const hasComma = /(let\s+manifest\s*=\s*#{[^}]*?)([,\s]*)(};)/.test(scriptForValidation);

        if (manifestRegex.test(scriptForValidation)) {
          if (hasComma) {
            scriptForValidation = scriptForValidation.replace(
              /(let\s+manifest\s*=\s*#{[^}]*?)([,\s]*)(};)/,
              `$1, plugin_type: "${pluginType}"$3`
            );
          } else {
            scriptForValidation = scriptForValidation.replace(
              /(let\s+manifest\s*=\s*#{[^}]*?)(};)/,
              `$1, plugin_type: "${pluginType}"$2`
            );
          }
        }
      }
    }

    // 确保name字段与tempName匹配
    const nameRegex = /name\s*:\s*"([^"]+)"/;
    scriptForValidation = scriptForValidation.replace(nameRegex, `name: "${tempName}"`);

    // 确保result_fields字段存在
    if (!scriptForValidation.includes('result_fields:') && !scriptForValidation.includes('result_fields :')) {
      const manifestRegex = /(let\s+manifest\s*=\s*#{[^}]*?)(};)/;
      const hasComma = /(let\s+manifest\s*=\s*#{[^}]*?)([,\s]*)(};)/.test(scriptForValidation);

      if (manifestRegex.test(scriptForValidation)) {
        const resultFieldsStr = `, result_fields: [
            #{
                name: "结果",
                key: "result",
                type: "string",
                description: "验证结果"
            }
        ]`;

        if (hasComma) {
          scriptForValidation = scriptForValidation.replace(
            /(let\s+manifest\s*=\s*#{[^}]*?)([,\s]*)(};)/,
            `$1${resultFieldsStr}$3`
          );
        } else {
          scriptForValidation = scriptForValidation.replace(
            /(let\s+manifest\s*=\s*#{[^}]*?)(};)/,
            `$1${resultFieldsStr}$2`
          );
        }
      }
    }

    console.log("验证脚本内容:", scriptForValidation);

    // 尝试上传脚本内容来验证
    await invoke('upload_asm_plugin_content', {
      filename: tempFilename,
      content: scriptForValidation,
    });

    // 等待以确保文件写入完成
    await new Promise(resolve => setTimeout(resolve, 500));

    // 重新加载所有插件以确保临时文件被加载
    await invoke('load_asm_plugins');

    // 检查插件是否成功加载 - 注意：必须使用type:name格式的ID
    try {
      // 构造正确的插件ID格式: type:name
      const pluginId = `${pluginType}:${tempName}`;
      console.log("尝试获取插件，ID:", pluginId);

      const result = await invoke('get_asm_plugin', { pluginId });
      console.log("获取到插件:", result);

      if (!result) {
        throw new Error('临时插件加载失败：找不到插件');
      }

      // 如果上述步骤都成功，说明脚本语法正确
      validationResult.value = {
        valid: true,
        message: '脚本验证成功！'
      };
      validationCompleted.value = true;

      // 显示成功通知
      Message.success('脚本验证成功！');
    } catch (e) {
      console.error("获取插件失败:", e);
      throw new Error(`插件验证失败: ${e}`);
    }
  } catch (error) {
    console.error("验证脚本失败:", error);
    validationResult.value = {
      valid: false,
      message: `验证失败: ${error}`
    };

    // 显示错误通知
    Message.error(`验证失败: ${error}`);
  } finally {
    // 无论成功还是失败，都尝试清理临时文件
    try {
      // 使用文件名删除临时文件
      await invoke('delete_asm_plugin', { pluginName: tempFilename });
      console.log("临时文件已删除:", tempFilename);

      // 再次重新加载插件列表以确保临时文件被清理
      await invoke('load_asm_plugins');
    } catch (e) {
      console.error('清理临时文件失败:', e);
    }
  }
};

// 将参数信息注入到脚本的manifest函数中
const injectParamsIntoScript = (script, params) => {
  if (!script) {
    return script;
  }

  // 提取脚本中的get_manifest函数
  const getManifestRegex = /fn\s+get_manifest\s*\(\s*\)\s*\{([\s\S]*?)\}/;
  const manifestMatch = script.match(getManifestRegex);

  if (!manifestMatch) {
    // 如果找不到get_manifest函数，添加一个基本的get_manifest函数
    const paramsCode = formatParamsAsRhai(params);
    const newManifest = `
// 自动添加以支持参数
fn get_manifest() {
    let manifest = #{
        name: "${newPlugin.value.name}",
        description: "${newPlugin.value.description}",
        author: "${newPlugin.value.author}",
        version: "${newPlugin.value.version}",
        plugin_type: "${newPlugin.value.type}",
        params: [
${paramsCode}
        ]
    };
    return to_json(manifest);
}`;

    // 在脚本末尾添加新的get_manifest函数
    return script + '\n\n' + newManifest;
  }

  // 检查script中是否包含rtype而不是plugin_type
  if (script.includes('rtype:') && !script.includes('plugin_type:')) {
    script = script.replace(/rtype\s*:/g, 'plugin_type:');
  }

  // 提取manifest对象定义
  const manifestContent = manifestMatch[1];
  const manifestObjRegex = /let\s+manifest\s*=\s*#{\s*([\s\S]*?)\}\s*;/;
  const manifestObjMatch = manifestContent.match(manifestObjRegex);

  if (!manifestObjMatch) {
    return script;
  }

  let manifestProps = manifestObjMatch[1];

  // 更新基本信息字段
  manifestProps = updateManifestField(manifestProps, 'name', newPlugin.value.name);
  manifestProps = updateManifestField(manifestProps, 'description', newPlugin.value.description);
  manifestProps = updateManifestField(manifestProps, 'author', newPlugin.value.author);
  manifestProps = updateManifestField(manifestProps, 'version', newPlugin.value.version);
  manifestProps = updateManifestField(manifestProps, 'plugin_type', newPlugin.value.type);

  // 更新参数
  if (params && params.length > 0) {
    const paramsCode = formatParamsAsRhai(params);

    // 检查脚本是否已经有params部分
    const paramsRegex = /params\s*:\s*\[\s*([\s\S]*?)\s*\]/;
    const paramsMatch = manifestProps.match(paramsRegex);

    if (paramsMatch) {
      // 替换现有参数
      manifestProps = manifestProps.replace(
        /params\s*:\s*\[\s*[\s\S]*?\s*\]/,
        `params: [
${paramsCode}
        ]`
      );
    } else {
      // 添加参数部分
      // 检查最后一个属性是否有逗号
      const lastPropHasComma = /,\s*$/.test(manifestProps.trim());
      const separator = lastPropHasComma ? '' : ',';

      manifestProps = manifestProps + `${separator}
        params: [
${paramsCode}
        ]`;
    }
  }

  // 构建新的manifest对象
  const newManifestObj = `let manifest = #{${manifestProps}};`;

  // 替换整个manifest对象
  return script.replace(
    /let\s+manifest\s*=\s*#{\s*[\s\S]*?\}\s*;/,
    newManifestObj
  );
};

// 辅助函数：更新manifest中的特定字段
const updateManifestField = (manifestStr, fieldName, fieldValue) => {
  // 如果没有值，则不更新
  if (fieldValue === undefined || fieldValue === null || fieldValue === '') {
    return manifestStr;
  }

  // 检查字段是否存在
  const fieldRegex = new RegExp(`${fieldName}\\s*:\\s*"[^"]*"`, 'g');

  if (fieldRegex.test(manifestStr)) {
    // 替换现有值
    return manifestStr.replace(
      new RegExp(`${fieldName}\\s*:\\s*"[^"]*"`),
      `${fieldName}: "${escapeString(fieldValue)}"`
    );
  } else {
    // 添加新字段
    // 检查最后一个属性是否有逗号
    const lastPropHasComma = /,\s*$/.test(manifestStr.trim());
    const separator = lastPropHasComma ? '' : ',';

    return manifestStr + `${separator}
        ${fieldName}: "${escapeString(fieldValue)}"`;
  }
};

// 从脚本中提取参数
const extractParamsFromScript = (script) => {
  if (!script) return [];

  // 提取get_manifest函数
  const getManifestRegex = /fn\s+get_manifest\s*\(\s*\)\s*\{([\s\S]*?)\}/;
  const manifestMatch = script.match(getManifestRegex);
  if (!manifestMatch) return [];

  // 查找params数组
  const paramsRegex = /params\s*:\s*\[\s*([\s\S]*?)\s*\]/;
  const paramsMatch = manifestMatch[1].match(paramsRegex);
  if (!paramsMatch) return [];

  // 提取参数对象
  const paramsContent = paramsMatch[1];
  const params = [];

  // 查找所有参数对象
  const paramObjRegex = /#{\s*([\s\S]*?)\s*}/g;
  let match;
  while ((match = paramObjRegex.exec(paramsContent)) !== null) {
    const paramContent = match[1];

    // 提取参数属性
    const keyMatch = /key\s*:\s*"([^"]+)"/.exec(paramContent);
    const nameMatch = /name\s*:\s*"([^"]+)"/.exec(paramContent);
    const typeMatch = /type\s*:\s*"([^"]+)"/.exec(paramContent);
    const requiredMatch = /required\s*:\s*(true|false)/.exec(paramContent);
    const descMatch = /description\s*:\s*"([^"]*)"/.exec(paramContent);

    // 提取默认值，根据类型处理
    let defaultValue = undefined;
    const defaultValueStrMatch = /default_value\s*:\s*"([^"]*)"/.exec(paramContent);
    const defaultValueNumMatch = /default_value\s*:\s*([0-9\.]+)/.exec(paramContent);
    const defaultValueBoolMatch = /default_value\s*:\s*(true|false)/.exec(paramContent);

    if (defaultValueStrMatch) {
      defaultValue = defaultValueStrMatch[1];
    } else if (defaultValueNumMatch) {
      defaultValue = Number(defaultValueNumMatch[1]);
    } else if (defaultValueBoolMatch) {
      defaultValue = defaultValueBoolMatch[1] === 'true';
    }

    if (keyMatch && nameMatch) {
      params.push({
        key: keyMatch[1],
        name: nameMatch[1],
        type: typeMatch ? typeMatch[1] : 'string',
        required: requiredMatch ? requiredMatch[1] === 'true' : false,
        description: descMatch ? descMatch[1] : '',
        default_value: defaultValue
      });
    }
  }

  return params;
};

// 将参数数组格式化为Rhai语法
const formatParamsAsRhai = (params) => {
  if (!params || params.length === 0) {
    return '';
  }

  return params.map(param => {
    const defaultValue = formatDefaultValueForRhai(param.type, param.default_value);

    return `            #{
                name: "${escapeString(param.name)}",
                key: "${escapeString(param.key)}",
                type: "${param.type}",
                required: ${param.required},
                ${defaultValue ? `default_value: ${defaultValue},` : ''}
                description: "${escapeString(param.description || '')}"
            }`;
  }).join(',\n');
};

// 根据参数类型格式化默认值
const formatDefaultValueForRhai = (type, value) => {
  if (value === undefined || value === null || value === '') {
    return '';
  }

  switch (type) {
    case 'string':
    case 'textarea':
      return `"${escapeString(value)}"`;
    case 'number':
      return isNaN(Number(value)) ? '0' : value.toString();
    case 'boolean':
      return value ? 'true' : 'false';
    default:
      return `"${escapeString(value)}"`;
  }
};

// 转义字符串中的特殊字符
const escapeString = (str) => {
  if (typeof str !== 'string') {
    return '';
  }

  return str
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
};

// 切换暗黑模式
const toggleDarkMode = () => {
  isDarkMode.value = !isDarkMode.value;
};

// 获取严重性颜色
const getSeverityColor = (severity) => {
  const severityMap = {
    'critical': 'red',
    'high': 'orange',
    'medium': 'gold',
    'low': 'blue',
    'info': 'green',
  };
  return severityMap[severity?.toLowerCase()] || 'default';
};

// 删除插件确认
const confirmDelete = (plugin) => {
  Modal.warning({
    title: '确认删除',
    content: `确定要删除插件"${plugin.name}"吗？`,
    okText: '删除',
    cancelText: '取消',
    onOk: () => deletePlugin(plugin)
  });
};

// 删除插件
const deletePlugin = async (plugin) => {
  try {
    // 尝试多种删除策略
    const strategies = [];

    if (typeof plugin === 'string') {
      // 如果直接传入字符串
      strategies.push(plugin); // 原始名称
      if (!plugin.endsWith('.rhai')) {
        strategies.push(`${plugin}.rhai`); // 添加扩展名
      }
    } else if (plugin && plugin.id && typeof plugin.id === 'string') {
      // 策略1: 使用完整ID (可能是 type:name 格式)
      strategies.push(plugin.id);

      // 策略2: 如果ID是type:name格式，提取name部分
      if (plugin.id.includes(':')) {
        const parts = plugin.id.split(':');
        strategies.push(parts[1]); // 仅name部分
        strategies.push(`${parts[1]}.rhai`); // name + 扩展名
      }

      // 策略3: 使用插件名称
      if (plugin.name) {
        strategies.push(plugin.name);
        if (!plugin.name.endsWith('.rhai')) {
          strategies.push(`${plugin.name}.rhai`);
        }
      }

      // 策略4: 如果ID已经是文件名
      if (plugin.id.endsWith('.rhai')) {
        strategies.push(plugin.id);
      } else {
        strategies.push(`${plugin.id}.rhai`);
      }
    } else if (plugin && plugin.name) {
      // 仅有名称的情况
      strategies.push(plugin.name);
      if (!plugin.name.endsWith('.rhai')) {
        strategies.push(`${plugin.name}.rhai`);
      }
    } else {
      throw new Error('无效的插件对象或ID');
    }

    // 去重
    const uniqueStrategies = [...new Set(strategies)];
    console.log("删除插件尝试策略:", uniqueStrategies);

    // 尝试所有策略，直到一个成功
    let success = false;
    let lastError = null;

    for (const pluginName of uniqueStrategies) {
      try {
        console.log("尝试删除插件:", pluginName);
        await invoke('delete_asm_plugin', { pluginName });
        success = true;
        console.log("成功删除插件:", pluginName);
        break; // 一旦成功就退出循环
      } catch (e) {
        console.error(`策略 ${pluginName} 删除失败:`, e);
        lastError = e;
        // 继续尝试下一个策略
      }
    }

    if (success) {
      Message.success('删除插件成功');
      await loadPlugins(); // 重新加载插件列表
    } else {
      throw lastError || new Error('所有删除策略均失败');
    }
  } catch (e) {
    Message.error(`删除插件失败：${e}`);
    console.error('删除插件错误:', e);
  }
};

// 处理文件变更
const handleFileChange = (info) => {
  fileList.value = info.fileList.slice(-1);

  if (info.fileList.length > 0) {
    uploadForm.file = info.fileList[0].originFile;
  } else {
    uploadForm.file = null;
  }
};

// 处理上传
const handleUpload = () => {
  // 实现上传逻辑
};

// 初始化
onMounted(async () => {
  try {
    await loadPlugins();
    Message.success(t('asm_plugin.load_success'));
  } catch (error) {
    Message.error(t('asm_plugin.load_failed'));
    console.error('Failed to load plugins:', error);
  }
});


// ... rest of the styles ...

// 更新脚本头部注释
const updateScriptHeaderComments = (script) => {
  if (!script) {
    return script;
  }

  // 首先获取插件的基本信息
  const name = newPlugin.value.name || '';
  const description = newPlugin.value.description || '';
  const author = newPlugin.value.author || '';
  const version = newPlugin.value.version || '';
  const type = newPlugin.value.type || '';

  // 提取并更新脚本头部注释
  const headerCommentRegex = /^(\/\/.*\n)+/;
  const headerMatch = script.match(headerCommentRegex);

  // 构建新的头部注释
  const newHeader = `// ${name} - ${type}插件
// 描述: ${description}
// 作者: ${author}
// 版本: ${version}
`;

  if (headerMatch) {
    // 替换现有头部注释
    return script.replace(headerCommentRegex, newHeader);
  } else {
    // 添加头部注释到脚本开头
    return newHeader + script;
  }
};

// 从脚本中提取基本信息
const extractBasicInfoFromScript = (script) => {
  if (!script) return;

  // 提取头部注释中的信息
  const headerRegex = /^\/\/\s*(.*?)\s*-\s*(.*?)插件\s*\n\/\/\s*描述:\s*(.*?)\s*\n\/\/\s*作者:\s*(.*?)\s*\n\/\/\s*版本:\s*(.*?)\s*\n/;
  const headerMatch = script.match(headerRegex);

  if (headerMatch) {
    // 如果找到匹配的头部注释，提取信息
    if (headerMatch[1] && headerMatch[1].trim()) {
      newPlugin.value.name = headerMatch[1].trim();
    }
    if (headerMatch[2] && headerMatch[2].trim()) {
      newPlugin.value.type = headerMatch[2].trim();
    }
    if (headerMatch[3] && headerMatch[3].trim()) {
      newPlugin.value.description = headerMatch[3].trim();
    }
    if (headerMatch[4] && headerMatch[4].trim()) {
      newPlugin.value.author = headerMatch[4].trim();
    }
    if (headerMatch[5] && headerMatch[5].trim()) {
      newPlugin.value.version = headerMatch[5].trim();
    }
  }

  // 提取manifest中的信息（优先级更高）
  const getManifestRegex = /fn\s+get_manifest\s*\(\s*\)\s*\{([\s\S]*?)\}/;
  const manifestMatch = script.match(getManifestRegex);

  if (manifestMatch) {
    const manifestContent = manifestMatch[1];

    // 提取name
    const nameRegex = /name\s*:\s*"([^"]*)"/;
    const nameMatch = manifestContent.match(nameRegex);
    if (nameMatch && nameMatch[1]) {
      newPlugin.value.name = nameMatch[1];
    }

    // 提取description
    const descRegex = /description\s*:\s*"([^"]*)"/;
    const descMatch = manifestContent.match(descRegex);
    if (descMatch && descMatch[1]) {
      newPlugin.value.description = descMatch[1];
    }

    // 提取author
    const authorRegex = /author\s*:\s*"([^"]*)"/;
    const authorMatch = manifestContent.match(authorRegex);
    if (authorMatch && authorMatch[1]) {
      newPlugin.value.author = authorMatch[1];
    }

    // 提取version
    const versionRegex = /version\s*:\s*"([^"]*)"/;
    const versionMatch = manifestContent.match(versionRegex);
    if (versionMatch && versionMatch[1]) {
      newPlugin.value.version = versionMatch[1];
    }

    // 提取plugin_type
    const typeRegex = /plugin_type\s*:\s*"([^"]*)"/;
    const typeMatch = manifestContent.match(typeRegex);
    if (typeMatch && typeMatch[1]) {
      newPlugin.value.type = typeMatch[1];
    }
  }
};

// ... 在 script setup 部分添加 showTemplateModal 方法
const showTemplateModal = () => {
  Modal.confirm({
    title: t('asm_plugin.template_title'),
    content: h('div', { class: 'template-list' }, [
      h('div', { class: 'template-item', onClick: () => loadTemplate('risk') }, [
        h('h4', [t('asm_plugin.template_risk')]),
        h('p', [t('asm_plugin.template_risk_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('domain') }, [
        h('h4', [t('asm_plugin.template_domain')]),
        h('p', [t('asm_plugin.template_domain_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('component') }, [
        h('h4', [t('asm_plugin.template_component')]),
        h('p', [t('asm_plugin.template_component_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('website') }, [
        h('h4', [t('asm_plugin.template_website')]),
        h('p', [t('asm_plugin.template_website_desc')])
      ]),
      h('div', { class: 'template-item', onClick: () => loadTemplate('port') }, [
        h('h4', [t('asm_plugin.template_port')]),
        h('p', [t('asm_plugin.template_port_desc')])
      ])
    ]),
    footer: false,
    width: 600,
    onCancel: () => {
      templateModalVisible.value = false;
    }
  });
};

// 添加 handleReady 方法
const handleReady = (view) => {
  // 编辑器准备就绪时的处理逻辑
  console.log('CodeMirror editor is ready');
};

// 添加 onToggleDarkMode 方法
const onToggleDarkMode = (value) => {
  isDarkMode.value = value;
};
</script>

<style lang="less" scoped>
.clickable {
  cursor: pointer;
}

.parameter-help-text {
  font-size: 12px;
  color: #666;
  margin-top: 4px;
}

.parameter-default-value {
  margin-left: 8px;
  color: #999;
}

.parameter-section-description {
  margin-bottom: 16px;
  color: #666;
}

.dark-mode-toggle {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  margin-bottom: 12px;

  span {
    margin-right: 8px;
  }
}

.template-list {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
  padding: 16px;
}

.code-container {
  max-height: 400px;
  overflow-y: auto;
  padding: 12px;
  border-radius: 4px;
  background-color: #f5f5f5;
  font-family: 'Courier New', Courier, monospace;

  &.dark-mode {
    background-color: #1e1e1e;
    color: #d4d4d4;
  }
}

.param-key {
  color: #666;
  margin-left: 4px;
}

.param-required {
  color: red;
  margin-left: 2px;
}

.param-desc {
  margin: 4px 0 12px 0;
  padding-left: 4px;
  border-left: 2px solid #e0e0e0;
}

.result-data {
  background-color: #f5f5f5;
  padding: 12px;
  border-radius: 4px;
  overflow-x: auto;
}

.plugin-wizard-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 16px;
}

.steps-container {
  padding: 0 16px;
}

.step-content {
  flex: 1;
  overflow: hidden;
  padding: 0 16px;
}

.script-step {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 16px;
}

.script-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;

  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
  }
}

.code-editor-container {
  flex: 1;
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  overflow: hidden;
}

.script-footer {
  display: flex;
  justify-content: flex-end;
  padding: 16px 0;
}

.validation-step {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 16px;
}

.validation-header {
  margin-bottom: 16px;

  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
  }
}

.validation-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.validation-result {
  margin-bottom: 16px;
}

.validation-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.plugin-form-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.script-editor-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.script-header {
  display: flex;
  justify-content: space-between;
  align-items: center;

  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
  }
}

.pagination-info {
  margin-top: 8px;
  text-align: right;
  color: var(--color-text-3);
}
</style>