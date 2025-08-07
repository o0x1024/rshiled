<template>

	<a-space direction="vertical" fill >
		<a-row justify="space-between" >
			<a-col :span="12">
				<a-radio-group type="button" size="small" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.port.all') }}</a-radio>
					<a-radio value="component">{{ $t('asm.port.component') }}</a-radio>
					<a-radio value="service_name">{{ $t('asm.port.service_name') }}</a-radio>
					<a-radio value="port_no">{{ $t('asm.port.port_no') }}</a-radio>
					<a-radio value="service_type">{{ $t('asm.port.service_type') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="95px">
						<a-space>
							<a-button style="width: 95px;" size="small" type="primary" @click="onExport">{{ $t('asm.export') }}</a-button>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="ip">{{ $t('asm.port.ip') }}</a-option>
					<a-option value="component">{{ $t('asm.port.component') }}</a-option>
					<a-option value="service_name">{{ $t('asm.port.service_name') }}</a-option>
					<a-option value="port_no">{{ $t('asm.port.port_no') }}</a-option>
					<a-option value="service_type">{{ $t('asm.port.service_type') }}</a-option>
				</a-select>

			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'component'" :columns="component_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'service_name'" :columns="service_name_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'port_no'" :columns="port_no_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'service_type'" :columns="service_type_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>

	</a-space>


	<!-- <template #time="{ record }">
		<span> {{ formatDateTime(record.created_at) }}</span>
	</template> -->

</template>

<script setup lang="ts">
import { Pagination } from '@/types/global';
// import { formatDateTime } from '@/utils/format';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { Domain } from 'domain';
import { computed, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
const { t } = useI18n();

defineOptions({
	name: 'asm-port',
})

const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 10,
	showTotal: true,
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

const domains: { list: Domain[] } = reactive({ list: [] })
const asset_status = ref('valid')
const filterValue = ref('ip')
const search_key = ref('')
const rdtype = ref('all')

async function RefreshData() {
	let res: any = await invoke("get_domain_list", { page: pagination.current, pagesize: pagination.pageSize });
	if (res.code == 20000) {
		domains.list = res.data
	}
}

const onRDTypeChange = async (value: string | number | boolean) => {
	switch (value) {
		case 'all':
			let res: any = await invoke("get_root_domain_list", { page: pagination.current, pagesize: pagination.pageSize, group_by: 'all', status: asset_status.value, filter: [] });
			if (res.code == 20000) {
				domains.list = res.data
			}
			break
		case 'icp':
			res = await invoke("get_icp_task_list", { page: pagination.current, pagesize: pagination.pageSize, group_by: 'all', status: asset_status.value, filter: [] });
			if (res.code == 20000) {
				domains.list = res.data
			}
			break
	}
}



const onPageChange = (_page: number) => {
	pagination.current = _page;

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
	return [
		{
			title: t('asm.port.ip'),
			dataIndex: 'ip',
		},
		{
			title: t('asm.port.port_no'),
			dataIndex: 'port_no',
		},
		{
			title: t('asm.port.service_name'),  //端口数
			dataIndex: 'service',
		},
		{
			title: t('asm.port.component'),
			dataIndex: 'component',
		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});


const component_columns = computed(() => {
	return [
		{
			title: t('asm.port.component'),
			dataIndex: 'component',
		},
		{
			title: t('asm.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const service_name_columns = computed(() => {
	return [
		{
			title: t('asm.port.service_name'),
			dataIndex: 'service_name',
		},
		{
			title: t('asm.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const port_no_columns = computed(() => {
	return [
		{
			title: t('asm.port.port_no'),
			dataIndex: 'port_no',
		},
		{
			title: t('asm.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const service_type_columns = computed(() => {
	return [
		{
			title: t('asm.port.service_type'),
			dataIndex: 'service_type',
		},
		{
			title: t('asm.port.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});




</script>