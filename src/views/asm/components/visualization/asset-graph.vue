<template>
  <div class="container">
    <a-card class="general-card">
      <template #title>
        {{ $t('asm.visualization.asset_graph') }}
        <a-tooltip content="资产图谱展示了域名、IP、端口、网站和组件之间的关联关系">
          <icon-question-circle style="margin-left: 8px;" />
        </a-tooltip>
      </template>
      
      <div class="toolbar">
        <a-space>
          <a-select v-model="taskId" placeholder="选择扫描任务" style="width: 200px;">
            <a-option v-for="task in taskList" :key="task.id" :value="task.id">{{ task.name }}</a-option>
          </a-select>
          
          <a-select v-model="graphType" style="width: 150px;">
            <a-option value="domain">以域名为中心</a-option>
            <a-option value="ip">以IP为中心</a-option>
            <a-option value="website">以网站为中心</a-option>
          </a-select>
          
          <a-button type="primary" @click="fetchData">
            {{ $t('asm.visualization.apply_filter') }}
          </a-button>
          
          <a-button @click="resetGraph">
            {{ $t('asm.visualization.reset') }}
          </a-button>

          <a-button type="outline" @click="exportGraph">
            {{ $t('asm.visualization.export') }}
          </a-button>
        </a-space>
      </div>
      
      <div class="graph-legend">
        <div class="legend-item">
          <div class="legend-icon domain"></div>
          <span>域名</span>
        </div>
        <div class="legend-item">
          <div class="legend-icon ip"></div>
          <span>IP</span>
        </div>
        <div class="legend-item">
          <div class="legend-icon port"></div>
          <span>端口</span>
        </div>
        <div class="legend-item">
          <div class="legend-icon website"></div>
          <span>网站</span>
        </div>
        <div class="legend-item">
          <div class="legend-icon component"></div>
          <span>组件</span>
        </div>
        <div class="legend-item">
          <div class="legend-icon risk"></div>
          <span>风险</span>
        </div>
      </div>
      
      <a-spin :loading="loading" style="width: 100%;">
        <div ref="chartRef" class="chart-container"></div>
      </a-spin>
      
      <div v-if="selectedNode" class="node-details">
        <a-divider>{{ selectedNode.name }} 详情</a-divider>
        <a-descriptions :data="selectedNodeDetails" layout="inline-vertical" bordered />
      </div>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import * as echarts from 'echarts';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';

interface GraphNode {
  name: string;
  category: string;
  symbolSize?: number;
  itemStyle?: { color: string };
  data?: any;
}

interface GraphData {
  nodes: GraphNode[];
  links: any[];
}

// 状态变量
const chartRef = ref(null);
let chart:any = null;
const loading = ref(false);
const taskId = ref('');
const taskList = ref<any[]>([]);
const graphType = ref('domain');
const graphData = ref<GraphData>({ nodes: [], links: [] });
const selectedNode = ref<any>(null);

// 选中节点的详细信息
const selectedNodeDetails = computed(() => {
  if (!selectedNode.value) return [];
  
  const node = selectedNode.value;
  const details = [];
  
  switch (node.category) {
    case 'domain':
      details.push({ label: '域名', value: node.name });
      if (node.data) {
        details.push({ label: 'A记录', value: node.data.aaa || '-' });
        details.push({ label: 'CNAME', value: node.data.cname || '-' });
        details.push({ label: 'NS', value: node.data.ns || '-' });
        details.push({ label: 'MX', value: node.data.mx || '-' });
        details.push({ label: '创建时间', value: formatTime(node.data.create_at) });
        details.push({ label: '更新时间', value: formatTime(node.data.update_at) });
      }
      break;
    case 'ip':
      details.push({ label: 'IP地址', value: node.name });
      if (node.data) {
        details.push({ label: '创建时间', value: formatTime(node.data.create_at) });
        details.push({ label: '更新时间', value: formatTime(node.data.update_at) });
      }
      break;
    case 'port':
      details.push({ label: '端口', value: node.name });
      if (node.data) {
        details.push({ label: 'IP地址', value: node.data.ip_addr || '-' });
        details.push({ label: '服务', value: node.data.service || '-' });
        details.push({ label: '状态', value: node.data.status === 1 ? '开放' : '关闭' });
      }
      break;
    case 'website':
      details.push({ label: 'URL', value: node.name });
      if (node.data) {
        details.push({ label: '标题', value: node.data.title || '-' });
        details.push({ label: '指纹', value: (node.data.finger || []).join(', ') || '-' });
        details.push({ label: '创建时间', value: formatTime(node.data.create_at) });
      }
      break;
    case 'component':
      details.push({ label: '组件名称', value: node.name });
      if (node.data) {
        details.push({ label: '组件类型', value: node.data.ctype || '-' });
        details.push({ label: '所属网站', value: node.data.website || '-' });
      }
      break;
    case 'risk':
      details.push({ label: '风险名称', value: node.name });
      if (node.data) {
        details.push({ label: '风险类型', value: node.data.risk_type || '-' });
        details.push({ label: '风险等级', value: node.data.risk_level || '-' });
        details.push({ label: '风险描述', value: node.data.risk_desc || '-' });
      }
      break;
  }
  
  return details;
});

// 格式化时间戳
function formatTime(timestamp:any) {
  if (!timestamp) return '-';
  return new Date(timestamp * 1000).toLocaleString();
}

// 初始化图表
function initChart() {
  if (!chartRef.value) return;
  
  chart = echarts.init(chartRef.value);
  
  chart.on('click', (params:any) => {
    if (params.dataType === 'node') {
      selectedNode.value = params.data;
    }
  });
  
  window.addEventListener('resize', () => {
    chart?.resize();
  });
}

// 更新图表数据
function updateChart() {
  if (!chart) return;
  
  const option = {
    tooltip: {
      trigger: 'item',
      formatter: (params:any) => {
        if (params.dataType === 'node') {
          return `${params.name}<br/>${params.data.category}`;
        }
        return params.name;
      }
    },
    legend: {
      show: false
    },
    animationDuration: 1500,
    animationEasingUpdate: 'quinticInOut',
    series: [
      {
        name: '资产关联图',
        type: 'graph',
        layout: 'force',
        force: {
          repulsion: 500,
          edgeLength: 100
        },
        roam: true,
        data: graphData.value.nodes,
        links: graphData.value.links,
        categories: [
          { name: 'domain' },
          { name: 'ip' },
          { name: 'port' },
          { name: 'website' },
          { name: 'component' },
          { name: 'risk' }
        ],
        emphasis: {
          focus: 'adjacency',
          lineStyle: {
            width: 3
          }
        },
        label: {
          show: true,
          position: 'right',
          formatter: '{b}'
        },
        lineStyle: {
          color: 'source',
          curveness: 0.3
        }
      }
    ]
  };
  
  chart.setOption(option);
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

// 获取资产关联数据
async function fetchData() {
  if (!taskId.value) {
    Message.warning('请先选择扫描任务');
    return;
  }
  
  loading.value = true;
  try {
    const result:any = await invoke('get_asset_graph_data', { 
      task_id: taskId.value,
      graph_type: graphType.value
    });
    
    // 转换数据格式
    let nodes:any[] = [];
    let links:any[] = [];
    
    // 处理节点
    if (result.nodes) {
      result.nodes.forEach((node:any) => {
        const nodeItem = {
          ...node,
          symbolSize: getNodeSize(node.category),
          itemStyle: { color: getNodeColor(node.category) },
          category: node.category
        };
        nodes.push(nodeItem);
      });
    }
    
    // 处理连线
    if (result.links) {
      result.links.forEach((link:any) => {
        links.push({
          ...link,
          lineStyle: {
            width: 2,
            opacity: 0.7
          }
        });
      });
    }
    
    graphData.value = { nodes, links };
    updateChart();
    
    if (nodes.length === 0) {
      Message.info('没有找到相关的资产关联数据');
    }
  } catch (error) {
    console.error('获取资产关联数据失败:', error);
    Message.error('获取资产关联数据失败');
  } finally {
    loading.value = false;
  }
}

// 根据节点类型获取节点大小
function getNodeSize(category:any) {
  switch (category) {
    case 'domain': return 25;
    case 'ip': return 20;
    case 'port': return 15;
    case 'website': return 18;
    case 'component': return 15;
    case 'risk': return 20;
    default: return 15;
  }
}

// 根据节点类型获取节点颜色
function getNodeColor(category:any) {
  switch (category) {
    case 'domain': return '#3498db';
    case 'ip': return '#2ecc71';
    case 'port': return '#9b59b6';
    case 'website': return '#e67e22';
    case 'component': return '#1abc9c';
    case 'risk': return '#e74c3c';
    default: return '#95a5a6';
  }
}

// 重置图表
function resetGraph() {
  if (!chart) return;
  chart.clear();
  graphData.value = { nodes: [], links: [] };
  selectedNode.value = null;
  fetchData();
}

// 导出图表
function exportGraph() {
  if (!chart) return;
  
  const dataURL = chart.getDataURL({
    type: 'png',
    pixelRatio: 2,
    backgroundColor: '#fff'
  });
  
  // 创建下载链接
  const link = document.createElement('a');
  link.download = '资产关联图.png';
  link.href = dataURL;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

// 监听图谱类型变化，自动刷新数据
watch([graphType, taskId], () => {
  if (chart && taskId.value) {
    resetGraph();
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
  height: 600px;
  width: 100%;
  background-color: #fdfdfd;
  border-radius: 4px;
  border: 1px solid var(--color-border-2);
}

.node-details {
  margin-top: 16px;
  padding: 12px;
  border-radius: 4px;
  background-color: #f8f9fa;
}

.graph-legend {
  display: flex;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 16px;
}

.legend-item {
  display: flex;
  align-items: center;
  margin-right: 16px;
}

.legend-icon {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  margin-right: 6px;
  
  &.domain { background-color: #3498db; }
  &.ip { background-color: #2ecc71; }
  &.port { background-color: #9b59b6; }
  &.website { background-color: #e67e22; }
  &.component { background-color: #1abc9c; }
  &.risk { background-color: #e74c3c; }
}
</style> 