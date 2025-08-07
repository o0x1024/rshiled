<template>
  <div class="container">
    <a-card class="general-card">
      <template #title>
        {{ $t('asm.visualization.compliance_report') }}
        <a-tooltip content="生成符合等保、GDPR等标准的合规报告">
          <icon-question-circle style="margin-left: 8px;" />
        </a-tooltip>
      </template>
      
      <a-form :model="formData" layout="vertical" :style="{ maxWidth: '800px', margin: '0 auto' }">
        <a-form-item field="taskId" label="选择扫描任务" required>
          <a-select v-model="formData.taskId" placeholder="请选择扫描任务" :loading="tasksLoading">
            <a-option v-for="task in taskList" :key="task.id" :value="task.id">{{ task.name }}</a-option>
          </a-select>
        </a-form-item>
        
        <a-form-item field="reportType" label="报告类型" required>
          <a-radio-group v-model="formData.reportType" type="button">
            <a-radio value="djcp">等保合规报告</a-radio>
            <a-radio value="gdpr">GDPR合规报告</a-radio>
            <a-radio value="iso27001">ISO 27001合规报告</a-radio>
            <a-radio value="custom">自定义报告</a-radio>
          </a-radio-group>
        </a-form-item>
        
        <a-form-item field="reportTitle" label="报告标题">
          <a-input v-model="formData.reportTitle" placeholder="请输入报告标题" />
        </a-form-item>
        
        <a-form-item field="companyInfo" label="公司信息">
          <a-textarea v-model="formData.companyInfo" placeholder="请输入公司基本信息" :auto-size="{ minRows: 2, maxRows: 4 }" />
        </a-form-item>
        
        <a-form-item field="sections" label="报告章节">
          <a-space direction="vertical" style="width: 100%">
            <a-checkbox-group v-model="formData.sections" direction="vertical">
              <a-checkbox value="overview">概述</a-checkbox>
              <a-checkbox value="assets">资产清单</a-checkbox>
              <a-checkbox value="vulnerabilities">安全漏洞分析</a-checkbox>
              <a-checkbox value="risks">风险评估</a-checkbox>
              <a-checkbox value="compliance">合规评估</a-checkbox>
              <a-checkbox value="recommendations">安全建议</a-checkbox>
            </a-checkbox-group>
          </a-space>
        </a-form-item>
        
        <a-form-item field="reportFormat" label="报告格式">
          <a-radio-group v-model="formData.reportFormat" type="button">
            <a-radio value="pdf">PDF</a-radio>
            <a-radio value="html">HTML</a-radio>
            <a-radio value="word">Word</a-radio>
          </a-radio-group>
        </a-form-item>
        
        <a-divider />
        
        <a-form-item>
          <a-space>
            <a-button type="primary" @click="generateReport" :loading="generating">
              生成报告
            </a-button>
            <a-button @click="resetForm">
              重置
            </a-button>
          </a-space>
        </a-form-item>
      </a-form>
      
      <a-divider />
      
      <div class="preview-section" v-if="reportPreview">
        <div class="preview-header">
          <h3>报告预览</h3>
          <a-space>
            <a-tag :color="getReportFormatColor(formData.reportFormat)">
              {{ formData.reportFormat.toUpperCase() }}
            </a-tag>
            <a-button size="small" type="primary" @click="downloadReport">
              <template #icon><icon-download /></template>
              下载报告
            </a-button>
          </a-space>
        </div>
        
        <a-spin :loading="previewLoading">
          <div class="report-preview">
            <div class="report-title">{{ reportPreview.title }}</div>
            
            <div class="report-meta">
              <div>报告类型: {{ getReportTypeName(reportPreview.type) }}</div>
              <div>生成时间: {{ reportPreview.generatedAt }}</div>
              <div>任务名称: {{ reportPreview.taskName }}</div>
            </div>
            
            <div class="report-toc">
              <div class="toc-title">目录</div>
              <div class="toc-items">
                <div v-for="(section, index) in reportPreview.sections" :key="index" class="toc-item">
                  {{ index + 1 }}. {{ section.title }}
                </div>
              </div>
            </div>
            
            <div class="report-summary">
              <h4>报告摘要</h4>
              <p>{{ reportPreview.summary }}</p>
              
              <div class="summary-stats">
                <div class="stat-item">
                  <div class="stat-value red">{{ reportPreview.stats.criticalIssues }}</div>
                  <div class="stat-label">严重问题</div>
                </div>
                <div class="stat-item">
                  <div class="stat-value orange">{{ reportPreview.stats.highIssues }}</div>
                  <div class="stat-label">高危问题</div>
                </div>
                <div class="stat-item">
                  <div class="stat-value yellow">{{ reportPreview.stats.mediumIssues }}</div>
                  <div class="stat-label">中危问题</div>
                </div>
                <div class="stat-item">
                  <div class="stat-value green">{{ reportPreview.stats.lowIssues }}</div>
                  <div class="stat-label">低危问题</div>
                </div>
              </div>
              
              <div class="compliance-score">
                <div class="score-label">合规评分</div>
                <a-progress 
                  type="circle" 
                  :percent="reportPreview.stats.complianceScore" 
                  :stroke-color="getScoreColor(reportPreview.stats.complianceScore)"
                  size="large"
                />
                <div class="score-description">
                  {{ getScoreDescription(reportPreview.stats.complianceScore) }}
                </div>
              </div>
            </div>
            
            <a-collapse accordion>
              <a-collapse-item v-for="(section, index) in reportPreview.sections" :key="index" :header="section.title">
                <p style="white-space: pre-line">{{ section.content }}</p>
              </a-collapse-item>
            </a-collapse>
            
            <a-divider>预览内容仅供参考，完整报告请下载查看</a-divider>
          </div>
        </a-spin>
      </div>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
// import { open } from '@tauri-apps/api/dialog';
import { Message } from '@arco-design/web-vue';

interface Task {
  id: string;
  name: string;
}

interface ReportPreview {
  title: string;
  type: string;
  generatedAt: string;
  taskName: string;
  sections: { title: string; content: string }[];
  summary: string;
  stats: {
    criticalIssues: number;
    highIssues: number;
    mediumIssues: number;
    lowIssues: number;
    complianceScore: number;
  };
  filePath?: string;
}

// 状态变量
const taskList = ref<Task[]>([]);
const tasksLoading = ref(false);
const generating = ref(false);
const previewLoading = ref(false);
const reportPreview = ref<ReportPreview | null>(null);

// 表单数据
const formData = reactive({
  taskId: '',
  reportType: 'djcp',
  reportTitle: '',
  companyInfo: '',
  sections: ['overview', 'vulnerabilities', 'risks', 'compliance', 'recommendations'],
  reportFormat: 'pdf'
});

// 获取报告类型名称
function getReportTypeName(type: string): string {
  const types: Record<string, string> = {
    'djcp': '等保合规报告',
    'gdpr': 'GDPR合规报告',
    'iso27001': 'ISO 27001合规报告',
    'custom': '自定义报告'
  };
  
  return types[type] || type;
}

// 获取报告格式颜色
function getReportFormatColor(format: string): string {
  const colors: Record<string, string> = {
    'pdf': 'red',
    'html': 'blue',
    'word': 'purple'
  };
  
  return colors[format] || 'gray';
}

// 获取评分颜色
function getScoreColor(score: number): string {
  if (score >= 90) return '#00b42a';
  if (score >= 70) return '#168cff';
  if (score >= 50) return '#ff7d00';
  return '#f53f3f';
}

// 获取评分描述
function getScoreDescription(score: number): string {
  if (score >= 90) return '优秀';
  if (score >= 70) return '良好';
  if (score >= 50) return '一般';
  if (score >= 30) return '较差';
  return '差';
}

// 获取任务列表
async function fetchTaskList() {
  tasksLoading.value = true;
  try {
    const result:any  = await invoke('get_task_list', { page: 1, pagesize: 100 });
    taskList.value = result.list || [];
    if (taskList.value.length > 0) {
      formData.taskId = taskList.value[0].id;
    }
  } catch (error) {
    console.error('获取任务列表失败:', error);
    Message.error('获取任务列表失败');
  } finally {
    tasksLoading.value = false;
  }
}

// 生成报告
async function generateReport() {
  if (!formData.taskId) {
    Message.warning('请选择扫描任务');
    return;
  }
  
  if (!formData.reportType) {
    Message.warning('请选择报告类型');
    return;
  }
  
  if (formData.sections.length === 0) {
    Message.warning('请至少选择一个报告章节');
    return;
  }
  
  generating.value = true;
  previewLoading.value = true;
  
  try {
    // 显示生成中的消息通知
    Message.loading({
      content: '正在生成报告，请稍候...',
      duration: 0
    });
    
    // 调用后端生成报告
    const result:any = await invoke<{ preview: ReportPreview; filePath: string }>('generate_compliance_report', {
      task_id: formData.taskId,
      report_type: formData.reportType,
      report_title: formData.reportTitle || undefined,
      company_info: formData.companyInfo || undefined,
      sections: formData.sections,
      report_format: formData.reportFormat
    });
    
    // 关闭生成中的消息通知
    Message.clear();
    
    if (result && result.preview) {
      reportPreview.value = {
        title: result.preview.title,
        type: result.preview.type_,
        generatedAt: result.preview.generated_at,
        taskName: result.preview.task_name,
        sections: result.preview.sections.map((s:any) => ({ title: s.title, content: s.content })),
        summary: result.preview.summary,
        stats: {
          criticalIssues: result.preview.stats.critical_issues,
          highIssues: result.preview.stats.high_issues,
          mediumIssues: result.preview.stats.medium_issues,
          lowIssues: result.preview.stats.low_issues,
          complianceScore: result.preview.stats.compliance_score
        },
        filePath: result.file_path
      };
      Message.success('报告生成成功');
    } else {
      Message.error('报告生成失败');
    }
  } catch (error) {
    console.error('生成报告失败:', error);
    Message.clear(); // 确保出错时也关闭加载提示
    Message.error(`生成报告失败: ${error}`);
  } finally {
    generating.value = false;
    previewLoading.value = false;
  }
}

// 重置表单
function resetForm() {
  formData.reportTitle = '';
  formData.companyInfo = '';
  formData.sections = ['overview', 'vulnerabilities', 'risks', 'compliance', 'recommendations'];
  formData.reportFormat = 'pdf';
  reportPreview.value = null;
}

// 下载报告
async function downloadReport() {
  if (!reportPreview.value || !reportPreview.value.filePath) {
    Message.warning('没有可下载的报告');
    return;
  }
  
  try {
    // 显示下载中的消息通知
    Message.loading({
      content: '正在打开报告，请稍候...',
      duration: 0
    });
    
    // 打开报告文件
    await invoke('open_file', { path: reportPreview.value.filePath });
    
    // 关闭下载中的消息通知
    Message.clear();
    Message.success('报告已打开');
  } catch (error) {
    console.error('打开报告失败:', error);
    Message.clear(); // 确保出错时也关闭加载提示
    Message.error(`打开报告失败: ${error}`);
  }
}

// 初始化
onMounted(() => {
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

.preview-section {
  margin-top: 20px;
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  background-color: #f8f9fa;
  padding: 10px 15px;
  border-radius: 4px;
}

.preview-header h3 {
  margin: 0;
  color: var(--color-text-1);
}

.report-preview {
  padding: 20px;
  background-color: #fdfdfd;
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.report-title {
  font-size: 24px;
  font-weight: bold;
  text-align: center;
  margin-bottom: 20px;
  color: var(--color-text-1);
}

.report-meta {
  display: flex;
  justify-content: space-between;
  margin-bottom: 20px;
  color: var(--color-text-3);
  font-size: 14px;
  background-color: #f8f9fa;
  padding: 10px;
  border-radius: 4px;
}

.report-toc {
  margin-bottom: 30px;
  border-left: 3px solid var(--color-primary-light-4);
  padding-left: 16px;
  background-color: #f9f9fa;
  padding: 15px;
  border-radius: 4px;
}

.toc-title {
  font-weight: bold;
  margin-bottom: 10px;
  color: var(--color-text-1);
}

.toc-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toc-item {
  color: var(--color-text-2);
  transition: color 0.3s;
  
  &:hover {
    color: var(--color-primary);
  }
}

.report-summary {
  margin: 20px 0 30px;
  background-color: #f8f9fa;
  padding: 20px;
  border-radius: 8px;
  border-left: 4px solid var(--color-primary);
}

.report-summary h4 {
  margin-top: 0;
  color: var(--color-text-1);
  font-size: 18px;
}

.summary-stats {
  display: flex;
  justify-content: space-between;
  margin: 20px 0;
  flex-wrap: wrap;
}

.stat-item {
  text-align: center;
  flex: 1;
  min-width: 100px;
  padding: 10px;
  background-color: white;
  border-radius: 8px;
  margin: 5px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
  transition: transform 0.3s;
  
  &:hover {
    transform: translateY(-5px);
  }
}

.stat-value {
  font-size: 28px;
  font-weight: bold;
  margin-bottom: 5px;
  
  &.red { color: #f53f3f; }
  &.orange { color: #ff7d00; }
  &.yellow { color: #ffb400; }
  &.green { color: #00b42a; }
}

.stat-label {
  font-size: 14px;
  color: var(--color-text-3);
}

.compliance-score {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin: 30px 0;
}

.score-description {
  margin-top: 10px;
  font-size: 16px;
  font-weight: bold;
}

.score-label {
  margin-bottom: 10px;
  font-size: 16px;
  color: var(--color-text-2);
}
</style> 