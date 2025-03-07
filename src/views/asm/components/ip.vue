<template>

	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.ip.all') }}</a-radio>
					<a-radio value="provider">{{ $t('asm.ip.provider') }}</a-radio>
					<a-radio value="location">{{ $t('asm.ip.location') }}</a-radio>
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
					<a-option value="provider">{{ $t('asm.ip.provider') }}</a-option>
					<a-option value="location">{{ $t('asm.ip.location') }}</a-option>
				</a-select>

			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="ips.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">

			<template #time="{ record }">
				<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
				<template v-else>创建时间:--</template>
				<br />
				<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
				<template v-else>更新时间:--</template>
			</template>

			<template #port_count="{ record }">
				<a-link @click="toPort(record.ip_addr)">{{ record.port_count }}</a-link>

			</template>
		</a-table>
		<a-table v-if="rdtype === 'provider'" :columns="provider_columns" :data="ips.list" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
		</a-table>

		<a-table v-if="rdtype === 'location'" :columns="location_columns" :data="ips.list" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
		</a-table>

	</a-space>

</template>

<script setup lang="ts">
import { Pagination } from '@/types/global';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { formatDateTime } from '@/utils/format';

import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { IPs } from './types';
const { t } = useI18n();
import { useRouter } from 'vue-router'
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

const add_visible = ref(false)
const ips: { list: IPs[] } = reactive({ list: [] })
const asset_status = ref('valid')
const filterValue = ref('ip')
const search_key = ref('')
const rdtype = ref('all')

async function RefreshData() {
	await invoke("get_ips", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, query: search_key.value }).then((res: any) => {
		if (res) {
			ips.list = res.ip_list
			pagination.total = res.total
		}
	}).catch((err: any) => {
		console.log(err);
	})
}


const toPort = async (ipaddr: string) => {
	router.push({
		name: 'asm-port', query: { ipaddr: ipaddr }
	})
}



onMounted(async () => {

	await RefreshData()

})


const onRDTypeChange = async (value: string | number | boolean) => {
	switch (value) {
		case 'all':
			let res: any = await invoke("get_ips", { page: pagination.current, pagesize: pagination.pageSize, group_by: 'all', status: asset_status.value, filter: [] });
			if (res.code == 20000) {
				ips.list = res.ip_list
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
			dataIndex: 'ip_addr',
		},
		{
			title: t('asm.ip.domain'),
			dataIndex: 'domain',
		},
		{
			title: t('asm.ip.port'),  //端口数
			dataIndex: 'port_count',
			slotName: 'port_count',

		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: 'time',
			width: 210

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



</script>