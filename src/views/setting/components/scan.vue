<template>
	<a-tabs default-active-key="1" lazy-load destroy-on-hide >
		<a-tab-pane key="1">
			<template #title>
				<icon-calendar /> 正则配置
			</template>
			<a-row justify='start'>
				<a-col flex="95px">
					<a-button style="width: 95px;" size="small" type="primary" @click="onAdd">{{ $t('setting.regex.add')
					}}</a-button>
				</a-col>
			</a-row>
			<a-table :columns="regex_column" :bordered="false" :data="regexs.list" :pagination="pagination"
				@page-change="onPageChange" size="small" style="margin-top: 10px;">

				<template #status="{ record }">
					<a-switch v-model="record.status" :checked-value="1" :unchecked-value="0"
						@change="onSwitchChange(record.id, record.status)" />
				</template>
				<template #time="{ record }">
					<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
					<template v-else>创建时间:--</template>
					<br />
					<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
					<template v-else>更新时间:--</template>
				</template>


				<template #operation="{ record }">
					<a-dropdown>
						<div class="clickable"><icon-more /></div>
						<template #content>
							<a-doption @click="onEdit(record)">
								<template #icon>
									<icon-search />
								</template>
								<template #default>{{ $t('setting.regex.update') }}</template>
							</a-doption>
							<a-doption @click="onDel(record.id)">
								<template #icon>
									<icon-delete />
								</template>
								<template #default>{{ $t('setting.regex.del') }}</template>
							</a-doption>
						</template>
					</a-dropdown>
				</template>
			</a-table>
		</a-tab-pane>
		<a-tab-pane key="2">
			<template #title>
				<icon-clock-circle /> 字典配置
			</template>
			Content of Tab Panel 2
		</a-tab-pane>

	</a-tabs>

	<a-modal v-model:visible="add_visible" title-align="start" @ok="handleOk" @cancel="handleCancel">
		<template #title>
			{{ $t('setting.regex.add') }}
		</template>
		<a-space direction="vertical">
			<a-space>
				<span>正则名称</span>
				<a-input :style="{ width: '320px' }" v-model:model-value="cregex.name"
					:placeholder="$t('asm.add-task-placeholder')" allow-clear />
			</a-space>
			<a-space>
				<span>正则规则</span>
				<a-input :style="{ width: '320px' }" v-model:model-value="cregex.regex"
					:placeholder="$t('asm.add-task-placeholder')" allow-clear />
			</a-space>
			<!-- <a-space>
				<span>正则描述</span>
				<a-input :style="{ width: '320px' }" v-model:model-value="cregex.desc" :placeholder="$t('asm.add-task-placeholder')" allow-clear />
			</a-space> -->
			<a-space>
				<span>正则类型</span>
				<a-select v-model="cregex.rtype" style="width: 320px;">
					<a-option value="domain">子域名</a-option>
					<a-option value="email">邮箱</a-option>
					<a-option value="ip">IP</a-option>
					<a-option value="url">URL</a-option>
					<a-option value="RISK">安全风险</a-option>
				</a-select>
			</a-space>
		</a-space>
	</a-modal>
</template>



<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatDateTime } from '@/utils/format';
import { CRegex } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';
import { Pagination } from '@/types/global';

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

let add_visible = ref(false)
let isupdate = ref(false)



let cregex: CRegex = reactive({})

const regexs: { list: CRegex[] } = reactive({ list: [] })
let rdtype = ref('')
let search_key = ref('')

const onPageChange = (_page: number) => {
	pagination.current = _page;

};

async function handleOk() {
	console.log(isupdate.value)

	if (isupdate.value) {
		await invoke("update_regex", { id: cregex.id, status: cregex.status, name: cregex.name, regex: cregex.regex, rtype: cregex.rtype }).then((res: any) => {
			if (res) {
				Message.success(res)
			}
			RefreshData()
		});
	} else {
		await invoke("add_regex", { name: cregex.name, regex: cregex.regex, rtype: cregex.rtype }).then((res: any) => {
			if (res) {
				Message.success(res)
			}
			RefreshData()
		});
	}

}

const onAdd = () => {
	Object.assign(cregex, {})
	isupdate.value = false
	add_visible.value = true
	console.log('onAdd:', isupdate.value)

}

function handleCancel() {
	add_visible.value = false
}


async function RefreshData() {
	await invoke("get_regexs", { page: pagination.current, pagesize: pagination.pageSize, dtype: rdtype.value, query: search_key.value }).then((res: any) => {
		if (res) {
			regexs.list = res.list
			pagination.total = res.total
		}
	}).catch((err: any) => {
		console.log(err);
	})
}


onMounted(() => {
	// 初始化时设置 selectedKeys
	RefreshData()

})

const onEdit = async (_cregex: any) => {
	Object.assign(cregex, _cregex)
	add_visible.value = true
	isupdate.value = true
}

const onDel = async (cid: any) => {
	await invoke("del_regex_by_id", { cid: cid }).then((res: any) => {
		if (res) {
			Message.success(res)
		}
	}).catch((err) => {
		Message.error("失败，原因：" + err)
	})

	RefreshData()
}


const onSwitchChange = async (cid: number, status: any) => {
	await invoke("switch_regex_status", { cid: cid, status: status }).then((res: any) => {
		if (res) {
			Message.success("切换成功")
		}
	}).catch((err) => {
		Message.error("暂停失败，原因：" + err)
	})

};

const regex_column = computed(() => {
	return [
		{
			title: t('setting.scan.regex_name'),
			dataIndex: 'name',
		},
		{
			title: t('setting.scan.regex_content'),
			dataIndex: 'regex',
		},
		{
			title: t('setting.scan.regex_status'),  //API返回的长度
			dataIndex: 'status',
			slotName: 'status',
		},
		{
			title: t('setting.scan.regex_rtype'),  //API返回的长度
			dataIndex: 'rtype',
			slotName: 'rtype',
		},
		{
			title: t('setting.scan.time'),
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


</script>