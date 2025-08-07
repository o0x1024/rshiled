<template>

	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" size="small" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.ip.all') }}</a-radio>
					<a-radio value="atype">{{ $t('asm.api.atype') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-row>

			</a-row>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="95px">
						<a-space>
							<a-button size="small" :disabled="selectedKeys.length == 0" @click="onBulkProcess">
								批量处理
							</a-button>
							<a-button type="primary" size="small" @click="onExport">{{ $t('asm.export') }}</a-button>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="task_id">{{ $t('asm.api.task_id') }}</a-option>
					<a-option value="uri">{{ $t('asm.api.uri') }}</a-option>
					<a-option value="http_status">{{ $t('asm.api.http_status') }}</a-option>
					<a-option value="ufrom">{{ $t('asm.api.ufrom') }}</a-option>
				</a-select>

			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="apis.list" :pagination="pagination"
			:scroll="scroll" size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange"
			:bordered="false" row-key="id" :row-selection="rowSelection" v-model:selectedKeys="selectedKeys"
			@filter-change="onfilterChange">

			<template #len="{ record }">
				<a-space direction="vertical">
					<div v-if="record.get_body_length == record.post_body_length">
						<div v-if="record.get_body_length > record.get_body_length">
							<a-tag color="green" v-if="record.get_body_length > 0"> G:{{ record.get_body_length }}
							</a-tag>
						</div>
						<div v-else>
							<a-tag color="red" v-if="record.post_body_length > 0"> P:{{ record.post_body_length }} </a-tag>
						</div>
					</div>
					<div v-else>
						<a-tag color="blue" v-if="record.get_body_length > 0"> G:{{ record.get_body_length }} </a-tag>
						<a-tag color="red" v-if="record.post_body_length > 0"> P:{{ record.post_body_length }} </a-tag>
					</div>
				</a-space>
			</template>

			<template #http_status="{ record }">
				<a-tag color="green" v-if="record.http_status == 200">成功</a-tag>
				<a-tag color="gray" v-else>其它</a-tag>
			</template>
			<template #uri="{ record }">
				<a-link @click="onURIClick(record)">{{ record.uri }}</a-link>
			</template>
			<template #url="{ record }">
				<!-- <a-link @click="onURLClick(record)">{{ record.url }}</a-link> -->
				<a-link :href="record.url" @click="onURLClick(record.url)" target="_blank">{{ record.url }}</a-link>

			</template>
			<template #handle_status="{ record }">
				<a-tag v-if="record.handle_status == 0">未处理</a-tag>
				<a-tag v-else-if="record.handle_status == 1">已处理</a-tag>
				<a-tag v-else>未知</a-tag>
			</template>

			<template #time="{ record }">
				<template v-if="record.update_at != 0">{{ formatDateTime(record.update_at) }}</template>
				<template v-else>--</template>
			</template>

		</a-table>
		<a-table v-if="rdtype === 'atype'" :columns="type_columns" :data="apis.list" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
			<template #uri="{ record }">
				<a-link @click="onTypeURIClick(record.uri)">{{ record.uri }}</a-link>
			</template>
		</a-table>
	</a-space>

	<a-drawer v-model:visible="detail_visible" :title="$t('asm.ip.add-ip')" :width="800">
		<a-form :model="api">
			<a-form-item :label="$t('asm.api.uri')">
				<a-input v-model:model-value="api.uri" />
			</a-form-item>
			<a-form-item :label="$t('asm.api.url')">
				<a-input v-model:model-value="api.url" />
			</a-form-item>
			<a-form-item :label="$t('asm.api.status')">
				<a-tag color="green" v-if="api.status == 200">成功</a-tag>
				<a-tag color="red" v-else-if="api.status == 404">失败</a-tag>
				<a-tag color="gray" v-else>{{ api.status }}</a-tag>
			</a-form-item>
			<a-form-item :label="$t('asm.api.get_body_length')">
				<div v-if="api.get_body_length > 0"> {{ api.get_body_length }} </div>
				<div v-else> -- </div>
			</a-form-item>
			<a-form-item :label="$t('asm.api.post_body_length')">
				<div v-if="api.post_body_length > 0"> {{ api.post_body_length }} </div>
				<div v-else> -- </div>
			</a-form-item>
			<a-form-item :label="$t('asm.api.ufrom')">
				<a-input v-model:model-value="api.ufrom" />
			</a-form-item>

			<a-form-item :label="$t('asm.api.method')">
				<a-input v-model:model-value="api.method" />
			</a-form-item>
			<a-form-item :label="$t('asm.api.get_response')">
				<a-textarea v-model:model-value="api.get_response" auto-size />
			</a-form-item>
			<a-form-item :label="$t('asm.api.post_response')">
				<a-textarea v-model:model-value="api.post_response" auto-size />
			</a-form-item>
			<a-form-item :label="$t('asm.api.update_at')">
				<div v-if="api.update_at > 0"> {{ formatDateTime(api.update_at) }} </div>
				<div v-else> -- </div>
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
import { invoke } from '@tauri-apps/api/core';
import { formatDateTime } from '@/utils/format';
import { Message, TableColumnData, TableRowSelection } from '@arco-design/web-vue';
import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Apis } from './types';
const { t } = useI18n();
// import { useRouter } from 'vue-router'
// const router = useRouter()


defineOptions({
	name: 'asm-api',
})

const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 15,
	showTotal: true,
	pageSizeOptions: [10, 20, 30, 40, 50, 100, 500],
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

const detail_visible = ref(false)
const apis: { list: Apis[] } = reactive({ list: [] })
const api = reactive<Apis>({
	id: 0,
	task_id: 0,
	method: '',
	uri: '',
	url: '',
	get_response: '',
	post_response: '',
	ufrom: '',
	status: 0,
	get_body_length: 0,
	post_body_length: 0,
	update_at: 0,
	count: 0,
})

const filterValue = ref('uri')
const search_key = ref('')
const rdtype = ref('all')
const selectedKeys = ref([]);
const rowSelection: TableRowSelection = reactive({
	type: 'checkbox',
	showCheckedAll: true,
	onlyCurrent: false,
});
const process_visible = ref(false)
const radioKey = ref('1')


const handleCancel = async () => {
	process_visible.value = false
}



async function RefreshData() {
	await invoke("get_apis", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, filter: filterValue.value, query: search_key.value, handle_status: handle_statusfilter.value, http_status: http_statusfilter.value }).then((res: any) => {
		if (res) {
			apis.list = res.list
			pagination.total = res.total
		}
	}).catch((err: any) => {
		console.log(err);
	})
}

let handle_statusfilter = ref<string[]>(['0'])
let http_statusfilter = ref<string[]>(['200', "0"])

const onfilterChange = (dataIndex: string, filters: string[]) => {
	pagination.current = 1
	pagination.pageSize = 15
	if (dataIndex === "handle_status") {
		handle_statusfilter.value = filters
	} else if (dataIndex === "http_status") {
		http_statusfilter.value = filters
	}

	RefreshData()

}

const onBulkProcess = async (_status: any) => {
	// selectedKeys.value.forEach((v) => {
	//     chosenKeys.push(v)
	// })
	process_visible.value = true
}



const onProcessChange = async (_status: any) => {

	await invoke("process_apis", { handle_status: Number(radioKey.value), api_ids: selectedKeys.value }).then((res: any) => {
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


const onTypeURIClick = async (uri: any) => {
	rdtype.value = 'all'
	search_key.value = uri
	await RefreshData();
}

const onURIClick = async (record: any) => {
	detail_visible.value = true;
	Object.assign(api, record)

	await RefreshData();
}


const onURLClick = async (link: any) => {
	await invoke('open_url', { url: link });

}




const scroll = {
	y: 600
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



const onRDTypeChange = async (value: string | number | boolean) => {
	switch (value) {
		case 'all':
			let res: any = await invoke("get_apis", { page: pagination.current, pagesize: pagination.pageSize, dtype: 'all', filter: filterValue.value, query: search_key.value, handle_status: handle_statusfilter.value, http_status: http_statusfilter.value });
			if (res.list.length > 0) {
				apis.list = res.list
				pagination.total = res.total
			}
			break
		case 'atype':
			let resx: any = await invoke("get_apis", { page: pagination.current, pagesize: pagination.pageSize, dtype: 'atype', filter: filterValue.value, query: search_key.value, handle_status: [], http_status: [] });
			if (resx.list.length > 0) {
				apis.list = resx.list
				pagination.total = resx.total
			}
			break
	}
}

const onPageChange = async (_page: number) => {
	pagination.current = _page;
	await RefreshData()
};

const onPageSizeChange = (_pagesize: number) => {
	pagination.pageSize = _pagesize
	RefreshData()
}

const onExport = async () => {
	let res: any = await invoke("export_icpdomain", {});
	if (res.code == 20000) {
		Message.success("导出成功")
	} else {
		Message.success("导出失败")
	}
}


const all_columns = computed(() => {
	let columns: Array<TableColumnData> = [
		{
			title: t('asm.api.uri'),
			dataIndex: 'uri',
			width: 280,
			slotName: 'uri'
		},
		{
			title: t('asm.api.url'),
			slotName: 'url',
			dataIndex: 'url',
			width: 300
		},
		{
			title: t('asm.api.http_status'),
			slotName: 'http_status',
			dataIndex: 'http_status',
			filterable: {
				filters: [
					{ text: '成功', value: '200' },
					{ text: '其它', value: '0' },
				],
				filter: (_value: any, _record: any) => true,
				multiple: true,
				defaultFilteredValue: ['200', '0'],
			},
			width: 120
		},
		{
			title: t('asm.api.handle_status'),
			slotName: 'handle_status',
			dataIndex: 'handle_status',
			filterable: {
				filters: [
					{ text: '未处理', value: '0' },
					{ text: '已处理', value: '1' },
				],
				filter: (_value: any, _record: any) => true,
				multiple: true,
				defaultFilteredValue: ['0'],
			},
			width: 120
		},
		{
			title: t('asm.api.len'),  //API返回的长度
			dataIndex: 'len',
			slotName: 'len',
			width: 120
		},
		// {
		// 	title: t('asm.api.from'),
		// 	dataIndex: 'ufrom',
		// 	slotName: 'ufrom',
		// 	width: 400
		// },
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: 'time',
			width: 200
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
			width: 80,
			fixed: 'right'
		},
	];
	return columns;
});




const type_columns = computed(() => {
	return [
		{
			title: t('asm.api.uri'),
			slotName: 'uri',
			dataIndex: 'uri',
			width: 500
		},

		{
			title: t('asm.api.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});





</script>