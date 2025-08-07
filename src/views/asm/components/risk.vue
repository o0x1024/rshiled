<template>
	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" size="small" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<a-radio value="risk_type">{{ $t('asm.risk.risk_type') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="60px">
						<a-space>
							<a-button size="small" :disabled="selectedKeys.length == 0" @click="onBulkProcess">
								批量处理
							</a-button>
							<a-button type="primary" size="small" @click="onExport">{{ $t('asm.risk.export')
								}}</a-button>

							<a-dropdown trigger="click">
								<a-button size="small">
									{{ $t('asm.risk.columns') }}
									<icon-down />
								</a-button>
								<template #content>
									<a-doption>
										<a-checkbox-group v-model="visibleColumns" :options="columnOptions" />
									</a-doption>
								</template>
							</a-dropdown>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="task_id">{{ $t('asm.risk.id') }}</a-option>
					<a-option value="name">{{ $t('asm.risk.name') }}</a-option>
					<a-option value="level">{{ $t('asm.risk.level') }}</a-option>
					<a-option value="status">{{ $t('asm.risk.status') }}</a-option>
				</a-select>
			</a-col>
			<a-col :span="21">
				<a-input-search v-if="filterValue === 'name'" placeholder="请输入待搜索的内容" @keyup.enter="RefreshData"
					v-model:model-value="search_key" @click="RefreshData" size="small" />
				<a-input-search v-if="filterValue === 'task_id'" placeholder="请输入待搜索的内容" @keyup.enter="RefreshData"
					v-model:model-value="search_key" @click="RefreshData" size="small" />
				<a-checkbox-group style="line-height: 30px;" v-if="filterValue === 'level'" size="small"
					:default-value="['1']">
					<a-checkbox value="critical">{{ $t('asm.critical') }}</a-checkbox>
					<a-checkbox value="high">{{ $t('asm.high') }}</a-checkbox>
					<a-checkbox value="medium">{{ $t('asm.medium') }}</a-checkbox>
					<a-checkbox value="low">{{ $t('asm.low') }}</a-checkbox>
				</a-checkbox-group>
				<a-checkbox-group style="line-height: 30px;" size="small" v-if="filterValue === 'status'"
					:default-value="['1']">
					<a-checkbox value="processed">{{ $t('asm.processed') }}</a-checkbox>
					<a-checkbox value="untreated">{{ $t('asm.untreated') }}</a-checkbox>
					<a-checkbox value="ignore">{{ $t('asm.ignore') }}</a-checkbox>
				</a-checkbox-group>
			</a-col>
		</a-row>


		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="risks.list" :pagination="pagination"
			@row-dblclick="onRowDblClick" size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange"
			:bordered="false" :scroll="scroll" row-key="id" :row-selection="rowSelection" @filter-change="onfilterChange"
			v-model:selectedKeys="selectedKeys">
			<template #risk_detail="{ record }">
				<a-space direction="vertical" style="font-size: 13px;">
					<span>{{ record.risk_desc }}</span>
					<span>{{ record.risk_detail }}</span>
				</a-space>
			</template>
			<template #update_at="{ record }">
				{{ formatDateTime(record.update_at) }}
			</template>

			<template #risk_name="{ record }">
				<a-link :href="record.ufrom" @click="onLinkClick(record)" target="_blank">{{ record.risk_name }}</a-link>
			</template>
			<template #risk_status="{ record }">
				<a-tag v-if="record.risk_status === 1" color="green">{{ $t('asm.processed') }}</a-tag>
				<a-tag v-if="record.risk_status === 0" color="red">{{ $t('asm.untreated') }}</a-tag>
				<a-tag v-if="record.risk_status === 2" color="blue">{{ $t('asm.ignore') }}</a-tag>
			</template>
			<template #ufrom="{ record }">
				<a-link :href="record.ufrom" @click="onFromClick(record.ufrom)"
					target="_blank">{{ record.ufrom }}</a-link>
			</template>
			<template #risk_level="{ record }">
				<a-tag v-if="record.risk_level === 'critical'" color="red">{{ $t('asm.critical') }}</a-tag>
				<a-tag v-if="record.risk_level === 'high'" color="orange">{{ $t('asm.high') }}</a-tag>
				<a-tag v-if="record.risk_level === 'medium'" color="blue">{{ $t('asm.medium') }}</a-tag>
				<a-tag v-if="record.risk_level === 'low'" color="green">{{ $t('asm.low') }}</a-tag>
			</template>
			<template #operation>
				<a-dropdown>
					<div class="clickable"><icon-more /></div>
					<template #content>
						<a-doption>
							<template #icon>
								<icon-edit />
							</template>
							<template #default>{{ $t('asm.risk.handle') }}</template>
						</a-doption>
						<a-doption>
							<template #icon>
								<icon-search />
							</template>
							<template #default>{{ $t('asm.task.run') }}</template>
						</a-doption>
						<a-doption>
							<template #icon>
								<icon-delete />
							</template>
							<template #default>{{ $t('asm.del-task') }}</template>
						</a-doption>
					</template>
				</a-dropdown>
			</template>
		</a-table>
		<a-table v-if="rdtype === 'risk_type'" :columns="name_columns" :data="risk_type" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
		</a-table>
	</a-space>

	<a-drawer v-model:visible="visible" :title="t('asm.risk.name')" :width="800" :height="400">
		<a-form :model="risk">
			<a-form-item :label="t('asm.risk.name')" :label-col="{ span: 4 }" :wrapper-col="{ span: 14 }">
				<a-input v-model:model-value="risk.risk_name" />
			</a-form-item>
			<a-form-item :label="t('asm.risk.level')" :label-col="{ span: 4 }" :wrapper-col="{ span: 14 }">
				<a-tag v-if="risk.risk_level === 'critical'" color="red">{{ $t('asm.critical') }}</a-tag>
				<a-tag v-if="risk.risk_level === 'high'" color="orange">{{ $t('asm.high') }}</a-tag>
				<a-tag v-if="risk.risk_level === 'medium'" color="blue">{{ $t('asm.medium') }}</a-tag>
				<a-tag v-if="risk.risk_level === 'low'" color="green">{{ $t('asm.low') }}</a-tag> </a-form-item>
			<a-form-item :label="t('asm.risk.status')" :label-col="{ span: 4 }" :wrapper-col="{ span: 14 }">
				<a-tag v-if="risk.risk_status === 1" color="green">{{ $t('asm.processed') }}</a-tag>
				<a-tag v-if="risk.risk_status === 0" color="red">{{ $t('asm.untreated') }}</a-tag>
				<a-tag v-if="risk.risk_status === 2" color="blue">{{ $t('asm.ignore') }}</a-tag>
			</a-form-item>
			<a-form-item :label="t('asm.risk.desc')" :label-col="{ span: 4 }" :wrapper-col="{ span: 14 }">
				<a-textarea v-model:model-value="risk.risk_desc" auto-size />
			</a-form-item>
			<a-form-item :label="t('asm.risk.detail')">
				<a-textarea v-model:model-value="risk.risk_detail" auto-size />
			</a-form-item>
			<a-form-item :label="t('asm.risk.ufrom')">
				<a-input v-model:model-value="risk.ufrom" auto-size />
			</a-form-item>
		</a-form>

	</a-drawer>


	<a-modal v-model:visible="process_visible" @ok="onProcessChange" @cancel="handleCancel">
		<a-tab-pane key="2" title="漏洞状态变更">
			<a-radio-group v-model:model-value="radioKey">
				<a-radio value="1">已处理</a-radio>
				<a-radio value="0">未处理</a-radio>
			</a-radio-group>
		</a-tab-pane>
	</a-modal>
</template>

<script setup lang="ts">
import { Pagination } from '@/types/global';
import { formatDateTime } from '@/utils/format';
import { Message, TableData, TableRowSelection } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { Risk } from '@/views/asm/components/types';
import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
const { t } = useI18n();

defineOptions({
	name: 'asm-risk',
})

const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 10,
	pageSizeOptions: [10, 20, 30, 40, 50, 100, 500],
	showTotal: true,
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

const risk = reactive<Risk>({
	id: 0,
	task_id: 0,
	risk_name: '',
	risk_type: '',
	risk_level: '',
	risk_desc: '',
	risk_detail: '',
	risk_status: 0,
	ufrom: '',
	update_at: 0,
})

const risk_type = ref([])

const selectedKeys = ref([]);
const rowSelection: TableRowSelection = reactive({
	type: 'checkbox',
	showCheckedAll: true,
	onlyCurrent: false,
});

const risks: { list: Risk[] } = reactive({ list: [] })
const filterValue = ref('task_id')
const search_key = ref('')
const rdtype = ref('all')
const visible = ref(false)
const process_visible = ref(false)
const radioKey = ref('1')
const risk_statusfilter = ref<string[]>(['0'])

// 添加列显示控制
const visibleColumns = ref(['risk_name', 'risk_level', 'risk_status', 'ufrom', 'update_at', 'operation']);

// 列选项
const columnOptions = computed(() => [
	{ label: t('asm.risk.name'), value: 'risk_name' },
	{ label: t('asm.risk.risk_level'), value: 'risk_level' },
	{ label: t('asm.risk.status'), value: 'risk_status' },
	{ label: t('asm.risk.ufrom'), value: 'ufrom' },
	{ label: t('asm.time'), value: 'update_at' },
	{ label: t('asm.operation'), value: 'operation' },
]);

const onRowDblClick = (record: TableData, _ev: Event) => {
	console.log(record)
	visible.value = true
	Object.assign(risk, record)
}

const onLinkClick = (record: Risk) => {
	window.open(record.ufrom, '_blank')
	visible.value = true
	Object.assign(risk, record)
}

const onBulkProcess = async (_status: any) => {
	// selectedKeys.value.forEach((v) => {
	//     chosenKeys.push(v)
	// })
	process_visible.value = true
}


const onProcessChange = async (_status: any) => {

	await invoke("process_risks", { risk_status: Number(radioKey.value), risk_ids: selectedKeys.value }).then((res: any) => {
		if (res) {
			Message.success("处理成功")
		}
	}).catch((err: any) => {
		console.log(err);
	})
	process_visible.value = false
	selectedKeys.value = []

	RefreshData()
}

const handleCancel = async () => {
	process_visible.value = false
}


const onFromClick = async (link: string) => {
	await invoke('open_url', { url: link });
}


const onfilterChange = (dataIndex: string, filters: string[]) => {
    pagination.current = 1
    pagination.pageSize = 10
    if (dataIndex === "risk_status") {
        risk_statusfilter.value = filters
    }

    RefreshData()

}


async function RefreshData() {
	let res: any = await invoke("get_risks", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, filter: filterValue.value, query: search_key.value,risk_status:risk_statusfilter.value });
	if (res) {
		risks.list = res.list
		pagination.total = res.total
	}
}

const scroll = {
	y: 550
}

const onRDTypeChange = async (value: string | number | boolean) => {
	switch (value) {
		case 'all':
			let res: any = await invoke("get_risks", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, filter: filterValue.value, query: search_key.value, risk_status: risk_statusfilter.value });
			if (res) {
				risks.list = res.list
				pagination.total = res.total
			}
			break
		case 'risk_type':
			let resx: any  = await invoke("get_risks", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, filter: filterValue.value, query: search_key.value, risk_status: ['0','1']	 });
			if (resx) {
				risk_type.value = resx.list
				pagination.total = resx.total
			}
			break
	}
}
import { useRoute } from 'vue-router';
const route = useRoute();

onMounted(async () => {
	if (route.query.id && route.query.id !== undefined) {
		filterValue.value = 'task_id'
		search_key.value = route.query.id as string | ""
	} else {
		search_key.value = ''
	}

	await RefreshData()
})

const onPageChange = (_page: number) => {
	pagination.current = _page;
	RefreshData()

};

const onPageSizeChange = (_pagesize: number) => {
	pagination.pageSize = _pagesize
	RefreshData()
}
const onExport = async () => {
	let res: any = await invoke("export_risks", {});
	if (res) {
		Message.success("导出成功")
	} else {
		Message.success("导出失败")
	}
}

// 修改列定义，添加显示控制
const all_columns = computed(() => {
	const columns = [
		{
			title: t('asm.risk.name'),
			slotName: 'risk_name',
			dataIndex: 'risk_name',
			width: 150,
		},
		{
			title: t('asm.risk.risk_level'),
			dataIndex: 'risk_level',
			slotName: 'risk_level',
			width: 100,
		},
		{
			title: t('asm.risk.status'),
			dataIndex: 'risk_status',
			slotName: 'risk_status',
			width: 100,
			filterable: {
				filters: [
					{ text: '未处理', value: '0' },
					{ text: '已处理', value: '1' },
				],
				filter: (_value: any, _record: any) => true,
				multiple: true,
				defaultFilteredValue: ['0'],
			},
		},
		{
			title: t('asm.risk.ufrom'),
			dataIndex: 'ufrom',
			slotName: 'ufrom',
		},
		{
			title: t('asm.time'),
			dataIndex: 'update_at',
			slotName: 'update_at',
			width: 200,
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
			width: 100,
		},
	];

	// 根据visibleColumns过滤列
	return columns.filter(col => visibleColumns.value.includes(col.dataIndex || col.slotName || ''));
});


const name_columns = computed(() => {
	return [
		{
			title: t('asm.risk.risk_type'),
			dataIndex: 'risk_type',
		},
		{
			title: t('asm.risk.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});





</script>