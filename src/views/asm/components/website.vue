<template>

	<a-space direction="vertical" fill >
		<a-row justify="space-between" >
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<a-radio value="status_code">{{ $t('asm.website.status_code') }}</a-radio>
					<a-radio value="render_title">{{ $t('asm.website.render_title') }}</a-radio>
					<a-radio value="server">{{ $t('asm.website.server') }}</a-radio>
					<a-radio value="category_key">{{ $t('asm.website.category_key') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col :span="9">
						<a-space>
							<a-button style="width: 100px;" type="primary" @click="onExport">{{ $t('asm.export') }}</a-button>
							<a-select v-model="asset_status">
								<a-option value="valid">存活网站</a-option>
								<a-option value="invalid">非存活网站</a-option>
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
					<a-option value="status_code">{{ $t('asm.website.status_code') }}</a-option>
					<a-option value="render_title">{{ $t('asm.website.render_title') }}</a-option>
					<a-option value="server">{{ $t('asm.website.server') }}</a-option>
					<a-option value="category_key">{{ $t('asm.website.category_key') }}</a-option>
				</a-select>
			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="website.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
			<template #time="{ record }">
				<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
				<template v-else>创建时间:--</template>
				<br />
				<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
				<template v-else>更新时间:--</template>
			</template>
			<template #title="{ record }">
				<a-link  :href="record.url" target="_blank">{{ record.title }}</a-link>
			</template>
			<template #screenshot="{ record }">
				<a-image width="50" :src="record.screenshot" alt="Base64 Image"  v-if="record.screenshot.length > 30" />
			</template>

		</a-table>
		<a-table v-if="rdtype === 'status_code'" :columns="status_code_columns" :data="website.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'render_title'" :columns="render_title_columns" :data="website.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'server'" :columns="server_columns" :data="website.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'category_key'" :columns="category_key_columns" :data="website.list" :pagination="pagination" size='small'
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">
		</a-table>

	</a-space>


	<!-- <template #time="{ record }">
		<span> {{ formatDateTime(record.created_at) }}</span>
	</template> -->

</template>

<script setup lang="ts">
import { Pagination } from '@/types/global';
import { formatDateTime } from '@/utils/format';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { WebSite } from './types';
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

const website: { list: WebSite[] } = reactive({ list: [] })
const asset_status = ref('valid')
const filterValue = ref('status_code')
const search_key = ref('')
const rdtype = ref('all')

async function RefreshData() {
	let res: any = await invoke("get_websites", {  page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value ,query:search_key.value });
	if (res) {
		website.list = res.list
		pagination.total = res.total

		website.list.forEach((item) =>{
			item.screenshot ='data:image/png;base64,'+ item.screenshot
		})
	}
}


onMounted(() => {
	RefreshData()
})


const onRDTypeChange = async (value: string | number | boolean) => {
	rdtype.value = value as string
	RefreshData()

}



const onPageChange = (_page: number) => {
	pagination.current = _page;
	RefreshData()
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
			title: t('asm.website.title'),
			dataIndex: 'title',
			slotName: 'title',
		},
		{
			title: t('asm.website.server'),  
			dataIndex: 'service',
		},
		{
			title: t('asm.website.url'),
			dataIndex: 'url',
		},
		{
			title: t('asm.website.screenshot'),
			dataIndex: 'screenshot',
			slotName: "screenshot",
		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: 'time',
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];
});


const status_code_columns = computed(() => {
	return [
		{
			title: t('asm.website.status_code'),
			dataIndex: 'status_code',
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

const render_title_columns = computed(() => {
	return [
		{
			title: t('asm.website.render_title'),
			dataIndex: 'render_title',
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

const server_columns = computed(() => {
	return [
		{
			title: t('asm.website.server'),
			dataIndex: 'server',
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

const category_key_columns = computed(() => {
	return [
		{
			title: t('asm.website.category_key'),
			dataIndex: 'category_key',
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

<style lang="less" scoped>

.screenshot-image {
  width: 100px; /* 设置宽度 */
  height: auto; /* 高度自动，保持纵横比 */
}

</style>