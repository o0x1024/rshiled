<template>
  <div class="container">
    <a-row :gutter="16">
      <a-col :span="24">
        <a-card class="general-card">
          <template #title>
            {{ $t('asm.dashboard.overview') }}
          </template>
          <a-space direction="vertical" fill size="large">
            <a-row :gutter="16">
              <a-col :span="6">
                <a-statistic 
                  :title="$t('asm.dashboard.total_domains')" 
                  :value="assetStats.total_domains" 
                  :loading="loading"
                  :animation="true">
                  <template #suffix>
                    <icon-apps size="22" />
                  </template>
                </a-statistic>
              </a-col>
              <a-col :span="6">
                <a-statistic 
                  :title="$t('asm.dashboard.total_ips')" 
                  :value="assetStats.total_ips" 
                  :loading="loading"
                  :animation="true">
                  <template #suffix>
                    <icon-computer size="22" />
                  </template>
                </a-statistic>
              </a-col>
              <a-col :span="6">
                <a-statistic 
                  :title="$t('asm.dashboard.total_ports')" 
                  :value="assetStats.total_ports" 
                  :loading="loading"
                  :animation="true">
                  <template #suffix>
                    <icon-select-all size="22" />
                  </template>
                </a-statistic>
              </a-col>
              <a-col :span="6">
                <a-statistic 
                  :title="$t('asm.dashboard.total_websites')" 
                  :value="assetStats.total_websites" 
                  :loading="loading"
                  :animation="true">
                  <template #suffix>
                    <icon-bug size="22" />
                  </template>
                </a-statistic>
              </a-col>
            </a-row>
          </a-space>
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="16" style="margin-top: 16px;">
      <a-col :span="16">
        <a-card class="general-card">
          <template #title>
            {{ $t('asm.dashboard.risk_distribution') }}
          </template>
          <div ref="chartRef" style="height: 300px;"></div>
        </a-card>
      </a-col>
      <a-col :span="8">
        <a-card class="general-card">
          <template #title>
            {{ $t('asm.dashboard.quick_actions') }}
          </template>
          <a-space direction="vertical" fill>
            <a-button type="primary" long @click="handleScan" :loading="scanLoading">
              {{ $t('asm.dashboard.start_scan') }}
            </a-button>
            <a-button long @click="refreshData">
              {{ $t('asm.dashboard.refresh_data') }}
            </a-button>
            <a-button long>
              {{ $t('asm.dashboard.export_report') }}
            </a-button>
          </a-space>
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="16" style="margin-top: 16px;">
      <a-col :span="24">
        <a-card class="general-card">
          <template #title>
            {{ $t('asm.dashboard.recent_findings') }}
          </template>
          <a-table :columns="columns" :data="vulnerabilities" :loading="loading" :pagination="false" stripe />
        </a-card>
      </a-col>
    </a-row>
  </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from 'vue';
import { Message } from '@arco-design/web-vue';
import * as echarts from 'echarts';
import scannerService, { AssetStatistics, Vulnerability } from '@/api/scanner';
// import useUserStore from '@/store/modules/user';


defineOptions({
	name: 'asm-dashboard',
})
// const userStore = useUserStore();
// const taskId = computed(() => userStore.role === 'admin' ? 1 : userStore.id);
const taskId = 1; // 使用固定的企业ID

const loading = ref(false);
const scanLoading = ref(false);
const assetStats = reactive<AssetStatistics>({
  total_domains: 0,
  total_ips: 0,
  total_ports: 0,
  total_websites: 0,
  total_vulnerabilities: 0,
  risk_distribution: {
    critical: 0,
    high: 0,
    medium: 0,
    low: 0,
    info: 0
  }
});

const vulnerabilities = ref<Vulnerability[]>([]);
const chartRef = ref<HTMLElement>();
let chart: echarts.ECharts | null = null;

const columns = [
  {
    title: 'ID',
    dataIndex: 'id',
    width: 70,
  },
  {
    title: '风险类型',
    dataIndex: 'vulnerability_type',
  },
  {
    title: '名称',
    dataIndex: 'name',
  },
  {
    title: 'URL',
    dataIndex: 'url',
    ellipsis: true,
  },
  {
    title: '风险等级',
    dataIndex: 'risk_level',
    width: 100,
    slotName: 'risk_level',
  },
];

const fetchData = async () => {
  loading.value = true;
  try {
    const stats = await scannerService.getAssetStatistics(taskId);
    // 更新状态
    Object.assign(assetStats, stats);
    
    // 获取漏洞列表
    vulnerabilities.value = await scannerService.getVulnerabilities(5);
    
    // 更新图表
    updateChart();
  } catch (error) {
    console.error('获取数据失败:', error);
  } finally {
    loading.value = false;
  }
};

const updateChart = () => {
  if (!chartRef.value) return;
  
  if (!chart) {
    chart = echarts.init(chartRef.value);
  }
  
  const option = {
    tooltip: {
      trigger: 'item',
      formatter: '{a} <br/>{b}: {c} ({d}%)'
    },
    legend: {
      orient: 'vertical',
      right: 10,
      top: 'center',
      data: ['严重', '高危', '中危', '低危', '信息']
    },
    color: ['#f53f3f', '#ff7d00', '#ff9a2e', '#ffb400', '#168cff'],
    series: [
      {
        name: '风险分布',
        type: 'pie',
        radius: ['50%', '70%'],
        avoidLabelOverlap: false,
        itemStyle: {
          borderRadius: 10,
          borderColor: '#fff',
          borderWidth: 2
        },
        label: {
          show: false,
          position: 'center'
        },
        emphasis: {
          label: {
            show: true,
            fontSize: 16,
            fontWeight: 'bold'
          }
        },
        labelLine: {
          show: false
        },
        data: [
          { value: assetStats.risk_distribution.critical, name: '严重' },
          { value: assetStats.risk_distribution.high, name: '高危' },
          { value: assetStats.risk_distribution.medium, name: '中危' },
          { value: assetStats.risk_distribution.low, name: '低危' },
          { value: assetStats.risk_distribution.info, name: '信息' }
        ]
      }
    ]
  };
  
  chart.setOption(option);
};

const handleScan = async () => {
  scanLoading.value = true;
  try {
    const result = await scannerService.triggerPortScan(taskId);
    if (result) {
      Message.success('扫描已开始执行');
    } else {
      Message.error('扫描启动失败');
    }
  } catch (error) {
    console.error('扫描执行失败:', error);
    Message.error('扫描执行失败');
  } finally {
    scanLoading.value = false;
  }
};

const refreshData = () => {
  fetchData();
};

onMounted(() => {
  fetchData();
  window.addEventListener('resize', () => {
    if (chart) {
      chart.resize();
    }
  });
});
</script>

<style scoped lang="less">
.container {
  padding: 0 0px 0px 0px;
  margin-bottom: 20px;
}

.general-card {
  border-radius: 4px;
  margin-bottom: 16px;
  transition: box-shadow 0.3s ease;
  
  &:hover {
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
  }
}

:deep(.arco-statistic-content) {
  font-weight: 600;
  font-size: 24px;
  margin-top: 8px;
}

:deep(.arco-statistic-title) {
  font-size: 14px;
  color: var(--color-text-3);
}

:deep(.arco-card-header) {
  border-bottom: 1px solid var(--color-border-2);
}
</style>
