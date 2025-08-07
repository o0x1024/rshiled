<template>

	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" size="small" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<a-radio value="url">{{ $t('asm.website.url') }}</a-radio>
					<a-radio value="status_code">{{ $t('asm.website.status_code') }}</a-radio>
					<a-radio value="render_title">{{ $t('asm.website.render_title') }}</a-radio>
					<a-radio value="category_key">{{ $t('asm.website.category_key') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="95px">
						<a-space>
							<a-button style="width: 100px;" type="primary" size="small" @click="onExport">{{ $t('asm.export')
							}}</a-button>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="task_id">{{ $t('asm.website.task_id') }}</a-option>
					<a-option value="status_code">{{ $t('asm.website.status_code') }}</a-option>
					<a-option value="title">{{ $t('asm.website.render_title') }}</a-option>
				</a-select>
			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table v-if="rdtype === 'all'" :columns="all_columns" :data="website.list" :pagination="pagination"
			size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false"
			row-key="id">
			<template #time="{ record }">
				<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
				<template v-else>创建时间:--</template>
				<br />
				<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
				<template v-else>更新时间:--</template>
			</template>
			<template #title="{ record }">
				<a-link :href="record.url" target="_blank">{{ record.title }}</a-link>
			</template>
			<template #screenshot="{ record }">
				<a-image width="50" :src="record.screenshot" alt="Base64 Image" v-if="record.screenshot.length > 30" />
			</template>
			<template #status_code="{ record }">
				{{ record.status_code }}
			</template>

			<template #headers="{ record }">
				<template v-if="record.headers != ''">
					<a-link @click="onViewHeader(record.headers)">响应头</a-link>
				</template>
			</template>

			<template #ssl_info="{ record }">
				<template v-if="record.ssl_info != ''">
					<a-link @click="onViewSslInfo(record.ssl_info)">证书信息</a-link>
				</template>
			</template>

			<template #operation="{ record }">
				<a-dropdown>
					<div class="clickable"><icon-more /></div>
					<template #content>
						<a-doption>
							<template #icon>
								<icon-search />
							</template>
							<template #default>{{ $t('asm.task.run') }}</template>
						</a-doption>
						<a-doption @click="onDelWebsite(record.id)">
							<template #icon>
								<icon-delete />
							</template>
							<template #default>{{ $t('asm.del-task') }}</template>
						</a-doption>
					</template>
				</a-dropdown>
			</template>

		</a-table>
		<a-table v-if="rdtype === 'status_code'" :columns="status_code_columns" :data="website.list"
			:pagination="pagination" size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange"
			:bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'render_title'" :columns="render_title_columns" :data="website.list"
			:pagination="pagination" size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange"
			:bordered="false" row-key="id">
		</a-table>
		<a-table v-if="rdtype === 'category_key'" :columns="category_key_columns" :data="website.list"
			:pagination="pagination" size='small' @page-change="onPageChange" @page-size-change="onPageSizeChange"
			:bordered="false" row-key="id">
		</a-table>

	</a-space>



	<a-drawer :width="540"  :visible="visible" @ok="handleOk" @cancel="handleCancel" unmountOnClose>
		<div>
			<span style="white-space: pre-line;">{{ drawer_content }}</span>
		</div>
	</a-drawer>

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

defineOptions({
	name: 'asm-website',
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

const website: { list: WebSite[] } = reactive({ list: [] })
const filterValue = ref('task_id')
const search_key = ref('')
const drawer_content = ref('')
const rdtype = ref('all')
const visible = ref(false)

async function RefreshData() {
	let res: any = await invoke("get_websites", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, filter: filterValue.value, query: search_key.value });
	if (res) {
		website.list = res.list
		pagination.total = res.total

		website.list.forEach((item) => {
			item.screenshot = 'data:image/png;base64,' + item.screenshot
		})
	}
}

const onViewSslInfo = (content:string) => {
	visible.value = true;
	drawer_content.value = content;
}

const onDelWebsite = async (id:string) => {
	let res: any = await invoke("del_website_by_id", {id:id});
	if (res) {
		Message.success("删除成功")
	} else {
		Message.success("删除失败")
	}
}

const onViewHeader = (content:string) => {
	visible.value = true;
	drawer_content.value = content;
}

const handleOk = () => {
	visible.value = false;
};
const handleCancel = () => {
	visible.value = false;
}

import { useRoute } from 'vue-router';
const route = useRoute();

onMounted(async () => {
	if(route.query.id && route.query.id  !== undefined){
			search_key.value = route.query.id as string | ""
		}else{
			search_key.value = ''
		}
		
	await RefreshData()
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
			// ellipsis: true,
			// tooltip: true,
			width: 200,
		},
		{
			title: t('asm.website.url'),
			dataIndex: 'url',
			ellipsis: true,
			tooltip: true,
			width: 250,
		},
		{
			title: t('asm.website.status_code'),
			dataIndex: 'status_code',
			slotName: "status_code",
		},
		{
			title: t('asm.website.headers'),
			dataIndex: 'headers',
			slotName: "headers",
		},
		{
			title: t('asm.website.screenshot'),
			dataIndex: 'screenshot',
			slotName: "screenshot",
		},
		// {
		// 	title: t('asm.website.ssl_info'),
		// 	dataIndex: 'ssl_info',
		// 	slotName: "ssl_info",
		// },
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: 'time',
			width: 220,
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
			width: 100,

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
	width: 100px;
	/* 设置宽度 */
	height: auto;
	/* 高度自动，保持纵横比 */
}
.clickable{
	cursor: pointer;
}
</style>