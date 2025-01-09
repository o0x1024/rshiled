<template>
	<a-row justify="space-between">
		<a-col :span="12">
			<a-typography-text style="font-size: large; font-weight: 540;">{{ $t('asm.enterprise-info')
				}}</a-typography-text>
		</a-col>
		<a-col :span="12">
			<a-row justify='end'>
				<a-col :span="4">
					<a-button style="width: 95px;" type="primary" @click="onAdd">{{ $t('asm.add-enterprise')
						}}</a-button>
				</a-col>
			</a-row>
		</a-col>
	</a-row>
	<a-row style="margin-top:20px">
		<a-col :span="24">
			<a-table :columns="columns" :bordered="false" :data="enterprise.list" :pagination="pagination"
				@page-change="onPageChange" size="small">

				<template #monitor_status="{ record }">
					<a-switch v-model="record.monitor_status" @change="onSwitchChange(record)" />
				</template>

				<template #running_status="{ record }">
					<a-tag v-if="record.running_status == 1" color="green">{{ $t('asm.running') }}</a-tag>
					<a-tag v-if="record.running_status == 0" >{{ $t('asm.not_running') }}</a-tag>
				</template>


				
				<template #operation="{ record }">
					<a-space>
						<a-popconfirm content="确认删除么?" @ok="onDel(record.id)">
							<a-link  size="small" type="primary" status="danger">{{
								$t('asm.del-enterprise') }}</a-link>
						</a-popconfirm>
					</a-space>
				</template>
			</a-table>
		</a-col>
	</a-row>

	<a-modal v-model:visible="add_visible" title-align="start" @ok="handleOk" @cancel="handleCancel">
		<template #title>
			{{ $t('asm.add-enterprise-model') }}
		</template>
		<a-space direction="vertical">
			<a-input :style="{ width: '320px' }" :placeholder="$t('asm.add-enterprise-placeholder')" allow-clear />
			<a-checkbox value="1">{{ $t('asm.check-enterprise-name') }}</a-checkbox>
		</a-space>
	</a-modal>
</template>
<!-- async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  greetMsg.value = await invoke("greet", { name: name.value });
} -->

<script lang="ts" setup>
import { computed, onMounted, reactive, ref } from 'vue';
import { Pagination } from '@/types/global';
import { Enterprise } from './types';
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
import { Message } from '@arco-design/web-vue';


const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 10,
	showTotal: true,
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

const enterprise: { list: Enterprise[] } = reactive({ list: [] })
const add_visible = ref(false)
const enterprise_name = ref('')


async function RefreshData() {
	let res: any = await invoke("get_enterprise_list", { page: pagination.current, pagesize: pagination.pageSize });
	if (res) {
		enterprise.list = res
	}
}

const onAdd = () => { add_visible.value = true }


async function handleOk() {
	let res: any = await invoke("add_enterprise", { enterprise_name: enterprise_name.value });
	if (res) {
		enterprise.list = res.data
		Message.success("删除成功")
	} else {
		Message.success("删除失败")
	}
}

function handleCancel() {
	add_visible.value = false
}

onMounted(() => {
	RefreshData()
})

const onDel = async (eid: string) => {
	let res: any = await invoke("del_enterprise_by_id", { eid: eid });
	if (res) {
		enterprise.list = res.data
		Message.success("删除成功")
	} else {
		Message.success("删除失败")
	}
}

const onPageChange = (_page: number) => {
	pagination.current = _page;

};

const onSwitchChange = (value: boolean) => {
	console.log(value)

};

const columns = computed(() => {
	return [
		{
			title: 'ID',
			dataIndex: 'id',
		},
		{
			title: t('asm.enterprise-name'),
			dataIndex: 'name',
		},
		{
			title: t('asm.monitor-status'),
			dataIndex: 'monitor_status',
			slotName: "monitor_status",
		},
		{
			title: t('asm.running-status'),
			dataIndex: 'running_status',
			slotName: "running_status",

		},
		{
			title: t('asm.next-runtime'),
			dataIndex: 'next_runtime`',
			slotName: "next_runtime",
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];

});

</script>



<style lang="less" scoped></style>