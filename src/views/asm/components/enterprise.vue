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
					<a-switch v-model="record.monitor_status" :checked-value="1" :unchecked-value="0"
						@change="onSwitchChange(record.id, record.monitor_status)" />
				</template>

				<template #running_status="{ record }">
					<a-tag color="green">{{ record.running_status }}</a-tag>
				</template>


				<template #time="{ record }">
					<a-space direction="vertical" fill>
						<span color="green">运行间隔时间:{{ formatTime(record.next_run_time) }}</span>
						<span color="green">最近运行时间:{{ formatDateTime(record.last_run_time) }}</span>
					</a-space>
				</template>


				<template #operation="{ record }">
						<a-dropdown>
							<div class="clickable"><icon-more /></div>
							<template #content>
								<a-doption @click="onRun(record.id)">
									<template #icon>
										<icon-search />
									</template>
									<template #default>{{$t('asm.enterprise.run') }}</template>
								</a-doption>
								<a-doption @click="onDel(record.id)">
									<template #icon>
										<icon-delete />
									</template>
									<template #default>{{$t('asm.del-enterprise') }}</template>
								</a-doption>
							</template>
						</a-dropdown>
					<!-- <a-space>
						<a-popconfirm content="确认删除么?" @ok="onDel(record.id)">
							<a-link size="small" type="primary" status="danger">{{
								$t('asm.del-enterprise') }}</a-link>
						</a-popconfirm>
					</a-space> -->
				</template>
			</a-table>
		</a-col>
	</a-row>

	<a-modal v-model:visible="add_visible" title-align="start" @ok="handleOk" @cancel="handleCancel">
		<template #title>
			{{ $t('asm.add-enterprise-model') }}
		</template>
		<a-space direction="vertical">
			<a-input :style="{ width: '320px' }" v-model:model-value="enterprise_name"
				:placeholder="$t('asm.add-enterprise-placeholder')" allow-clear />
			<a-checkbox value="1">{{ $t('asm.check-enterprise-name') }}</a-checkbox>
		</a-space>
	</a-modal>
</template>
<!-- async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  greetMsg.value = await invoke("greet", { name: name.value });
} -->

<script lang="ts" setup>
import { formatDateTime, formatTime } from '@/utils/format';
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
	await invoke("get_enterprise_list", { page: pagination.current, pagesize: pagination.pageSize }).then((res: any) => {
		if (res) {
			enterprise.list = res.list
		}
	}).catch((err) => {
		console.log(err)
	})

}



const onAdd = () => { add_visible.value = true }


async function handleOk() {
	await invoke("add_enterprise", { enterprise_name: enterprise_name.value }).then((res: any) => {
		if (res) {
			RefreshData()
			Message.success("添加成功")
		} else {
			Message.success("添加失败")
		}
	}).catch((err) => {
		console.log(err)
	})


}

function handleCancel() {
	add_visible.value = false
}

onMounted(() => {
	RefreshData()

	startPolling();
})


function startPolling() {
	setInterval(() => {
		RefreshData() // 定期请求数据
	}, 3000); // 每 5 秒请求一次
}
const onDel = async (eid: string) => {
	await invoke("del_enterprise_by_id", { eid: eid }).then((res: any) => {
		if (res) {
			enterprise.list = res.data
			Message.success("删除成功")
		} else {
			Message.success("删除失败")
		}
	}).catch((err) => {
		console.log(err)
	})

}

const onPageChange = (_page: number) => {
	pagination.current = _page;

};

const onSwitchChange = async (eid: number, status: any) => {
	await invoke("switch_task_status", { eid: eid, status: status }).then((res: any) => {
		if (res) {
			Message.success("切换成功")
		}
	}).catch((err) => {
		Message.error("暂停失败，原因：" + err)
	})

};



const onRun = async (eid:any) =>{
	Message.info("开始扫描,如有异常,请查看日志信息")
	await invoke("run_scan", { eid: eid}).then((res: any) => {
		if (res) {
			Message.success("切换成功")
		}
	}).catch((err) => {
		Message.error("暂停失败，原因：" + err)
	})
}


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
			dataIndex: 'time`',
			slotName: "time",
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];

});

</script>



<style lang="less" scoped>
.clickable:hover {
	cursor: pointer;
}
</style>