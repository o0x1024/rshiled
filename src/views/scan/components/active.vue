<template>
  <div class="container">
    <!-- 使用左侧边栏布局，给结果表格更多空间 -->
    <a-row :gutter="16" class="full-height">
      <!-- 左侧边栏 - 表单和状态信息 -->
      <a-col :span="7" class="sidebar">
        <!-- 表单卡片 -->
        <a-card class="general-card mb-16" :title="$t('scan.active_scan')" :bordered="false" size="small">
          <a-form :model="formData" layout="vertical" size="small">
            <a-row :gutter="16">
              <!-- 基本信息配置区域 -->
              <a-col :span="24">
                <a-form-item :label="$t('scan.target')">
                  <a-input 
                    v-model="singleTarget" 
                    :placeholder="$t('scan.enter_target_placeholder')" 
                    allow-clear
                  />
                </a-form-item>
              </a-col>
              
              <a-col :span="24">
                <a-form-item :label="$t('scan.target_list')">
                  <a-upload
                    :file-list="fileList"
                    :limit="1"
                    @change="handleFileChange"
                    @before-upload="() => false"
                  >
                    <template #upload-button>
                      <a-button>{{ $t('scan.upload_target_list') }}</a-button>
                    </template>
                  </a-upload>
                </a-form-item>
              </a-col>
              
              <a-col :span="24">
                <a-form-item :label="$t('scan.scan_type')">
                  <a-select
                    v-model="formData.scan_type"
                    :options="scanTypeOptions"
                    :placeholder="$t('scan.select_scan_type')"
                  />
                </a-form-item>
              </a-col>

              <!-- 详细扫描选项区域 -->
              <template v-if="formData.scan_type === 'custom' && formData.detailed_scan_options">
                <a-col :span="24">
                  <a-divider orientation="left">{{ $t('scan.custom_options_title') }}</a-divider>
                </a-col>
                
                <!-- 主机存活扫描选项 -->
                <a-col :span="12">
                  <a-form-item :label="$t('scan.host_survival_scan')">
                    <a-switch v-model="formData.detailed_scan_options.host_survival" />
                  </a-form-item>
                </a-col>
                
                <!-- 指纹识别选项 -->
                <a-col :span="12">
                  <a-form-item :label="$t('scan.fingerprint_scan')">
                    <a-switch v-model="formData.detailed_scan_options.fingerprint_scan" />
                  </a-form-item>
                </a-col>
                
                <!-- Web敏感信息扫描选项 -->
                <a-col :span="12">
                  <a-form-item :label="$t('scan.web_sensitive_info_scan')">
                    <a-switch v-model="formData.detailed_scan_options.web_sensitive_info" />
                  </a-form-item>
                </a-col>
                
                <!-- Nuclei扫描选项 -->
                <a-col :span="12">
                  <a-form-item :label="$t('scan.nuclei_scan_option')">
                    <a-switch v-model="formData.detailed_scan_options.nuclei_scan" />
                  </a-form-item>
                </a-col>

                <!-- 端口扫描选项 -->
                <template v-if="formData.detailed_scan_options.port_scan">
                  <a-col :span="24">
                    <a-form-item :label="$t('scan.port_scan_enable')">
                      <a-switch v-model="formData.detailed_scan_options.port_scan.enabled" />
                    </a-form-item>
                  </a-col>
                  <a-col :span="24" v-if="formData.detailed_scan_options.port_scan.enabled">
                    <a-form-item :label="$t('scan.port_scan_ports')">
                      <a-select 
                        v-model="portScanPreset" 
                        @change="handlePortPresetChange"
                        style="width: 100%; margin-bottom: 8px;"
                      >
                        <a-option value="top100">{{ $t('scan.port_preset_top100') }}</a-option>
                        <a-option value="top1000">{{ $t('scan.port_preset_top1000') }}</a-option>
                        <a-option value="all">{{ $t('scan.port_preset_all') }}</a-option>
                        <a-option value="custom">{{ $t('scan.port_preset_custom') }}</a-option>
                      </a-select>
                      <a-input 
                        v-model="formData.detailed_scan_options.port_scan.ports" 
                        :placeholder="$t('scan.port_scan_ports_placeholder')" 
                        :disabled="portScanPreset !== 'custom'"
                        allow-clear 
                      />
                      <small>{{ $t('scan.port_scan_ports_tip') }}</small>
                    </a-form-item>
                  </a-col>
                </template>

                <!-- 服务暴力破解选项 -->
                <a-col :span="24">
                  <a-form-item :label="$t('scan.service_bruteforce_enable')">
                    <a-switch 
                      :model-value="!!formData.detailed_scan_options?.service_bruteforce?.enabled"
                      @update:model-value="val => {
                        if (!formData.detailed_scan_options) formData.detailed_scan_options = {};
                        if (!formData.detailed_scan_options.service_bruteforce) formData.detailed_scan_options.service_bruteforce = {enabled: false, services: []};
                        formData.detailed_scan_options.service_bruteforce.enabled = !!val;
                      }"
                    />
                  </a-form-item>
                </a-col>
                
                <template v-if="formData.detailed_scan_options?.service_bruteforce?.enabled">
                  <a-col :span="24">
                    <a-form-item :label="$t('scan.service_bruteforce_services')">
                      <a-space direction="vertical" style="width: 100%">
                        <a-checkbox-group v-model="selectedBruteforceServices" @change="updateServiceBruteforceServices">
                          <a-checkbox value="ssh">SSH</a-checkbox>
                          <a-checkbox value="smb">SMB</a-checkbox>
                          <a-checkbox value="rdp">RDP</a-checkbox>
                          <a-checkbox value="ftp">FTP</a-checkbox>
                          <a-checkbox value="mysql">MySQL</a-checkbox>
                          <a-checkbox value="mssql">MSSQL</a-checkbox>
                          <a-checkbox value="redis">Redis</a-checkbox>
                          <a-checkbox value="postgresql">PostgreSQL</a-checkbox>
                          <a-checkbox value="oracle">Oracle</a-checkbox>
                          <a-checkbox value="all">{{ $t('scan.all_services') }}</a-checkbox>
                        </a-checkbox-group>
                      </a-space>
                    </a-form-item>
                  </a-col>
                  
                  <!-- 爆破字典配置 -->
                  <a-col :span="12">
                    <a-form-item :label="$t('scan.bruteforce_default_wordlist')">
                      <a-switch v-model="useDefaultWordlist" />
                    </a-form-item>
                  </a-col>
                  
                  <template v-if="!useDefaultWordlist">
                    <a-col :span="24">
                      <a-form-item :label="$t('scan.service_bruteforce_usernames')">
                        <a-textarea 
                          :model-value="formData.detailed_scan_options?.service_bruteforce?.usernames || ''"
                          @update:model-value="val => {
                            if (formData.detailed_scan_options?.service_bruteforce) {
                              formData.detailed_scan_options.service_bruteforce.usernames = val;
                            }
                          }"
                          :placeholder="$t('scan.service_bruteforce_usernames_placeholder')" 
                          allow-clear 
                          :auto-size="{ minRows: 2, maxRows: 5 }"
                        />
                        <small>{{ $t('scan.service_bruteforce_usernames_tip') }}</small>
                      </a-form-item>
                    </a-col>
                    <a-col :span="24">
                      <a-form-item :label="$t('scan.service_bruteforce_passwords')">
                        <a-textarea 
                          :model-value="formData.detailed_scan_options?.service_bruteforce?.passwords || ''"
                          @update:model-value="val => {
                            if (formData.detailed_scan_options?.service_bruteforce) {
                              formData.detailed_scan_options.service_bruteforce.passwords = val;
                            }
                          }"
                          :placeholder="$t('scan.service_bruteforce_passwords_placeholder')" 
                          allow-clear
                          :auto-size="{ minRows: 2, maxRows: 5 }"
                        />
                        <small>{{ $t('scan.service_bruteforce_passwords_tip') }}</small>
                      </a-form-item>
                    </a-col>
                  </template>
                </template>
                
                <!-- 漏洞利用选项 -->
                <a-col :span="24">
                  <a-form-item :label="$t('scan.vulnerability_exploit_enable')">
                    <a-switch 
                      :model-value="!!formData.detailed_scan_options?.vulnerability_exploit?.enabled"
                      @update:model-value="val => {
                        if (!formData.detailed_scan_options) formData.detailed_scan_options = {};
                        if (!formData.detailed_scan_options.vulnerability_exploit) formData.detailed_scan_options.vulnerability_exploit = {enabled: false, options: []};
                        formData.detailed_scan_options.vulnerability_exploit.enabled = !!val;
                      }"
                    />
                  </a-form-item>
                </a-col>
                
                <template v-if="formData.detailed_scan_options?.vulnerability_exploit?.enabled">
                  <a-col :span="24">
                    <a-form-item :label="$t('scan.exploit_options')">
                      <a-checkbox-group v-model="selectedExploitOptions">
                        <a-checkbox value="ssh_pubkey">{{ $t('scan.exploit_ssh_pubkey') }}</a-checkbox>
                        <a-checkbox value="cron_job">{{ $t('scan.exploit_cron_job') }}</a-checkbox>
                        <a-checkbox value="remote_command">{{ $t('scan.exploit_remote_command') }}</a-checkbox>
                        <a-checkbox value="ms17_010">{{ $t('scan.exploit_ms17_010') }}</a-checkbox>
                      </a-checkbox-group>
                    </a-form-item>
                  </a-col>
                </template>
              </template>
              
              <!-- 通用配置选项 -->
              <a-col :span="24">
                <a-form-item :label="$t('scan.threads')">
                  <a-input-number
                    style="width: 100%"
                    v-model="formData.threads"
                    :min="1"
                    :max="100"
                    :placeholder="$t('scan.enter_threads')"
                  />
                </a-form-item>
              </a-col>
              
              <a-col :span="24">
                <a-form-item :label="$t('scan.timeout')">
                  <a-input-number
                    style="width: 100%"
                    v-model="formData.timeout"
                    :min="1"
                    :max="300"
                    :placeholder="$t('scan.enter_timeout')"
                  />
                </a-form-item>
              </a-col>
              
              <a-col :span="12">
                <a-form-item :label="$t('scan.save_results')">
                  <a-switch v-model="formData.save_results" />
                </a-form-item>
              </a-col>
              
              <a-col :span="24" v-if="formData.save_results">
                <a-form-item :label="$t('scan.results_path')">
                  <a-input 
                    v-model="formData.results_path" 
                    :placeholder="$t('scan.enter_results_path')" 
                    allow-clear
                  />
                </a-form-item>
              </a-col>
              
              <!-- 操作按钮 -->
              <a-col :span="24">
                <a-space>
                  <a-button 
                    type="primary" 
                    @click="startScan" 
                    :loading="isScanning" 
                    v-if="!scannerStatus.running"
                  >
                    <template #icon><icon-play-circle /></template>
                    {{ $t('scan.start_scan') }}
                  </a-button>
                  <a-button 
                    type="primary" 
                    status="danger"
                    @click="stopScan" 
                    :loading="isStoppingScanner" 
                    v-else
                  >
                    <template #icon><icon-pause /></template>
                    {{ $t('scan.stop_scan') }}
                  </a-button>
                  <a-button @click="resetForm">
                    <template #icon><icon-refresh /></template>
                    {{ $t('scan.reset') }}
                  </a-button>
                  <a-button @click="exportResults">
                    <template #icon><icon-download /></template>
                    {{ $t('scan.export') }}
                  </a-button>
                </a-space>
              </a-col>
            </a-row>
          </a-form>
        </a-card>

        <!-- 扫描状态卡片 -->
        <a-card v-if="scannerStatus.running" class="general-card mb-16" size="small" :bordered="false">
          <template #title>
            {{ $t('scan.scanner_status') }}
            <a-badge status="processing" :text="$t('scan.scanning')" />
          </template>
          <a-descriptions :column="1" size="small" layout="vertical">
            <a-descriptions-item :label="$t('scan.scan_type')">
              {{ getScanTypeName(formData.scan_type) }}
            </a-descriptions-item>
            <a-descriptions-item :label="$t('scan.scan_count')">
              {{ scannerStatus.scan_count }}
            </a-descriptions-item>
            <a-descriptions-item :label="$t('scan.vulnerability_count')">
              {{ scannerStatus.vulnerability_count }}
            </a-descriptions-item>
          </a-descriptions>
        </a-card>
      </a-col>

      <!-- 右侧主内容区 - 扫描结果表格 -->
      <a-col :span="17" class="main-content">
        <div class="stat-cards">
          <a-card class="stat-card mb-8" size="small" :bordered="false">
            <div class="stat-content">
              <div class="stat-icon">
                <icon-exclamation-circle-fill style="color: #ff5252;"/>
              </div>
              <div class="stat-info">
                <div class="stat-title">{{ $t('scan.high_risk') }}</div>
                <div class="stat-value">{{ statistics.high }}</div>
              </div>
            </div>
          </a-card>
          
          <a-card class="stat-card mb-8" size="small" :bordered="false">
            <div class="stat-content">
              <div class="stat-icon">
                <icon-info-circle-fill style="color: #ffb400;"/>
              </div>
              <div class="stat-info">
                <div class="stat-title">{{ $t('scan.medium_risk') }}</div>
                <div class="stat-value">{{ statistics.medium }}</div>
              </div>
            </div>
          </a-card>
          
          <a-card class="stat-card mb-8" size="small" :bordered="false">
            <div class="stat-content">
              <div class="stat-icon">
                <icon-check-circle-fill style="color: #168cff;"/>
              </div>
              <div class="stat-info">
                <div class="stat-title">{{ $t('scan.low_risk') }}</div>
                <div class="stat-value">{{ statistics.low }}</div>
              </div>
            </div>
          </a-card>
          
          <a-card class="stat-card" size="small" :bordered="false">
            <div class="stat-content">
              <div class="stat-icon">
                <icon-bulb style="color: #86909c;"/>
              </div>
              <div class="stat-info">
                <div class="stat-title">{{ $t('scan.info') }}</div>
                <div class="stat-value">{{ statistics.info }}</div>
              </div>
            </div>
          </a-card>
        </div>
        
        <a-card class="general-card full-height" size="small">
          <template #title>
            {{ $t('scan.scan_results') }}
            <a-tag v-if="isScanning" status="processing">{{ $t('scan.scanning') }}</a-tag>
          </template>
          <template #extra>
            <a-space>
              <a-switch v-model="showAllResults" size="small">
                {{ showAllResults ? $t('scan.show_all') : $t('scan.show_latest') }}
              </a-switch>
              <a-button type="text" @click="refreshVulnerabilities" size="small">
                <template #icon><icon-refresh /></template>
              </a-button>
            </a-space>
          </template>

          <div v-if="loadingResults" class="loading-container">
            <a-spin />
          </div>
          <div v-else-if="vulnerabilities.length === 0" class="empty-container">
            <a-empty :description="$t('scan.no_results')" />
          </div>
          <div v-else class="table-container">
            <a-table
              :columns="columns"
              :data="paginatedVulnerabilities"
              :pagination="pagination"
              row-key="id"
              :loading="tableLoading"
              size="small"
              :bordered="false"
              :scroll="{y: '70vh'}"
            >
              <template #severity="{ record }">
                <a-tag :color="getRiskLevelColor(record.risk_level)" size="small">
                  {{ record.risk_level }}
                </a-tag>
              </template>
              <template #operations="{ record }">
                <a-button type="text" size="small" @click="viewDetails(record)">
                  <template #icon><icon-eye /></template>
                </a-button>
              </template>
            </a-table>
          </div>
        </a-card>
      </a-col>
    </a-row>

    <!-- 漏洞详情模态框 -->
    <a-modal
      v-model:visible="detailsVisible"
      :title="currentVulnerability?.name || $t('scan.vulnerability_details')"
      :footer="false"
      width="700px"
      size="small"
    >
      <template v-if="currentVulnerability">
        <a-descriptions :column="1" bordered size="small">
          <a-descriptions-item :label="$t('scan.severity')">
            <a-tag :color="getRiskLevelColor(currentVulnerability.risk_level)" size="small">
              {{ currentVulnerability.risk_level }}
            </a-tag>
          </a-descriptions-item>
          <a-descriptions-item :label="$t('scan.url')">
            {{ currentVulnerability.url }}
          </a-descriptions-item>
          <a-descriptions-item :label="$t('scan.description')">
            {{ currentVulnerability.description }}
          </a-descriptions-item>
          <a-descriptions-item :label="$t('scan.solution')">
            {{ currentVulnerability.solution }}
          </a-descriptions-item>
          <a-descriptions-item :label="$t('scan.found_time')">
            {{ currentVulnerability.timestamp }}
          </a-descriptions-item>
        </a-descriptions>

        <template v-if="currentVulnerability.details">
          <a-typography-title :heading="6" style="margin-top: 16px;">
            {{ $t('scan.request_details') }}
          </a-typography-title>
          <a-card size="small">
            <a-typography-paragraph>
              <pre>{{ currentVulnerability.details.request }}</pre>
            </a-typography-paragraph>
          </a-card>

          <a-typography-title :heading="6" style="margin-top: 16px;">
            {{ $t('scan.response_details') }}
          </a-typography-title>
          <a-card size="small">
            <a-typography-paragraph>
              <pre>{{ currentVulnerability.details.response }}</pre>
            </a-typography-paragraph>
          </a-card>
        </template>
      </template>
    </a-modal>
  </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted, onUnmounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Message } from '@arco-design/web-vue';
import IconPlayCircle from '@arco-design/web-vue/es/icon/icon-play-circle';
import IconPause from '@arco-design/web-vue/es/icon/icon-pause';
import IconRefresh from '@arco-design/web-vue/es/icon/icon-refresh';
import IconDownload from '@arco-design/web-vue/es/icon/icon-download';
import IconEye from '@arco-design/web-vue/es/icon/icon-eye';
import IconExclamationCircleFill from '@arco-design/web-vue/es/icon/icon-exclamation-circle-fill';
import IconInfoCircleFill from '@arco-design/web-vue/es/icon/icon-info-circle-fill';
import IconCheckCircleFill from '@arco-design/web-vue/es/icon/icon-check-circle-fill';
import IconBulb from '@arco-design/web-vue/es/icon/icon-bulb';
import scannerService, { ActiveScanConfig, Vulnerability, ScannerStatus } from '@/api/scanner';

const { t } = useI18n();

// 扫描类型选项
const scanTypeOptions = [
  { label: t('scan.full_scan'), value: 'full' },
  { label: t('scan.quick_scan'), value: 'quick' },
  { label: t('scan.custom_scan'), value: 'custom' },
  { label: t('scan.nuclei_scan'), value: 'nuclei' },
];

// 辅助变量声明
const portScanPreset = ref('top100');
const selectedVulnPlugins = ref<string[]>(['default']);
const selectedBruteforceServices = ref<string[]>([]);
const selectedExploitOptions = ref<string[]>([]);
const useDefaultWordlist = ref(true);
const showCustomVulnPlugins = ref(false);
const showCustomBruteforceServices = ref(false);

// 输入辅助变量
const vulnerabilityScanPluginsInput = ref('');
const serviceBruteforceServicesInput = ref('');

// 表单数据
const formData = reactive<ActiveScanConfig>({
  targets: [],
  scan_type: 'quick', // 默认为快速扫描
  threads: 10,
  timeout: 30,
  save_results: false,
  results_path: '',
  detailed_scan_options: { // 初始化详细选项
    host_survival: false,
    port_scan: {
      enabled: false,
      ports: 'top100', // 默认扫描top100端口
    },
    vulnerability_scan: {
      enabled: false,
      plugins: [], // 默认不使用特定插件（使用后端默认）
    },
    web_sensitive_info: false,
    service_bruteforce: {
      enabled: false,
      services: [],
      usernames: '',
      passwords: '',
    },
    fingerprint_scan: false,
    nuclei_scan: false,
    vulnerability_exploit: {
      enabled: false,
      options: []
    }
  },
});

// 单个目标
const singleTarget = ref('');

// 文件上传
const fileList = ref([]);

// 扫描器状态
const isScanning = ref(false);
const isStoppingScanner = ref(false);
const loadingResults = ref(false);
const showAllResults = ref(false);
const scannerStatus = ref<ScannerStatus>({
  running: false,
  proxy_address: '',
  proxy_port: 0,
  scan_count: 0,
  vulnerability_count: 0,
});

// 漏洞数据
const vulnerabilities = ref<Vulnerability[]>([]);
const statistics = reactive({
  high: 0,
  medium: 0,
  low: 0,
  info: 0,
});

// 分页
const pagination = reactive({
  current: 1,
  pageSize: 20,
  total: 0,
  size: 'small' as const,
  showTotal: true,
  showPageSize: true
});

// 表格加载状态
const tableLoading = ref(false);

// 详情弹窗
const detailsVisible = ref(false);
const currentVulnerability = ref<Vulnerability | null>(null);

// 表格列定义
const columns = [
  {
    title: 'ID',
    dataIndex: 'id',
    width: 60
  },
  {
    title: 'Type',
    dataIndex: 'vulnerability_type',
    width: 120
  },
  {
    title: 'URL',
    dataIndex: 'url',
    ellipsis: true,
  },
  {
    title: 'Severity',
    slotName: 'severity',
    width: 90
  },
  {
    title: 'Time',
    dataIndex: 'timestamp',
    width: 180
  },
  {
    title: '',
    slotName: 'operations',
    width: 50
  },
];

// 端口预设变更处理函数
const handlePortPresetChange = (value: string | number | boolean | Record<string, any> | (string | number | boolean | Record<string, any>)[]) => {
  const portValue = String(value);
  if (formData.detailed_scan_options?.port_scan) {
    switch (portValue) {
      case 'top100':
        formData.detailed_scan_options.port_scan.ports = 'top100';
        break;
      case 'top1000':
        formData.detailed_scan_options.port_scan.ports = 'top1000';
        break;
      case 'all':
        formData.detailed_scan_options.port_scan.ports = '1-65535';
        break;
      case 'custom':
        // 保持原值或设置合理的默认值
        if (!formData.detailed_scan_options.port_scan.ports || 
            ['top100', 'top1000', '1-65535'].includes(formData.detailed_scan_options.port_scan.ports)) {
          formData.detailed_scan_options.port_scan.ports = '80,443,22,3306,8080';
        }
        break;
    }
  }
};

// 监听selectedVulnPlugins变化，更新漏洞扫描插件和显示自定义输入框
watch(selectedVulnPlugins, (newValues) => {
  showCustomVulnPlugins.value = newValues.includes('all');
  
  if (formData.detailed_scan_options && formData.detailed_scan_options.vulnerability_scan) {
    if (newValues.includes('all')) {
      formData.detailed_scan_options.vulnerability_scan.plugins = ['all'];
    } else {
      formData.detailed_scan_options.vulnerability_scan.plugins = [...newValues];
    }
  }
});

// 监听selectedBruteforceServices变化，更新服务暴力破解选项和显示自定义输入框
watch(selectedBruteforceServices, (newValues) => {
  showCustomBruteforceServices.value = newValues.includes('all');
  
  if (formData.detailed_scan_options && formData.detailed_scan_options.service_bruteforce) {
    if (newValues.includes('all')) {
      formData.detailed_scan_options.service_bruteforce.services = ['all'];
    } else {
      formData.detailed_scan_options.service_bruteforce.services = [...newValues];
    }
  }
});

// 监听selectedExploitOptions变化，更新漏洞利用选项
watch(selectedExploitOptions, (newValues) => {
  if (formData.detailed_scan_options && formData.detailed_scan_options.vulnerability_exploit) {
    formData.detailed_scan_options.vulnerability_exploit.options = [...newValues];
  }
});

// 定时刷新
let statusPollInterval: number | null = null;
let vulnerabilitiesPollInterval: number | null = null;

onMounted(() => {
  refreshStatus();
  refreshVulnerabilities();
  
  // 如果扫描器已在运行，启动轮询
  if (scannerStatus.value.running) {
    startPolling();
  }
});

onUnmounted(() => {
  stopPolling();
});

// 获取扫描类型名称
const getScanTypeName = (value: string) => {
  const option = scanTypeOptions.find(opt => opt.value === value);
  return option ? option.label : value;
};

// 处理文件上传
const handleFileChange = (fileItem: any, fileListRaw: any) => {
  if (fileItem.file && fileItem.file.originFile) {
    const reader = new FileReader();
    reader.onload = (e) => {
      if (e.target && e.target.result) {
        const content = e.target.result as string;
        const lines = content.split('\n').map(line => line.trim()).filter(line => line !== '');
        formData.targets = lines;
        Message.success(t('scan.target_list_loaded', { count: lines.length }));
      }
    };
    reader.onerror = () => {
      Message.error(t('scan.file_read_error'));
    }
    reader.readAsText(fileItem.file.originFile);
  }
  fileList.value = fileListRaw;
};

// 开始扫描
const startScan = async () => {
  // 如果有单个目标，添加到目标列表
  if (singleTarget.value && !formData.targets.includes(singleTarget.value)) {
     formData.targets.push(singleTarget.value);
  }

  if (formData.targets.length === 0) {
    Message.error(t('scan.no_targets'));
    return;
  }

  // 深拷贝formData以避免直接修改原始响应式对象
  const configToScan: ActiveScanConfig = JSON.parse(JSON.stringify(formData));

  // 确保detailed_scan_options存在
  if (!configToScan.detailed_scan_options) {
    configToScan.detailed_scan_options = {}; // 如果未定义则初始化
  }

  // 根据scan_type配置detailed_scan_options
  if (configToScan.scan_type === 'full') {
    configToScan.detailed_scan_options = {
      host_survival: true,
      port_scan: { enabled: true, ports: '1-65535' }, 
      vulnerability_scan: { enabled: true, plugins: ['all'] }, 
      web_sensitive_info: true,
      service_bruteforce: { enabled: true, services: ['all'], usernames: '', passwords: '' }, 
      fingerprint_scan: true,
      nuclei_scan: true,
      vulnerability_exploit: { enabled: true, options: ['ssh_pubkey', 'cron_job', 'remote_command', 'ms17_010'] }
    };
  } else if (configToScan.scan_type === 'quick') {
     configToScan.detailed_scan_options = {
      host_survival: true,
      port_scan: { enabled: true, ports: 'top1000' }, 
      vulnerability_scan: { enabled: true, plugins: ['default'] }, 
      web_sensitive_info: true,
      service_bruteforce: { enabled: false, services: [], usernames: '', passwords: '' },
      fingerprint_scan: true,
      nuclei_scan: true,
      vulnerability_exploit: { enabled: false, options: [] }
    };
  } else if (configToScan.scan_type === 'nuclei') {
     configToScan.detailed_scan_options = { 
      host_survival: true,
      port_scan: { enabled: false, ports: '' },
      vulnerability_scan: { enabled: false, plugins: [] },
      web_sensitive_info: false,
      service_bruteforce: { enabled: false, services: [], usernames: '', passwords: '' },
      fingerprint_scan: false,
      nuclei_scan: true,
      vulnerability_exploit: { enabled: false, options: [] }
    };
  } // 对于'custom'，formData.detailed_scan_options已由用户通过UI设置
    // 并已通过JSON.stringify/parse深拷贝到configToScan.detailed_scan_options

  try {
    isScanning.value = true; 
    const success = await scannerService.startActiveScan(configToScan);
    if (success) {
      Message.success(t('scan.scan_started'));
      await refreshStatus(); 
      startPolling(); 
    } else {
      Message.error(t('scan.scan_start_failed'));
      isScanning.value = false; 
    }
  } catch (error) {
    console.error('Failed to start scan:', error);
    Message.error(t('scan.scan_start_failed'));
    isScanning.value = false; 
  }
};

// 停止扫描
const stopScan = async () => {
  try {
    isStoppingScanner.value = true;
    const success = await scannerService.stopScanner();
    if (success) {
      Message.success(t('scan.scan_stopped'));
      await refreshStatus();
      stopPolling(); // 停止轮询
    } else {
      Message.error(t('scan.scan_stop_failed'));
    }
  } catch (error) {
    console.error('Failed to stop scan:', error);
    Message.error(t('scan.scan_stop_failed'));
  } finally {
    isStoppingScanner.value = false;
  }
};

// 重置表单
const resetForm = () => {
  singleTarget.value = '';
  fileList.value = []; 
  formData.targets = [];
  formData.scan_type = 'quick';
  formData.threads = 10;
  formData.timeout = 30;
  formData.save_results = false;
  formData.results_path = '';
  
  formData.detailed_scan_options = {
    host_survival: false,
    port_scan: {
      enabled: false,
      ports: 'top100',
    },
    vulnerability_scan: {
      enabled: false,
      plugins: [],
    },
    web_sensitive_info: false,
    service_bruteforce: {
      enabled: false,
      services: [],
      usernames: '',
      passwords: '',
    },
    fingerprint_scan: false,
    nuclei_scan: false,
    vulnerability_exploit: {
      enabled: false,
      options: []
    }
  };
  vulnerabilityScanPluginsInput.value = ''; 
  serviceBruteforceServicesInput.value = '';
  
  // 重置辅助变量
  portScanPreset.value = 'top100';
  selectedVulnPlugins.value = ['default'];
  selectedBruteforceServices.value = [];
  selectedExploitOptions.value = [];
  useDefaultWordlist.value = true;
  showCustomVulnPlugins.value = false;
  showCustomBruteforceServices.value = false;
};

// 导出结果
const exportResults = async () => {
  try {
    const success = await scannerService.exportVulnerabilities('json', formData.results_path);
    if (success) {
      Message.success(t('scan.results_exported'));
    } else {
      Message.error(t('scan.export_failed'));
    }
  } catch (error) {
    console.error('Failed to export results:', error);
    Message.error(t('scan.export_failed'));
  }
};

// 刷新扫描器状态
const refreshStatus = async () => {
  try {
    scannerStatus.value = await scannerService.getStatus();
    isScanning.value = scannerStatus.value.running;
  } catch (error) {
    console.error('Failed to refresh status:', error);
  }
};

// 刷新漏洞列表
const refreshVulnerabilities = async () => {
  tableLoading.value = true;
  try {
    const limit = showAllResults.value ? undefined : 100;
    vulnerabilities.value = await scannerService.getVulnerabilities(limit);
    if (pagination) {
      pagination.total = vulnerabilities.value.length;
    }
    updateStatistics();
  } catch (error) {
    console.error('Failed to refresh vulnerabilities:', error);
  } finally {
    tableLoading.value = false;
  }
};

// 更新统计信息
const updateStatistics = () => {
  statistics.high = vulnerabilities.value.filter(v => v.risk_level === 'High' || v.risk_level === 'Critical').length;
  statistics.medium = vulnerabilities.value.filter(v => v.risk_level === 'Medium').length;
  statistics.low = vulnerabilities.value.filter(v => v.risk_level === 'Low').length;
  statistics.info = vulnerabilities.value.filter(v => v.risk_level === 'Info').length;
};

// 查看漏洞详情
const viewDetails = (vulnerability: Vulnerability) => {
  currentVulnerability.value = vulnerability;
  detailsVisible.value = true;
};

// 获取风险等级颜色
const getRiskLevelColor = (level: string) => {
  switch (level) {
    case 'Critical':
      return 'rgb(183, 9, 9)';
    case 'High':
      return 'rgb(245, 108, 108)';
    case 'Medium':
      return 'rgb(230, 162, 60)';
    case 'Low':
      return 'rgb(103, 194, 58)';
    default:
      return 'rgb(144, 147, 153)';
  }
};

// 开始轮询
const startPolling = () => {
  if (!statusPollInterval) {
    statusPollInterval = window.setInterval(fetchScannerStatus, 3000);
  }
  
  if (!vulnerabilitiesPollInterval) {
    vulnerabilitiesPollInterval = window.setInterval(fetchVulnerabilities, 5000);
  }
};

// 停止轮询
const stopPolling = () => {
  if (statusPollInterval) {
    clearInterval(statusPollInterval);
    statusPollInterval = null;
  }
  
  if (vulnerabilitiesPollInterval) {
    clearInterval(vulnerabilitiesPollInterval);
    vulnerabilitiesPollInterval = null;
  }
};

// 获取扫描器状态
const fetchScannerStatus = async () => {
  try {
    const status = await scannerService.getStatus();
    scannerStatus.value = status;
    isScanning.value = status.running;
  } catch (error) {
    console.error('Failed to fetch scanner status:', error);
  }
};

// 获取漏洞列表
const fetchVulnerabilities = async () => {
  try {
    loadingResults.value = true;
    const limit = showAllResults.value ? undefined : 100;
    const results = await scannerService.getVulnerabilities(limit);
    vulnerabilities.value = results;
    if (pagination) {
      pagination.total = results.length;
    }
    updateStatistics();
  } catch (error) {
    console.error('Failed to fetch vulnerabilities:', error);
  } finally {
    loadingResults.value = false;
  }
};

// 分页漏洞列表
const paginatedVulnerabilities = computed(() => {
  const start = (pagination.current - 1) * pagination.pageSize;
  const end = start + pagination.pageSize;
  return vulnerabilities.value.slice(start, end);
});

// 数组输入处理方法
const updateVulnerabilityScanPlugins = () => {
  if (formData.detailed_scan_options && formData.detailed_scan_options.vulnerability_scan) {
    formData.detailed_scan_options.vulnerability_scan.plugins = 
      vulnerabilityScanPluginsInput.value.split(',').map(p => p.trim()).filter(p => p);
  }
};

const updateServiceBruteforceServices = () => {
  if (formData.detailed_scan_options && formData.detailed_scan_options.service_bruteforce) {
    formData.detailed_scan_options.service_bruteforce.services = 
      serviceBruteforceServicesInput.value.split(',').map(s => s.trim()).filter(s => s);
  }
};
</script>

<style scoped>
.container {
  height: 100%;
  width: 100%;
}

.full-height {
  height: 100%;
}

.sidebar {
  height: 100%;
  overflow-y: auto;
  padding-right: 8px;
}

.main-content {
  height: 100%;
  min-height: calc(100vh - 136px); /* Adjust based on your layout */
}

.general-card {
  border-radius: 4px;
  transition: box-shadow 0.3s cubic-bezier(0, 0, 0.2, 1);
}

.general-card:hover {
  box-shadow: 0 2px 8px 0 rgba(0, 0, 0, 0.09);
}

.mb-8 {
  margin-bottom: 8px;
}

.mb-16 {
  margin-bottom: 16px;
}

.stat-cards {
  display: flex;
}

.stat-card {
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0, 0, 0.2, 1);
  flex: 1;
  margin-right: 8px;
  margin-bottom: 16px;
}

.stat-card:last-child {
  margin-right: 0;
}

.stat-card:hover {
  transform: translateX(2px);
  box-shadow: 0 2px 8px 0 rgba(0, 0, 0, 0.09);
}

.stat-content {
  display: flex;
  align-items: center;
}

.stat-icon {
  font-size: 24px;
  margin-right: 8px;
}

.stat-info {
  display: flex;
  flex-direction: column;
}

.stat-title {
  font-size: 12px;
  color: var(--color-text-3);
}

.stat-value {
  font-size: 20px;
  font-weight: bold;
  color: var(--color-text-1);
}

.loading-container, .empty-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 200px;
}

.table-container {
  height: calc(100vh - 160px);
  min-height: 400px;
}
</style>
