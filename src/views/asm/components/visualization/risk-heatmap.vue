<template>
  <div class="container">
    <a-card class="general-card">
      <template #title>
        {{ $t('asm.visualization.risk_heatmap') }}
        <a-tooltip content="风险热图可直观展示安全风险分布情况，帮助聚焦高危区域">
          <icon-question-circle style="margin-left: 8px;" />
        </a-tooltip>
      </template>
      
      <div class="toolbar">
        <a-space>
          <a-select v-model="taskId" placeholder="选择扫描任务" style="width: 200px;">
            <a-option v-for="task in taskList" :key="task.id" :value="task.id">{{ task.name }}</a-option>
          </a-select>
          
          <a-select v-model="viewType" style="width: 150px;">
            <a-option value="asset">按资产分布</a-option>
            <a-option value="risk">按风险类型</a-option>
            <a-option value="time">按时间趋势</a-option>
          </a-select>
          
          <a-button type="primary" @click="fetchData">
            {{ $t('asm.visualization.apply_filter') }}
          </a-button>
          
          <a-button @click="resetChart">
            {{ $t('asm.visualization.reset') }}
          </a-button>

          <a-button type="outline" @click="exportChart">
            {{ $t('asm.visualization.export') }}
          </a-button>
        </a-space>
      </div>
      
      <a-spin :loading="loading" style="width: 100%;">
        <div ref="chartRef" class="chart-container"></div>
      </a-spin>
      
      <div class="analysis-panel">
        <a-divider>{{ $t('asm.visualization.risk_analysis') }}</a-divider>
        
        <a-row :gutter="16">
          <a-col :span="8">
            <a-card class="analysis-card">
              <template #title>
                <div class="card-title danger">
                  <icon-exclamation-circle-fill />
                  高危风险
                </div>
              </template>
              <div class="risk-count">{{ highRiskCount }}</div>
              <div class="risk-description">
                <div v-if="highRiskCount > 0">主要分布在 {{ topHighRiskAsset }} 上</div>
                <div v-else>暂无高危风险</div>
              </div>
            </a-card>
          </a-col>
          
          <a-col :span="8">
            <a-card class="analysis-card">
              <template #title>
                <div class="card-title warning">
                  <icon-warning-circle-fill />
                  中危风险
                </div>
              </template>
              <div class="risk-count">{{ mediumRiskCount }}</div>
              <div class="risk-description">
                <div v-if="mediumRiskCount > 0">主要分布在 {{ topMediumRiskAsset }} 上</div>
                <div v-else>暂无中危风险</div>
              </div>
            </a-card>
          </a-col>
          
          <a-col :span="8">
            <a-card class="analysis-card">
              <template #title>
                <div class="card-title info">
                  <icon-info-circle-fill />
                  低危风险
                </div>
              </template>
              <div class="risk-count">{{ lowRiskCount }}</div>
              <div class="risk-description">
                <div v-if="lowRiskCount > 0">主要分布在 {{ topLowRiskAsset }} 上</div>
                <div v-else>暂无低危风险</div>
              </div>
            </a-card>
          </a-col>
        </a-row>
        
        <a-divider>{{ $t('asm.visualization.risk_recommendation') }}</a-divider>
        
        <div class="recommendation-area">
          <template v-if="riskAnalysis">
            <a-typography-paragraph>
              <a-typography-text bold>主要风险:</a-typography-text> {{ riskAnalysis.main_risk }}
            </a-typography-paragraph>
            <a-typography-paragraph>
              <a-typography-text bold>薄弱环节:</a-typography-text> {{ riskAnalysis.weak_point }}
            </a-typography-paragraph>
            <a-typography-paragraph>
              <a-typography-text bold>建议:</a-typography-text> {{ riskAnalysis.recommendation }}
            </a-typography-paragraph>
          </template>
          <template v-else>
            <a-empty description="请先选择扫描任务并应用筛选条件" />
          </template>
        </div>
      </div>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import * as echarts from 'echarts';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';

interface RiskData {
  asset: string;
  risk_type: string;
  risk_level: string;
  count: number;
  timestamp?: number;
}

interface RiskAnalysis {
  main_risk: string;
  weak_point: string;
  recommendation: string;
}

// 状态变量
const chartRef = ref<HTMLElement | null>(null);
let chart: echarts.ECharts | null = null;
const loading = ref(false);
const taskId = ref('');
const taskList = ref<any[]>([]);
const viewType = ref('asset');
const heatmapData = ref<RiskData[]>([]);
const riskAnalysis = ref<RiskAnalysis | null>(null);

// 统计信息
const highRiskCount = ref(0);
const mediumRiskCount = ref(0);
const lowRiskCount = ref(0);
const topHighRiskAsset = ref('');
const topMediumRiskAsset = ref('');
const topLowRiskAsset = ref('');

// 初始化图表
function initChart() {
  if (!chartRef.value) return;
  
  chart = echarts.init(chartRef.value);
  
  window.addEventListener('resize', () => {
    chart?.resize();
  });
}

// 更新图表数据
function updateChart() {
  if (!chart || !heatmapData.value.length) return;
  
  let option;
  
  if (viewType.value === 'asset') {
    // 按资产分布的热图
    const assets = Array.from(new Set(heatmapData.value.map(item => item.asset)));
    const riskTypes = Array.from(new Set(heatmapData.value.map(item => item.risk_type)));
    
    const data: [string, string, number][] = heatmapData.value.map(item => [
      item.asset,
      item.risk_type,
      item.count
    ]);
    
    option = {
      tooltip: {
        position: 'top',
        formatter: (params: any) => {
          const value = params.value;
          return `资产: ${value[0]}<br>风险类型: ${value[1]}<br>数量: ${value[2]}`;
        }
      },
      grid: {
        top: '60px',
        left: '3%',
        right: '8%',
        bottom: '10%',
        containLabel: true
      },
      xAxis: {
        type: 'category',
        data: assets,
        splitArea: {
          show: true
        },
        axisLabel: {
          interval: 0,
          rotate: 30
        }
      },
      yAxis: {
        type: 'category',
        data: riskTypes,
        splitArea: {
          show: true
        }
      },
      visualMap: {
        min: 0,
        max: Math.max(...heatmapData.value.map(item => item.count)),
        calculable: true,
        orient: 'horizontal',
        left: 'center',
        top: '0',
        inRange: {
          color: ['#ebedf0', '#c6e48b', '#7bc96f', '#239a3b', '#196127']
        }
      },
      series: [{
        name: '风险热图',
        type: 'heatmap',
        data: data,
        label: {
          show: true
        },
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowColor: 'rgba(0, 0, 0, 0.5)'
          }
        }
      }]
    };
  } else if (viewType.value === 'risk') {
    // 按风险类型分布的饼图
    const riskTypeData = {} as Record<string, number>;
    
    heatmapData.value.forEach(item => {
      if (riskTypeData[item.risk_type]) {
        riskTypeData[item.risk_type] += item.count;
      } else {
        riskTypeData[item.risk_type] = item.count;
      }
    });
    
    const pieData = Object.entries(riskTypeData).map(([name, value]) => ({ name, value }));
    
    option = {
      tooltip: {
        trigger: 'item',
        formatter: '{a} <br/>{b}: {c} ({d}%)'
      },
      legend: {
        orient: 'vertical',
        right: 10,
        top: 'center',
        data: Object.keys(riskTypeData)
      },
      series: [
        {
          name: '风险类型分布',
          type: 'pie',
          radius: ['40%', '70%'],
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
          data: pieData
        }
      ]
    };
  } else if (viewType.value === 'time') {
    // 按时间趋势的折线图
    const timeData = {} as Record<string, Record<string, number>>;
    const riskTypes = Array.from(new Set(heatmapData.value.map(item => item.risk_type)));
    
    // 处理时间数据
    heatmapData.value.forEach(item => {
      if (!item.timestamp) return;
      
      const date = new Date(item.timestamp * 1000);
      const dateStr = `${date.getMonth() + 1}/${date.getDate()}`;
      
      if (!timeData[dateStr]) {
        timeData[dateStr] = {};
      }
      
      if (!timeData[dateStr][item.risk_type]) {
        timeData[dateStr][item.risk_type] = 0;
      }
      
      timeData[dateStr][item.risk_type] += item.count;
    });
    
    const dates = Object.keys(timeData).sort((a, b) => {
      const [aMonth, aDay] = a.split('/').map(Number);
      const [bMonth, bDay] = b.split('/').map(Number);
      
      if (aMonth !== bMonth) {
        return aMonth - bMonth;
      }
      
      return aDay - bDay;
    });
    
    const series = riskTypes.map(type => {
      return {
        name: type,
        type: 'line',
        stack: 'Total',
        data: dates.map(date => timeData[date][type] || 0)
      };
    });
    
    option = {
      tooltip: {
        trigger: 'axis'
      },
      legend: {
        data: riskTypes
      },
      grid: {
        left: '3%',
        right: '4%',
        bottom: '3%',
        containLabel: true
      },
      xAxis: {
        type: 'category',
        boundaryGap: false,
        data: dates
      },
      yAxis: {
        type: 'value'
      },
      series: series
    };
  }
  
  if (option) {
    chart.setOption(option);
  }
}

// 获取任务列表
async function fetchTaskList() {
  try {
    const result:any = await invoke('get_task_list', { page: 1, pagesize: 100 });
    taskList.value = result.list || [];
    if (taskList.value.length > 0) {
      taskId.value = taskList.value[0].id;
    }
  } catch (error) {
    console.error('获取任务列表失败:', error);
    Message.error('获取任务列表失败');
  }
}

// 获取风险热图数据
async function fetchData() {
  if (!taskId.value) {
    Message.warning('请先选择扫描任务');
    return;
  }
  
  loading.value = true;
  try {
    const result = await invoke<{
      risk_data: RiskData[];
      analysis: RiskAnalysis;
      stats: {
        high_risk_count: number;
        medium_risk_count: number;
        low_risk_count: number;
        top_high_risk_asset: string;
        top_medium_risk_asset: string;
        top_low_risk_asset: string;
      }
    }>('get_risk_heatmap_data', { 
      taskId: taskId.value,
      viewType: viewType.value
    });
    
    // 适配后端返回的数据格式
    heatmapData.value = result.risk_data || [];
    riskAnalysis.value = result.analysis || null;
    
    // 更新统计信息
    highRiskCount.value = result.stats.high_risk_count || 0;
    mediumRiskCount.value = result.stats.medium_risk_count || 0;
    lowRiskCount.value = result.stats.low_risk_count || 0;
    topHighRiskAsset.value = result.stats.top_high_risk_asset || '-';
    topMediumRiskAsset.value = result.stats.top_medium_risk_asset || '-';
    topLowRiskAsset.value = result.stats.top_low_risk_asset || '-';
    
    updateChart();
    
    if (heatmapData.value.length === 0) {
      Message.info('没有找到相关的风险数据');
    }
  } catch (error) {
    console.error('获取风险热图数据失败:', error);
    Message.error('获取风险热图数据失败');
  } finally {
    loading.value = false;
  }
}

// 重置图表
function resetChart() {
  if (!chart) return;
  chart.clear();
  heatmapData.value = [];
  riskAnalysis.value = null;
  highRiskCount.value = 0;
  mediumRiskCount.value = 0;
  lowRiskCount.value = 0;
  topHighRiskAsset.value = '';
  topMediumRiskAsset.value = '';
  topLowRiskAsset.value = '';
  fetchData();
}

// 导出图表
function exportChart() {
  if (!chart) return;
  
  const dataURL = chart.getDataURL({
    type: 'png',
    pixelRatio: 2,
    backgroundColor: '#fff'
  });
  
  // 创建下载链接
  const link = document.createElement('a');
  link.download = '风险热图.png';
  link.href = dataURL;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

// 监听视图类型变化，自动刷新数据
watch([viewType, taskId], () => {
  if (chart && taskId.value) {
    resetChart();
  }
});

onMounted(() => {
  initChart();
  fetchTaskList();
});
</script>

<style scoped>
.container {
  padding: 0;
}

.general-card {
  border-radius: 4px;
  margin-bottom: 16px;
  transition: box-shadow 0.3s ease;
  
  &:hover {
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
  }
}

.toolbar {
  margin-bottom: 16px;
  display: flex;
  justify-content: space-between;
}

.chart-container {
  height: 450px;
  width: 100%;
  background-color: #fdfdfd;
  border-radius: 4px;
  border: 1px solid var(--color-border-2);
}

.analysis-panel {
  margin-top: 20px;
}

.analysis-card {
  height: 100%;
  transition: transform 0.3s;
  
  &:hover {
    transform: translateY(-5px);
  }
}

.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  
  &.danger {
    color: #f53f3f;
  }
  
  &.warning {
    color: #ff7d00;
  }
  
  &.info {
    color: #168cff;
  }
}

.risk-count {
  font-size: 32px;
  font-weight: bold;
  text-align: center;
  margin: 10px 0;
}

.risk-description {
  text-align: center;
  color: var(--color-text-3);
}

.recommendation-area {
  background-color: #f8f9fa;
  padding: 16px;
  border-radius: 4px;
  margin-top: 8px;
}
</style> 