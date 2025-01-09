<template>

	<a-space direction="vertical" fill style="">
		<a-row justify="space-between" >
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<a-radio value="sub_category">{{ $t('asm.component.sub_category') }}</a-radio>
					<a-radio value="name">{{ $t('asm.component.name') }}</a-radio>

				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col :span="9">
						<a-space>
							<a-button style="width: 100px;" type="primary" @click="onExport">{{ $t('asm.export') }}</a-button>
							<a-select v-model="asset_status">
								<a-option value="valid">存活组件</a-option>
								<a-option value="invalid">非存活组件</a-option>
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
					<a-option value="sub_category">{{ $t('asm.component.sub_category') }}</a-option>
					<a-option value="name">{{ $t('asm.component.name') }}</a-option>
				</a-select>

			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="domains.list" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'sub_category'" :columns="sub_category_columns" :data="domains.list"
			:pagination="pagination" size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange"
			:bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'name'" :columns="name_columns" :data="domains.list" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
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

const domains: { list: Domain[] } = reactive({ list: [] })
const asset_status = ref('valid')
const filterValue = ref('name')
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
			title: t('asm.component.name'),
			dataIndex: 'name',
		},
		{
			title: t('asm.component.type'),
			dataIndex: 'type',
		},
		{
			title: t('asm.component.url'),  //端口数
			dataIndex: 'url',
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


const sub_category_columns = computed(() => {
	return [
		{
			title: t('asm.component.sub_category'),
			dataIndex: 'sub_category',
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

const name_columns = computed(() => {
	return [
		{
			title: t('asm.component.name'),
			dataIndex: 'name',
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





</script>