<template>
  <div class="scan-plugin-manager">
    <div class="plugin-header">
      <h2>{{ t('scan_plugin.title') }}</h2>
      
      <a-space>
        <a-button type="primary" @click="fetchPlugins" :loading="loading">
          <template #icon><icon-refresh /></template>
          {{ t('scan_plugin.refresh') }}
        </a-button>
        
        <a-button @click="reloadAllPlugins" :loading="reloading">
          <template #icon><icon-code /></template>
          {{ t('scan_plugin.reload') }}
        </a-button>
        
        <a-button type="primary"  @click="handleUploadPlugin">
          <template #icon><icon-file /></template>
          {{ t('scan_plugin.upload') }}
        </a-button>
        
        <a-button type="primary"  @click="handleAddPluginClick">
          <template #icon><icon-plus /></template>
          {{ t('scan_plugin.addPlugin') }}
        </a-button>
      </a-space>
    </div>
    
    <!-- 插件列表表格 -->
    <a-table 
      :loading="loading" 
      :data="plugins" 
      :columns="columns" 
      :pagination="{ pageSize: 10 }"
      row-key="id"
      class="plugin-table"
    >
      <template #name="{ record }">
        <div class="plugin-name">
          <a-tooltip :content="record.name">
            <span>{{ record.name }}</span>
          </a-tooltip>
        </div>
      </template>
      
      <template #Operations="{ record }">
        <a-space>
          <a-button type="text" size="small" @click="handleViewPlugin(record)">
            <template #icon><icon-eye /></template>
          </a-button>
          
          <a-button type="text" size="small" @click="handleEditPlugin(record)">
            <template #icon><icon-edit /></template>
          </a-button>
          
          <a-button type="text" size="small" @click="handleExecutePlugin(record)">
            <template #icon><icon-play-circle /></template>
          </a-button>
          
          <a-button type="text" size="small" status="danger" @click="handleDeletePlugin(record)">
            <template #icon><icon-delete /></template>
          </a-button>
        </a-space>
      </template>
    </a-table>
    
    <!-- 上传插件模态框 -->
    <a-modal
      v-model:visible="uploadModalVisible"
      :title="t('scan_plugin.upload_title')"
      @cancel="uploadModalVisible = false"
      @ok="submitUpload"
      :ok-loading="uploadLoading"
    >
      <a-form :model="uploadForm">
        <a-form-item field="file" :label="t('scan_plugin.select_file')">
          <a-upload
            :file-list="fileList"
            :limit="1"
            @success="onUploadSuccess"
            accept=".rhai,.js"
            :auto-upload="false"
            :show-file-list="true"
            :unmount-on-close="false"
          />
        </a-form-item>
      </a-form>
    </a-modal>
    
    <!-- 执行插件模态框 -->
    <a-modal
      v-model:visible="executeModalVisible"
      :title="t('scan_plugin.execute_title')"
      @cancel="executeModalVisible = false"
      @ok="submitExecute"
      :ok-loading="executeLoading"
      :footer="executeResult ? null : undefined"
      width="800px"
    >
      <template v-if="!executeResult">
        <a-form :model="executeForm">
          <a-form-item field="target" :label="t('scan_plugin.target')">
            <a-input
              v-model="executeForm.target"
              :placeholder="t('scan_plugin.target_placeholder')"
              allow-clear
            />
          </a-form-item>
        </a-form>
      </template>
      
      <!-- 执行结果 -->
      <template v-else>
        <div class="execution-results">
          <h3>{{ t('scan_plugin.execute_results') }}</h3>
          
          <div class="result-status">
            <span>{{ t('scan_plugin.status') }}:</span>
            <a-tag :color="executeResult.status === 'success' ? 'green' : 'red'">
              {{ executeResult.status === 'success' ? t('scan_plugin.success') : t('scan_plugin.failure') }}
            </a-tag>
          </div>
          
          <div v-if="executeResult.message" class="result-message">
            <span>{{ t('scan_plugin.details') }}:</span>
            <pre>{{ executeResult.message }}</pre>
          </div>
          
          <div v-if="executeResult.raw_output" class="result-raw">
            <h4>{{ t('scan_plugin.raw_output') }}:</h4>
            <pre>{{ executeResult.raw_output }}</pre>
          </div>
          
          <div v-if="executeResult.data" class="result-data">
            <h4>{{ t('scan_plugin.data') }}:</h4>
            <pre>{{ JSON.stringify(executeResult.data, null, 2) }}</pre>
          </div>
        </div>
        
        <div class="execution-footer">
          <a-button type="primary" @click="executeModalVisible = false">
            {{ t('common.close') }}
          </a-button>
        </div>
      </template>
    </a-modal>
    
    <!-- 插件详情模态框 -->
    <a-modal
      v-model:visible="detailsModalVisible"
      :title="t('scan_plugin.view_details')"
      @cancel="detailsModalVisible = false"
      :footer="null"
      width="800px"
    >
      <div class="plugin-details">
        <div class="plugin-script">
          <div class="script-header">
            <h4>{{ t('scan_plugin.script') }}</h4>
            <a-switch 
              v-model="isDarkMode" 
              @change="onToggleDarkMode"
            >
              <template #checked-icon><icon-moon /></template>
              <template #unchecked-icon><icon-moon /></template>
              <template #checked>{{ t('scan_plugin.dark_mode') }}</template>
              <template #unchecked>{{ t('scan_plugin.dark_mode') }}</template>
            </a-switch>
          </div>
          
          <codemirror
            v-model="pluginScript"
            :extensions="extensions"
            :readonly="true"
            @ready="handleReady"
            placeholder="Plugin script will be displayed here"
            class="plugin-code-editor"
          />
        </div>
      </div>
    </a-modal>
    
    <!-- 添加/编辑插件模态框 -->
    <a-modal
      v-model:visible="addPluginModalVisible"
      :title="isEditingMode ? t('scan_plugin.edit_title') : t('scan_plugin.add_title')"
      @ok="handleAddPlugin"
      :ok-text="t('common.save')"
      :cancel-text="t('common.cancel')"
      :confirmLoading="addPluginLoading"
      width="90%"
    >
      <div class="plugin-editor-container">
        <!-- 脚本编辑器部分 -->
        <div class="script-editor-section">
          <div class="script-header">
            <div class="script-actions">
              <a-space>
                <a-dropdown>
                  <a-button type="outline">
                    <template #icon><icon-file /></template>
                    {{ t('scan_plugin.load_template') }}
                    <icon-down />
                  </a-button>
                  <template #content>
                    <a-doption @click="loadTemplateType('active')">
                      {{ t('scan_plugin.active_template') }}
                    </a-doption>
                    <a-doption @click="loadTemplateType('passive')">
                      {{ t('scan_plugin.passive_template') }}
                    </a-doption>
                  </template>
                </a-dropdown>
                <a-button type="outline" @click="toggleDarkMode">
                  <template #icon><icon-moon /></template>
                  {{ t('scan_plugin.dark_mode') }}
                </a-button>
                <a-button type="outline" @click="validateScript">
                  <template #icon><icon-check /></template>
                  {{ t('scan_plugin.validate') }}
                </a-button>
              </a-space>
            </div>
          </div>

          <div class="code-editor-container" :class="{ 'dark-mode': isDarkMode }">
            <codemirror
              v-model="newPluginScript"
              :style="{ height: '600px', width: '100%', fontSize: '12px' }"
              :autofocus="true"
              :indent-with-tab="true"
              :tab-size="2"
              :extensions="extensions"
              @ready="handleReady"
              placeholder="// Enter your plugin script here..."
            />
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
    
    <!-- 参数编辑模态框 -->
    <a-modal
      v-model:visible="parameterModalVisible"
      :title="editingParamIndex >= 0 ? t('scan_plugin.edit_param') : t('scan_plugin.add_param')"
      @cancel="parameterModalVisible = false"
      @ok="saveParameter"
    >
      <a-form :model="editingParam">
        <a-form-item field="key" :label="t('scan_plugin.param_key')" required>
          <a-input v-model="editingParam.key" allow-clear placeholder="parameter_key" />
        </a-form-item>
        
        <a-form-item field="name" :label="t('scan_plugin.param_name')" required>
          <a-input v-model="editingParam.name" allow-clear placeholder="Parameter Name" />
        </a-form-item>
        
        <a-form-item field="type" :label="t('scan_plugin.param_type')">
          <a-select v-model="editingParam.type">
            <a-option value="string">String</a-option>
            <a-option value="number">Number</a-option>
            <a-option value="boolean">Boolean</a-option>
          </a-select>
        </a-form-item>
        
        <a-form-item field="required" :label="t('scan_plugin.param_required')">
          <a-switch v-model="editingParam.required" />
        </a-form-item>
        
        <a-form-item field="description" :label="t('scan_plugin.param_description')">
          <a-textarea v-model="editingParam.description" allow-clear />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup>
import { ref, onMounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Message, Modal } from '@arco-design/web-vue';
import {
  getScanPlugins,
  getScanPlugin,
  updateScanPlugin,
  validatePluginScript,
  uploadScanPlugin,
  executeScanPlugin,
  deleteScanPlugin,
  reloadScanPlugins
} from '@/api/scan';
import { IconEye, IconEdit, IconPlayCircle, IconDelete, IconCode, IconLink, IconCheckCircle, IconCheckCircleFill, IconCloseCircleFill, IconPlus, IconQuestionCircle, IconInfoCircle, IconPlusCircle, IconCodeBlock, IconBookmark, IconFile, IconMoon, IconDown, IconRefresh } from '@arco-design/web-vue/es/icon';

import { Codemirror } from 'vue-codemirror'
import { rust } from "@codemirror/lang-rust"
import { oneDark } from '@codemirror/theme-one-dark'

const { t } = useI18n({
  messages: {
    'zh-CN': {
      'scan_plugin.title': '扫描插件管理',
      'scan_plugin.refresh': '刷新',
      'scan_plugin.reload': '从磁盘重新加载',
      'scan_plugin.upload': '上传',
      'scan_plugin.addPlugin': '添加插件',
      'scan_plugin.upload_title': '上传插件',
      'scan_plugin.select_file': '选择文件',
      'scan_plugin.edit_title': '编辑插件',
      'scan_plugin.add_title': '添加插件',
      'scan_plugin.load_template': '加载模板',
      'scan_plugin.active_template': '主动扫描模板',
      'scan_plugin.passive_template': '被动扫描模板',
      'scan_plugin.dark_mode': '暗黑模式',
      'scan_plugin.validate': '验证脚本',
      'scan_plugin.validation_success': '脚本验证成功！',
      'scan_plugin.validation_failed': '验证失败',
      'scan_plugin.refresh_success': '插件列表已刷新',
      'scan_plugin.load_failure': '加载插件失败',
      'scan_plugin.reload_success': '重新加载插件成功',
      'scan_plugin.reload_warning': '插件重新加载完成，但有警告',
      'scan_plugin.reload_failure': '重新加载插件失败',
      'scan_plugin.file_required': '请选择文件',
      'scan_plugin.upload_success': '插件上传成功',
      'scan_plugin.upload_failure': '上传插件失败',
      'scan_plugin.target_required': '请输入目标',
      'scan_plugin.plugin_invalid': '无效的插件',
      'scan_plugin.execute_failure': '执行失败',
      'scan_plugin.delete_confirm_title': '确认删除',
      'scan_plugin.delete_confirm_content': '确定要删除插件 {name} 吗？',
      'scan_plugin.delete_confirm_ok': '删除',
      'scan_plugin.delete_confirm_cancel': '取消',
      'scan_plugin.delete_success': '删除插件成功',
      'scan_plugin.delete_failure': '删除插件失败',
      'scan_plugin.fetch_script_error': '获取插件脚本失败',
      'scan_plugin.required_fields': '请填写必要的插件信息',
      'scan_plugin.edit_success': '插件保存成功',
      'scan_plugin.plugin_upload_success': '插件上传成功',
      'scan_plugin.edit_failed': '保存插件修改失败',
      'scan_plugin.plugin_upload_failed': '上传插件失败',
      'scan_plugin.view_details': '查看插件详情',
      'scan_plugin.script': '脚本',
      'scan_plugin.execute_title': '执行插件',
      'scan_plugin.target': '目标',
      'scan_plugin.target_placeholder': '输入目标 URL 或 IP',
      'scan_plugin.proxy_url': '代理 URL',
      'scan_plugin.proxy_url_placeholder': 'HTTP 代理 URL，例如 http://127.0.0.1:8080',
      'scan_plugin.proxy_url_help': '可选的 HTTP 代理，格式：http://host:port',
      'scan_plugin.plugin_parameters': '插件参数',
      'scan_plugin.parameters_section_description': '配置插件执行所需的参数：',
      'scan_plugin.required': '必填',
      'scan_plugin.default_value': '默认值',
      'scan_plugin.execute_results': '执行结果',
      'scan_plugin.status': '状态',
      'scan_plugin.success': '成功',
      'scan_plugin.failure': '失败',
      'scan_plugin.details': '详情',
      'scan_plugin.raw_output': '原始输出',
      'scan_plugin.data': '数据',
      'scan_plugin.plugin_name': '插件名称',
      'scan_plugin.plugin_author': '作者',
      'scan_plugin.plugin_type': '类型',
      'scan_plugin.plugin_version': '版本',
      'scan_plugin.plugin_severity': '严重程度',
      'scan_plugin.plugin_description': '描述',
      'scan_plugin.plugin_references': '参考链接',
      'scan_plugin.params': '参数',
      'common.save': '保存',
      'common.cancel': '取消',
      'common.yes': '是',
      'common.no': '否',
      'common.close': '关闭',
      'common.true': '是',
      'common.false': '否'
    },
    'en-US': {
      'scan_plugin.title': 'Scan Plugin Manager',
      'scan_plugin.refresh': 'Refresh',
      'scan_plugin.reload': 'Reload from Disk',
      'scan_plugin.upload': 'Upload',
      'scan_plugin.addPlugin': 'Add Plugin',
      'scan_plugin.upload_title': 'Upload Plugin',
      'scan_plugin.select_file': 'Select File',
      'scan_plugin.edit_title': 'Edit Plugin',
      'scan_plugin.add_title': 'Add Plugin',
      'scan_plugin.load_template': 'Load Template',
      'scan_plugin.active_template': 'Active Scan Template',
      'scan_plugin.passive_template': 'Passive Scan Template',
      'scan_plugin.dark_mode': 'Dark Mode',
      'scan_plugin.validate': 'Validate Script',
      'scan_plugin.validation_success': 'Script validation successful!',
      'scan_plugin.validation_failed': 'Validation failed',
      'scan_plugin.refresh_success': 'Plugin list refreshed',
      'scan_plugin.load_failure': 'Failed to load plugins',
      'scan_plugin.reload_success': 'Plugins reloaded successfully',
      'scan_plugin.reload_warning': 'Plugin reload completed with warnings',
      'scan_plugin.reload_failure': 'Failed to reload plugins',
      'scan_plugin.file_required': 'Please select a file',
      'scan_plugin.upload_success': 'Plugin uploaded successfully',
      'scan_plugin.upload_failure': 'Failed to upload plugin',
      'scan_plugin.target_required': 'Please enter a target',
      'scan_plugin.plugin_invalid': 'Invalid plugin',
      'scan_plugin.execute_failure': 'Execution failed',
      'scan_plugin.delete_confirm_title': 'Confirm Delete',
      'scan_plugin.delete_confirm_content': 'Are you sure you want to delete plugin {name}?',
      'scan_plugin.delete_confirm_ok': 'Delete',
      'scan_plugin.delete_confirm_cancel': 'Cancel',
      'scan_plugin.delete_success': 'Plugin deleted successfully',
      'scan_plugin.delete_failure': 'Failed to delete plugin',
      'scan_plugin.fetch_script_error': 'Failed to fetch plugin script',
      'scan_plugin.required_fields': 'Please fill all required fields',
      'scan_plugin.edit_success': 'Plugin saved successfully',
      'scan_plugin.plugin_upload_success': 'Plugin uploaded successfully',
      'scan_plugin.edit_failed': 'Failed to save plugin changes',
      'scan_plugin.plugin_upload_failed': 'Failed to upload plugin',
      'scan_plugin.view_details': 'View Plugin Details',
      'scan_plugin.script': 'Script',
      'scan_plugin.execute_title': 'Execute Plugin',
      'scan_plugin.target': 'Target',
      'scan_plugin.target_placeholder': 'Enter target URL or IP',
      'scan_plugin.proxy_url': 'Proxy URL',
      'scan_plugin.proxy_url_placeholder': 'HTTP proxy URL, e.g. http://127.0.0.1:8080',
      'scan_plugin.proxy_url_help': 'Optional HTTP proxy for all requests. Format: http://host:port',
      'scan_plugin.plugin_parameters': 'Plugin Parameters',
      'scan_plugin.parameters_section_description': 'Configure parameters for this plugin execution:',
      'scan_plugin.required': 'Required',
      'scan_plugin.default_value': 'Default',
      'scan_plugin.execute_results': 'Execution Results',
      'scan_plugin.status': 'Status',
      'scan_plugin.success': 'Success',
      'scan_plugin.failure': 'Failure',
      'scan_plugin.details': 'Details',
      'scan_plugin.raw_output': 'Raw Output',
      'scan_plugin.data': 'Data',
      'scan_plugin.plugin_name': 'Name',
      'scan_plugin.plugin_author': 'Author',
      'scan_plugin.plugin_type': 'Type',
      'scan_plugin.plugin_version': 'Version',
      'scan_plugin.plugin_severity': 'Severity',
      'scan_plugin.plugin_description': 'Description',
      'scan_plugin.plugin_references': 'References',
      'scan_plugin.params': 'Parameters',
      'common.save': 'Save',
      'common.cancel': 'Cancel',
      'common.yes': 'Yes',
      'common.no': 'No',
      'common.close': 'Close',
      'common.true': 'True',
      'common.false': 'False'
    }
  },
  locale: 'zh-CN',
  fallbackLocale: 'en-US'
});

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
const example_script_active = `// Rhai脚本 - 主动扫描插件示例
fn get_manifest() {
  return #{
    "name": "主动扫描示例插件",
    "author": "安全团队",
    "type": "active",
    "version": "1.0.0",
    "description": "这是一个主动扫描示例插件，用于检测基本的HTTP安全头",
    "severity": "low",
    "references": [
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Content-Type-Options",
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options",
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP"
    ],
    "parameters": [
      {
        "key": "timeout",
        "name": "超时设置",
        "type": "number",
        "required": false,
        "description": "HTTP请求超时时间(毫秒)",
        "default": 5000
      }
    ]
  };
}

fn analyze(request_json) {
  // 解析请求参数
  let request = parse_json(request_json);
  let target = request["target"];
  
  print_info("开始主动扫描目标: " + target);
  
  // 执行HTTP请求
  let response = http_request("GET", target, #{}, #{});
  
  // 检查响应
  if response.len() > 0 {
    print_info("获取到HTTP响应，长度: " + response.len());
    
    // 简单的安全检查示例
    let security_issues = [];
    
    // 检查是否缺少安全头
    if !response.contains("X-Content-Type-Options") {
      security_issues.push("缺少X-Content-Type-Options安全头");
    }
    
    if !response.contains("X-Frame-Options") {
      security_issues.push("缺少X-Frame-Options安全头，可能存在点击劫持风险");
    }
    
    if !response.contains("Content-Security-Policy") {
      security_issues.push("缺少Content-Security-Policy内容安全策略");
    }
    
    // 返回结果
    if security_issues.len() > 0 {
      return to_json(#{
        "status": "success",
        "message": "发现" + security_issues.len() + "个安全问题",
        "raw_output": response,
        "data": security_issues
      });
    } else {
      return to_json(#{
        "status": "success",
        "message": "未发现安全问题",
        "raw_output": response,
        "data": []
      });
    }
  } else {
    return to_json(#{
      "status": "failure",
      "message": "无法获取HTTP响应",
      "raw_output": "",
      "data": []
    });
  }
}
`;

const example_script_passive = `// Rhai脚本 - 被动扫描插件示例
fn get_manifest() {
  return #{
    "name": "被动扫描示例插件",
    "author": "安全团队",
    "type": "passive",
    "version": "1.0.0",
    "description": "这是一个被动扫描示例插件，用于检测响应中的敏感信息和安全头",
    "severity": "medium",
    "references": [
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security",
      "https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-XSS-Protection"
    ],
    "parameters": [
      {
        "key": "check_credit_card",
        "name": "检查信用卡信息",
        "type": "boolean",
        "required": false,
        "description": "是否检查信用卡信息泄露",
        "default": true
      }
    ]
  };
}

fn analyze(request_json) {
  // 解析请求参数
  let request = parse_json(request_json);
  let target = request["target"];
  
  print_info("开始被动扫描分析: " + target);
  
  // 解析目标（假设是一个HTTP请求/响应对）
  let parts = target.split("\\n\\n");
  let request_headers = "";
  let response_headers = "";
  let response_body = "";
  
  if parts.len() >= 2 {
    request_headers = parts[0];
    let response = parts[1];
    
    // 进一步解析响应
    let resp_parts = response.split("\\n\\n");
    if resp_parts.len() >= 2 {
      response_headers = resp_parts[0];
      response_body = resp_parts[1];
    } else {
      response_headers = response;
    }
  }
  
  // 分析响应
  let findings = [];
  
  // 检查敏感信息泄露
  if response_body.contains("password") || response_body.contains("密码") {
    findings.push("响应中可能包含密码信息");
  }
  
  if response_body.contains("credit card") || response_body.contains("信用卡") {
    findings.push("响应中可能包含信用卡信息");
  }
  
  // 检查安全头
  if !response_headers.contains("Strict-Transport-Security") {
    findings.push("缺少HSTS安全头");
  }
  
  if !response_headers.contains("X-XSS-Protection") {
    findings.push("缺少XSS保护头");
  }
  
  // 返回结果
  if findings.len() > 0 {
    return to_json(#{
      "status": "success",
      "message": "发现" + findings.len() + "个安全问题",
      "raw_output": target,
      "data": findings
    });
  } else {
    return to_json(#{
      "status": "success",
      "message": "未发现安全问题",
      "raw_output": target,
      "data": []
    });
  }
}
`;

// 示例脚本集合
const example_scripts = {
  active: example_script_active,
  passive: example_script_passive
};

// 当前选择的模板类型
const selectedTemplateType = ref('active');

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
  type: 'scan',
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

// 生命周期钩子
onMounted(() => {
  fetchPlugins();
});

// 方法
const fetchPlugins = async () => {
  loading.value = true;
  try {
    const response = await getScanPlugins();
    plugins.value = response.data;
    Message.success(t('scan_plugin.refresh_success'));
  } catch (error) {
    console.error('Error fetching plugins:', error);
    Message.error(t('scan_plugin.load_failure'));
  } finally {
    loading.value = false;
  }
};

const reloadAllPlugins = async () => {
  reloading.value = true;
  try {
    const response = await reloadScanPlugins();
    plugins.value = response.data.plugins;
    
    if (response.data.status === 'success') {
      Message.success(t('scan_plugin.reload_success'));
    } else {
      Message.warning(t('scan_plugin.reload_warning'));
    }
  } catch (error) {
    console.error('Error reloading plugins:', error);
    Message.error(t('scan_plugin.reload_failure'));
  } finally {
    reloading.value = false;
  }
};

const handleUploadPlugin = () => {
  uploadModalVisible.value = true;
  uploadForm.value = { file: null };
  fileList.value = [];
};

const onUploadSuccess = (file) => {
  uploadForm.value.file = file;
};

const submitUpload = async () => {
  if (!uploadForm.value.file) {
    Message.error(t('scan_plugin.file_required'));
    return;
  }

  uploadLoading.value = true;
  try {
    const formData = new FormData();
    formData.append('file', uploadForm.value.file.file);
    
    const response = await uploadScanPlugin(formData);
    
    Message.success(t('scan_plugin.upload_success'));
    uploadModalVisible.value = false;
    fetchPlugins();
  } catch (error) {
    console.error('Error uploading plugin:', error);
    Message.error(t('scan_plugin.upload_failure'));
  } finally {
    uploadLoading.value = false;
  }
};

const handleExecutePlugin = (plugin) => {
  currentPlugin.value = plugin;
  executeModalVisible.value = true;
  executeForm.value = {
    target: '',
    customParams: {},
  };
  executeResult.value = null;
  
  // 初始化自定义参数
  if (plugin.parameters) {
    plugin.parameters.forEach(param => {
      if (param.default) {
        executeForm.value.customParams[param.key] = param.default;
      } else {
        executeForm.value.customParams[param.key] = '';
      }
    });
  }
};

const submitExecute = async () => {
  if (!executeForm.value.target) {
    Message.error(t('scan_plugin.target_required'));
    return;
  }
  
  if (!currentPlugin.value.id) {
    Message.error(t('scan_plugin.plugin_invalid'));
    return;
  }
  
  executeLoading.value = true;
  executeResult.value = null;
  
  try {
    const payload = {
      plugin_id: currentPlugin.value.id,
      target: executeForm.value.target,
      params: executeForm.value.customParams || {},
    };
    
    const response = await executeScanPlugin(payload);
    executeResult.value = response.data;
  } catch (error) {
    console.error('Error executing plugin:', error);
    Message.error(t('scan_plugin.execute_failure'));
    executeResult.value = {
      status: 'error',
      message: error.message || t('scan_plugin.execute_failure'),
    };
  } finally {
    executeLoading.value = false;
  }
};

const handleViewPlugin = async (plugin) => {
  currentPlugin.value = plugin;
  detailsModalVisible.value = true;
  pluginScript.value = '';
  
  try {
    const response = await getScanPlugin(plugin.id);
    if (response.data && response.data.script) {
      pluginScript.value = response.data.script;
    }
  } catch (error) {
    console.error('Error fetching plugin script:', error);
    Message.error(t('scan_plugin.fetch_script_error'));
  }
};

const handleDeletePlugin = (plugin) => {
  Modal.warning({
    title: t('scan_plugin.delete_confirm_title'),
    content: t('scan_plugin.delete_confirm_content', { name: plugin.name }),
    okText: t('scan_plugin.delete_confirm_ok'),
    cancelText: t('scan_plugin.delete_confirm_cancel'),
    onOk: async () => {
      try {
        await deleteScanPlugin(plugin.id);
        Message.success(t('scan_plugin.delete_success'));
        fetchPlugins();
      } catch (error) {
        console.error('Error deleting plugin:', error);
        Message.error(t('scan_plugin.delete_failure'));
      }
    }
  });
};

// 添加新插件方法
const handleAddPlugin = async () => {
  if (!newPluginScript.value) {
    Message.error(t('scan_plugin.required_fields'));
    return;
  }
  
  addPluginLoading.value = true;
  
  try {
    const pluginData = {
      script: newPluginScript.value
    };
    
    if (isEditingMode.value && editingPluginId.value) {
      // 更新现有插件
      await updateScanPlugin(editingPluginId.value, pluginData);
      Message.success(t('scan_plugin.edit_success'));
    } else {
      // 上传新插件
      const filename = `plugin_${Date.now()}.rhai`; // 生成一个唯一的文件名
      await uploadScanPlugin({
        filename: filename,
        content: newPluginScript.value
      });
      Message.success(t('scan_plugin.plugin_upload_success'));
    }
    
    addPluginModalVisible.value = false;
    fetchPlugins();
  } catch (error) {
    console.error('Error saving plugin:', error);
    
    if (isEditingMode.value) {
      Message.error(t('scan_plugin.edit_failed'));
    } else {
      Message.error(t('scan_plugin.plugin_upload_failed'));
    }
  } finally {
    addPluginLoading.value = false;
  }
};

const handleEditPlugin = async (plugin) => {
  editingPluginId.value = plugin.id;
  isEditingMode.value = true;
  addPluginModalVisible.value = true;
  validationResult.value = null;
  validationCompleted.value = false;
  
  // 加载插件信息和脚本
  try {
    const response = await getScanPlugin(plugin.id);
    const pluginData = response.data;
    newPluginScript.value = pluginData.script || '';
  } catch (error) {
    console.error('Error fetching plugin details:', error);
    Message.error(t('scan_plugin.fetch_script_error'));
  }
};

const addParameter = () => {
  parameterModalVisible.value = true;
  editingParam.value = {
    key: '',
    name: '',
    type: 'string',
    required: false,
    description: ''
  };
  editingParamIndex.value = -1;
};

const editParameter = (index) => {
  const param = pluginParams.value[index];
  editingParam.value = { ...param };
  editingParamIndex.value = index;
  parameterModalVisible.value = true;
};

const saveParameter = () => {
  // 验证参数
  if (!editingParam.value.key || !editingParam.value.name) {
    Message.error('参数键和名称是必填的');
    return;
  }
  
  // 检查键是否重复
  const isDuplicate = pluginParams.value.some((param, index) => 
    param.key === editingParam.value.key && index !== editingParamIndex.value
  );
  
  if (isDuplicate) {
    Message.error('参数键必须唯一');
    return;
  }
  
  if (editingParamIndex.value >= 0) {
    // 编辑现有参数
    pluginParams.value[editingParamIndex.value] = { ...editingParam.value };
  } else {
    // 添加新参数
    pluginParams.value.push({ ...editingParam.value });
  }
  
  parameterModalVisible.value = false;
};

const removeParameter = (index) => {
  pluginParams.value.splice(index, 1);
};

const validateScript = async () => {
  if (!newPluginScript.value) {
    Message.error('请输入脚本内容');
    return;
  }
  
  try {
    // 先在本地做基本语法检查
    try {
      // 简单检查常见的Rhai语法错误
      let scriptLines = newPluginScript.value.split('\n');
      let openBraces = 0;
      let closeBraces = 0;
      let openBrackets = 0;
      let closeBrackets = 0;
      
      for (let i = 0; i < scriptLines.length; i++) {
        const line = scriptLines[i];
        // 检查花括号是否平衡
        for (let j = 0; j < line.length; j++) {
          if (line[j] === '{') openBraces++;
          if (line[j] === '}') closeBraces++;
          if (line[j] === '[') openBrackets++;
          if (line[j] === ']') closeBrackets++;
        }
        
        // 检查常见的语句没有分号
        if (line.trim().length > 0 && 
            !line.trim().endsWith('{') && 
            !line.trim().endsWith('}') && 
            !line.trim().endsWith(';') && 
            !line.trim().endsWith('[') && 
            !line.trim().endsWith(']') && 
            !line.trim().startsWith('//') && 
            !line.trim().startsWith('fn ') && 
            !line.trim().startsWith('if ') && 
            !line.trim().startsWith('else')) {
          console.warn(`可能缺少分号: 第${i+1}行: ${line}`);
        }
      }
      
      if (openBraces !== closeBraces) {
        console.warn(`花括号不平衡: 开始${openBraces}个, 结束${closeBraces}个`);
      }
      
      if (openBrackets !== closeBrackets) {
        console.warn(`方括号不平衡: 开始${openBrackets}个, 结束${closeBrackets}个`);
      }
    } catch (syntaxError) {
      console.error('本地语法检查失败:', syntaxError);
    }
    
    // 调用后端验证
    const response = await validatePluginScript({ script: newPluginScript.value });
    validationResult.value = response.data;
    validationCompleted.value = true;
    
    if (response.data.valid) {
      Message.success(t('scan_plugin.validation_success'));
    } else {
      const errorMsg = response.data.message || t('scan_plugin.validation_failed');
      console.error('脚本验证失败:', errorMsg);
      
      // 如果错误消息中包含行号和位置信息，尝试提取并高亮错误位置
      const lineMatch = errorMsg.match(/line (\d+), position (\d+)/);
      if (lineMatch && lineMatch.length >= 3) {
        const lineNum = parseInt(lineMatch[1]);
        const position = parseInt(lineMatch[2]);
        
        // 获取错误所在行的代码
        const scriptLines = newPluginScript.value.split('\n');
        if (lineNum <= scriptLines.length) {
          const errorLine = scriptLines[lineNum - 1];
          console.error(`错误位置: 第${lineNum}行, 位置${position}: "${errorLine}"`);
          
          // 在错误消息中添加代码上下文
          let contextMsg = errorMsg + '\n\n';
          if (lineNum > 1) contextMsg += `第${lineNum-1}行: ${scriptLines[lineNum-2]}\n`;
          contextMsg += `第${lineNum}行: ${errorLine}\n`;
          contextMsg += ' '.repeat(position + 7) + '^ 错误位置\n';
          if (lineNum < scriptLines.length) contextMsg += `第${lineNum+1}行: ${scriptLines[lineNum]}\n`;
          
          Message.error(contextMsg);
        } else {
          Message.error(`${errorMsg}\n(错误行号${lineNum}超出了脚本总行数${scriptLines.length})`);
        }
      } else {
        Message.error(errorMsg);
      }
    }
  } catch (error) {
    console.error('Error validating script:', error);
    validationResult.value = {
      valid: false,
      message: error.message || t('scan_plugin.validation_failed')
    };
    validationCompleted.value = true;
    Message.error(`${t('scan_plugin.validation_failed')}: ${error.message}`);
  }
};

const nextStep = () => {
  if (currentStep.value === 1) {
    // 基本信息验证
    if (!newPlugin.value.name || !newPlugin.value.author) {
      Message.error(t('scan_plugin.required_fields'));
      return;
    }
  }
  
  currentStep.value += 1;
};

const prevStep = () => {
  currentStep.value -= 1;
};

const loadScriptTemplate = () => {
  newPluginScript.value = example_scripts[selectedTemplateType.value];
};

const toggleDarkMode = () => {
  isDarkMode.value = !isDarkMode.value;
  onToggleDarkMode(isDarkMode.value);
};

const handleAddPluginClick = () => {
  addPluginModalVisible.value = true;
  newPluginScript.value = example_scripts[selectedTemplateType.value];
  isEditingMode.value = false;
  validationResult.value = null;
  validationCompleted.value = false;
};

const loadTemplateType = (type) => {
  console.log(`Loading template type: ${type}`);
  console.log(`Available templates: ${Object.keys(example_scripts).join(', ')}`);
  selectedTemplateType.value = type;
  
  if (example_scripts[type]) {
    console.log(`Template found, length: ${example_scripts[type].length}`);
    newPluginScript.value = example_scripts[type];
    Message.success(`已加载${type === 'active' ? '主动' : '被动'}扫描模板`);
  } else {
    console.error(`Template not found for type: ${type}`);
    Message.error(`模板类型 ${type} 不存在`);
  }
};
</script>

<style scoped>
.plugin-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.plugin-name {
  cursor: pointer;
  color: var(--color-text-2);
}

.plugin-name:hover {
  color: rgb(var(--primary-6));
}

.parameter-section-header {
  margin-top: 16px;
  margin-bottom: 8px;
}

.execution-results {
  padding: 16px;
}

.result-status {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.result-message {
  margin-bottom: 16px;
}

.result-message pre,
.result-raw pre,
.result-data pre {
  background-color: var(--color-fill-2);
  padding: 12px;
  border-radius: 4px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 300px;
  overflow: auto;
}

.execution-footer {
  margin-top: 24px;
  display: flex;
  justify-content: flex-end;
}

.plugin-details {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.plugin-info {
  margin-bottom: 16px;
}

.info-row {
  margin-bottom: 8px;
  display: flex;
}

.info-label {
  font-weight: 500;
  width: 100px;
}

.parameters-list {
  margin-top: 16px;
}

.script-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.plugin-code-editor {
  height: 400px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
}

.add-plugin-wizard {
  padding: 8px;
}

.step-content {
  margin-top: 24px;
  margin-bottom: 16px;
}

.script-editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.script-editor-container {
  margin-bottom: 16px;
}

.validation-result {
  margin-bottom: 16px;
}

.step-actions {
  margin-top: 24px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.parameters-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.plugin-editor-container {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.script-editor-section {
  margin-bottom: 16px;
}

.script-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.code-editor-container {
  height: 600px;
  width: 100%;
  font-size: 12px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
}

.validation-result {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
}

.validation-icon {
  font-size: 16px;
}

.validation-message {
  flex: 1;
}

.plugin-info-section {
  margin-bottom: 16px;
}

.parameters-section {
  margin-top: 16px;
}
</style> 