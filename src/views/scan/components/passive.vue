<template>
  <div class="container">
    <!-- 使用左侧边栏布局，给结果表格更多空间 -->
    <a-row :gutter="16" class="full-height">
      <!-- 左侧边栏 - 表单和状态信息 -->
      <a-col :span="7" class="sidebar">
        <!-- 表单卡片 -->
        <a-card class="general-card" :title="$t('scan.passive_scan')" :bordered="false" size="small">
          <a-form :model="formData" layout="vertical" size="small">
            <a-row :gutter="16">
              <a-col :span="24">
                <a-form-item :label="$t('scan.proxy_port')">
                  <a-input-number
                    style="width: 100%"
                    v-model="formData.port"
                    :min="1"
                    :max="65535"
                    :placeholder="$t('scan.enter_port')"
                  />
                </a-form-item>
              </a-col>
              
              <a-col :span="12">
                <a-form-item :label="$t('scan.intercept_tls')">
                  <a-switch v-model="formData.intercept_tls">
                    <template #checked>{{ $t('scan.enabled') }}</template>
                    <template #unchecked>{{ $t('scan.disabled') }}</template>
                  </a-switch>
                  <a-tooltip v-if="formData.intercept_tls" position="right">
                    <template #content>
                      {{ $t('scan.tls_intercept_tooltip') }}
                    </template>
                    <icon-info-circle style="margin-left: 8px; color: var(--color-text-3);" />
                  </a-tooltip>
                </a-form-item>
              </a-col>
              
              <a-col :span="12">
                <a-form-item :label="$t('scan.save_results')">
                  <a-switch v-model="formData.save_results" />
                </a-form-item>
              </a-col>
              
              <a-col :span="12">
                <a-form-item :label="$t('scan.use_plugins')">
                  <a-switch v-model="formData.use_plugins" />
                </a-form-item>
              </a-col>
              
              <a-col :span="24">
                <a-form-item :label="$t('scan.results_path')">
                  <a-input
                    v-model="formData.results_path"
                    :placeholder="$t('scan.enter_results_path')"
                    allow-clear
                  />
                </a-form-item>
              </a-col>
              
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
            <a-descriptions-item :label="$t('scan.proxy_address')">
              {{ scannerStatus.proxy_address }}:{{ scannerStatus.proxy_port }}
            </a-descriptions-item>
            <a-descriptions-item :label="$t('scan.scan_count')">
              {{ scannerStatus.scan_count }}
            </a-descriptions-item>
            <a-descriptions-item :label="$t('scan.vulnerability_count')">
              {{ scannerStatus.vulnerability_count }}
            </a-descriptions-item>
            <a-descriptions-item :label="$t('scan.tls_intercept_status')">
              <a-tag :color="formData.intercept_tls ? 'green' : 'gray'">
                {{ formData.intercept_tls ? $t('scan.enabled') : $t('scan.disabled') }}
              </a-tag>
            </a-descriptions-item>
            <a-descriptions-item v-if="scannerStatus.message" :label="$t('scan.status_message')">
              {{ scannerStatus.message }}
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
        
        <!-- 添加结果操作栏 -->
        <div class="results-action-bar mb-8">
          <div class="left-actions">
            <a-button 
              status="warning" 
              @click="clearVulnerabilities" 
              size="small"
              :disabled="vulnerabilities.length === 0"
            >
              <template #icon><icon-delete /></template>
              {{ $t('scan.clear_results') }}
            </a-button>
          </div>
          <div class="right-actions">
            <a-space>
              <a-switch v-model="showAllResults" size="small">
                {{ showAllResults ? $t('scan.show_all') : $t('scan.show_latest') }}
              </a-switch>
              <a-button type="text" @click="refreshVulnerabilities" size="small">
                <template #icon><icon-refresh /></template>
              </a-button>
            </a-space>
          </div>
        </div>
        
        <a-card class="general-card full-height" size="small">
          <template #title>
            {{ $t('scan.scan_results') }}
            <a-tag v-if="isScanning" status="processing">{{ $t('scan.scanning') }}</a-tag>
          </template>
          <template #extra>
            <a-space>
              <a-badge :count="vulnerabilities.length" :dot="false" />
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
              :pagination="false"
              row-key="id"
              :loading="tableLoading"
              size="small"
              :bordered="false"
            >
              <template #severity="{ record }">
                <a-tag :color="getRiskLevelColor(record.risk_level)" size="small">
                  {{ record.risk_level }}
                </a-tag>
              </template>
              <template #parameter="{ record }">
                <a-link @click="viewDetails(record)">{{ record.parameter }}</a-link>
              </template>
              <template #operations="{ record }">
                <a-button type="text" size="small" @click="viewDetails(record)">
                  <template #icon><icon-eye /></template>
                </a-button>
              </template>
              <template #url="{ record }">
                <a-tooltip>
                  <template #content>
                    <div style="max-width: 500px; word-break: break-all;">{{ record.url }}</div>
                  </template>
                  <div class="url-cell">{{ record.url }}</div>
                </a-tooltip>
              </template>
            </a-table>
            
            <!-- 自定义分页组件 -->
            <div class="pagination-wrapper">
              <div class="pagination-info">
                {{ paginationSummary }}
              </div>
              <a-pagination
                v-model:current="pagination.current"
                v-model:page-size="pagination.pageSize"
                :total="vulnerabilities.length"
                :page-size-options="pagination.pageSizeOptions"
                show-total
                show-jumper
                show-page-size
                size="small"
                @change="handlePageChange"
                @page-size-change="handlePageSizeChange"
              >
                <template #page-item="{ page }">
                  {{ page }}
                </template>
              </a-pagination>
            </div>
          </div>
        </a-card>
      </a-col>
    </a-row>

    <!-- Vulnerability detail modal -->
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
          <a-descriptions-item :label="$t('scan.type')">
            {{ currentVulnerability.vulnerability_type }}
          </a-descriptions-item>
          <a-descriptions-item :label="$t('scan.parameter')" v-if="currentVulnerability.parameter">
            {{ currentVulnerability.parameter }}
          </a-descriptions-item>
          <a-descriptions-item :label="$t('scan.value')" v-if="currentVulnerability.value">
            <code style="background-color: var(--color-bg-2); padding: 4px 8px; border-radius: 4px; word-break: break-all;">
              {{ currentVulnerability.value }}
            </code>
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

        <!-- 显示漏洞证据 -->
        <template v-if="currentVulnerability.evidence">
          <a-typography-title :heading="6" style="margin-top: 16px;">
            {{ $t('scan.evidence') }}
          </a-typography-title>
          <a-card size="small">
            <a-typography-paragraph>
              <pre style="white-space: pre-wrap; word-wrap: break-word;">{{ currentVulnerability.evidence }}</pre>
            </a-typography-paragraph>
          </a-card>
        </template>

        <!-- 显示详细技术信息 -->
        <template v-if="currentVulnerability.details">
          <a-typography-title :heading="6" style="margin-top: 16px;">
            {{ $t('scan.technical_details') }}
          </a-typography-title>
          
          <!-- 注意说明 -->
          <a-alert v-if="isObject(currentVulnerability.details) && currentVulnerability.details.note" 
                   type="info" 
                   :content="currentVulnerability.details.note"
                   class="mb-8" 
                   show-icon > {{ currentVulnerability.details.note }}</a-alert>
          
          <!-- 将请求和响应放在tabs中 -->
          <template v-if="isObject(currentVulnerability.details) && 
                         (currentVulnerability.details.request || currentVulnerability.details.response)">
            <a-typography-title :heading="6" style="margin-top: 16px;">
              {{ $t('scan.http_details') }}
            </a-typography-title>
            
            <a-tabs type="card" size="small" class="detail-tabs">
              <!-- 请求标签页 -->
              <a-tab-pane :key="1">
                <template #title>
                  <div class="tab-title">
                    <icon-import />
                    {{ $t('scan.request_details') }}
                  </div>
                </template>
                <a-card size="small" class="code-card">
                  <pre class="http-code request-code" v-html="highlightValue(formatHttpMessage(currentVulnerability.details.request || ''), currentVulnerability.value)"></pre>
                </a-card>
              </a-tab-pane>
              
              <!-- 响应标签页 -->
              <a-tab-pane :key="2">
                <template #title>
                  <div class="tab-title">
                    <icon-export />
                    {{ $t('scan.response_details') }}
                  </div>
                </template>
                <a-card size="small" class="code-card">
                  <pre class="http-code response-code" v-html="highlightValue(formatHttpMessage(currentVulnerability.details.response || ''), currentVulnerability.value)"></pre>
                </a-card>
              </a-tab-pane>
            </a-tabs>
          </template>
          
          <!-- 其他详情 -->
          <template v-if="!isDetailFormatted(currentVulnerability.details)">
            <a-card size="small">
              <a-typography-paragraph>
                <pre style="white-space: pre-wrap; word-wrap: break-word;">{{ 
                  typeof currentVulnerability.details === 'string' 
                    ? currentVulnerability.details 
                    : JSON.stringify(currentVulnerability.details, null, 2) 
                }}</pre>
              </a-typography-paragraph>
            </a-card>
          </template>
        </template>
      </template>
    </a-modal>
  </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted, onUnmounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Message } from '@arco-design/web-vue';
import {
  IconPlayCircle,
  IconPause, 
  IconRefresh, 
  IconDownload, 
  IconEye, 
  IconExclamationCircleFill, 
  IconInfoCircleFill, 
  IconCheckCircleFill, 
  IconBulb,
  IconDelete,
  IconImport,
  IconExport
} from '@arco-design/web-vue/es/icon';
import scannerService, { Vulnerability, ScannerStatus } from '@/api/scanner';
import { listen } from '@tauri-apps/api/event';

const { t } = useI18n();

// 扩展ScannerConfig接口
interface ScanConfig {
  port: number;
  intercept_tls: boolean;
  save_results: boolean;
  results_path: string;
  use_plugins: boolean;
}

// Form data
const formData = reactive<ScanConfig>({
  port: 8081,
  intercept_tls: true,
  save_results: false,
  results_path: '',
  use_plugins: false
});

// Scanner status
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
  message: undefined,
});

// Vulnerability data
const vulnerabilities = ref<Vulnerability[]>([]);
const statistics = reactive({
  high: 0,
  medium: 0,
  low: 0,
  info: 0
});

// Detail view
const detailsVisible = ref(false);
const currentVulnerability = ref<Vulnerability | null>(null);

// Table columns
const columns = [
  {
    title: t('scan.id'),
    dataIndex: 'id',
    width: 100,
  },
  {
    title: t('scan.type'),
    dataIndex: 'vulnerability_type',
    width: 100,

  },
  {
    title: t('scan.severity'),
    slotName: 'severity',

  },
  {
    title: t('scan.parameter'),
    slotName: 'parameter',
    dataIndex: 'parameter',

  },
  {
    title: t('scan.time'),
    dataIndex: 'timestamp',
    width: 200,
  },
  {
    title: '',
    slotName: 'operations',
    width: 50,
  },
];

// Pagination
const pagination = reactive({
  current: 1,
  pageSize: 10,
  total: 0,
  size: 'small' as const,
  showTotal: true,
  showPageSize: true,
  pageSizeOptions: [10, 20, 50, 100],
  showJumper: true,
});

// Table loading
const tableLoading = ref(false);

// Polling intervals
let statusPollInterval: number | null = null;
let vulnerabilitiesPollInterval: number | null = null;

// Fetch scanner status
const fetchScannerStatus = async () => {
  try {
    const status = await scannerService.getStatus();
    scannerStatus.value = status;
    isScanning.value = status.running;
  } catch (error) {
    console.error(t('scan.failed_fetch_scanner_status'), error);
  }
};

// Fetch vulnerabilities
const fetchVulnerabilities = async () => {
  try {
    loadingResults.value = true;
    tableLoading.value = true;
    
    // 调用后端API获取所有漏洞
    const results = await scannerService.getVulnerabilities();
    
    // 调试输出
    if (results.length > 0) {
      console.log('获取到漏洞数量:', results.length);
      console.log('第一个漏洞示例:', JSON.stringify(results[0], null, 2));
    }
    
    // 更新数据
    vulnerabilities.value = results;
    pagination.total = results.length;
    
    // 如果当前页超出范围则重置为第一页
    if (results.length > 0 && pagination.current > Math.ceil(results.length / pagination.pageSize)) {
      pagination.current = 1;
    }
    
    updateStatistics();
  } catch (error) {
    console.error(t('scan.failed_fetch_vulnerabilities'), error);
    Message.error(t('scan.failed_fetch_vulnerabilities'));
  } finally {
    loadingResults.value = false;
    tableLoading.value = false;
  }
};

// Update statistics
const updateStatistics = () => {
  statistics.high = vulnerabilities.value.filter(v => v.risk_level === 'High' || v.risk_level === 'Critical').length;
  statistics.medium = vulnerabilities.value.filter(v => v.risk_level === 'Medium').length;
  statistics.low = vulnerabilities.value.filter(v => v.risk_level === 'Low').length;
  statistics.info = vulnerabilities.value.filter(v => v.risk_level === 'Info').length;
};

// Start scanning
const startScan = async () => {
  try {
    // 转换为API期望的类型，确保兼容性
    const scanConfig = {
      port: formData.port,  // 添加port字段
      target_url: `http://localhost:${formData.port}`,
      depth: 2,
      max_pages: 100,
      intercept_tls: formData.intercept_tls,  // 添加其他配置
      save_results: formData.save_results,
      results_path: formData.results_path,
      use_plugins: formData.use_plugins
    };
    
    const success = await scannerService.startPassiveScanner(scanConfig);
    if (success) {
      Message.success(t('scan.scan_started'));
      await refreshStatus();
      startPolling(); // 启动轮询
    } else {
      Message.error(t('scan.scan_start_failed'));
    }
  } catch (error) {
    console.error(t('scan.failed_start_scan'), error);
    let errorMsg = t('scan.scan_start_failed');
    
    // 检查是否为地址被占用错误
    if (typeof error === 'string') {
      if (error.includes('Address already in use')) {
        errorMsg = t('scan.error_port_in_use', { port: formData.port });
      } else if (error.includes('扫描已在运行中')) {
        errorMsg = t('scan.error_scan_running');
      }
    }
    
    Message.error(errorMsg);
    // 确保扫描状态正确
    isScanning.value = false;
    await refreshStatus();
  }
};

// Stop scanning
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
    console.error(t('scan.failed_stop_scan'), error);
    Message.error(t('scan.scan_stop_failed'));
  } finally {
    isStoppingScanner.value = false;
  }
};



// Export results
const exportResults = async () => {
  try {
    const success = await scannerService.exportVulnerabilities('json', formData.results_path);
    if (success) {
      Message.success(t('scan.results_exported'));
    } else {
      Message.error(t('scan.export_failed'));
    }
  } catch (error) {
    console.error(t('scan.failed_export_results'), error);
    Message.error(t('scan.export_failed'));
  }
};

// Refresh vulnerabilities
const refreshVulnerabilities = async () => {
  tableLoading.value = true;
  try {
    vulnerabilities.value = await scannerService.getVulnerabilities();
    pagination.total = vulnerabilities.value.length;
    
    // 如果当前页超出范围则重置为第一页
    if (vulnerabilities.value.length > 0 && 
        pagination.current > Math.ceil(vulnerabilities.value.length / pagination.pageSize)) {
      pagination.current = 1;
    }
    
    updateStatistics();
  } catch (error) {
    console.error(t('scan.failed_refresh_vulnerabilities'), error);
  } finally {
    tableLoading.value = false;
  }
};

// View vulnerability details
const viewDetails = (vulnerability: Vulnerability) => {
  // 输出调试信息
  console.log('漏洞详情:', JSON.stringify(vulnerability, null, 2));
  
  currentVulnerability.value = vulnerability;
  detailsVisible.value = true;
};

// Get risk level color
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

// Start polling
const startPolling = () => {
  if (!statusPollInterval) {
    statusPollInterval = window.setInterval(fetchScannerStatus, 2000);
  }
  
  if (!vulnerabilitiesPollInterval) {
    vulnerabilitiesPollInterval = window.setInterval(fetchVulnerabilities, 3000);
  }
  
  console.log('开始轮询扫描状态和漏洞...');
};

// Stop polling
const stopPolling = () => {
  if (statusPollInterval) {
    clearInterval(statusPollInterval);
    statusPollInterval = null;
  }
  
  if (vulnerabilitiesPollInterval) {
    clearInterval(vulnerabilitiesPollInterval);
    vulnerabilitiesPollInterval = null;
  }
  
  console.log('停止轮询...');
};

// Lifecycle hooks
onMounted(async () => {
  fetchScannerStatus();
  fetchVulnerabilities();
  
  // 启动事件监听器
  const unlistenScanError = await listen('scan_error', (event) => {
    console.error(t('scan.scan_error'), event.payload);
    const payload = event.payload as any;
    if (payload.error) {
      // 检查端口占用错误
      if (payload.error.includes('无法绑定地址') && payload.error.includes('Address already in use')) {
        Message.error(t('scan.error_port_in_use', { port: formData.port }));
      } else {
        Message.error(payload.error);
      }
      
      // 重置扫描状态
      isScanning.value = false;
      refreshStatus();
    }
  });
  
  const unlistenScanCompleted = await listen('scan_completed', () => {
    Message.success(t('scan.scan_completed'));
    refreshStatus();
    fetchVulnerabilities();
  });
  
  // 添加对发现漏洞事件的监听
  const unlistenVulnerabilityFound = await listen('vulnerability_found', (event) => {
    console.log('收到漏洞事件:', event.payload);
    const payload = event.payload as any;
    
    // 更新漏洞计数，无需完全刷新漏洞列表
    if (payload.count !== undefined) {
      if (vulnerabilities.value.length < payload.count) {
        // 仅当后端漏洞数量大于前端时才刷新列表
        fetchVulnerabilities();
      }
    }
    
    // 如果有新漏洞，可以显示通知
    if (payload.latest) {
      // Message.info(`发现${payload.latest.risk_level}级漏洞: ${payload.latest.name}`);
    }
  });
  
  // 启动轮询如果扫描器已经在运行
  if (scannerStatus.value.running) {
    startPolling();
  }
  
  // 在组件卸载时清理事件监听器
  onUnmounted(() => {
    stopPolling();
    unlistenScanError();
    unlistenScanCompleted();
    unlistenVulnerabilityFound();
  });
});

onUnmounted(() => {
  stopPolling();
});

// Computed property: paginated vulnerabilities
const paginatedVulnerabilities = computed(() => {
  if (vulnerabilities.value.length === 0) {
    return [];
  }
  
  const start = (pagination.current - 1) * pagination.pageSize;
  const end = Math.min(start + pagination.pageSize, vulnerabilities.value.length);
  const result = vulnerabilities.value.slice(start, end);
  
  // 更新分页信息
  pagination.total = vulnerabilities.value.length;
  
  return result;
});

// 计算分页摘要信息
const paginationSummary = computed(() => {
  if (vulnerabilities.value.length === 0) {
    return t('scan.showing_results', { start: 0, end: 0, total: 0 });
  }
  
  const start = (pagination.current - 1) * pagination.pageSize + 1;
  const end = Math.min(pagination.current * pagination.pageSize, vulnerabilities.value.length);
  return t('scan.showing_results', { start, end, total: vulnerabilities.value.length });
});

// Refresh status
const refreshStatus = async () => {
  try {
    scannerStatus.value = await scannerService.getStatus();
    isScanning.value = scannerStatus.value.running;
  } catch (error) {
    console.error(t('scan.failed_refresh_status'), error);
  }
};

// Clear vulnerabilities
const clearVulnerabilities = async () => {
  try {
    const success = await scannerService.clearVulnerabilities();
    if (success) {
      vulnerabilities.value = [];
      pagination.total = 0;
      updateStatistics();
      Message.success(t('scan.results_cleared'));
    } else {
      Message.error(t('scan.clear_failed'));
    }
  } catch (error) {
    console.error(t('scan.failed_clear_vulnerabilities'), error);
    Message.error(t('scan.clear_failed'));
  }
};

// 分页事件处理
const handlePageChange = (current: number) => {
  pagination.current = current;
};

const handlePageSizeChange = (pageSize: number) => {
  pagination.pageSize = pageSize;
  pagination.current = 1; // 切换每页显示数量时重置为第一页
};

// 检查是否为对象
const isObject = (value: any): boolean => {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
};

// 检查details是否已格式化显示
const isDetailFormatted = (details: any): boolean => {
  if (!isObject(details)) return false;
  return details.request !== undefined || details.response !== undefined || details.note !== undefined;
};

// 格式化HTTP消息
const formatHttpMessage = (message: string): string => {
  if (!message) return '';
  
  // 分割行并格式化
  return message.split('\n')
    .map(line => line.trim())
    .join('\n');
};

// 高亮与value相同的文本
const highlightValue = (text: string, value: string | undefined): string => {
  if (!text || !value) return text;
  
  // 对值进行HTML转义以防XSS
  const escapeHtml = (str: string): string => {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  };
  
  // 将输入文本HTML转义
  let escapedText = escapeHtml(text);
  
  // 如果value为空，直接返回转义后的文本
  if (!value.trim()) return escapedText;
  
  // 转义后的值
  const escapedValue = escapeHtml(value);
  
  // 使用正则表达式匹配并替换所有出现的value
  // 注意：需要对正则特殊字符进行转义
  const escapeRegExp = (str: string): string => {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  };
  
  const regex = new RegExp(escapeRegExp(escapedValue), 'g');
  return escapedText.replace(regex, '<span class="highlight-value">$&</span>');
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
  /* flex-direction: column; */
}

.stat-card {
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0, 0, 0.2, 1);
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

.full-height {
  height: 100%;
}

.url-cell {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 250px;
}

.tag-with-tooltip {
  cursor: pointer;
}

.vulnerability-card {
  margin-bottom: 16px;
}

.response-details-container {
  margin-top: 16px;
  max-height: 400px;
  overflow-y: auto;
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  background-color: var(--color-bg-2);
}

/* 添加分页样式 */
.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.pagination-info {
  margin-right: 16px;
  font-size: 12px;
  color: var(--color-text-3);
  display: flex;
  align-items: center;
}

/* 添加HTTP代码展示样式 */
.code-card {
  background-color: var(--color-bg-1);
  border: 1px solid var(--color-border-2);
  border-radius: 0 0 4px 4px;
}

.http-code {
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.5;
  padding: 8px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 300px;
  overflow-y: auto;
}

.request-code {
  color: var(--color-text-1);
}

.response-code {
  color: var(--color-text-1);
}

.detail-tabs :deep(.arco-tabs-content) {
  padding: 0;
}

.detail-tabs :deep(.arco-tabs-header) {
  margin-bottom: 0;
}

.tab-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}

/* 高亮样式 */
:deep(.highlight-value) {
  background-color: #ffff00;
  color: #000000;
  font-weight: bold;
  padding: 2px 0;
  border-radius: 2px;
}

/* 暗色模式下的高亮样式 */
html.dark :deep(.highlight-value) {
  background-color: #ffd700;
  color: #000000;
}

/* 添加结果操作栏样式 */
.results-action-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 8px;
  background-color: var(--color-bg-1);
  border-radius: 4px;
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.05);
}

.results-action-bar .left-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.results-action-bar .right-actions {
  display: flex;
  align-items: center;
}
</style>