<template>

	<a-space direction="vertical" fill >
		<a-row justify="space-between" >
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.ip.all') }}</a-radio>
					<a-radio value="as_name">{{ $t('asm.ip.as_name') }}</a-radio>
					<a-radio value="provider">{{ $t('asm.ip.provider') }}</a-radio>
					<a-radio value="ip_range">{{ $t('asm.ip.ip_range') }}</a-radio>
					<a-radio value="location">{{ $t('asm.ip.location') }}</a-radio>
					<a-radio value="tag">{{ $t('asm.ip.tag') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col :span="13">
						<a-space>
							<a-button type="primary" @click="onExport">{{ $t('asm.export') }}</a-button>
							<a-button type="primary" @click="onAddDomain">{{ $t('asm.ip.add-ip')
								}}</a-button>
							<a-select v-model="asset_status">
								<a-option value="valid">现存资产</a-option>
								<a-option value="invalid">历史资产</a-option>
								<a-option value="ignored">白名单</a-option>
							</a-select>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="ip">{{ $t('asm.ip.ip') }}</a-option>
					<a-option value="as_name">{{ $t('asm.ip.as_name') }}</a-option>
					<a-option value="provider">{{ $t('asm.ip.provider') }}</a-option>
					<a-option value="ip_range">{{ $t('asm.ip.ip_range') }}</a-option>
					<a-option value="location">{{ $t('asm.ip.location') }}</a-option>
					<a-option value="tag">{{ $t('asm.ip.tag') }}</a-option>

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
		<a-table v-if="rdtype === 'as_name'" :columns="as_name_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'provider'" :columns="provider_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'ip_range'" :columns="ip_range_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'location'" :columns="location_columns" :data="domains.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'tag'" :columns="tag_columns" :data="domains.list" :pagination="pagination" size='small'
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

const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 10,
	showTotal: true,
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

const add_visible = ref(false)
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
			res = await invoke("get_icp_enterprise_list", { page: pagination.current, pagesize: pagination.pageSize, group_by: 'all', status: asset_status.value, filter: [] });
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

const onAddDomain = async () => {
	add_visible.value = true
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
			title: t('asm.ip.ip'),
			dataIndex: 'ip',
		},
		{
			title: t('asm.ip.domain'),
			dataIndex: 'domain',
		},
		{
			title: t('asm.ip.port'),  //端口数
			dataIndex: 'port',
		},
		{
			title: t('asm.ip.as_name'),
			dataIndex: 'as_name',
		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
		},
		{
			title: t('asm.ip.tags'),
			dataIndex: 'tags',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});


const as_name_columns = computed(() => {
	return [
		{
			title: t('asm.ip.as_name'),
			dataIndex: 'asn',
		},
		{
			title: t('asm.ip.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const provider_columns = computed(() => {
	return [
		{
			title: t('asm.ip.provider'),
			dataIndex: 'provider',
		},
		{
			title: t('asm.ip.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const ip_range_columns = computed(() => {
	return [
		{
			title: t('asm.ip.ip_range'),
			dataIndex: 'ip_range',
		},
		{
			title: t('asm.ip.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const location_columns = computed(() => {
	return [
		{
			title: t('asm.ip.location'),
			dataIndex: 'location',
		},
		{
			title: t('asm.ip.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});

const tag_columns = computed(() => {
	return [
		{
			title: t('asm.ip.tag'),
			dataIndex: 'tag',
		},
		{
			title: t('asm.ip.count'),
			dataIndex: 'count',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});


</script>