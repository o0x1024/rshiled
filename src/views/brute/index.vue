<template>
	<a-layout class="layout">
		<a-layout>
			<a-layout-sider class="layout-sider" @collapse="setCollapsed"  :width="menuWidth" :style="{ paddingTop: '60px' }">
				<div class="menu-wrapper">
					<a-menu :style="{ height: 'calc(100% - 0px)' }" :collapsed="menuCollapse"  @menu-item-click="onMenuClick" mode="vertical"
						showCollapseButton>
						<a-menu-item key="task-management">
							<template #icon><icon-calendar /></template>
							{{ $t('brute.task_management') }}
						</a-menu-item>
					</a-menu>
				</div>
			</a-layout-sider>
			<a-layout-content class="layout-content" :style="paddingStyle">
				<!-- 任务管理界面 -->
				<div class="content-container">
					<a-card :title="$t('brute.task_management')">
						<template #extra>
							<a-button type="primary" @click="showCreateTaskModal">
								<template #icon><icon-plus /></template>
								{{ $t('brute.create_task') }}
							</a-button>
						</template>

						<a-table :data="tasks" :loading="loading" row-key="id" :pagination="{ pageSize: 10 }">
							<template #columns>
								<a-table-column title="#" data-index="id" :width="60" />
								<a-table-column :title="$t('brute.task_name')" data-index="name" />
								<a-table-column :title="$t('brute.target')" data-index="target" />
								<a-table-column :title="$t('brute.port')" data-index="port" :width="80" />
								<a-table-column :title="$t('brute.protocol')" data-index="protocol" :width="100">
									<template #cell="{ record }">
										{{ $t(`brute.protocol.${record.protocol.toLowerCase()}`) }}
									</template>
								</a-table-column>
								<a-table-column :title="$t('brute.task_status')" data-index="status" :width="100">
									<template #cell="{ record }">
										<a-tag :color="getStatusColor(record.status)">
											{{ $t(`brute.status.${record.status.toLowerCase()}`) }}
										</a-tag>
									</template>
								</a-table-column>
								<a-table-column :title="$t('brute.created_at')" data-index="created_at" :width="180">
									<template #cell="{ record }">
										{{ formatDateTime(record.created_at) }}
									</template>
								</a-table-column>
								<a-table-column :title="$t('brute.action')" :width="240" fixed="right">
									<template #cell="{ record }">
										<a-button-group>
											<a-button type="primary" size="small" status="success" 
												v-if="record.status !== 'Running'" 
												@click="handleStartTask(record.id)"
												:loading="actionLoading">
												{{ $t('brute.start') }}
											</a-button>
											<a-button type="primary" size="small" status="warning" 
												v-if="record.status === 'Running'" 
												@click="handleStopTask(record.id)"
												:loading="actionLoading">
												{{ $t('brute.stop') }}
											</a-button>
											<a-button type="primary" size="small" 
												@click="handleViewResults(record.id)">
												{{ $t('brute.view_results') }}
											</a-button>
											<a-popconfirm :content="$t('brute.confirm_delete')" @ok="handleDeleteTask(record.id)">
												<a-button type="primary" size="small" status="danger">
													{{ $t('brute.delete') }}
												</a-button>
											</a-popconfirm>
										</a-button-group>
									</template>
								</a-table-column>
							</template>
						</a-table>
					</a-card>
				</div>
			</a-layout-content>
		</a-layout>
	</a-layout>

	<!-- 创建任务模态框 -->
	<a-modal
		v-model:visible="createTaskModalVisible"
		:title="$t('brute.create_task')"
		@ok="handleCreateTask"
		@cancel="createTaskModalVisible = false"
	>
		<a-form :model="taskForm" layout="vertical">
			<a-form-item :label="$t('brute.task_name')" field="name" :rules="[{ required: true }]">
				<a-input v-model="taskForm.name" />
			</a-form-item>
			<a-form-item :label="$t('brute.target')" field="target" :rules="[{ required: true }]">
				<a-input v-model="taskForm.target" />
			</a-form-item>
			<a-form-item :label="$t('brute.port')" field="port" :rules="[{ required: true }]">
				<a-input-number v-model="taskForm.port" :min="1" :max="65535" />
			</a-form-item>
			<a-form-item :label="$t('brute.protocol')" field="protocol" :rules="[{ required: true }]">
				<a-select v-model="taskForm.protocol">
					<a-option value="SSH">SSH</a-option>
					<a-option value="SMB">SMB</a-option>
					<a-option value="RDP">RDP</a-option>
					<a-option value="MySQL">MySQL</a-option>
					<a-option value="MSSQL">MSSQL</a-option>
					<a-option value="Redis">Redis</a-option>
					<a-option value="PostgreSQL">PostgreSQL</a-option>
					<a-option value="Oracle">Oracle</a-option>
					<a-option value="FTP">FTP</a-option>
					<a-option value="Telnet">Telnet</a-option>
				</a-select>
			</a-form-item>

			<a-divider orientation="center">{{ $t('brute.username') }} / {{ $t('brute.password') }}</a-divider>
			
			<a-tabs default-active-key="1">
				<a-tab-pane key="1" :title="$t('brute.username')">
					<a-radio-group v-model="usernameMode" type="button">
						<a-radio value="list">列表</a-radio>
						<a-radio value="file">文件</a-radio>
					</a-radio-group>
					
					<div v-if="usernameMode === 'list'" class="mt-2">
						<a-textarea v-model="usernameInput" placeholder="每行一个用户名" allow-clear />
					</div>
					<div v-else class="mt-2">
						<a-input v-model="usernameFilePath" placeholder="选择或输入文件路径" readonly>
							<template #append>
								<a-button @click="selectUsernameFile">
									<icon-folder />
								</a-button>
							</template>
						</a-input>
					</div>
				</a-tab-pane>
				<a-tab-pane key="2" :title="$t('brute.password')">
					<a-radio-group v-model="passwordMode" type="button">
						<a-radio value="list">列表</a-radio>
						<a-radio value="file">文件</a-radio>
					</a-radio-group>
					
					<div v-if="passwordMode === 'list'" class="mt-2">
						<a-textarea v-model="passwordInput" placeholder="每行一个密码" allow-clear />
					</div>
					<div v-else class="mt-2">
						<a-input v-model="passwordFilePath" placeholder="选择或输入文件路径" readonly>
							<template #append>
								<a-button @click="selectPasswordFile">
									<icon-folder />
								</a-button>
							</template>
						</a-input>
					</div>
				</a-tab-pane>
			</a-tabs>

			<a-divider orientation="center">{{ $t('brute.threads') }} / {{ $t('brute.timeout') }}</a-divider>

			<a-row :gutter="16">
				<a-col :span="12">
					<a-form-item :label="$t('brute.threads')" field="threads">
						<a-input-number v-model="taskForm.threads" :min="1" :max="100" />
					</a-form-item>
				</a-col>
				<a-col :span="12">
					<a-form-item :label="$t('brute.timeout')" field="timeout">
						<a-input-number v-model="taskForm.timeout" :min="1" :max="120" />
					</a-form-item>
				</a-col>
			</a-row>
		</a-form>
	</a-modal>

	<!-- 查看结果模态框 -->
	<a-modal
		v-model:visible="viewResultsModalVisible"
		:title="$t('brute.result')"
		:footer="false"
		width="800px"
	>
		<a-table 
			:data="results" 
			:loading="resultsLoading" 
			:pagination="{ pageSize: 10 }"
		>
			<template #columns>
				<a-table-column :title="$t('brute.username')" data-index="username" />
				<a-table-column :title="$t('brute.password')" data-index="password" />
				<a-table-column :title="$t('brute.success')" data-index="success" :width="80">
					<template #cell="{ record }">
						<a-tag :color="record.success ? 'green' : 'red'">
							{{ record.success ? $t('brute.success') : $t('brute.failed') }}
						</a-tag>
					</template>
				</a-table-column>
				<a-table-column :title="$t('brute.time_taken')" data-index="time_taken" :width="120">
					<template #cell="{ record }">
						{{ `${record.time_taken}ms` }}
					</template>
				</a-table-column>
				<a-table-column :title="$t('brute.error')" data-index="error">
					<template #cell="{ record }">
						{{ record.error || '-' }}
					</template>
				</a-table-column>
			</template>
		</a-table>
	</a-modal>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { Message } from '@arco-design/web-vue';
import * as BruteApi from '@/api/brute';
import type { BruteForceTask, BruteForceResult } from '@/api/brute';
import { Protocol, TaskStatus } from '@/api/brute';
import { open } from '@tauri-apps/plugin-dialog';

defineOptions({
	name: 'brute-home',
})

const menuCollapse = ref(false)
let mwidth = 180
const menuWidth = computed(() => {
	return menuCollapse.value ? 48 : mwidth;
});

const setCollapsed = (val: boolean) => {
	menuCollapse.value = val
};


const paddingStyle = computed(() => {
	const paddingLeft = { paddingLeft: menuCollapse.value ? '68px' : '220px' }
	const paddingTop = { paddingTop: '20px' }
	const paddingRight = { paddingRight: '20px' }
	return { ...paddingLeft, ...paddingTop, ...paddingRight };
});

// 任务状态对应的颜色
const getStatusColor = (status: string) => {
	switch (status) {
		case 'Running': return 'blue';
		case 'Completed': return 'green';
		case 'Failed': return 'red';
		case 'Stopped': return 'orange';
		default: return 'gray';
	}
};

// 格式化日期时间
const formatDateTime = (timestamp: number) => {
	if (!timestamp) return '-';
	const date = new Date(timestamp * 1000);
	return date.toLocaleString();
};

// 处理菜单点击
const onMenuClick = (key: string) => {
	console.log(key);
	// 菜单项处理
};

// 任务数据
const tasks = ref<BruteForceTask[]>([]);
const loading = ref(false);
const actionLoading = ref(false);

// 加载任务列表
const loadTasks = async () => {
	loading.value = true;
	try {
		const result = await BruteApi.getTasks();
		tasks.value = result || [];
	} catch (error) {
		console.error(error);
		Message.error('加载任务列表失败');
	} finally {
		loading.value = false;
	}
};

// 创建任务模态框
const createTaskModalVisible = ref(false);
const usernameMode = ref('list');
const passwordMode = ref('list');
const usernameInput = ref('');
const passwordInput = ref('');
const usernameFilePath = ref('');
const passwordFilePath = ref('');

// 任务表单
const taskForm = ref<BruteForceTask>({
	name: '',
	target: '',
	port: 22,
	protocol: Protocol.SSH,
	threads: 10,
	timeout: 30,
	status: TaskStatus.Pending
});

// 选择用户名文件
const selectUsernameFile = async () => {
	try {
		const selected = await open({
			multiple: false,
			filters: [{ name: 'Text', extensions: ['txt'] }]
		});
		if (selected) {
			usernameFilePath.value = selected as string;
			taskForm.value.username_file = usernameFilePath.value;
		}
	} catch (error) {
		console.error('选择文件出错', error);
		Message.error('选择文件失败');
	}
};

// 选择密码文件
const selectPasswordFile = async () => {
	try {
		const selected = await open({
			multiple: false,
			filters: [{ name: 'Text', extensions: ['txt'] }]
		});
		if (selected) {
			passwordFilePath.value = selected as string;
			taskForm.value.password_file = passwordFilePath.value;
		}
	} catch (error) {
		console.error('选择文件出错', error);
		Message.error('选择文件失败');
	}
};

// 显示创建任务模态框
const showCreateTaskModal = () => {
	taskForm.value = {
		name: '',
		target: '',
		port: 22,
		protocol: Protocol.SSH,
		threads: 10,
		timeout: 30,
		status: TaskStatus.Pending
	};
	usernameInput.value = '';
	passwordInput.value = '';
	usernameFilePath.value = '';
	passwordFilePath.value = '';
	usernameMode.value = 'list';
	passwordMode.value = 'list';
	createTaskModalVisible.value = true;
};

// 处理创建任务
const handleCreateTask = async () => {
	if (!taskForm.value.name || !taskForm.value.target) {
		Message.error('请填写必要的信息');
		return;
	}

	// 处理用户名和密码
	if (usernameMode.value === 'list' && usernameInput.value) {
		taskForm.value.usernames = usernameInput.value.split('\n').filter(item => item.trim());
	}
	
	if (passwordMode.value === 'list' && passwordInput.value) {
		taskForm.value.passwords = passwordInput.value.split('\n').filter(item => item.trim());
	}

	try {
		await BruteApi.createTask(taskForm.value);
		Message.success('创建任务成功');
		createTaskModalVisible.value = false;
		loadTasks();
	} catch (error) {
		console.error(error);
		Message.error('创建任务失败');
	}
};

// 启动任务
const handleStartTask = async (id: number) => {
	actionLoading.value = true;
	try {
		await BruteApi.startTask(id);
		Message.success('任务已启动');
		loadTasks();
	} catch (error) {
		console.error(error);
		Message.error('启动任务失败');
	} finally {
		actionLoading.value = false;
	}
};

// 停止任务
const handleStopTask = async (id: number) => {
	actionLoading.value = true;
	try {
		await BruteApi.stopTask(id);
		Message.success('任务已停止');
		loadTasks();
	} catch (error) {
		console.error(error);
		Message.error('停止任务失败');
	} finally {
		actionLoading.value = false;
	}
};

// 删除任务
const handleDeleteTask = async (id: number) => {
	actionLoading.value = true;
	try {
		await BruteApi.deleteTask(id);
		Message.success('任务已删除');
		loadTasks();
	} catch (error) {
		console.error(error);
		Message.error('删除任务失败');
	} finally {
		actionLoading.value = false;
	}
};

// 结果数据
const results = ref<BruteForceResult[]>([]);
const resultsLoading = ref(false);
const viewResultsModalVisible = ref(false);
let currentTaskId = ref<number | null>(null);

// 查看结果
const handleViewResults = async (id: number) => {
	currentTaskId.value = id;
	viewResultsModalVisible.value = true;
	await loadResults(id);
};

// 加载结果
const loadResults = async (id: number) => {
	resultsLoading.value = true;
	try {
		const result = await BruteApi.getResults(id);
		results.value = result || [];
	} catch (error) {
		console.error(error);
		Message.error('加载结果失败');
	} finally {
		resultsLoading.value = false;
	}
};

// 自动刷新
let refreshInterval: number | null = null;

// 页面加载时获取任务列表
onMounted(() => {
	loadTasks();
	
	// 每10秒刷新一次任务列表
	refreshInterval = window.setInterval(() => {
		loadTasks();
		// 如果正在查看某个任务的结果，也刷新结果
		if (viewResultsModalVisible.value && currentTaskId.value) {
			loadResults(currentTaskId.value);
		}
	}, 10000);
});

// 组件卸载时清除定时器
onUnmounted(() => {
	if (refreshInterval !== null) {
		clearInterval(refreshInterval);
	}
});


</script>

<style scoped>
.layout {
	width: 100%;
	height: 100%;
}

.layout-sider {
	position: fixed;
	top: 0;
	left: 0;
	z-index: 99;
	height: 100%;
	transition: all 0.2s cubic-bezier(0.34, 0.69, 0.1, 1);

	&::after {
		position: absolute;
		top: 0;
		right: -1px;
		display: block;
		width: 1px;
		height: 100%;
		background-color: var(--color-border);
		content: '';
	}

	> :deep(.arco-layout-sider-children) {
		overflow-y: hidden;
	}
}

.layout-content {
	overflow-y: auto;
	transition: padding 0.2s cubic-bezier(0.34, 0.69, 0.1, 1);
	height: calc(100vh - 60px);
}

.content-container {
	padding-bottom: 20px;
}

.menu-wrapper {
	height: 100%;
	overflow: auto;
	overflow-x: hidden;

	:deep(.arco-menu) {
		::-webkit-scrollbar {
			width: 12px;
			height: 4px;
		}

		::-webkit-scrollbar-thumb {
			border: 4px solid transparent;
			background-clip: padding-box;
			border-radius: 7px;
			background-color: var(--color-text-4);
		}

		::-webkit-scrollbar-thumb:hover {
			background-color: var(--color-text-3);
		}
	}
}

.mt-2 {
	margin-top: 8px;
}
</style>