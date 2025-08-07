<template>

  <a-space direction="vertical" fill>
    <a-typography-text style="font-size: large; font-weight: 540;">{{ $t('rhai_plugin.title') || 'Rhai Plugin Manager'
      }}</a-typography-text>
    <a-row>
      <a-col :span="24">
        <a-button type="primary" size="small" @click="loadPlugins" :loading="loading">
          {{ $t('rhai_plugin.refresh') || 'Refresh' }}
        </a-button>
        <a-button type="primary" style="margin-left: 8px" size="small" @click="reloadFromDisk" :loading="reloading">
          {{ $t('rhai_plugin.reload') || 'Reload from Disk' }}
        </a-button>
        <a-button type="primary" style="margin-left: 8px" size="small" @click="showUploadModal">
          {{ $t('rhai_plugin.upload') || 'Upload' }}
        </a-button>
        <a-button type="primary" style="margin-left: 8px" size="small" @click="showAddPluginModal">
          {{ $t('rhai_plugin.addPlugin') || 'Add Plugin' }}
        </a-button>
      </a-col>
    </a-row>

    <a-table :bordered="false" :loading="loading" :columns="columns" :data="plugins" :pagination="false" size="small">
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
              <template #default>{{ $t('rhai_plugin.view') || 'View' }}</template>
            </a-doption>
            <a-doption @click="editPlugin(record)">
              <template #icon>
                <icon-edit />
              </template>
              <template #default>{{ $t('rhai_plugin.edit') || 'Edit' }}</template>
            </a-doption>
            <a-doption @click="executePlugin(record)">
              <template #icon>
                <icon-play-circle />
              </template>
              <template #default>{{ $t('rhai_plugin.execute') || 'Execute' }}</template>
            </a-doption>
            <a-doption @click="confirmDelete(record)">
              <template #icon>
                <icon-delete />
              </template>
              <template #default>{{ $t('rhai_plugin.delete') || 'Delete' }}</template>
            </a-doption>
          </template>
        </a-dropdown>
      </template>
    </a-table>

    <!-- 上传插件模态框 -->
    <a-modal v-model:visible="uploadModalVisible" :title="$t('rhai_plugin.upload_title') || 'Upload Plugin'"
      @ok="handleUpload" :ok-button-props="{ disabled: !uploadForm.file }">
      <a-upload :file-list="fileList" :custom-request="() => { }" @change="handleFileChange" :multiple="false" :limit="1"
        accept=".rhai">
        <template #upload-button>
          <a-button>
            {{ $t('rhai_plugin.select_file') || 'Select File' }}
          </a-button>
        </template>
      </a-upload>
    </a-modal>

    <!-- 执行插件模态框 -->
    <a-modal v-model:visible="executeModalVisible" :title="$t('rhai_plugin.execute_title') || 'Execute Rhai Plugin'"
      @ok="handleExecute" :confirmLoading="executeLoading" width="700px">
      <a-form :model="executeForm" layout="vertical">
        <a-form-item :label="$t('rhai_plugin.target') || 'Target'" name="target">
          <a-input v-model="executeForm.target"
            :placeholder="$t('rhai_plugin.target_placeholder') || 'Enter target URL or IP'" />
        </a-form-item>

        <!-- 参数调试信息 -->
        <div v-if="false">
          <pre>{{ JSON.stringify(currentPlugin?.params, null, 2) }}</pre>
        </div>

        <!-- 动态参数 -->
        <template v-if="currentPlugin && currentPlugin.params && currentPlugin.params.length > 0">
          <!-- 强制添加代理URL参数 -->
          <a-form-item :label="$t('rhai_plugin.proxy_url') || '代理URL'" :required="false">
            <a-input v-model="executeForm.customParams['proxy_url']"
              :placeholder="$t('rhai_plugin.proxy_url_placeholder') || 'HTTP代理URL，例如http://127.0.0.1:8080'" />
            <div class="parameter-help-text">
              {{ $t('rhai_plugin.proxy_url_help') || 'Optional HTTP proxy for all requests. Format: http://host:port' }}
            </div>
          </a-form-item>

          <!-- 其他参数 -->
          <a-divider>{{ $t('rhai_plugin.plugin_parameters') || 'Plugin Parameters' }}</a-divider>

          <div class="parameter-section-description">
            {{ $t('rhai_plugin.parameters_section_description') || 'Configure the parameters for this plugin execution:'
            }}
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
                <a-tag color="blue" size="small" style="margin-right: 8px;">{{ $t('rhai_plugin.required') || 'Required'
                  }}</a-tag>
              </template>
              <span>{{ param.description }}</span>
              <template v-if="param.default_value !== undefined || param.default !== undefined">
                <span class="parameter-default-value">
                  {{ $t('rhai_plugin.default_value') || 'Default' }}: {{ param.default_value !== undefined ?
                    param.default_value : param.default }}
                </span>
              </template>
            </div>
          </a-form-item>
        </template>
      </a-form>

      <a-divider />

      <div v-if="executeResult">
        <h3>{{ $t('rhai_plugin.execute_results') || 'Execution Results' }}</h3>
        <a-descriptions bordered>
          <a-descriptions-item :label="$t('rhai_plugin.status') || 'Status'" :span="3">
            <a-tag :color="executeResult.success ? 'success' : 'error'">
              {{ executeResult.success ? ($t('rhai_plugin.success') || 'Success') : ($t('rhai_plugin.failure') ||
              'Failure')
              }}
            </a-tag>
          </a-descriptions-item>
          <a-descriptions-item :label="$t('rhai_plugin.details') || 'Details'" :span="3">
            {{ executeResult.details }}
          </a-descriptions-item>
          <a-descriptions-item v-if="executeResult.raw_output" :label="$t('rhai_plugin.raw_output') || 'Raw Output'"
            :span="3">
            {{ executeResult.raw_output }}
          </a-descriptions-item>
        </a-descriptions>

        <a-divider />

        <h3 v-if="executeResult.data">{{ $t('rhai_plugin.data') || 'Data' }}</h3>
        <pre v-if="executeResult.data" class="result-data">{{ JSON.stringify(executeResult.data, null, 2) }}</pre>
      </div>
    </a-modal>

    <!-- 查看插件详情模态框 -->
    <a-modal v-model:visible="detailsModalVisible" :title="$t('rhai_plugin.view_details') || 'View Plugin Details'"
      width="800px" :footer="false">
      <div class="dark-mode-toggle">
        <span>{{ $t('rhai_plugin.dark_mode') || 'Dark Mode' }}</span>
        <a-switch v-model="isDarkMode" @change="onToggleDarkMode" />
      </div>
      <a-descriptions :title="currentPlugin?.name || ''" bordered :column="1">
        <a-descriptions-item :label="$t('rhai_plugin.plugin_name') || 'Name'">
          {{ currentPlugin?.name || '' }}
        </a-descriptions-item>
        <a-descriptions-item :label="$t('rhai_plugin.plugin_author') || 'Author'">
          {{ currentPlugin?.author || '' }}
        </a-descriptions-item>
        <a-descriptions-item :label="$t('rhai_plugin.plugin_type') || 'Type'">
          <a-tag>{{ currentPlugin?.type || currentPlugin?.rtype || '' }}</a-tag>
        </a-descriptions-item>
        <a-descriptions-item :label="$t('rhai_plugin.plugin_version') || 'Version'">
          {{ currentPlugin?.version || '' }}
        </a-descriptions-item>
        <a-descriptions-item v-if="currentPlugin?.severity" :label="$t('rhai_plugin.plugin_severity') || 'Severity'">
          <a-tag :color="getSeverityColor(currentPlugin.severity)">{{ currentPlugin?.severity || '' }}</a-tag>
        </a-descriptions-item>
        <a-descriptions-item :label="$t('rhai_plugin.plugin_description') || 'Description'">
          {{ currentPlugin?.description || '' }}
        </a-descriptions-item>
        <a-descriptions-item v-if="currentPlugin?.references && currentPlugin.references.length > 0"
          :label="$t('rhai_plugin.plugin_references') || 'References'">
          <div v-for="(ref, index) in currentPlugin?.references" :key="index">
            <a-link :href="ref" target="_blank">{{ ref }}</a-link>
          </div>
        </a-descriptions-item>
        <a-descriptions-item v-if="currentPlugin?.params && currentPlugin.params.length > 0"
          :label="$t('rhai_plugin.params') || 'Parameters'">
          <div v-for="(param, index) in currentPlugin?.params || []" :key="index">
            <a-tag color="blue">{{ param.name }}</a-tag>
            <span> {{ param.description }}</span>
          </div>
        </a-descriptions-item>
        <a-descriptions-item :label="$t('rhai_plugin.script') || 'Script'">
          <codemirror v-model="pluginScript" :placeholder="example_script"
            :style="{ height: '600px', width: '100%', fontSize: '12px' }" :autofocus="true" :indent-with-tab="true"
            :tab-size="2" :extensions="extensions" @ready="handleReady" />
        </a-descriptions-item>
      </a-descriptions>
      <div style="margin-top: 16px; text-align: right;">
        <a-button @click="detailsModalVisible = false">
          {{ $t('common.close') || 'Close' }}
        </a-button>
      </div>
    </a-modal>

    <!-- 添加/编辑插件模态框 -->
    <a-modal v-model:visible="addPluginModalVisible"
      :title="isEditingMode ? ($t('rhai_plugin.edit_title') || 'Edit Plugin') : ($t('rhai_plugin.add_title') || 'Add Plugin')"
      @ok="handleAddPlugin" :ok-text="$t('common.save') || 'Save'" :cancel-text="$t('common.cancel') || 'Cancel'"
      :confirmLoading="addPluginLoading" width="90%">
      <div class="plugin-editor-container">
        <!-- 脚本编辑器部分 -->
        <div class="script-editor-section">
          <div class="script-header">
            <div class="script-actions">
              <a-space>
                <a-button type="outline" @click="loadScriptTemplate">
                  <template #icon><icon-file /></template>
                  {{ $t('rhai_plugin.load_template') || 'Load Template' }}
                </a-button>
                <a-button type="outline" @click="toggleDarkMode">
                  <template #icon><icon-moon /></template>
                  {{ $t('rhai_plugin.dark_mode') || 'Dark Mode' }}
                </a-button>
                <a-button type="outline" @click="validateScript">
                  <template #icon><icon-check /></template>
                  {{ $t('rhai_plugin.validate') || 'Validate Script' }}
                </a-button>
              </a-space>
            </div>
          </div>

          <div class="code-editor-container" :class="{ 'dark-mode': isDarkMode }">
            <codemirror v-model="newPluginScript" :style="{ height: '600px', width: '100%', fontSize: '12px' }"
              :autofocus="true" :indent-with-tab="true" :tab-size="2" :extensions="extensions" @ready="handleReady" />
          </div>

          <div v-if="validationResult" class="validation-result" :class="{ 'valid': validationResult.valid }">
            <div class="validation-icon">
              <icon-check-circle-fill v-if="validationResult.valid" />
              <icon-close-circle-fill v-else />
            </div>
            <div class="validation-message">
              {{ validationResult.message }}
            </div>
          </div>
        </div>
      </div>
    </a-modal>
  </a-space>
</template>

<script setup>
import { ref, onMounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Message, Modal } from '@arco-design/web-vue';
import {
  getRhaiPlugins,
  getRhaiPlugin,
  updateRhaiPlugin,
  validatePluginScript,
  uploadRhaiPlugin,
  executeRhaiPlugin,
  deleteRhaiPlugin,
  reloadRhaiPlugins
} from '@/api/vuln';
import { IconEye, IconEdit, IconPlayCircle, IconDelete, IconCode, IconLink, IconCheckCircle, IconCheckCircleFill, IconCloseCircleFill, IconPlus, IconQuestionCircle, IconInfoCircle, IconPlusCircle, IconCodeBlock, IconBookmark, IconFile, IconMoon } from '@arco-design/web-vue/es/icon';

import { Codemirror } from 'vue-codemirror'
import { rust } from "@codemirror/lang-rust"
import { oneDark } from '@codemirror/theme-one-dark'


// CodeMirror configuration
const isDarkMode = ref(false);

const extensions = ref([
  rust(),
  isDarkMode.value ? oneDark : []
]);

const handleReady = () => {
  // CodeMirror is ready
  console.log('CodeMirror is ready');
};

const onToggleDarkMode = (value) => {
  isDarkMode.value = value;
  // 更新编辑器主题
  extensions.value = [
    rust(),
    isDarkMode.value ? oneDark : []
  ];
};

// Define example script for placeholder
const example_script = `// Rhai script example
let plugin_name = "Example Plugin";
`;

// 定义插件列表列
const columns = computed(() => [
  {
    title: '名称',
    dataIndex: 'name',
    slotName: "name",
    width: 260,
  },
  {
    title: '作者',
    dataIndex: 'author',
    width: 150
  },
  {
    title: '类型',
    dataIndex: 'type',
    width: 150
  },
  {
    title: '版本',
    dataIndex: 'version',
    width: 100
  },
  {
    title: '描述',
    dataIndex: 'description',
    ellipsis: true,
  },
  {
    title: '操作',
    slotName: "Operations",
    width: 100
  },
]);

// 状态
const plugins = ref([]);
const loading = ref(false);
const reloading = ref(false);
const uploadModalVisible = ref(false);
const uploadLoading = ref(false);
const executeModalVisible = ref(false);
const executeLoading = ref(false);
const detailsModalVisible = ref(false);
const currentPlugin = ref({});
const executeResult = ref(null);
const fileList = ref([]);
const pluginScript = ref('');

// 表单状态
const uploadForm = ref({
  file: null,
});

const executeForm = ref({
  target: '',
  customParams: {},
});

// Add new state variables for the Add Plugin modal
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

// Add new state variables for the plugin wizard
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

// Parameter examples data
const paramExampleColumns = computed(() => [
  {
    title: '参数',
    dataIndex: 'name',
  },
  {
    title: '描述',
    dataIndex: 'description',
  },
  {
    title: '使用',
    slotName: 'example',
    width: 80,
  }
]);

// Common parameter examples that users can quickly apply
const paramExamples = ref([
  {
    name: '超时',
    description: 'HTTP 请求超时（秒）',
    template: {
      key: 'timeout',
      name: '超时',
      type: 'number',
      required: false,
      default_value: 30,
      description: 'HTTP 请求超时（秒）'
    }
  },
  {
    name: '线程',
    description: '用于扫描的并发线程数',
    template: {
      key: 'threads',
      name: '线程',
      type: 'number',
      required: false,
      default_value: 5,
      description: '用于扫描的并发线程数'
    }
  },
  {
    name: '载荷',
    description: '用于测试的自定义载荷',
    template: {
      key: 'payload',
      name: '载荷',
      type: 'textarea',
      required: false,
      description: '用于测试的自定义载荷'
    }
  },
  {
    name: '详细模式',
    description: '启用详细输出',
    template: {
      key: 'verbose',
      name: '详细模式',
      type: 'boolean',
      required: false,
      default_value: false,
      description: '启用详细日志记录和输出'
    }
  }
]);

// Function to apply a parameter example template
const applyParamExample = (example) => {
  editingParam.value = { ...example.template };
  Message.success( '示例参数已应用');
};

// 处理文件变更
const handleFileChange = (files) => {
  fileList.value = files;
  uploadForm.value.file = files.length > 0 ? files[0].file : null;
};

// 加载插件列表
const loadPlugins = async () => {
  loading.value = true;
  try {
    // 获取插件列表
    const response = await getRhaiPlugins();
    console.log('Rhai Plugin API response:', response);

    // 确保插件数据格式正确
    plugins.value = response.map(plugin => {
      console.log(`处理插件: ${plugin.name}, params:`, plugin.params);

      // 确保params是一个数组
      const processedParams = Array.isArray(plugin.params) ? plugin.params : [];

      // 确保有一个有效的ID，必须采用 "type:name" 格式
      let fullName = plugin.id;

      // 如果没有id但有type和name，则构造一个
      if (!fullName && plugin.rtype && plugin.name) {
        fullName = `${plugin.rtype}:${plugin.name}`;
      } else if (!fullName && plugin.type && plugin.name) {
        fullName = `${plugin.type}:${plugin.name}`;
      }

      // 如果依然没有ID，使用其他可能的字段组合
      if (!fullName) {
        const typeField = plugin.rtype || plugin.type || plugin.r_type || plugin.type_name || 'unknown';
        fullName = `${typeField}:${plugin.name || 'unnamed'}`;
      }

      return {
        name: plugin.name,
        author: plugin.author,
        type: plugin.rtype || plugin.type || plugin.r_type || plugin.type_name,
        version: plugin.version,
        description: plugin.description,
        severity: plugin.severity,
        references: plugin.references,
        params: processedParams,
        result_fields: plugin.result_fields,
        script: plugin.script,
        id: plugin.id,
        fullName: fullName
      };
    });

    console.log('Formatted Rhai plugins for display:', plugins.value);
    Message.success('插件刷新成功');
  } catch (error) {
    Message.error('插件加载失败');
    console.error('Error loading Rhai plugins:', error);
  } finally {
    loading.value = false;
  }
};

// 从磁盘重新加载插件
const reloadFromDisk = async () => {
  reloading.value = true;
  try {
    // 从磁盘重新加载插件
    const result = await reloadRhaiPlugins();

    if (result === 'success') {
      Message.success('插件从磁盘重新加载成功');
    } else {
      Message.warning('插件重新加载完成，但存在警告');
    }

    // 重新加载插件列表
    await loadPlugins();
  } catch (error) {
    Message.error('插件从磁盘重新加载失败');
    console.error('Error reloading plugins:', error);
  } finally {
    reloading.value = false;
  }
};

// 上传插件相关方法
const showUploadModal = () => {
  fileList.value = [];
  uploadForm.value.file = null;
  uploadModalVisible.value = true;
};

const handleUpload = async () => {
  if (!uploadForm.value.file) {
    Message.error('请选择一个文件');
    return;
  }

  try {
    const result = await uploadRhaiPlugin(uploadForm.value.file);
    uploadModalVisible.value = false;
    Message.success('插件上传成功');

    // 成功后刷新插件列表
    await loadPlugins();
  } catch (error) {
    Message.error('插件上传失败');
  }
};

// 执行插件相关方法
const executePlugin = async (plugin) => {
  // 执行前检查
  const prepareExecution = (plugin) => {
    if (!executeForm.value.target) {
      Message.error('请输入目标');
      return;
    }

    if (!plugin) {
      Message.error('无效的插件选择');
      return;
    }
  };

  prepareExecution(plugin);

  if (!plugin || !plugin.id) {
    Message.error('无效的插件选择');
    return;
  }

  executeLoading.value = true;
  executeResult.value = null;

  try {
    const result = await executeRhaiPlugin({
      plugin_id: plugin.id,
      target: executeForm.value.target,
      proxy_url: executeForm.value.proxy,
      custom_params: executeForm.value.customParams || {}
    });

    executeResult.value = result;
    executeModalVisible.value = true;
  } catch (error) {
    Message.error('执行失败');
    console.error('执行Rhai插件失败:', error);
  } finally {
    executeLoading.value = false;
  }
};

const handleExecute = async () => {
  // 执行前检查
  const prepareExecution = (plugin) => {
    if (!executeForm.value.target) {
      Message.error('请输入目标');
      return;
    }

    if (!currentPlugin.value || !currentPlugin.value.fullName) {
      Message.error('无效的插件选择');
      return;
    }
  };

  prepareExecution(currentPlugin.value);

  console.log('Executing plugin with name:', currentPlugin.value.fullName);

  executeLoading.value = true;
  try {
    const result = await executeRhaiPlugin({
      plugin_name: currentPlugin.value.fullName,
      target: executeForm.value.target,
      custom_params: executeForm.value.customParams
    });

    executeResult.value = result;
    console.log('Execution result:', result);
  } catch (error) {
    Message.error('执行失败');
    console.error('执行Rhai插件失败:', error);
    executeResult.value = {
      success: false,
      details: `Error: ${error.message || error}`,
      data: null
    };
  } finally {
    executeLoading.value = false;
  }
};

// 删除插件
const confirmDelete = (plugin) => {
  Modal.warning({
    title: '确认删除',
    content: `Are you sure you want to delete plugin ${plugin.name}?`,
    okText: '删除',
    cancelText: '取消',
    onOk: () => deletePlugin(plugin)
  });
};

const deletePlugin = async (plugin) => {
  if (!plugin || !plugin.id) {
    return; // 静默忽略无效插件
  }

  try {
    await deleteRhaiPlugin(plugin.id);
    Message.success('插件删除成功');
    loadPlugins(); // 重新加载插件列表
  } catch (error) {
    Message.error('插件删除失败');
  }
};

// 查看插件详情
const viewPluginDetails = async (plugin) => {
  if (!plugin) {
    Message.error('无效的插件选择');
    return;
  }

  loading.value = true;

  try {
    console.log('Viewing plugin details:', plugin);
    // 获取插件ID
    let pluginId = plugin.id;

    // 如果没有id但有type和name，则构造一个
    if (!pluginId && plugin.rtype && plugin.name) {
      pluginId = `${plugin.rtype}:${plugin.name}`;
      console.log('Constructed ID from rtype:', pluginId);
    } else if (!pluginId && plugin.type && plugin.name) {
      pluginId = `${plugin.type}:${plugin.name}`;
      console.log('Constructed ID from type:', pluginId);
    }

    if (!pluginId) {
      console.error('Cannot determine plugin ID:', plugin);
      throw new Error('No valid plugin ID available');
    }

    console.log('Fetching plugin with ID:', pluginId);
    // 获取包含脚本内容的完整插件信息
    const fullPlugin = await getRhaiPlugin(pluginId);

    if (!fullPlugin) {
      console.error('No plugin data returned for ID:', pluginId);
      throw new Error(`No plugin data returned for ID: ${pluginId}`);
    }

    console.log('Received plugin details:', fullPlugin);
    // 更新当前插件
    currentPlugin.value = { ...fullPlugin };

    // 确保类型字段存在
    if (!currentPlugin.value.type && currentPlugin.value.rtype) {
      currentPlugin.value.type = currentPlugin.value.rtype;
    }

    // 设置脚本内容
    pluginScript.value = fullPlugin.script || '';

    // 显示详情对话框
    detailsModalVisible.value = true;
  } catch (error) {
    console.error('Error fetching plugin script:', error);
    Message.error('获取插件脚本失败');
  } finally {
    loading.value = false;
  }
};

// 编辑插件
const editPlugin = async (plugin) => {
  if (!plugin) {
    Message.error('无效的插件选择');
    return;
  }

  loading.value = true;

  try {
    console.log('Editing plugin:', plugin);

    // 获取插件ID
    let pluginId = plugin.id;

    // 如果没有id但有type和name，则构造一个
    if (!pluginId && plugin.rtype && plugin.name) {
      pluginId = `${plugin.rtype}:${plugin.name}`;
      console.log('Constructed ID from rtype:', pluginId);
    } else if (!pluginId && plugin.type && plugin.name) {
      pluginId = `${plugin.type}:${plugin.name}`;
      console.log('Constructed ID from type:', pluginId);
    }

    if (!pluginId) {
      console.error('Cannot determine plugin ID:', plugin);
      throw new Error('No valid plugin ID available');
    }

    console.log('Fetching plugin with ID:', pluginId);

    // 获取包含脚本内容的完整插件信息
    const fullPlugin = await getRhaiPlugin(pluginId);

    if (!fullPlugin) {
      console.error('No plugin data returned for ID:', pluginId);
      throw new Error(`No plugin data returned for ID: ${pluginId}`);
    }

    console.log('Received plugin details for editing:', fullPlugin);

    // 更新编辑中的插件
    newPlugin.value = {
      id: pluginId,
      name: fullPlugin.name || '',
      description: fullPlugin.description || '',
      type: fullPlugin.rtype || fullPlugin.type || 'web',
      author: fullPlugin.author || '',
      version: fullPlugin.version || '1.0.0'
    };

    // 设置脚本内容
    newPluginScript.value = fullPlugin.script || '';

    // 设置参数
    pluginParams.value = Array.isArray(fullPlugin.params)
      ? fullPlugin.params.map(param => ({
        key: param.key || '',
        name: param.name || '',
        type: param.type || 'string',
        required: !!param.required,
        description: param.description || '',
        default_value: param.default || param.default_value
      }))
      : [];

    // 使用向导式模态框
    currentStep.value = 1;
    validationResult.value = null;
    validationCompleted.value = false;
    isEditingMode.value = true;
    editingPluginId.value = pluginId;
    addPluginModalVisible.value = true;
  } catch (error) {
    console.error('Error fetching plugin for editing:', error);
    Message.error('获取插件编辑失败');
  } finally {
    loading.value = false;
  }
};

// 获取严重级别颜色
const getSeverityColor = (severity) => {
  switch (severity) {
    case 'critical':
      return 'rgb(var(--danger-6))';
    case 'high':
      return 'rgb(var(--orange-6))';
    case 'medium':
      return 'rgb(var(--warning-6))';
    case 'low':
      return 'rgb(var(--success-6))';
    case 'info':
      return 'rgb(var(--gray-6))';
    default:
      return '';
  }
};

// Function to show the Add Plugin modal
const showAddPluginModal = () => {
  console.log('showAddPluginModal', currentStep.value);
  newPlugin.value = {
    name: '',
    type: 'web',
    description: '',
    author: '',
    version: '1.0.0'
  };
  newPluginScript.value = example_script;
  pluginParams.value = [];
  currentStep.value = 1;
  validationResult.value = null;
  validationCompleted.value = false;
  isEditingMode.value = false;
  editingPluginId.value = null;
  addPluginModalVisible.value = true;
};

// Updated handleAddPlugin function
const handleAddPlugin = async () => {
  // 检查基本字段
  if (!newPlugin.value.name || !newPluginScript.value) {
    Message.error('请填写所有必填字段');
    return;
  }

  console.log('Saving plugin with params:', pluginParams.value);
  addPluginLoading.value = true;

  try {
    let result;

    // 为确保参数信息包含在脚本中，我们将参数信息添加到脚本的manifest函数中
    // 这需要修改脚本内容，将参数信息注入到manifest中
    const scriptWithParams = injectParamsIntoScript(newPluginScript.value, pluginParams.value);

    console.log('Original script length:', newPluginScript.value.length);
    console.log('Modified script length:', scriptWithParams.length);

    if (isEditingMode.value) {
      console.log('Updating existing plugin:', editingPluginId.value);
      // 更新插件
      result = await updateRhaiPlugin({
        plugin_id: editingPluginId.value,
        name: newPlugin.value.name,
        description: newPlugin.value.description,
        script: scriptWithParams
      });

      console.log('Update result:', result);
      Message.success('插件保存成功');
    } else {
      console.log('Creating new plugin:', newPlugin.value.name);
      // 上传新插件
      const filename = `${newPlugin.value.type || 'web'}_${newPlugin.value.name}.rhai`;
      const pluginFile = new File([scriptWithParams], filename);

      // 上传插件
      result = await uploadRhaiPlugin(pluginFile);

      console.log('Upload result:', result);
      Message.success('插件上传成功');
    }

    addPluginModalVisible.value = false;

    // Reset form and state
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

    // Reload the plugin list
    loadPlugins();
  } catch (error) {
    console.error('Error saving plugin:', error);
    const errorMessage = isEditingMode.value
      ? '保存插件失败'
      : '上传插件失败';

    Message.error(errorMessage);
  } finally {
    addPluginLoading.value = false;
  }
};

// Function to load a script template
const loadScriptTemplate = () => {
  const name = newPlugin.value.name || 'New Plugin';
  const description = newPlugin.value.description || 'Add description here';
  const author = newPlugin.value.author || 'User';
  const version = newPlugin.value.version || '1.0.0';
  const type = newPlugin.value.type || 'web';

  newPluginScript.value = `// ${name} - Rhai Plugin
// Description: ${description}
// Author: ${author} 
// Version: ${version}

// 返回插件的元数据}
fn get_manifest() {
    let manifest = #{
        name: "${name}",
        description: "${description}",
        author: "${author}",
        version: "${version}",
        rtype: "${type}",
        severity: "medium",
        references: [
            "https://example.com/reference1",
            "https://example.com/reference2"
        ],
        params: [
            #{
                name: "Timeout (seconds)",
                key: "timeout",
                type: "number",
                required: false,
                default_value: 30,
                description: "HTTP request timeout in seconds"
            },
            #{
                name: "Custom Header",
                key: "custom_header",
                type: "string",
                required: false,
                default_value: "",
                description: "Custom HTTP header to send with requests"
            }
        ],
        result_fields: [
            #{
                name: "Vulnerability Type",
                key: "type",
                type: "string",
                description: "Type of vulnerability detected"
            },
            #{
                name: "Risk Level",
                key: "risk_level",
                type: "string",
                description: "Risk level: high, medium, or low"
            },
            #{
                name: "Test Results",
                key: "test_results",
                type: "object",
                description: "Detailed test results"
            }
        ]
    };
    
    // Return JSON string
    return to_json(manifest);
}

// HTTP request helper function
fn send_http_request(params) {
    let request_json = to_json(params);
    
    // Print request information
    print_debug("Sending HTTP request: " + request_json);
    
    // Use built-in HTTP request function
    let response = http_request(request_json);
    
    // Print response information
    print_debug("Received HTTP response: " + response);
    
    // Parse response
    let parsed_response = parse_json(response);
    
    // Check for errors
    if parsed_response.error != () {
        print_error("HTTP request error: " + parsed_response.error);
    }
    
    return parsed_response;
}

// Main analysis function
fn analyze(request_json) {
    // Parse input request
    let request = parse_json(request_json);
    let target = request.target;
    
    // Get custom parameters or use defaults
    let params = request.params;
    if params == () {
        params = #{};
    }
    
    // Get timeout parameter or use default
    let timeout = 30;
    if params.timeout != () {
        timeout = params.timeout;
    }
    
    print_info("Analyzing target: " + target);
    
    // Construct HTTP request parameters
    let http_params = #{
        url: target,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        },
        timeout: timeout,
        follow_redirects: true
    };
    
    // Add custom request header if provided
    if params.custom_header != () && params.custom_header != "" {
        let header_parts = params.custom_header.split(":");
        if header_parts.len() >= 2 {
            let header_name = header_parts[0].trim();
            let header_value = header_parts[1].trim();
            http_params.headers[header_name] = header_value;
        }
    }
    
    // Send HTTP request
    let response = send_http_request(http_params);
    
    // Process response
    let success = false;
    let details = "No vulnerability detected";
    let test_results = #{};
    
    // Store HTTP request and response information
    let http_request = http_params;
    let http_response = response;
    let http_status = 0;
    
    if response.status != () {
        http_status = response.status;
    }
    
    // Check for vulnerability indicators in response
    if response.body != () {
        // Add your vulnerability detection logic here
        let body = response.body;
        
        // Example: Check for sensitive information disclosure
        if body.contains("password") || body.contains("api_key") || body.contains("secret") {
            success = true;
            details = "Potential information disclosure detected";
            test_results.sensitive_info = true;
        }
        
        // Example: Check for error message disclosure
        if body.contains("SQL syntax") || body.contains("ODBC Driver") {
            success = true;
            details = "SQL error disclosure detected";
            test_results.sql_error = true;
        }
        
        // Example: Check for server information disclosure
        if response.headers != () {
            let headers = to_json(response.headers).to_lower();
            if headers.contains("server:") || headers.contains("x-powered-by:") {
                test_results.server_info = true;
            }
        }
    }
    
    // Prepare result data
    let result_data = #{
        type: "info_disclosure",
        risk_level: "medium",
        test_results: test_results,
        http_request: http_params,
        http_response: response,
        http_status: http_status
    };
    
    // Return result JSON
    return to_json(#{
        success: success,
        details: details,
        data: result_data,
        raw_output: to_json(test_results),
        request: to_json(http_params),
        response: to_json(response),
        status_code: http_status
    });
}`;
};

// Function to insert code snippets
const insertCodeSnippet = (type) => {
  let snippet = '';

  switch (type) {
    case 'http':
      snippet = `// HTTP Request example
let response = http::get(target + "/path");
print("Status: " + response.status_code);

if response.status_code == 200 {
  // Check response body
  if regex::match("sensitive_data", response.body) {
    result.success = true;
    result.data.test_results["found_sensitive_data"] = true;
  }
}`;
      break;
    case 'success':
      snippet = `// Return success result example
return {
  success: true,
  data: {
    type: "information_disclosure",
    risk_level: "medium",
    test_results: {
      "endpoint_accessible": true,
      "sensitive_data_found": "user credentials exposed"
    },
    info: "The application exposes sensitive information",
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
    // Insert at cursor position or append to end
    newPluginScript.value += '\n\n' + snippet;
  }
};

// Parameter management functions
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

const editParameter = (index) => {
  const param = pluginParams.value[index];
  if (!param) {
    console.error('Parameter not found at index', index);
    return;
  }

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
  console.log('Editing parameter:', editingParam.value, 'at index', index);
};

const saveParameter = () => {
  // Basic validation
  if (!editingParam.value.key || !editingParam.value.name) {
    Message.error('Parameter key and name are required');
    return;
  }

  // Check for spaces in key
  if (/\s/.test(editingParam.value.key)) {
    Message.error('Parameter key cannot contain spaces');
    return;
  }

  // Validate key uniqueness (only for new parameters)
  if (editingParamIndex.value === -1) {
    const keyExists = pluginParams.value.some(param => param.key === editingParam.value.key);
    if (keyExists) {
      Message.error('Parameter key must be unique');
      return;
    }
  }

  // Type-specific validation and formatting
  if (editingParam.value.type === 'number' && editingParam.value.default_value !== undefined && editingParam.value.default_value !== '') {
    // Ensure number default values are actually numbers
    const numValue = Number(editingParam.value.default_value);
    if (isNaN(numValue)) {
      Message.error('Default value must be a valid number');
      return;
    }
    editingParam.value.default_value = numValue;
  }

  // Create clean parameter object
  const newParam = {
    key: editingParam.value.key.trim(),
    name: editingParam.value.name.trim(),
    type: editingParam.value.type,
    required: editingParam.value.required,
    description: editingParam.value.description?.trim() || '',
    default_value: editingParam.value.default_value
  };

  if (editingParamIndex.value === -1) {
    // Add new parameter
    pluginParams.value.push(newParam);
  } else {
    // Update existing parameter
    pluginParams.value[editingParamIndex.value] = newParam;
  }

  Message.success(
    editingParamIndex.value === -1
      ? 'Parameter added'
      : 'Parameter updated'
  );

  parameterModalVisible.value = false;
};

const deleteParameter = (index) => {
  pluginParams.value.splice(index, 1);
};

// Validate script function
const validateScript = async () => {
  if (!newPlugin.value.name || !newPluginScript.value) {
    validationResult.value = {
      valid: false,
      message: 'Plugin name and script are required'
    };
    return;
  }

  validationResult.value = null;

  try {
    // Call API to validate the script
    await validatePluginScript(newPluginScript.value);

    validationResult.value = {
      valid: true,
      message: 'Script validation successful!'
    };
    validationCompleted.value = true;

    // Show success notification
    Message.success({
      content: 'Script validation successful!',
      duration: 3000
    });
  } catch (error) {
    validationResult.value = {
      valid: false,
      message: `Validation failed: ${error.message || String(error)}`
    };

    // Show error notification
    Message.error({
      content: `Validation failed: ${error.message || String(error)}`,
      duration: 5000
    });
  }
};

// 组件挂载时加载插件列表
onMounted(() => {
  loadPlugins();
});

// Watch for parameter type changes and adjust the default value 
watch(
  () => editingParam.value.type,
  (newType, oldType) => {
    if (newType !== oldType) {
      // Reset default value when type changes
      if (newType === 'boolean') {
        editingParam.value.default_value = false;
      } else if (newType === 'number') {
        editingParam.value.default_value = 0;
      } else {
        editingParam.value.default_value = '';
      }

      console.log(`Parameter type changed from ${oldType} to ${newType}, reset default value`);
    }
  }
);

// 将参数信息注入到脚本的manifest函数中
const injectParamsIntoScript = (script, params) => {
  if (!script || !params || params.length === 0) {
    return script;
  }

  console.log('Injecting params into script:', params);

  // 提取脚本中的get_manifest函数
  const getManifestRegex = /fn\s+get_manifest\s*\(\s*\)\s*\{([\s\S]*?)\}/;
  const manifestMatch = script.match(getManifestRegex);

  if (!manifestMatch) {
    console.warn('Could not find get_manifest function in script');

    // 如果找不到get_manifest函数，添加一个基本的get_manifest函数
    const paramsCode = formatParamsAsRhai(params);
    const newManifest = `
// Added automatically to support parameters
fn get_manifest() {
    let manifest = #{
        name: "Plugin",
        description: "Auto-generated manifest",
        author: "System",
        version: "1.0.0",
        rtype: "web",
        params: [
${paramsCode}
        ]
    };
    manifest
}`;

    // 在脚本末尾添加新的get_manifest函数
    return script + '\n\n' + newManifest;
  }

  // 提取manifest对象定义
  const manifestContent = manifestMatch[1];
  const manifestObjRegex = /let\s+manifest\s*=\s*#{\s*([\s\S]*?)\}\s*;/;
  const manifestObjMatch = manifestContent.match(manifestObjRegex);

  if (!manifestObjMatch) {
    console.warn('Could not find manifest object in get_manifest function');
    return script;
  }

  // 检查脚本是否已经有params部分
  const paramsRegex = /params\s*:\s*\[\s*([\s\S]*?)\s*\]/;
  const paramsMatch = manifestObjMatch[1].match(paramsRegex);

  const paramsCode = formatParamsAsRhai(params);

  if (paramsMatch) {
    console.log('Found existing params section, replacing');

    // 构建新的manifest对象内容
    const manifestProps = manifestObjMatch[1];
    const newManifestProps = manifestProps.replace(
      /params\s*:\s*\[\s*[\s\S]*?\s*\]/,
      `params: [
${paramsCode}
        ]`
    );

    // 构建新的manifest对象
    const newManifestObj = `let manifest = #{${newManifestProps}};`;

    // 替换整个manifest对象
    return script.replace(
      /let\s+manifest\s*=\s*#{\s*[\s\S]*?\}\s*;/,
      newManifestObj
    );
  } else {
    console.log('No params section found, adding one');

    // 构建新的manifest对象内容
    const manifestProps = manifestObjMatch[1];

    // 检查最后一个属性是否有逗号
    const lastPropHasComma = /,\s*$/.test(manifestProps.trim());
    const separator = lastPropHasComma ? '' : ',';

    const newManifestProps = manifestProps + `${separator}
        params: [
${paramsCode}
        ]`;

    // 构建新的manifest对象
    const newManifestObj = `let manifest = #{${newManifestProps}};`;

    // 替换整个manifest对象
    return script.replace(
      /let\s+manifest\s*=\s*#{\s*[\s\S]*?\}\s*;/,
      newManifestObj
    );
  }
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

const toggleDarkMode = () => {
  isDarkMode.value = !isDarkMode.value;
  // 触发 onToggleDarkMode 以确保状态同步
  onToggleDarkMode(isDarkMode.value);
};
</script>

<style scoped>
.rhai-plugin-manager {
  padding: 16px;
}

.result-data {
  background-color: var(--color-fill-1);
  padding: 12px;
  border-radius: 4px;
  overflow-x: auto;
}

.dark-mode-toggle {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  margin-bottom: 12px;
  gap: 8px;
  z-index: 100;
  position: relative;
}

.clickable {
  cursor: pointer;
}

.plugin-wizard-container {
  display: flex;
  gap: 24px;
}

.plugin-wizard-steps {
  width: 200px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border-2);
  padding-right: 20px;
}

.plugin-wizard-content {
  flex: 1;
  min-width: 0;
  max-width: 60%;
}

.wizard-step {
  padding: 16px 0;
}

.wizard-step h3 {
  margin-top: 0;
  margin-bottom: 16px;
  font-size: 18px;
  color: var(--color-text-1);
}

.wizard-step-actions {
  margin-top: 24px;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.script-editor-controls {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.code-editor-wrapper {
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  overflow: hidden;
}

.code-editor-container {
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  overflow: hidden;

  :deep(.cm-editor) {
    height: 500px;
    font-size: 12px;
    background-color: var(--color-bg-2);
  }

  :deep(.cm-gutters) {
    border: none;
    background-color: var(--color-bg-2);
  }

  :deep(.cm-activeLineGutter) {
    background-color: var(--color-fill-2);
  }

  :deep(.cm-activeLine) {
    background-color: var(--color-fill-2);
  }

  :deep(.cm-content) {
    font-family: 'Fira Code', monospace;
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

.validation-result {
  display: flex;
  align-items: center;
  margin-top: 16px;
  padding: 12px;
  border-radius: 4px;
  background-color: var(--color-danger-1);

  &.valid {
    background-color: var(--color-success-1);
  }
}

.validation-icon {
  margin-right: 12px;
  font-size: 20px;
  color: var(--color-danger-6);
}

.validation-result.valid .validation-icon {
  color: var(--color-success-6);
}

.validation-message {
  flex: 1;
}

.api-doc-content {
  padding: 8px 0;

  h4 {
    margin-top: 0;
    margin-bottom: 16px;
    font-size: 16px;
    color: var(--color-text-1);
  }
}

.api-method {
  margin-bottom: 16px;
  padding: 8px;
  border-radius: 4px;
  background-color: var(--color-fill-2);
}

.method-signature {
  font-family: monospace;
  font-weight: 600;
  color: var(--color-text-1);
}

.method-desc {
  margin-top: 4px;
  color: var(--color-text-2);
  font-size: 13px;
}

.code-example {
  margin: 16px 0;
  padding: 12px;
  background-color: var(--color-fill-1);
  border-radius: 4px;
  border-left: 3px solid var(--color-primary-6);
  white-space: pre-wrap;
  font-family: monospace;
  font-size: 12px;
  line-height: 1.5;
  overflow-x: auto;
}

.plugin-api-docs {
  width: 30%;
  flex-shrink: 0;
  border-left: 1px solid var(--color-border-2);
  padding-left: 20px;
  height: 600px;
  overflow-y: auto;
}

.parameters-container {
  display: flex;
  flex-direction: column;
  gap: 20px;
  margin-bottom: 20px;
}

.parameters-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.parameter-cards-container {
  margin: 16px 0;
}

.parameter-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.param-card {
  border-radius: 8px;
  transition: all 0.3s ease;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  border: 1px solid var(--color-border-2);

  &:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    transform: translateY(-2px);
  }
}

.param-card-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.param-card-key code {
  font-family: monospace;
  font-size: 14px;
  background-color: var(--color-fill-2);
  padding: 2px 6px;
  border-radius: 4px;
}

.param-card-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.param-card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 8px;
}

.param-tags {
  display: flex;
  gap: 8px;
}

.param-name {
  margin: 0;
  font-size: 16px;
  color: var(--color-text-1);
}

.param-description {
  font-size: 14px;
  color: var(--color-text-2);
  line-height: 1.5;
}

.param-default {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.param-default-label {
  color: var(--color-text-3);
}

.param-default code {
  font-family: monospace;
  background-color: var(--color-fill-2);
  padding: 2px 6px;
  border-radius: 4px;
}

.usage-guide-container {
  margin: 20px 0;
}

.param-code-container {
  padding: 16px;
  border-radius: 8px;
  background-color: var(--color-fill-1);
}

.code-example-header {
  display: flex;
  align-items: center;
  margin-bottom: 10px;
  font-weight: 500;
  color: var(--color-text-1);
}

.code-block {
  padding: 12px;
  background-color: var(--color-fill-2);
  border-radius: 6px;
  font-family: monospace;
  overflow-x: auto;
  margin: 0;
  line-height: 1.5;
}

.input-with-tooltip {
  display: flex;
  align-items: flex-start;
  width: 100%;

  >*:first-child {
    flex: 1;
  }

  >a-tooltip,
  >a-button {
    margin-left: 8px;
    margin-top: 5px;
  }
}

.required-switch-container,
.boolean-default-container {
  display: flex;
  align-items: center;
  gap: 8px;
}

.required-label,
.boolean-label {
  font-size: 12px;
  color: var(--color-text-2);
}

.parameter-help-text {
  margin-top: 8px;
  font-size: 12px;
  color: var(--color-text-2);
}

.parameter-default-value {
  margin-left: 8px;
  font-size: 12px;
  color: var(--color-text-2);
}

.parameter-section-description {
  margin-top: 16px;
  margin-bottom: 16px;
  color: var(--color-text-2);
}

.method-structure {
  font-family: monospace;
  padding: 5px 10px;
  margin-top: 5px;
  background-color: var(--color-fill-2);
  border-radius: 4px;
  color: var(--color-text-1);
  line-height: 1.5;
}

.plugin-editor-container {
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
}

.script-actions {
  display: flex;
  gap: 8px;
}

.code-editor-container {
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  overflow: hidden;

  :deep(.cm-editor) {
    height: 600px;
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
    font-family: 'Fira Code', monospace;
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

.validation-result {
  display: flex;
  align-items: center;
  padding: 12px;
  border-radius: 4px;
  background-color: var(--color-danger-1);

  &.valid {
    background-color: var(--color-success-1);
  }
}

.validation-icon {
  margin-right: 12px;
  font-size: 20px;
  color: var(--color-danger-6);
}

.validation-result.valid .validation-icon {
  color: var(--color-success-6);
}

.validation-message {
  flex: 1;
}

:deep(.arco-modal) {
  .arco-modal-header {
    background-color: var(--color-bg-2);
    color: var(--color-text-1);
    border-bottom: 1px solid var(--color-border);
  }
  
  .arco-modal-content {
    background-color: var(--color-bg-2);
    color: var(--color-text-1);
  }
  
  .arco-modal-footer {
    background-color: var(--color-bg-2);
    border-top: 1px solid var(--color-border);
  }
}


</style>