<template>
	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" size="small"  @change="onGroupByChange" v-model:model-value="group_by">
					<a-radio value="all">{{ $t('asm.all') }}</a-radio>
					<!-- <a-radio value="ent">{{ $t('root-domain.task') }}</a-radio> -->
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="95px">
						<a-space>
							<a-button style="width: 95px;" size="small" type="primary" @click="onAddDomain">{{
								$t('asm.root-domain.add-domain') }}</a-button>
							<a-button style="width: 95px;" size="small"  type="primary" @click="onExport">{{ $t('asm.export')
								}}</a-button>

						</a-space>
					</a-col>
				</a-row>

			</a-col>
		</a-row>
		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="task_id">任务ID</a-option>
					<a-option value="task_name">任务名称</a-option>
				</a-select>
			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>
		<a-row>
			<a-col :span="24">
				<template v-if="group_by === 'all'">

					<a-table :columns="all_columns" :bordered="false" :data="rtds.list" :pagination="pagination"
						:scroll="scroll" @page-change="onPageChange" size="small">
						<template #operation="{ record }">
							<a-space>
								<a-popconfirm content="确认删除么?" @ok="onRootDomainDel(record.id)">
									<a-link size="small" type="primary" status="danger">{{
										$t('asm.del-task') }}</a-link>
								</a-popconfirm>
							</a-space>
						</template>

						<template #name="{ record }">
							<template v-if="record.name">{{ record.name }}</template>
							<template v-else>--</template>
						</template>
						<template #subdomain_count="{ record }">
							<template v-if="record.subdomain_count">{{ record.subdomain_count }}</template>
							<template v-else>--</template>
						</template>
						<template #time="{ record }">
							<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
							<template v-else>创建时间:--</template>
							<br />
							<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
							<template v-else>更新时间:--</template>
						</template>
						<template #count="{ record }">
							<a-link @click="toDomain(record.domain)">{{ record.count }}</a-link>
						</template>
					</a-table>
				</template>
				<template v-else>
					sdfsdf
					<a-table :columns="icp_columns" :bordered="false" :data="etp_domain.list" :pagination="pagination"
						@page-change="onPageChange" size="small">

						<template #count="{ record }">
							<a-tag color="green">{{ record.count }}</a-tag>
						</template>
					</a-table>
				</template>
			</a-col>
		</a-row>
	</a-space>
	<a-modal v-model:visible="add_visible" title-align="start" @ok="handleOk" @cancel="handleCancel">
		<template #title>
			{{ $t('asm.root-domain.add-asset') }}
		</template>
		<a-space direction="vertical">
			<a-select :style="{ width: '200px' }" placeholder="企业信息" v-model:model-value="task_id"
				@search="handlerSearch" allow-search @change="handlerOpsChange">
				<template v-for="item in ents.list" :key="item.id">
					<a-option :value="item.id">{{ item.name }}</a-option>
				</template>
			</a-select>
			<a-textarea :style="{ width: '320px' }" v-model:model-value="textare_domains"
				:placeholder="$t('asm.root-domain.add-asset-placeholder')" allow-clear auto-size />
		</a-space>
	</a-modal>
</template>


<script setup lang="ts">
import { Pagination } from '@/types/global';
import { computed, onMounted, reactive, ref } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import { Message } from '@arco-design/web-vue';
import { formatDateTime } from "@/utils/format"
import { Task } from './types';
import { useRouter } from 'vue-router'
import  {useRoute} from 'vue-router'

import { ETPDomain, RootDomain } from './types';
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
const router = useRouter()

defineOptions({
	name: 'asm-root-domain',
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



const ents: { list: Task[] } = reactive({ list: [] });
const rtds: { list: RootDomain[] } = reactive({ list: [] });


const route = useRoute()
const task_id = ref('')
const search_key = ref('')
const filterValue=ref('task_name')
const add_visible = ref(false)
const asset_status = ref('valid')
const etp_domain: { list: ETPDomain[] } = reactive({ list: [] })
const group_by = ref('all')
const textare_domains = ref('')

const onPageChange = (_page: number) => {
	pagination.current = _page;
	RefreshData()
};


const scroll = {
	y: 900
}

async function RefreshData() {
	await invoke("get_root_domains", { page: pagination.current, pagesize: pagination.pageSize,filter:filterValue.value ,search_key:search_key.value}).then((res: any) => {
		if (res) {
			rtds.list = res.list
			pagination.total = res.total
		}
	}).catch((error) => console.error(error))
}



const onGroupByChange = async (value: string | number | boolean) => {
	console.log(value)

	switch (value) {
		case 'all':
			let res: any = await invoke("get_root_domains", { page: pagination.current, pagesize: pagination.pageSize,filter:filterValue.value ,search_key:search_key.value});
			if (res) {
				rtds.list = res
			}
			break
		case 'ent':
			let res2: any = await invoke("get_ent_domain", { page: pagination.current, pagesize: pagination.pageSize,filter:filterValue.value ,search_key:search_key.value});
			if (res2) {
				etp_domain.list = res2
			}
			break
	}
}

onMounted(() => {
	if(route.query.task_name && route.query.task_name  !== undefined){
		search_key.value = route.query.task_name as string | ""
	}else{
		search_key.value = ''
	}
	
	RefreshData()
})

const onRootDomainDel = async (did: string) => {
	await invoke("del_rootdomain_by_id", { did: did }).then((res: any) => {
		RefreshData()
		Message.info(res);
	});

}

const handlerOpsChange = (value: string | number | boolean | Record<string, any> | (string | number | boolean | Record<string, any>)[]) => {
	task_id.value = value as string
}


const handlerSearch = async () => {
	await invoke("get_task_list").then((res: any) => {
		if (res) {
			ents.list = res.list;
		}
	});

}


const onExport = async () => {
	await invoke("export_icpdomain", {}).then((res: any) => {
		if (res) {
			Message.success(res)
		}
	});
}

const toDomain = async (domain: string) => {
	router.push({
		name: 'asm-domain', query: { domain: domain }
	})
}

const onAddDomain = async () => {
	add_visible.value = true
	await invoke("get_task_list", { page: pagination.current, pagesize: pagination.pageSize }).then((res: any) => {
		if (res) {
			ents.list = res.list
		}
	}).catch(error => console.error(error))


}

async function handleOk() {
	const dataArray = textare_domains.value.split('\n');
	const eid = Number(task_id.value)
	await invoke("add_root_domain", { task_id: eid, root_domain: dataArray }).then((res: any) => {
		if (res) {
			Message.success(res)
		}
		RefreshData()
	});
}

function handleCancel() {
	add_visible.value = false
}

const all_columns = computed(() => {
	return [

		{
			title: t('asm.root-domain.root-domain'),
			dataIndex: 'domain',
		},
		{
			title: t('asm.root-domain.task_name'),
			dataIndex: 'task_name',
			slotName: 'task_name',

		},
		{
			title: t('asm.count'),
			dataIndex: 'count',
			slotName: 'count',
		},
		{
			title: t('asm.time'),
			dataIndex: 'time',
			slotName: 'time',
			width: 250
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];

});

const icp_columns = computed(() => {
	return [
		{
			title: t('asm.root-domain.task-name'),
			dataIndex: 'task_name',
		},
		{
			title: t('asm.root-domain.domain-count'),
			dataIndex: 'count',
			slotName: "count",
		},
	];

});


</script>


<style lang="less" scoped></style>