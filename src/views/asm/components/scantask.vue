<template>
	<a-space direction="vertical" fill>
		<a-row justify="space-between" align="center">
			<a-col :span="12">
				<a-typography-text style="font-size: large; font-weight: 540;">{{ $t('asm.task-info')
				}}</a-typography-text>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="510px">
						<a-space>
							<a-button type="dashed" size="small" @click="RefreshData">
                                    <template #icon>
                                        <icon-refresh />
                                    </template>
                                </a-button>
							<a-input-search v-model="searchKeyword" :placeholder="$t('asm.search-task')"
								style="width: 200px" @search="onSearch" size="small" />
							<a-radio-group type="button" size="small" v-model="viewMode">
								<a-radio value="list">
									<template #icon><icon-list /></template>
									{{ $t('asm.list-view') }}
								</a-radio>
								<a-radio value="card">
									<template #icon><icon-apps /></template>
									{{ $t('asm.card-view') }}
								</a-radio>
							</a-radio-group>
							<a-button type="primary" size="small" @click="onAdd">
								{{ $t('asm.add-task') }}
							</a-button>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row v-if="viewMode === 'list'">
			<a-col :span="24">
				<a-table :columns="columns" :bordered="false" :data="filteredTaskList" :pagination="pagination"
					@page-change="onPageChange" size="small">

					<template #monitor_status="{ record }">
						<a-switch v-model="record.monitor_status" :checked-value="1" :unchecked-value="0"
							@change="onSwitchChange(record.id, record.monitor_status)" />
					</template>

					<template #running_status="{ record }">
						<a-tag color="green">{{ record.running_status }}</a-tag>
					</template>

					<template #domain_count="{ record }">
						<a-link @click="toRootDomain(record.name)">{{ record.domain_count }}</a-link>
					</template>

					<template #time="{ record }">
						<a-space direction="vertical" fill>
							<span color="green">运行间隔时间:{{ formatTime(record.next_run_time) }}</span>
							<span color="green">最近运行时间:{{ formatDateTime(record.last_run_time) }}</span>
						</a-space>
					</template>


					<template #operation="{ record }">
						<a-dropdown>
							<div class="clickable"><icon-more /></div>
							<template #content>
								<a-doption @click="onSelectScanType('all', record.id)">
									<template #icon>
										<icon-search />
									</template>
									<template #default>{{ $t('asm.task.run') }}</template>
								</a-doption>
								<a-doption @click="onDel(record.id)">
									<template #icon>
										<icon-delete />
									</template>
									<template #default>{{ $t('asm.del-task') }}</template>
								</a-doption>
							</template>
						</a-dropdown>
					</template>
				</a-table>
			</a-col>
		</a-row>
		<a-row v-else :gutter="[16, 16]">
			<a-col v-for="record in pagedCardList" :key="record.id" :xs="24" :sm="12" :md="8" :lg="6">
				<a-card class="task-card" :bordered="true" :hoverable="true">
					<template #title>
						<div class="card-header">
							<span class="task-name truncate">{{ record.name }} - {{ record.id }}</span>
							<a-tag color="green">{{ record.running_status }}</a-tag>
						</div>
					</template>
					<div class="card-content">
						<div class="card-switch-row">
							<span class="monitor-label">{{ $t('asm.monitor-status') }}</span>
							<a-switch size="small" v-model="record.monitor_status" :checked-value="1"
								:unchecked-value="0"
								@change="onSwitchChange(Number(record.id), record.monitor_status)" />
						</div>
						<div class="info-grid">
							<div class="info-row">
								<a-badge @click="toRootDomain(record.name)" :count="record.rootdomain_count"
									:max-count="99" dot class="info-btn">
									<a-tooltip content="根域名">
										<div class="info-icon-wrap">
											<icon-list class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>



								<a-badge @click="toDomain(record.id)" :count="record.domain_count" :max-count="99" dot
									class="info-btn info-btn-2">
									<a-tooltip content="域名解析">
										<div class="info-icon-wrap">
											<icon-share-alt class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>

								<a-badge @click="toIPS(record.id)" :count="record.ips_count" :max-count="99" dot
									class="info-btn info-btn-2">
									<a-tooltip content="IP地址">
										<div class="info-icon-wrap">
											<icon-italic class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>

								<a-badge @click="toPort(record.id)" :count="record.port_count" :max-count="99" dot
									class="info-btn info-btn-2">
									<a-tooltip content="端口">
										<div class="info-icon-wrap">
											<icon-minus-circle class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>

								<a-badge @click="toWebSite(record.id)" :count="record.website_count" :max-count="99" dot
									class="info-btn info-btn-2">
									<a-tooltip content="网站">
										<div class="info-icon-wrap">
											<icon-public class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>

								<a-badge @click="toAPI(record.id)" :count="record.api_count" :max-count="99" dot
									class="info-btn info-btn-2">
									<a-tooltip content="API接口">
										<div class="info-icon-wrap">
											<icon-font-colors class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>

							</div>
							<div class="info-row">
								<a-badge @click="toWebComp(record.id)" :count="record.webcomp_count" :max-count="99" dot
									class="info-btn ">
									<a-tooltip content="WEB组件">
										<div class="info-icon-wrap">
											<icon-common class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>

								<a-badge @click="toRisk(record.id)" :count="record.risk_count" :max-count="99" dot
									class="info-btn ">
									<a-tooltip content="风险">
										<div class="info-icon-wrap ">
											<icon-bug class="info-icon" />
										</div>
									</a-tooltip>
								</a-badge>
							</div>
							<div class="info-row">
								<div class="info-icon-wrap">
									<icon-clock-circle class="info-icon" />
								</div>
								<div class="info-content">
									<div class="info-label">{{ $t('asm.next-runtime') }}
										<icon-edit class="edit-icon-wrap" @click="onEditNextRunTime(record.id)" />
									</div>
									<div v-if="editingTaskId !== record.id" class="info-value">
										{{ formatTime(record.next_run_time || '') }}
									</div>
									<div v-else class="info-value">
										<a-space>
											<a-input-number size="mini" :style="{ width: '100px' }" button-text="Search"
												search-button v-model:model-value="record.next_run_time" />
											<a-button type="primary" size="mini"
												@click="onSaveNextRunTime(record.id, record.next_run_time)">{{
													$t('asm.save')
												}}</a-button>
											<a-button size="mini" @click="onCannelEditNextRunTime()">{{ $t('asm.cannel')
												}}</a-button>

										</a-space>
									</div>
								</div>
							</div>
							<div class="info-row">
								<div class="info-icon-wrap">
									<icon-clock-circle class="info-icon" />
								</div>
								<div class="info-content">
									<div class="info-label">{{ $t('asm.last-runtime') }}</div>
									<div class="info-value">{{ formatDateTime(record.last_run_time || '') }}</div>
								</div>
							</div>
						</div>
					</div>
					<template #actions>
						<div class="card-actions">
							<a-dropdown position="br" @select="(value) => onSelectScanType(value as string, record.id)">
								<a-button type="outline" size="mini">
									<template #icon><icon-search /></template>
									{{ $t('asm.task.run') }}
									<template #suffix><icon-down /></template>
								</a-button>
								<template #content>
									<a-doption value="all">
										<template #icon><icon-search /></template>
										{{ $t('asm.scan_all') }}
									</a-doption>
									<a-doption value="domain">
										<template #icon><icon-share-alt /></template>
										{{ $t('asm.scan_domain') }}
									</a-doption>
									<a-doption value="ip">
										<template #icon><icon-italic /></template>
										{{ $t('asm.scan_ip') }}
									</a-doption>
									<a-doption value="port">
										<template #icon><icon-minus-circle /></template>
										{{ $t('asm.scan_port') }}
									</a-doption>
									<a-doption value="website">
										<template #icon><icon-calendar /></template>
										{{ $t('asm.scan_website') }}
									</a-doption>
									<a-doption value="api">
										<template #icon><icon-underline /></template>
										{{ $t('asm.scan_api') }}
									</a-doption>
									<a-doption value="webcomp">
										<template #icon><icon-common /></template>
										{{ $t('asm.scan_webcomp') }}
									</a-doption>
									<a-doption value="risk">
										<template #icon><icon-bug /></template>
										{{ $t('asm.scan_risk') }}
									</a-doption>

								</template>
							</a-dropdown>
							<a-popconfirm content="确定删除任务吗？" okText="确定" @ok="onDel(record.id)" cancelText="取消">
								<a-button type="outline" status="danger" size="mini">
									<template #icon><icon-delete /></template>
									{{ $t('asm.del-task') }}
								</a-button>
							</a-popconfirm>
						</div>
					</template>
				</a-card>
			</a-col>

			<a-col :span="24" style="margin-top: 16px; display: flex; justify-content: center;">
				<a-pagination v-model:current="cardPagination.current" :total="filteredTaskList.length"
					:page-size="cardPagination.pageSize" show-total simple @change="onCardPageChange" />
			</a-col>
		</a-row>
	</a-space>
	<a-modal v-model:visible="add_visible" title-align="start" @ok="handleOk" @cancel="handleCancel">
		<template #title>
			{{ $t('asm.add-task-model') }}
		</template>
		<a-space direction="vertical">
			<a-input :style="{ width: '320px' }" v-model:model-value="task_name"
				:placeholder="$t('asm.add-task-placeholder')" allow-clear />
			<!-- <a-checkbox value="1">{{ $t('asm.check-task-name') }}</a-checkbox> -->
		</a-space>
	</a-modal>
</template>

<script lang="ts" setup>
import { formatDateTime, formatTime } from '@/utils/format';
import { computed, onMounted, reactive, ref, onBeforeUnmount } from 'vue';
import { Pagination } from '@/types/global';
import { Task } from './types';
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
import { Message } from '@arco-design/web-vue';
import { useRouter } from 'vue-router'
import { IconList, IconApps, IconSearch, IconDelete, IconBug, IconDown } from '@arco-design/web-vue/es/icon';
const router = useRouter()
const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 10,
	showTotal: true,
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

defineOptions({
	name: 'asm-scan-object',
})

const task: { list: Task[] } = reactive({ list: [] })
const add_visible = ref(false)
const task_name = ref('')
const viewMode = ref('card')
const searchKeyword = ref('')
const editingTaskId = ref<string | null>(null)

const cardPagination = reactive({
	current: 1,
	pageSize: 8,  // 每页显示8个卡片
});

const pagedCardList = computed(() => {
	const startIndex = (cardPagination.current - 1) * cardPagination.pageSize;
	const endIndex = startIndex + cardPagination.pageSize;
	return filteredTaskList.value.slice(startIndex, endIndex);
});

async function RefreshData() {
	await invoke("get_task_list", { page: pagination.current, pagesize: pagination.pageSize }).then((res: any) => {
		if (res) {
			task.list = res.list
			pagination.total = res.total
			handleDataRefresh(); // 处理数据刷新后的分页状态
		}
	}).catch((err) => {
		console.log(err)
	})
}

const onEditNextRunTime = async (id: string) => {
	// if (editingTaskId.value === id) {
	// 	// 如果当前已经在编辑这个任务，则保存修改
	// 	// const taskToEdit = task.list.find(task => task.id === id);
	// } else {
	// 	// 如果当前不在编辑这个任务，则进入编辑模式
	// 	editingTaskId.value = id;
	// }
	editingTaskId.value = id;

}

const onSaveNextRunTime = async (id: string, next_run_time: any) => {
	await invoke("save_next_run_time", { id: id, next_run_time: next_run_time }).then((res: any) => {
		if (res) {
			Message.success("修改成功")
			editingTaskId.value = null; // 保存后清除正在编辑的任务ID
		}
	})
}

const onCannelEditNextRunTime = async () => {
	editingTaskId.value = null; // 保存后清除正在编辑的任务ID
}

const toDomain = async (id: string) => {
	router.push({
		name: 'asm-domain', query: { id: id }
	})
}

const toWebSite = async (id: string) => {
	router.push({
		name: 'asm-website', query: { id: id }
	})
}

const toAPI = async (id: string) => {
	router.push({
		name: 'asm-api', query: { id: id }
	})
}

const toIPS = async (id: string) => {
	router.push({
		name: 'asm-ip', query: { id: id }
	})
}

const toPort = async (id: string) => {
	router.push({
		name: 'asm-port', query: { id: id }
	})
}

const toWebComp = async (id: string) => {
	router.push({
		name: 'asm-web-component', query: { id: id }
	})
}

const toRisk = async (id: string) => {
	router.push({
		name: 'asm-risk', query: { id: id }
	})
}

const toRootDomain = async (task_name: string) => {
	router.push({
		name: 'asm-root-domain', query: { task_name: task_name }
	})
}

const onAdd = () => { add_visible.value = true }

async function handleOk() {
	await invoke("add_task", { task_name: task_name.value }).then((res: any) => {
		if (res) {
			RefreshData()
			Message.success("添加成功")
		} else {
			Message.success("添加失败")
		}
	}).catch((err) => {
		console.log(err)
	})
}

function handleCancel() {
	add_visible.value = false
}

onMounted(() => {
	RefreshData()
	// startPolling();
})

let pollingInterval: any;

onBeforeUnmount(() => {
	if (pollingInterval) {
		clearInterval(pollingInterval); // 清除定时器
		pollingInterval = null;
	}
})

function startPolling() {
	pollingInterval = setInterval(() => {
		RefreshData() // 定期请求数据
	}, 5000); // 每 5 秒请求一次
}

const onDel = async (eid: string) => {
	await invoke("del_task_by_id", { eid: eid }).then((res: any) => {
		if (res) {
			task.list = res.data
			Message.success("删除成功")
		} else {
			Message.success("删除失败")
		}
	}).catch((err) => {
		console.log(err)
	})
}

const onPageChange = (_page: number) => {
	pagination.current = _page;
}

const onSwitchChange = async (eid: number, status: any) => {
	await invoke("switch_task_status", { eid: eid, status: status }).then((res: any) => {
		if (res) {
			Message.success("切换成功")
		}
	}).catch((err) => {
		Message.error("暂停失败，原因：" + err)
	})
}

const onRun = async (eid: any) => {
	Message.info("开始扫描,如有异常,请查看日志信息")
	await invoke("run_scan", { eid: eid }).then((res: any) => {
		if (res) {
			Message.success("扫描开始")
		}
	}).catch((err) => {
		Message.error("扫描失败，原因：" + err)
	})
}

const onSelectScanType = async (scanType: string, taskId: string | number) => {
	Message.info(`开始${scanType}扫描，如有异常，请查看日志信息`)
	await invoke("run_scan_by_type", {
		eid: taskId,
		scan_type: scanType
	}).then((res: any) => {
		if (res) {
			Message.success("扫描开始")
		}
	}).catch((err) => {
		Message.error("扫描失败，原因：" + err)
	})
}

const filteredTaskList = computed(() => {
	if (!searchKeyword.value) {
		return task.list
	}
	return task.list.filter(item =>
		item.name.toLowerCase().includes(searchKeyword.value.toLowerCase())
	)
})

const onSearch = () => {
	pagination.current = 1;
	cardPagination.current = 1;  // 重置卡片分页
	RefreshData();
}

const onCardPageChange = (page: number) => {
	cardPagination.current = page;
};

const handleDataRefresh = () => {
	// 计算最大页数
	const maxCardPage = Math.ceil(filteredTaskList.value.length / cardPagination.pageSize);
	// 如果当前页大于最大页数，则调整到最大页数
	if (cardPagination.current > maxCardPage && maxCardPage > 0) {
		cardPagination.current = maxCardPage;
	}
};

const columns = computed(() => {
	return [
		{
			title: 'ID',
			dataIndex: 'id',
		},
		{
			title: t('asm.task-name'),
			dataIndex: 'name',
		},
		{
			title: t('asm.monitor-status'),
			dataIndex: 'monitor_status',
			slotName: "monitor_status",
		},
		{
			title: t('asm.running-status'),
			dataIndex: 'running_status',
			slotName: "running_status",
		},
		{
			title: t('asm.ent.root-domain'),
			dataIndex: 'domain_count',
			slotName: "domain_count",

		},
		{
			title: t('asm.next-runtime'),
			dataIndex: 'time`',
			slotName: "time",
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];

});

</script>



<style lang="less">
.clickable:hover {
	cursor: pointer;
}

.info-btn-2 {
	margin-left: 10px;
}

.info-btn {
	cursor: pointer;
}

.task-card {
	height: 100%;
	transition: transform 0.2s ease, box-shadow 0.3s ease;
	border-radius: 8px;
	overflow: hidden;
	border: 1px solid var(--color-border-2);
	box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);

	&:hover {
		transform: translateY(-2px);
		box-shadow: 0 6px 16px rgba(0, 0, 0, 0.12);
		border-color: var(--color-border-3);
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-bottom: 8px;

		.task-name {
			font-size: 15px;
			font-weight: 500;
			color: var(--color-text-1);
			max-width: 70%;
		}

		.truncate {
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}
	}

	.card-content {
		padding: 0;

		.card-switch-row {
			display: flex;
			justify-content: space-between;
			align-items: center;
			padding: 6px 0;
			margin-bottom: 5px;
			border-bottom: 1px dashed var(--color-border-2);

			.monitor-label {
				font-size: 13px;
				color: var(--color-text-3);
			}
		}

		.info-grid {
			display: flex;
			flex-direction: column;
			gap: 8px;
		}

		.info-row {
			display: flex;
			align-items: flex-start;
			gap: 8px;
			padding: 3px 0;

			&:hover .highlight {
				color: var(--color-primary);
			}

			.info-icon-wrap {
				display: flex;
				justify-content: center;
				align-items: center;
				width: 24px;
				height: 24px;
				background-color: var(--color-fill-2);
				border-radius: 4px;

				.info-icon {
					font-size: 14px;
					color: var(--color-text-2);
				}
			}

			.info-content {
				flex: 1;
				min-width: 0;

				.info-label {
					font-size: 12px;
					color: var(--color-text-3);
				}

				.info-value {
					font-size: 13px;
					color: var(--color-text-1);
					white-space: nowrap;
					overflow: hidden;
					text-overflow: ellipsis;

					&.highlight {
						color: var(--color-primary);
						cursor: pointer;
					}
				}
			}
		}
	}

	.card-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;

		.arco-btn {
			padding: 0 8px;
			height: 24px;
			font-size: 12px;
		}
	}

	.clickable {
		cursor: pointer;
	}
}

.arco-pagination-simple {
	margin-top: 8px;

	.arco-pagination-item {
		min-width: 28px;
		height: 28px;
		line-height: 28px;
	}

	.arco-pagination-jumper {
		height: 28px;
		margin-left: 8px;

		.arco-pagination-jumper-input {
			height: 24px;
			min-width: 48px;
		}
	}
}

.edit-icon-wrap {
	cursor: pointer;
}
</style>