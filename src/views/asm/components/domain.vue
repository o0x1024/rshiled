<template>
	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" size="small" v-model:model-value="rdtype">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<a-radio value="AAAA">{{ $t('asm.domain.a') }}</a-radio>
					<a-radio value="CNAME">{{ $t('asm.domain.cname') }}</a-radio>
					<a-radio value="NS">{{ $t('asm.domain.ns') }}</a-radio>
					<a-radio value="MX">{{ $t('asm.domain.mx') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="95px">
						<a-space>
							<a-button style="width: 95px;" size="small"  type="primary" @click="onAddDomain">{{
								$t('asm.root-domain.add-domain')
							}}</a-button>
							<a-button style="width: 95px;" size="small"  type="primary" @click="onExport">{{ $t('asm.export')
								}}</a-button>

							<!-- <a-select v-model="asset_status" size="small" >
								<a-option value="valid">现存资产</a-option>
								<a-option value="invalid">历史资产</a-option>
								<a-option value="ignored">白名单</a-option>
							</a-select> -->
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="task_id">{{ $t('asm.task-id') }}</a-option>
					<a-option value="domain">{{ $t('asm.domain.domain') }}</a-option>
					<a-option value="A">{{ $t('asm.domain.a') }}</a-option>
					<a-option value="CNAME">{{ $t('asm.domain.cname') }}</a-option>
					<a-option value="NS">{{ $t('asm.domain.ns') }}</a-option>
					<a-option value="MX">{{ $t('asm.domain.mx') }}</a-option>
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
							$t('asm.del-task') }}</a-link>
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

	
	<a-modal v-model:visible="add_visible" title-align="start" @ok="handleOk" @cancel="handleCancel">
		<template #title>
			{{ $t('asm.root-domain.add-asset') }}
		</template>
		<a-space direction="vertical">
			<a-select :style="{ width: '200px' }" placeholder="域名信息" v-model:model-value="task_id"
				@search="handlerSearch" allow-search @change="handlerOpsChange">
				<template v-for="item in ents.list" :key="item.id">
					<a-option :value="item.id">{{ item.name }}</a-option>
				</template>
			</a-select>
			<a-textarea :style="{ width: '320px' }" v-model:model-value="textare_domains"
			placeholder="请输入域名，每行一个" allow-clear auto-size />
		</a-space>
	</a-modal>

</template>

<script setup lang="ts">
import { Pagination } from '@/types/global';
import { formatDateTime } from '@/utils/format';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { Domain, Task } from './types';
import { computed, reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import  {useRoute} from 'vue-router'
const { t } = useI18n();


defineOptions({
	name: 'asm-domain',
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


const scroll = {
  y: 600
}


const search_key = ref('')
const route = useRoute()
const add_visible = ref(false)
const task_id = ref('')
const ents: { list: Task[] } = reactive({ list: [] });
const domains: { list: Domain[] } = reactive({ list: [] })
const filterValue = ref('domain')
const rdtype = ref('all')
const textare_domains = ref('')



async function RefreshData() {
	console.log({ page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value ,query:search_key.value})
	await invoke("get_domains", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value ,filter:filterValue.value,query:search_key.value}).then((res: any) => {
		if (res) {
			domains.list = res.list
			pagination.total = res.total
		}
	}).catch((error) => {
		console.error(error)
	})

}



const handlerSearch = async () => {
	await invoke("get_task_list").then((res: any) => {
		if (res) {
			ents.list = res.list;
		}
	});

}


function handleCancel() {
	add_visible.value = false
}

const handlerOpsChange = (value: string | number | boolean | Record<string, any> | (string | number | boolean | Record<string, any>)[]) => {
	task_id.value = value as string
}

async function handleOk() {
	//清空textarea


	console.log('textare_domains',textare_domains.value)
	const dataArray = textare_domains.value.split('\n');
	
	const tid = Number(task_id.value)
	await invoke("add_domain", { task_id: tid, domains: dataArray }).then((res: any) => {
		if (res) {
			Message.success(res)
		}
		RefreshData()
	});
	textare_domains.value = ''
}


const onRDTypeChange = async (value: string | number | boolean) => {
	rdtype.value = value as string
	RefreshData()
}



const onDel = async (id: string) => {
	let res: any = await invoke("delete_domain_by_id", { id: id });
	if (res) {
		Message.success("删除成功")
	} else {
		Message.success("删除失败")
	}
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

const onAddDomain = async () => {
	add_visible.value = true
	await invoke("get_task_list", { page: pagination.current, pagesize: pagination.pageSize }).then((res: any) => {
		if (res) {
			ents.list = res.list
		}
	}).catch(error => console.error(error))

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
		filterValue.value = 'domain'
		search_key.value = route.query.domain as string | ""
	}
	

	if(route.query.id && route.query.id  !== undefined){
		filterValue.value = 'task_id'
		search_key.value = route.query.id as string | ""
	}

	await RefreshData()

})


const all_columns = computed(() => {
	return [
		{
			title: t('asm.domain.table'),
			dataIndex: 'domain',
			width: 350,
		},
		{
			title: t('asm.domain.a'),
			dataIndex: 'aaa',
		},
		{
			title: t('asm.domain.cname'),
			dataIndex: 'cname',
		},
		// {
		// 	title: t('asm.domain.ns'),
		// 	dataIndex: 'ns',
		// },
		// {
		// 	title: t('asm.domain.mx'),
		// 	dataIndex: 'mx',
		// },
		{
			title: t('asm.domain.ufrom'),
			dataIndex: 'ufrom',
		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: "time",
			width:250
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
			width:100
		},
	];

});
</script>