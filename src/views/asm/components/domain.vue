<template>
	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<a-radio value="AAAA">{{ $t('asm.domain.a') }}</a-radio>
					<a-radio value="CNAME">{{ $t('asm.domain.cname') }}</a-radio>
					<a-radio value="NS">{{ $t('asm.domain.ns') }}</a-radio>
					<a-radio value="MX">{{ $t('asm.domain.mx') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col :span="13">
						<a-space>
							<a-button style="width: 95px;" type="primary" @click="onExport">{{ $t('asm.export')
								}}</a-button>
							<a-button style="width: 95px;" type="primary" @click="onAddDomain">{{
								$t('asm.root-domain.add-domain')
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
					<a-option value="subdomain">域名后缀</a-option>
					<a-option value="A">AAAA</a-option>
					<a-option value="CNAME">CNAME</a-option>
					<a-option value="NS">NS</a-option>
					<a-option value="MX">MX</a-option>
				</a-select>

			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-table :columns="all_columns" :data="domains.list" :pagination="pagination" size='small' :scroll="scroll"
			@page-change="onPageChange" @page-size-change="onPageSizeChange" :bordered="false" row-key="id">


			<template #operation="{ record }">
				<a-space>
					<a-popconfirm content="确认删除么?" @ok="onDel(record.id)">
						<a-link size="small" type="primary" status="danger">{{
							$t('asm.del-enterprise') }}</a-link>
					</a-popconfirm>
				</a-space>
			</template>

			<template #time="{ record }">
				<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
				<template v-else>创建时间:--</template>
				<br />
				<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
				<template v-else>更新时间:--</template>
			</template>

		</a-table>

	</a-space>


</template>

<script setup lang="ts">
import { Pagination } from '@/types/global';
import { formatDateTime } from '@/utils/format';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { Domain } from './types';
import { computed, reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import  {useRoute} from 'vue-router'
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


const scroll = {
  y: 600
}


const search_key = ref('search')
const route = useRoute()
const add_visible = ref(false)
const domains: { list: Domain[] } = reactive({ list: [] })
const asset_status = ref('valid')
const filterValue = ref('subdomain')
const rdtype = ref('all')

async function RefreshData() {
	console.log({ page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value ,query:search_key.value})
	await invoke("get_domains", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value ,query:search_key.value}).then((res: any) => {
		if (res) {
			domains.list = res.domain_list
			pagination.total = res.total
		}
	}).catch((error) => {
		console.error(error)
	})


}

const onRDTypeChange = async (value: string | number | boolean) => {
	rdtype.value = value as string
	RefreshData()
}



const onDel = async (eid: string) => {
	let res: any = await invoke("del_enterprise_by_id", { eid: eid });
	if (res) {
		Message.success("删除成功")
	} else {
		Message.success("删除失败")
	}
}



const onPageChange = (_page: number) => {
	pagination.current = _page;
	RefreshData()

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


onMounted(async () => {
	console.log('route.query.domain:',route.query.domain)
	if(route.query.domain && route.query.domain  !== undefined){
		search_key.value = route.query.domain as string | "all"
	}else{
		search_key.value = ''
	}
	
	await RefreshData()

})


const all_columns = computed(() => {
	return [
		{
			title: t('asm.domain.table'),
			dataIndex: 'domain',
		},
		{
			title: t('asm.domain.a'),
			dataIndex: 'aaa',
		},
		{
			title: t('asm.domain.cname'),
			dataIndex: 'cname',
		},
		{
			title: t('asm.domain.ns'),
			dataIndex: 'ns',
		},
		{
			title: t('asm.domain.mx'),
			dataIndex: 'mx',
		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: "time",
			width:210
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
			width:100
		},
	];

});
</script>